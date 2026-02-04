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
import AppKit
import CoreGraphics

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

    // Store the current tab metadata from extension messages
    private var currentTabURL: String?
    private var currentTabTitle: String?
    private var currentTabIconBase64: String?
    private let metadataLock = NSLock()
    
    private init() {}
    
    /// Update the current tab metadata (called when extension sends a message with URL/icon info)
    func updateCurrentTab(url: String?, title: String?, iconBase64: String?) {
        metadataLock.lock()
        if let url = url {
            currentTabURL = url
        }
        if let title = title {
            currentTabTitle = title
        }
        if let iconBase64 = iconBase64 {
            currentTabIconBase64 = iconBase64
        }
        metadataLock.unlock()
        logger.debug("Updated current tab - URL: \(url ?? "nil"), Title: \(title ?? "nil"), hasIcon: \(iconBase64 != nil)")
    }

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
        // Parse the frame to check if it's a Request or Response
        guard let json = try? JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any],
              let kind = json["kind"] as? [String: Any] else {
            logger.error("Failed to parse frame JSON")
            return
        }
        
        if let request = kind["Request"] as? [String: Any] {
            // This is a Request from the native host - handle it
            handleIncomingRequest(request, fullFrame: json)
        } else if kind["Response"] != nil || kind["Event"] != nil || kind["Error"] != nil {
            // This is a Response/Event/Error - pass to pending callback
            responseLock.lock()
            let callback = pendingCallbacks.isEmpty ? nil : pendingCallbacks.removeFirst()
            responseLock.unlock()
            
            if let callback = callback {
                DispatchQueue.main.async {
                    callback(jsonData)
                }
            } else {
                if let jsonString = String(data: jsonData, encoding: .utf8) {
                    logger.debug("Received response with no pending callback: \(jsonString.prefix(200))")
                }
            }
        } else {
            logger.warning("Unknown frame kind: \(kind.keys)")
        }
    }
    
    private func handleIncomingRequest(_ request: [String: Any], fullFrame: [String: Any]) {
        // Handle requests from the native messaging host
        guard let action = request["action"] as? String,
              let requestId = request["id"] else {
            logger.error("Invalid request format")
            return
        }
        
        logger.debug("Handling request: action=\(action), id=\(requestId as! NSObject)")
        
        switch action {
        case "GET_METADATA":
            handleGetMetadata(requestId: requestId, action: action)
        default:
            // Send empty response for unknown actions
            let response: [String: Any] = [
                "kind": [
                    "Response": [
                        "id": requestId,
                        "action": action,
                        "payload": NSNull()
                    ]
                ]
            ]
            sendRawFrame(response)
        }
    }
    
    private func handleGetMetadata(requestId: Any, action: String) {
        // Get the current tab metadata
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            
            // Get URL and icon from stored metadata (set by extension messages)
            self.metadataLock.lock()
            let url = self.currentTabURL
            let storedIcon = self.currentTabIconBase64
            self.metadataLock.unlock()
            
            self.logger.info("GET_METADATA: stored URL=\(url ?? "nil"), hasStoredIcon=\(storedIcon != nil)")
            
            // Priority order:
            // 1. Stored icon from extension (website favicon)
            // 2. Fetched favicon from URL (using Google's service)
            // 3. Safari app icon (last resort fallback)
            var finalIcon = storedIcon
            
            if finalIcon == nil {
                self.logger.debug("No stored icon, attempting to fetch favicon...")
                
                // Try to fetch favicon from URL
                if let url = url {
                    finalIcon = self.fetchFavicon(for: url)
                    if finalIcon != nil {
                        self.logger.info("Successfully fetched favicon for URL")
                    } else {
                        self.logger.warning("Failed to fetch favicon for URL: \(url)")
                    }
                } else {
                    self.logger.warning("No URL available to fetch favicon")
                }
                
                // Last resort: Safari app icon
                if finalIcon == nil {
                    self.logger.info("Using Safari app icon as fallback")
                    finalIcon = self.getSafariIconBase64()
                }
            } else {
                self.logger.debug("Using stored icon from extension")
            }
            
            // Build the metadata response
            let metadata: [String: Any] = [
                "kind": "NativeMetadata",
                "data": [
                    "url": url as Any,
                    "icon_base64": finalIcon as Any
                ]
            ]
            
            let payloadData = try? JSONSerialization.data(withJSONObject: metadata, options: [])
            let payloadString = payloadData.flatMap { String(data: $0, encoding: .utf8) }
            
            let response: [String: Any] = [
                "kind": [
                    "Response": [
                        "id": requestId,
                        "action": action,
                        "payload": payloadString as Any
                    ]
                ]
            ]
            
            self.queue.async {
                self.sendRawFrame(response)
            }
        }
    }
    
    /// Get Safari's application icon using native macOS APIs
    private func getSafariIconBase64() -> String? {
        // Find Safari's bundle path
        let workspace = NSWorkspace.shared
        
        // Try to find Safari by bundle identifier
        guard let safariURL = workspace.urlForApplication(withBundleIdentifier: "com.apple.Safari") else {
            logger.warning("Could not find Safari bundle URL")
            return nil
        }
        
        // Get the icon for Safari's bundle
        let icon = workspace.icon(forFile: safariURL.path)
        
        // Convert NSImage to PNG data
        guard let pngData = nsImageToPNGData(icon, size: 64) else {
            logger.warning("Could not convert Safari icon to PNG")
            return nil
        }
        
        let base64 = pngData.base64EncodedString()
        return "data:image/png;base64,\(base64)"
    }
    
    /// Convert NSImage to PNG data at specified size
    private func nsImageToPNGData(_ image: NSImage, size: Int) -> Data? {
        let targetSize = NSSize(width: size, height: size)
        
        // Create a bitmap representation at the target size
        guard let bitmapRep = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: size,
            pixelsHigh: size,
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .calibratedRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ) else {
            return nil
        }
        
        bitmapRep.size = targetSize
        
        // Draw the image into the bitmap
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmapRep)
        
        image.draw(
            in: NSRect(origin: .zero, size: targetSize),
            from: .zero,
            operation: .copy,
            fraction: 1.0
        )
        
        NSGraphicsContext.restoreGraphicsState()
        
        // Convert to PNG data
        return bitmapRep.representation(using: .png, properties: [:])
    }
    
    private func fetchFavicon(for urlString: String) -> String? {
        guard let url = URL(string: urlString),
              let host = url.host else {
            logger.warning("fetchFavicon: Invalid URL or no host: \(urlString)")
            return nil
        }
        
        logger.debug("fetchFavicon: Attempting to fetch favicon for host: \(host)")
        
        // Try Google's favicon service first (most reliable)
        // Then try common favicon locations
        let faviconURLs = [
            "https://www.google.com/s2/favicons?domain=\(host)&sz=64",
            "https://t2.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=\(urlString)&size=64",
            "https://\(host)/favicon.ico",
            "https://\(host)/favicon.png",
            "https://\(host)/apple-touch-icon.png"
        ]
        
        for faviconURLString in faviconURLs {
            guard let faviconURL = URL(string: faviconURLString) else {
                continue
            }
            
            logger.debug("fetchFavicon: Trying \(faviconURLString)")
            
            do {
                // Use URLSession with timeout for better reliability
                let semaphore = DispatchSemaphore(value: 0)
                var resultData: Data?
                var resultError: Error?
                
                let task = URLSession.shared.dataTask(with: faviconURL) { data, response, error in
                    if let httpResponse = response as? HTTPURLResponse {
                        self.logger.debug("fetchFavicon: Got HTTP \(httpResponse.statusCode) from \(faviconURLString)")
                    }
                    resultData = data
                    resultError = error
                    semaphore.signal()
                }
                task.resume()
                
                // Wait up to 5 seconds
                let waitResult = semaphore.wait(timeout: .now() + 5.0)
                if waitResult == .timedOut {
                    task.cancel()
                    logger.warning("fetchFavicon: Timeout fetching \(faviconURLString)")
                    continue
                }
                
                if let error = resultError {
                    logger.debug("fetchFavicon: Error fetching \(faviconURLString): \(error.localizedDescription)")
                    continue
                }
                
                guard let data = resultData, !data.isEmpty else {
                    logger.debug("fetchFavicon: Empty data from \(faviconURLString)")
                    continue
                }
                
                // Google's service returns 16x16 default icon which is ~726 bytes
                // Real favicons are typically larger
                if data.count > 100 {
                    // Determine MIME type
                    let mimeType: String
                    if faviconURLString.contains("google.com") || faviconURLString.contains("gstatic.com") || faviconURLString.hasSuffix(".png") {
                        mimeType = "image/png"
                    } else {
                        mimeType = "image/x-icon"
                    }
                    let base64 = data.base64EncodedString()
                    logger.info("fetchFavicon: Success from \(faviconURLString) (\(data.count) bytes)")
                    return "data:\(mimeType);base64,\(base64)"
                } else {
                    logger.debug("fetchFavicon: Data too small from \(faviconURLString) (\(data.count) bytes)")
                }
            } catch {
                logger.debug("fetchFavicon: Exception for \(faviconURLString): \(error.localizedDescription)")
                continue
            }
        }
        
        logger.warning("fetchFavicon: All sources failed for host: \(host)")
        return nil
    }
    
    private func sendRawFrame(_ frame: [String: Any]) {
        guard let stdinPipe = self.stdinPipe, self.process?.isRunning == true else {
            logger.error("Cannot send frame - process not running")
            return
        }
        
        do {
            let jsonData = try JSONSerialization.data(withJSONObject: frame, options: [])
            
            // Write length prefix (4 bytes, little-endian)
            var length = UInt32(jsonData.count).littleEndian
            let lengthData = Data(bytes: &length, count: 4)
            
            let fileHandle = stdinPipe.fileHandleForWriting
            fileHandle.write(lengthData)
            fileHandle.write(jsonData)
            
            logger.debug("Sent raw frame: \(jsonData.count) bytes")
        } catch {
            logger.error("Failed to send raw frame: \(error.localizedDescription)")
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
