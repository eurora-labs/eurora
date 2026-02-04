//
//  AppDelegate.swift
//  macos
//
//  Container app for the Eurora Safari extension.
//  Manages the native messaging bridge lifecycle.
//

import Cocoa

@main
class AppDelegate: NSObject, NSApplicationDelegate {

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Start the native messaging bridge
        NativeMessagingBridge.shared.start()
    }

    func applicationWillTerminate(_ notification: Notification) {
        // Stop the native messaging bridge
        NativeMessagingBridge.shared.stop()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }
}
