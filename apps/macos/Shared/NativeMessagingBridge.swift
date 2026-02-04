//
//  NativeMessagingBridge.swift
//  Eurora
//
//  Bridge between Safari extension and euro-native-messaging binary.
//  Manages the subprocess lifecycle and handles the native messaging protocol.
//
//  This file is shared between the container app and the Safari extension.
//

import Foundation
import os.log

/// Singleton bridge that manages communication with euro-native-messaging
@available(macOS 11.0, *)
class NativeMessagingBridge {

    static let shared = NativeMessagingBridge()

    private var process: Process?
    private var stdinPipe: Pipe?
    private var stdoutPipe: Pipe?
    private var stderrPipe: Pipe?

    private let queue = DispatchQueue(label: "com.eurora.native-messaging-bridge", qos: .userInitiated)
    private let responseLock = NSLock()
    private var pendingCallbacks: [(Data) -> Void] = []

    private let logger = Logger(subsystem: "com.eurora.macos", category: "NativeMessagingBridge")

    private init() {}

    /// Start the native messaging host process
    func start() {
        queue.async { [weak self] in
            self?.startProcess()
        }
    }

    /// Stop the native messaging host process
    func stop() {
        queue.async { [weak self] in
            self?.stopProcess()
        }
    }

    /// Send a message to the native messaging host and wait for response
    func sendMessage(_ message: [String: Any], completion: @escaping (Result<[String: Any], Error>) -> Void) {
        queue.async { [weak self] in
            guard let self = self else {
                completion(.failure(BridgeError.bridgeDeallocated))
                return
            }

            guard let stdinPipe = self.stdinPipe, self.process?.isRunning == true else {
                self.logger.error("Native messaging host not running, attempting restart...")
                self.startProcess()

                // Retry after short delay
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                    self.sendMessage(message, completion: completion)
                }
                return
            }

            do {
                // Serialize to JSON - pass through as-is, no wrapping
                let jsonData = try JSONSerialization.data(withJSONObject: message, options: [])

                // Register callback for response
                self.responseLock.lock()
                self.pendingCallbacks.append { responseData in
                    do {
                        if let response = try JSONSerialization.jsonObject(with: responseData, options: []) as? [String: Any] {
                            completion(.success(response))
                        } else {
                            completion(.failure(BridgeError.invalidResponse))
                        }
                    } catch {
                        completion(.failure(error))
                    }
                }
                self.responseLock.unlock()

                // Write length prefix (4 bytes, little-endian)
                var length = UInt32(jsonData.count).littleEndian
                let lengthData = Data(bytes: &length, count: 4)

                // Write to stdin
                let fileHandle = stdinPipe.fileHandleForWriting
                fileHandle.write(lengthData)
                fileHandle.write(jsonData)

                self.logger.debug("Sent message to native host: \(jsonData.count) bytes")

            } catch {
                self.logger.error("Failed to send message: \(error.localizedDescription)")
                completion(.failure(error))
            }
        }
    }

    /// Send a message synchronously (blocks until response)
    func sendMessageSync(_ message: [String: Any], timeout: TimeInterval = 30.0) -> Result<[String: Any], Error> {
        let semaphore = DispatchSemaphore(value: 0)
        var result: Result<[String: Any], Error> = .failure(BridgeError.timeout)

        sendMessage(message) { response in
            result = response
            semaphore.signal()
        }

        let waitResult = semaphore.wait(timeout: .now() + timeout)
        if waitResult == .timedOut {
            return .failure(BridgeError.timeout)
        }

        return result
    }

    // MARK: - Private Methods

    private func startProcess() {
        guard process == nil || process?.isRunning == false else {
            logger.debug("Process already running")
            return
        }

        let process = Process()
        let stdinPipe = Pipe()
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()

        process.standardInput = stdinPipe
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        // Find the euro-native-messaging binary
        // Check common installation paths in order of preference
        let possiblePaths = [
            // Installed Eurora desktop app location
            "/Applications/Eurora.app/Contents/MacOS/euro-native-messaging",
            // Fallback paths for development
            "/usr/local/bin/euro-native-messaging",
            "/opt/homebrew/bin/euro-native-messaging",
            NSHomeDirectory() + "/.local/bin/euro-native-messaging",
            // Development paths - relative to the project
            Bundle.main.bundlePath + "/../../../../../target/release/euro-native-messaging",
            Bundle.main.bundlePath + "/../../../../../target/debug/euro-native-messaging"
        ]

        var foundPath: String?
        for path in possiblePaths {
            if FileManager.default.isExecutableFile(atPath: path) {
                foundPath = path
                break
            }
        }

        guard let executablePath = foundPath else {
            logger.error("euro-native-messaging binary not found. Please ensure Eurora desktop app is installed.")
            return
        }

        process.executableURL = URL(fileURLWithPath: executablePath)
        logger.info("Using euro-native-messaging at: \(executablePath)")

        // Set up stdout reading
        stdoutPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            self?.handleStdoutData(handle)
        }

        // Set up stderr reading for logging
        stderrPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if !data.isEmpty, let str = String(data: data, encoding: .utf8) {
                self?.logger.warning("Native host stderr: \(str)")
            }
        }

        // Handle process termination
        process.terminationHandler = { [weak self] proc in
            self?.logger.info("Native messaging host terminated with code: \(proc.terminationStatus)")
            self?.queue.asyncAfter(deadline: .now() + 2.0) {
                self?.startProcess() // Auto-restart
            }
        }

        do {
            try process.run()
            logger.info("Started euro-native-messaging process (PID: \(process.processIdentifier))")

            self.process = process
            self.stdinPipe = stdinPipe
            self.stdoutPipe = stdoutPipe
            self.stderrPipe = stderrPipe

        } catch {
            logger.error("Failed to start native messaging host: \(error.localizedDescription)")
        }
    }

    private func stopProcess() {
        stdoutPipe?.fileHandleForReading.readabilityHandler = nil
        stderrPipe?.fileHandleForReading.readabilityHandler = nil

        if process?.isRunning == true {
            process?.terminate()
        }

        process = nil
        stdinPipe = nil
        stdoutPipe = nil
        stderrPipe = nil

        // Cancel all pending requests
        responseLock.lock()
        pendingCallbacks.removeAll()
        responseLock.unlock()

        logger.info("Native messaging host stopped")
    }

    private var readBuffer = Data()

    private func handleStdoutData(_ handle: FileHandle) {
        let data = handle.availableData
        guard !data.isEmpty else { return }

        readBuffer.append(data)

        // Try to parse complete frames from buffer
        while readBuffer.count >= 4 {
            // Read length prefix (4 bytes, little-endian)
            let lengthData = readBuffer.prefix(4)
            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }

            let totalLength = 4 + Int(length)
            guard readBuffer.count >= totalLength else {
                // Not enough data yet
                break
            }

            // Extract the JSON payload
            let jsonData = readBuffer.subdata(in: 4..<totalLength)
            readBuffer.removeFirst(totalLength)

            // Call the next pending callback
            responseLock.lock()
            let callback = pendingCallbacks.isEmpty ? nil : pendingCallbacks.removeFirst()
            responseLock.unlock()

            callback?(jsonData)
        }
    }
}

// MARK: - Error Types

enum BridgeError: Error, LocalizedError {
    case bridgeDeallocated
    case processNotRunning
    case processStopped
    case timeout
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .bridgeDeallocated:
            return "Native messaging bridge was deallocated"
        case .processNotRunning:
            return "Native messaging host is not running"
        case .processStopped:
            return "Native messaging host was stopped"
        case .timeout:
            return "Request timed out"
        case .invalidResponse:
            return "Invalid response from native messaging host"
        }
    }
}
