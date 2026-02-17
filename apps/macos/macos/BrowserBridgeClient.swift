import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2
import os.log

private let kBrowserBridgePort: Int = 1431
private let kRetryIntervalSecs: TimeInterval = 2.0

@available(macOS 15.0, *)
protocol BrowserBridgeClientDelegate: AnyObject {
    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient)
    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?)
    func browserBridgeClient(
        _ client: BrowserBridgeClient, didReceiveFrame frame: BrowserBridge_Frame)
}

@available(macOS 15.0, *)
final class BrowserBridgeClient: @unchecked Sendable {

    // MARK: - Properties

    weak var delegate: BrowserBridgeClientDelegate?

    private let logger = Logger(subsystem: "com.eurora.macos", category: "BrowserBridgeClient")

    private let hostPid: UInt32
    private var browserPid: UInt32

    private var connectionTask: Task<Void, Never>?
    private var shouldReconnect = true
    private let lock = NSLock()

    private var outboundContinuation: AsyncStream<BrowserBridge_Frame>.Continuation?

    var isConnected: Bool {
        lock.lock()
        defer { lock.unlock() }
        return outboundContinuation != nil
    }

    // MARK: - Initialization

    init(hostPid: UInt32, browserPid: UInt32) {
        self.hostPid = hostPid
        self.browserPid = browserPid
    }

    deinit {
        disconnect()
    }

    // MARK: - Public API

    func connect() {
        shouldReconnect = true
        connectionTask = Task { [weak self] in
            await self?.connectLoop()
        }
    }

    func disconnect() {
        shouldReconnect = false

        lock.lock()
        let continuation = outboundContinuation
        outboundContinuation = nil
        lock.unlock()

        // Finish the outbound stream, which causes the producer to return,
        // which closes the request stream (sends END_STREAM).
        continuation?.finish()

        connectionTask?.cancel()
        connectionTask = nil
    }

    func updateBrowserPid(_ newPid: UInt32) {
        lock.lock()
        browserPid = newPid
        let continuation = outboundContinuation
        lock.unlock()

        logger.info("Browser PID updated to \(newPid)")

        // Re-register on the existing stream so the server sees the new PID
        if let continuation {
            let regFrame = buildRegistrationFrame(browserPid: newPid)
            continuation.yield(regFrame)
        }
    }

    func send(frame: BrowserBridge_Frame) {
        lock.lock()
        let continuation = outboundContinuation
        lock.unlock()

        guard let continuation else {
            logger.warning("Cannot send frame — not connected")
            return
        }

        continuation.yield(frame)
    }

    // MARK: - Connection Loop

    private func connectLoop() async {
        while shouldReconnect && !Task.isCancelled {
            logger.info("Attempting to connect to gRPC server at [::1]:\(kBrowserBridgePort)")

            do {
                try await runOneConnection()
            } catch is CancellationError {
                logger.info("Connection cancelled")
            } catch {
                if !Task.isCancelled {
                    logger.error("gRPC connection error: \(error.localizedDescription)")
                }
            }

            lock.lock()
            let continuation = outboundContinuation
            outboundContinuation = nil
            lock.unlock()
            continuation?.finish()

            if !Task.isCancelled {
                await MainActor.run { [weak self] in
                    guard let self else { return }
                    self.delegate?.browserBridgeClientDidDisconnect(self, error: nil)
                }
            }

            guard shouldReconnect && !Task.isCancelled else { break }

            logger.info("Reconnecting in \(kRetryIntervalSecs) seconds...")
            try? await Task.sleep(for: .seconds(kRetryIntervalSecs))
        }
    }

    private func buildRegistrationFrame(browserPid: UInt32) -> BrowserBridge_Frame {
        var registerFrame = BrowserBridge_RegisterFrame()
        registerFrame.hostPid = self.hostPid
        registerFrame.browserPid = browserPid
        var frame = BrowserBridge_Frame()
        frame.register = registerFrame
        return frame
    }

    private func makeOutboundProducer(
        _ outboundStream: AsyncStream<BrowserBridge_Frame>
    ) -> @Sendable (RPCWriter<BrowserBridge_Frame>) async throws -> Void {
        return { writer in
            self.logger.debug("Producer started, forwarding outbound frames...")
            do {
                for await frame in outboundStream {
                    try await writer.write(frame)
                }
                self.logger.debug("Producer: stream ended normally")
            } catch is CancellationError {
                self.logger.debug("Producer cancelled")
            } catch {
                self.logger.error("Producer error: \(error.localizedDescription)")
            }
        }
    }

    private func processInboundFrames(
        _ response: StreamingClientResponse<BrowserBridge_Frame>
    ) async {
        await MainActor.run { self.delegate?.browserBridgeClientDidConnect(self) }
        do {
            var frameCount = 0
            for try await receivedFrame in response.messages {
                frameCount += 1
                self.logger.debug("Received frame #\(frameCount) from server")
                await MainActor.run {
                    self.delegate?.browserBridgeClient(self, didReceiveFrame: receivedFrame)
                }
            }
            self.logger.info("Server stream ended after \(frameCount) frames")
        } catch { self.logger.error("Error receiving from server: \(error)") }
    }

    private func runOneConnection() async throws {
        let transport = try HTTP2ClientTransport.Posix(
            target: .ipv6(address: "::1", port: kBrowserBridgePort),
            transportSecurity: .plaintext
        )

        try await withGRPCClient(transport: transport) { grpcClient in
            let bridgeClient = BrowserBridge_BrowserBridge.Client(wrapping: grpcClient)

            self.lock.lock()
            let currentBrowserPid = self.browserPid
            self.lock.unlock()

            let regFrame = self.buildRegistrationFrame(browserPid: currentBrowserPid)
            self.logger.info(
                "Sending registration: host=\(self.hostPid), browser=\(currentBrowserPid)")

            let (outboundStream, continuation) = AsyncStream.makeStream(
                of: BrowserBridge_Frame.self)
            self.lock.lock()
            self.outboundContinuation = continuation
            self.lock.unlock()
            continuation.yield(regFrame)

            let request = StreamingClientRequest<BrowserBridge_Frame>(
                metadata: [:], producer: self.makeOutboundProducer(outboundStream)
            )

            try await bridgeClient.open(request: request) { [self] response in
                self.logger.info("Bidirectional stream opened successfully")
                await self.processInboundFrames(response)
            }
            self.logger.info("RPC completed, connection will be retried")
        }
    }
}
