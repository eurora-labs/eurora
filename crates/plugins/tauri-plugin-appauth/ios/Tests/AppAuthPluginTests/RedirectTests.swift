// SPDX-License-Identifier: Apache-2.0

import XCTest
@testable import tauri_plugin_appauth

/// Covers `validateRedirect` (Phase 1.4) and `redirectKind` (Phase 1.2). The
/// happy paths assert what AppAuth and `ASWebAuthenticationSession` actually
/// receive; the rejection paths lock in the synchronous `INVALID_REQUEST` exit
/// so malformed input never reaches AppAuth's late, opaque error path.
final class RedirectTests: XCTestCase {

    // MARK: - validateRedirect

    func testCustomSchemeWithPathIsAccepted() throws {
        let url = try validateRedirect("com.example.app:/oauth/callback")
        XCTAssertEqual(url.scheme, "com.example.app")
        XCTAssertEqual(url.path, "/oauth/callback")
    }

    func testHttpsRedirectIsAccepted() throws {
        let url = try validateRedirect("https://login.example.com/oauth/callback")
        XCTAssertEqual(url.scheme, "https")
        XCTAssertEqual(url.host, "login.example.com")
    }

    func testLeadingAndTrailingWhitespaceIsTrimmed() throws {
        let url = try validateRedirect("  com.example.app:/oauth  ")
        XCTAssertEqual(url.scheme, "com.example.app")
    }

    func testEmptyStringIsRejected() {
        assertInvalidRequest(try validateRedirect(""))
    }

    func testWhitespaceOnlyIsRejected() {
        assertInvalidRequest(try validateRedirect("   \n\t "))
    }

    func testBareSchemeIsRejected() {
        // `com.example:` parses as a URL but has no host and no path — exactly
        // the case that used to leak into AppAuth and fail with a confusing
        // late error.
        assertInvalidRequest(try validateRedirect("com.example:"))
    }

    func testMissingSchemeIsRejected() {
        assertInvalidRequest(try validateRedirect("login.example.com/oauth"))
    }

    // MARK: - redirectKind

    func testRedirectKindResolvesCustomScheme() throws {
        let url = try validateRedirect("com.example.app:/oauth/callback")
        XCTAssertEqual(redirectKind(for: url), .customScheme("com.example.app"))
    }

    func testRedirectKindResolvesUniversalLink() throws {
        let url = try validateRedirect("https://login.example.com/oauth/callback")
        XCTAssertEqual(
            redirectKind(for: url),
            .universalLink(host: "login.example.com", path: "/oauth/callback")
        )
    }

    func testRedirectKindUniversalLinkDefaultsEmptyPathToRoot() throws {
        let url = try validateRedirect("https://login.example.com")
        XCTAssertEqual(
            redirectKind(for: url),
            .universalLink(host: "login.example.com", path: "/")
        )
    }

    func testRedirectKindIsCaseInsensitiveOnHttpsScheme() throws {
        let url = try validateRedirect("HTTPS://login.example.com/cb")
        XCTAssertEqual(
            redirectKind(for: url),
            .universalLink(host: "login.example.com", path: "/cb")
        )
    }

    // MARK: - Helpers

    private func assertInvalidRequest(
        _ expression: @autoclosure () throws -> Any,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertThrowsError(try expression(), file: file, line: line) { error in
            guard case AppAuthBridgeError.invalidRequest = error else {
                XCTFail("expected AppAuthBridgeError.invalidRequest, got \(error)", file: file, line: line)
                return
            }
        }
    }
}
