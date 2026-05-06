import AppKit
import Foundation
import os.log
import SafariServices

/// Source of authority for "is the Eurora Safari Web Extension currently
/// enabled in Safari." Polls `SFSafariExtensionManager` and publishes a
/// fresh state to the desktop bridge via [`EXTENSION_STATE_EVENT`] every
/// time the answer changes.
///
/// `SFSafariExtensionManager` doesn't post notifications when the user
/// flips the toggle in Safari Settings → Extensions, so polling is the
/// only signal available. The user only cares about Safari's extension
/// state while they're using Safari, so the poll loop is gated on Safari
/// being the frontmost app — when anything else is in front, the timer
/// is suspended entirely. This keeps the launcher idle when the user
/// never opens Safari at all.
///
/// Lifecycle:
/// - Constructor wires `NSWorkspace` activation/deactivation observers
///   and (if Safari is already frontmost) does an immediate probe and
///   starts the poll timer.
/// - `stop()` invalidates observers and the timer; safe to call from any
///   thread, idempotent.
/// - The single in-flight probe is guarded so concurrent
///   activation/timer ticks coalesce.
@available(macOS 13.0, *)
final class SafariExtensionMonitor {
    typealias Publisher = (BundledExtensionState) -> Void

    private let logger = Logger(subsystem: "com.eurora.macos", category: "SafariExtensionMonitor")
    private let extensionBundleIdentifier: String
    private let safariBundleIdentifiers: Set<String>
    private let pollInterval: TimeInterval
    private let publisher: Publisher

    private let stateLock = NSLock()
    private var lastPublished: BundledExtensionState?
    private var pollTimer: Timer?
    private var probeInFlight = false

    /// `false` when initialized — we don't know yet. `true` after the
    /// first activation/deactivation observer notification or initial
    /// `probeIfSafariFrontmost()` run. Determines whether we need to
    /// poll/publish on activation.
    private var initialPolled = false

    init(
        extensionBundleIdentifier: String,
        safariBundleIdentifiers: [String],
        pollInterval: TimeInterval = 1.0,
        publisher: @escaping Publisher
    ) {
        self.extensionBundleIdentifier = extensionBundleIdentifier
        self.safariBundleIdentifiers = Set(safariBundleIdentifiers)
        self.pollInterval = pollInterval
        self.publisher = publisher

        let center = NSWorkspace.shared.notificationCenter
        center.addObserver(
            self,
            selector: #selector(workspaceAppDidActivate(_:)),
            name: NSWorkspace.didActivateApplicationNotification,
            object: nil
        )
        center.addObserver(
            self,
            selector: #selector(workspaceAppDidDeactivate(_:)),
            name: NSWorkspace.didDeactivateApplicationNotification,
            object: nil
        )

        // Catch the case where Safari is already frontmost when the
        // launcher comes up — the activation notification was fired
        // before our observer registered, so we'd otherwise sit idle
        // until the user clicks somewhere else and back.
        DispatchQueue.main.async { [weak self] in
            self?.syncWithCurrentFrontmostApp()
        }
    }

    deinit {
        stop()
    }

    /// Tear down observers and timers. Safe to call multiple times.
    func stop() {
        NSWorkspace.shared.notificationCenter.removeObserver(self)
        stateLock.lock()
        pollTimer?.invalidate()
        pollTimer = nil
        stateLock.unlock()
    }

    // MARK: - Workspace observers

    @objc private func workspaceAppDidActivate(_ notification: Notification) {
        guard isSafari(notification: notification) else { return }
        logger.debug("Safari activated — starting extension state poll")
        startPolling()
        probe()
    }

    @objc private func workspaceAppDidDeactivate(_ notification: Notification) {
        guard isSafari(notification: notification) else { return }
        logger.debug("Safari deactivated — stopping extension state poll")
        stopPolling()
    }

    private func syncWithCurrentFrontmostApp() {
        guard
            let app = NSWorkspace.shared.frontmostApplication,
            let bundleId = app.bundleIdentifier,
            safariBundleIdentifiers.contains(bundleId)
        else { return }
        logger.debug("Safari already frontmost at startup — starting poll")
        startPolling()
        probe()
    }

    private func isSafari(notification: Notification) -> Bool {
        guard
            let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
            let bundleId = app.bundleIdentifier
        else { return false }
        return safariBundleIdentifiers.contains(bundleId)
    }

    // MARK: - Polling

    private func startPolling() {
        stateLock.lock()
        if pollTimer != nil {
            stateLock.unlock()
            return
        }
        // Fixed-rate timer on the main run loop. Each tick schedules an
        // async `getStateOfSafariExtension` call; the call itself is
        // off-main, completion is delivered back via `probe`.
        let timer = Timer(timeInterval: pollInterval, repeats: true) { [weak self] _ in
            self?.probe()
        }
        // `.common` so Safari/Finder context-menu modal loops don't pause us.
        RunLoop.main.add(timer, forMode: .common)
        pollTimer = timer
        stateLock.unlock()
    }

    private func stopPolling() {
        stateLock.lock()
        pollTimer?.invalidate()
        pollTimer = nil
        stateLock.unlock()
    }

    /// Issue an `SFSafariExtensionManager` probe and publish the result if
    /// it differs from what we last sent. Concurrent ticks coalesce — the
    /// API call is async and we don't want to stack outstanding requests
    /// (each one rebuilds Safari's preference cache; ~10ms).
    private func probe() {
        stateLock.lock()
        if probeInFlight {
            stateLock.unlock()
            return
        }
        probeInFlight = true
        stateLock.unlock()

        SFSafariExtensionManager.getStateOfSafariExtension(
            withIdentifier: extensionBundleIdentifier
        ) { [weak self] state, error in
            guard let self else { return }
            handleProbeResult(state: state, error: error)
        }
    }

    private func handleProbeResult(state: SFSafariExtensionState?, error: Error?) {
        let resolved = mapProbe(state: state, error: error)
        stateLock.lock()
        probeInFlight = false
        let changed = resolved != lastPublished
        if changed {
            lastPublished = resolved
        }
        initialPolled = true
        stateLock.unlock()

        if changed {
            logger.info(
                "Safari extension state changed: \(resolved.rawValue, privacy: .public)"
            )
            publisher(resolved)
        }
    }

    private func mapProbe(state: SFSafariExtensionState?, error: Error?) -> BundledExtensionState {
        if let error {
            // The most reliable "Safari hasn't indexed our .appex" signal
            // is an error from the manager. We surface it as
            // `notDiscovered` so the desktop can prompt the user to launch
            // Eurora once before enabling the extension.
            logger.info(
                """
                SFSafariExtensionManager error (treating as not_discovered): \
                \(error.localizedDescription, privacy: .public)
                """
            )
            return .notDiscovered
        }
        guard let state else { return .unknown }
        return state.isEnabled ? .enabled : .disabled
    }
}
