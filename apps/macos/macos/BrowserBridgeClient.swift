//
//  BrowserBridgeClient.swift
//  Eurora
//
//  gRPC client for connecting to the euro-activity browser bridge service.
//  Implements bidirectional streaming for Frame messages using grpc-swift 2.x.
//
//  This is the Swift equivalent of crates/app/euro-native-messaging/src/main.rs.
//  It connects to the gRPC server, sends a registration frame, and then
//  forwards Frame messages bidirectionally.
//
//  Architecture (matching the Rust code):
//    - connect_with_retry → connectLoop() with retry
//    - async_stream::stream! { yield reg; loop { recv → yield } } → AsyncStream + producer closure
//    - client.open(outbound_stream) → bridgeClient.open(request:onResponse:)
//    - inbound_stream.message() loop → for try await frame in response.messages
//

import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2
import os.log

/// Port for the browser bridge gRPC server (running in euro-activity)
/// Matches euro-native-messaging PORT constant and BROWSER_BRIDGE_PORT in server.rs
private let kBrowserBridgePort: Int = 1431

/// Retry interval for connecting to the server (in seconds)
/// Matches RETRY_INTERVAL_SECS in the Rust code
private let kRetryIntervalSecs: TimeInterval = 2.0

/// Protocol for receiving events from the gRPC client
@available(macOS 15.0, *)
protocol BrowserBridgeClientDelegate: AnyObject {
    /// Called when the gRPC connection is established and stream is open
    func browserBridgeClientDidConnect(_ client: BrowserBridgeClient)

    /// Called when the gRPC connection is lost
    func browserBridgeClientDidDisconnect(_ client: BrowserBridgeClient, error: Error?)

    /// Called when a frame is received from the gRPC server
    func browserBridgeClient(
        _ client: BrowserBridgeClient, didReceiveFrame frame: BrowserBridge_Frame)
}

/// gRPC client for the BrowserBridge service.
///
/// Mirrors the architecture of `euro-native-messaging/src/main.rs`:
/// - Connects to gRPC server with retry
/// - Opens a bidirectional `Open` stream
/// - Sends a `RegisterFrame` first
/// - Forwards frames in both directions via delegate + `send(frame:)`
///
/// Uses `AsyncStream` for the outbound producer, matching the Rust code's
/// `async_stream::stream!` pattern. Uses `withGRPCClient` for correct
/// gRPC client lifecycle management.
@available(macOS 15.0, *)
final class BrowserBridgeClient: @unchecked Sendable {

    // MARK: - Properties

    weak var delegate: BrowserBridgeClientDelegate?

    private let logger = Logger(subsystem: "com.eurora.macos", category: "BrowserBridgeClient")

    private let hostPid: UInt32
    private var browserPid: UInt32

    /// Task running the connect-with-retry loop
    private var connectionTask: Task<Void, Never>?

    /// Whether reconnection should continue after disconnection
    private var shouldReconnect = true

    /// Lock protecting mutable state
    private let lock = NSLock()

    /// The continuation for the outbound AsyncStream.
    /// Yielding a frame to this continuation sends it through the gRPC stream.
    /// This is the Swift equivalent of the Rust `broadcast::Sender<Frame>`.
    private var outboundContinuation: AsyncStream<BrowserBridge_Frame>.Continuation?

    /// Whether the client currently has an active gRPC stream
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

    /// Start the client and begin connecting to the gRPC server.
    /// Mirrors the `server_connection_handle` task in the Rust code.
    func connect() {
        shouldReconnect = true
        connectionTask = Task { [weak self] in
            await self?.connectLoop()
        }
    }

    /// Disconnect from the server and stop reconnecting.
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

    /// Update the browser PID at runtime (e.g. when Safari launches or quits).
    ///
    /// If the client is currently connected, a fresh registration frame is sent
    /// immediately so the gRPC server picks up the new PID without waiting for
    /// the next reconnection cycle.
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

    /// Send a frame to the gRPC server.
    /// Equivalent to `to_server_tx.send(frame)` in the Rust code.
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

    /// Main connection loop with retry logic.
    /// Mirrors the outer `loop { ... }` in the Rust `server_connection_handle` task.
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

            // Clean up outbound continuation after stream ends
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

            // Wait before reconnecting (like Rust's RETRY_INTERVAL_SECS)
            guard shouldReconnect && !Task.isCancelled else { break }

            logger.info("Reconnecting in \(kRetryIntervalSecs) seconds...")
            try? await Task.sleep(for: .seconds(kRetryIntervalSecs))
        }
    }

    /// Build the registration frame to send on connect.
    /// The caller must pass the current `browserPid` captured under `lock`
    /// so the frame is constructed without a data race.
    private func buildRegistrationFrame(browserPid: UInt32) -> BrowserBridge_Frame {
        var registerFrame = BrowserBridge_RegisterFrame()
        registerFrame.hostPid = self.hostPid
        registerFrame.browserPid = browserPid
        var frame = BrowserBridge_Frame()
        frame.register = registerFrame
        return frame
    }

    /// Create the outbound producer for streaming
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

    /// Process inbound frames from the server
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

    /// Run a single gRPC connection using the canonical `withGRPCClient` pattern.
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
