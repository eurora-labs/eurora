// SPDX-License-Identifier: Apache-2.0

import XCTest
@testable import tauri_plugin_appauth

/// Phase 4.2 replaced per-struct `init(from:)` decoders with `DecodableDefault`
/// property wrappers. The contract: a missing **or null** key falls back to the
/// declared default; a present key wins. These tests pin both halves so a
/// future refactor of the wrapper can't silently flip an opt-out flag.
final class DecodableDefaultTests: XCTestCase {

    private struct Wrapped: Decodable {
        @DefaultTrue var flag: Bool
        @DefaultEmptyArray<String> var items: [String]
        @DefaultEmptyDictionary<String, String> var pairs: [String: String]
    }

    func testMissingFieldsUseDefaults() throws {
        let value = try decode(#"{}"#)
        XCTAssertTrue(value.flag)
        XCTAssertEqual(value.items, [])
        XCTAssertEqual(value.pairs, [:])
    }

    func testNullFieldsUseDefaults() throws {
        let value = try decode(#"{"flag": null, "items": null, "pairs": null}"#)
        XCTAssertTrue(value.flag)
        XCTAssertEqual(value.items, [])
        XCTAssertEqual(value.pairs, [:])
    }

    func testPresentFieldsOverrideDefaults() throws {
        let value = try decode(#"""
        {
            "flag": false,
            "items": ["openid", "profile"],
            "pairs": {"audience": "api.example.com"}
        }
        """#)
        XCTAssertFalse(value.flag)
        XCTAssertEqual(value.items, ["openid", "profile"])
        XCTAssertEqual(value.pairs, ["audience": "api.example.com"])
    }

    func testPartialOverrideKeepsOtherDefaults() throws {
        let value = try decode(#"{"flag": false}"#)
        XCTAssertFalse(value.flag)
        XCTAssertEqual(value.items, [])
        XCTAssertEqual(value.pairs, [:])
    }

    /// `AuthorizeRequest` is the highest-traffic consumer; verify the wrapped
    /// fields actually round-trip through it so the tests cover real usage and
    /// not just an isolated fixture struct.
    func testAuthorizeRequestAppliesDefaults() throws {
        let json = #"""
        {
            "config": {"kind": "discovery", "issuer": "https://issuer.example.com"},
            "clientId": "client",
            "redirectUri": "com.example.app:/oauth"
        }
        """#
        let request = try JSONDecoder().decode(AuthorizeRequest.self, from: Data(json.utf8))
        XCTAssertEqual(request.scopes, [])
        XCTAssertEqual(request.additionalParameters, [:])
        XCTAssertTrue(request.prefersEphemeralSession)
        XCTAssertTrue(request.useNonce)
    }

    func testAuthorizeRequestRespectsExplicitFalse() throws {
        let json = #"""
        {
            "config": {"kind": "discovery", "issuer": "https://issuer.example.com"},
            "clientId": "client",
            "redirectUri": "com.example.app:/oauth",
            "useNonce": false,
            "prefersEphemeralSession": false,
            "scopes": ["openid"],
            "additionalParameters": {"audience": "x"}
        }
        """#
        let request = try JSONDecoder().decode(AuthorizeRequest.self, from: Data(json.utf8))
        XCTAssertFalse(request.useNonce)
        XCTAssertFalse(request.prefersEphemeralSession)
        XCTAssertEqual(request.scopes, ["openid"])
        XCTAssertEqual(request.additionalParameters, ["audience": "x"])
    }

    // MARK: - Helpers

    private func decode(_ json: String) throws -> Wrapped {
        try JSONDecoder().decode(Wrapped.self, from: Data(json.utf8))
    }
}
