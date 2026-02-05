//
//  AppDelegate.swift
//  macos
//
//  Container app for the Eurora Safari extension.
//  This app is mainly for installation and configuration.
//  The native messaging bridge is managed by the Safari extension itself.
//

import Cocoa

@main
class AppDelegate: NSObject, NSApplicationDelegate {

    func applicationDidFinishLaunching(_ notification: Notification) {
        // NOTE: The NativeMessagingBridge is NOT started here.
        // It runs in the Safari extension process, not the container app.
        // The extension handler (SafariWebExtensionHandler) manages its own bridge instance.
    }

    func applicationWillTerminate(_ notification: Notification) {
        // Nothing to clean up - bridge is managed by the extension
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }
}
