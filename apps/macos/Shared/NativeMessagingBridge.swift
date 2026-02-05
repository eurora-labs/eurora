//
//  NativeMessagingBridge.swift
//  Eurora
//
//  Bridge between Safari extension and euro-native-messaging binary.
//  Manages the subprocess lifecycle and handles the native messaging protocol.
//

import Foundation
import os.log
import AppKit

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

    /// Send a message to the native messaging host and wait for response with timeout
    func sendMessage(_ message: [String: Any], timeout: TimeInterval = 10.0, completion: @escaping (Result<[String: Any], Error>) -> Void) {
        queue.async { [weak self] in
            guard let self = self else {
                completion(.failure(BridgeError.bridgeDeallocated))
                return
            }

            guard let stdinPipe = self.stdinPipe, self.process?.isRunning == true else {
                self.logger.error("Native messaging host not running, attempting restart...")
                self.startProcess()

                // Retry after short delay (only once)
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                    // Check again after restart attempt
                    if self.process?.isRunning == true {
                        self.sendMessage(message, timeout: timeout, completion: completion)
                    } else {
                        completion(.failure(BridgeError.processNotRunning))
                    }
                }
                return
            }

            do {
                // Serialize to JSON - pass through as-is, no wrapping
                let jsonData = try JSONSerialization.data(withJSONObject: message, options: [])
                
                // Create a unique callback ID for timeout tracking
                let callbackId = UUID()
                var callbackFired = false
                let callbackLock = NSLock()

                // Register callback for response
                self.responseLock.lock()
                self.pendingCallbacks.append { responseData in
                    callbackLock.lock()
                    guard !callbackFired else {
                        callbackLock.unlock()
                        return
                    }
                    callbackFired = true
                    callbackLock.unlock()
                    
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
                
                // Set up timeout
                DispatchQueue.main.asyncAfter(deadline: .now() + timeout) {
                    callbackLock.lock()
                    guard !callbackFired else {
                        callbackLock.unlock()
                        return
                    }
                    callbackFired = true
                    callbackLock.unlock()
                    
                    self.logger.warning("Request timed out after \(timeout) seconds")
                    completion(.failure(BridgeError.timeout))
                }

                // Write length prefix (4 bytes, little-endian)
                var length = UInt32(jsonData.count).littleEndian
                let lengthData = Data(bytes: &length, count: 4)

                // Write to stdin
                let fileHandle = stdinPipe.fileHandleForWriting
                fileHandle.write(lengthData)
                fileHandle.write(jsonData)

                self.logger.debug("Sent message to native host: \(jsonData.count) bytes (callbackId: \(callbackId))")

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
        // Development paths are checked first for convenience during development
        
        // Get project root from source file location (works during development)
        // #file gives path like: /path/to/eurora/apps/macos/Shared/NativeMessagingBridge.swift
        let sourceFileURL = URL(fileURLWithPath: #file)
        let projectRoot = sourceFileURL
            .deletingLastPathComponent()  // Remove NativeMessagingBridge.swift
            .deletingLastPathComponent()  // Remove Shared
            .deletingLastPathComponent()  // Remove macos
            .deletingLastPathComponent()  // Remove apps
            .path
        
        logger.info("Source file: \(#file)")
        logger.info("Project root: \(projectRoot)")
        
        let possiblePaths = [
            // Development paths - relative to project root (derived from source file)
            "\(projectRoot)/target/debug/euro-native-messaging",
            "\(projectRoot)/target/release/euro-native-messaging",
            // Installed Eurora desktop app location
            "/Applications/Eurora.app/Contents/MacOS/euro-native-messaging",
            // Other fallback paths
            "/usr/local/bin/euro-native-messaging",
            "/opt/homebrew/bin/euro-native-messaging",
            NSHomeDirectory() + "/.local/bin/euro-native-messaging"
        ]

        var foundPath: String?
        for path in possiblePaths {
            let exists = FileManager.default.isExecutableFile(atPath: path)
            logger.debug("Checking path: \(path) - exists: \(exists)")
            if exists && foundPath == nil {
                foundPath = path
            }
        }

        guard let executablePath = foundPath else {
            logger.error("euro-native-messaging binary not found. Please ensure Eurora desktop app is installed.")
            return
        }

        process.executableURL = URL(fileURLWithPath: executablePath)
        logger.info("Using euro-native-messaging at: \(executablePath)")
        
        // Find Safari's PID and pass it via environment variable
        // euro-native-messaging uses parent_id() which would return this Swift app's PID
        // We need to provide the actual Safari PID for proper browser tracking
        var environment = ProcessInfo.processInfo.environment
        if let safariPid = findSafariPid() {
            environment["EURORA_BROWSER_PID"] = String(safariPid)
            logger.info("Found Safari PID: \(safariPid)")
        } else {
            logger.warning("Could not find Safari PID")
        }
        process.environment = environment

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

    /// Find Safari's PID by looking for running Safari processes
    private func findSafariPid() -> pid_t? {
        // Use NSRunningApplication to find Safari
        let workspace = NSWorkspace.shared
        let runningApps = workspace.runningApplications
        
        // Look for Safari by bundle identifier
        for app in runningApps {
            if app.bundleIdentifier == "com.apple.Safari" {
                return app.processIdentifier
            }
        }
        
        // Fallback: look for Safari Technology Preview
        for app in runningApps {
            if app.bundleIdentifier == "com.apple.SafariTechnologyPreview" {
                return app.processIdentifier
            }
        }
        
        return nil
    }
    
    private func handleStdoutData(_ handle: FileHandle) {
        // Read data on the callback thread
        let data = handle.availableData
        
        // Process on our queue for thread safety
        queue.async { [weak self] in
            guard let self = self else { return }
            guard !data.isEmpty else { return }
            
            self.readBuffer.append(data)
            self.processReadBuffer()
        }
    }
    
    private func processReadBuffer() {
        // Must be called on self.queue
        
        // Try to parse complete frames from buffer
        while readBuffer.count >= 4 {
            // Convert to Array for safe indexing (Data indices can be non-zero after removeFirst)
            let headerBytes = [UInt8](readBuffer.prefix(4))
            
            // Read length prefix (4 bytes, little-endian)
            let length = UInt32(headerBytes[0]) |
                        (UInt32(headerBytes[1]) << 8) |
                        (UInt32(headerBytes[2]) << 16) |
                        (UInt32(headerBytes[3]) << 24)
            
            // Sanity check: max 8MB frame size
            let maxFrameSize: UInt32 = 8 * 1024 * 1024
            guard length > 0 && length <= maxFrameSize else {
                logger.error("Invalid frame length: \(length), clearing buffer")
                readBuffer.removeAll()
                return
            }
            
            let totalLength = 4 + Int(length)
            guard readBuffer.count >= totalLength else {
                // Not enough data yet
                break
            }
            
            // Extract the JSON payload - copy to new Data to avoid index issues
            let jsonBytes = [UInt8](readBuffer.prefix(totalLength).dropFirst(4))
            let jsonData = Data(jsonBytes)
            
            // Remove processed data from buffer - reset to clean Data to fix indices
            let remaining = [UInt8](readBuffer.dropFirst(totalLength))
            readBuffer = Data(remaining)
            
            logger.debug("Received frame: \(jsonData.count) bytes")
            
            // Handle the received frame
            handleReceivedFrame(jsonData)
        }
    }
    
    private func handleReceivedFrame(_ jsonData: Data) {
        // All frames from euro-native-messaging are passed to pending callbacks
        // This handles both responses to our requests and unsolicited requests from the host
        responseLock.lock()
        let callback = pendingCallbacks.isEmpty ? nil : pendingCallbacks.removeFirst()
        responseLock.unlock()
        
        if let callback = callback {
            DispatchQueue.main.async {
                callback(jsonData)
            }
        } else {
            if let jsonString = String(data: jsonData, encoding: .utf8) {
                logger.debug("Received frame with no pending callback: \(jsonString.prefix(200))")
            }
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
