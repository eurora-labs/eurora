// SPDX-License-Identifier: Apache-2.0

import XCTest
@testable import tauri_plugin_appauth

/// `ConfigSource` mirrors the Rust `ConfigSource` tagged union over the wire.
/// These tests pin the JSON shape both directions: the discriminator field,
/// optional explicit endpoints, and the unknown-kind error path.
final class ConfigSourceTests: XCTestCase {

    private let decoder = JSONDecoder()

    func testDiscoveryKindDecodes() throws {
        let json = #"{"kind":"discovery","issuer":"https://issuer.example.com"}"#
        let source = try decode(json)
        guard case .discovery(let issuer) = source else {
            XCTFail("expected .discovery, got \(source)")
            return
        }
        XCTAssertEqual(issuer, "https://issuer.example.com")
    }

    func testExplicitKindWithAllFieldsDecodes() throws {
        let json = #"""
        {
            "kind": "explicit",
            "authorizationEndpoint": "https://auth.example.com/oauth/authorize",
            "tokenEndpoint": "https://auth.example.com/oauth/token",
            "endSessionEndpoint": "https://auth.example.com/oauth/logout",
            "registrationEndpoint": "https://auth.example.com/oauth/register"
        }
        """#
        let source = try decode(json)
        guard
            case let .explicit(authEndpoint, tokenEndpoint, endSessionEndpoint, registrationEndpoint) = source
        else {
            XCTFail("expected .explicit, got \(source)")
            return
        }
        XCTAssertEqual(authEndpoint, "https://auth.example.com/oauth/authorize")
        XCTAssertEqual(tokenEndpoint, "https://auth.example.com/oauth/token")
        XCTAssertEqual(endSessionEndpoint, "https://auth.example.com/oauth/logout")
        XCTAssertEqual(registrationEndpoint, "https://auth.example.com/oauth/register")
    }

    func testExplicitKindWithoutOptionalFields() throws {
        let json = #"""
        {
            "kind": "explicit",
            "authorizationEndpoint": "https://auth.example.com/oauth/authorize",
            "tokenEndpoint": "https://auth.example.com/oauth/token"
        }
        """#
        let source = try decode(json)
        guard
            case let .explicit(_, _, endSessionEndpoint, registrationEndpoint) = source
        else {
            XCTFail("expected .explicit, got \(source)")
            return
        }
        XCTAssertNil(endSessionEndpoint)
        XCTAssertNil(registrationEndpoint)
    }

    func testUnknownKindThrowsDataCorrupted() {
        let json = #"{"kind":"telepathy","issuer":"https://issuer.example.com"}"#
        XCTAssertThrowsError(try decode(json)) { error in
            guard case DecodingError.dataCorrupted = error else {
                XCTFail("expected DecodingError.dataCorrupted, got \(error)")
                return
            }
        }
    }

    func testMissingKindThrowsKeyNotFound() {
        let json = #"{"issuer":"https://issuer.example.com"}"#
        XCTAssertThrowsError(try decode(json)) { error in
            guard case DecodingError.keyNotFound = error else {
                XCTFail("expected DecodingError.keyNotFound, got \(error)")
                return
            }
        }
    }

    func testExplicitEndpointResolveBuildsConfigurationWithoutNetwork() async throws {
        // The explicit branch of `resolve` constructs an `OIDServiceConfiguration`
        // synchronously from the supplied URLs — it should never hit the wire.
        let source = ConfigSource.explicit(
            authorizationEndpoint: "https://auth.example.com/authorize",
            tokenEndpoint: "https://auth.example.com/token",
            endSessionEndpoint: nil,
            registrationEndpoint: nil
        )
        let config = try await source.resolve()
        XCTAssertEqual(
            config.authorizationEndpoint.absoluteString,
            "https://auth.example.com/authorize"
        )
        XCTAssertEqual(config.tokenEndpoint.absoluteString, "https://auth.example.com/token")
        XCTAssertNil(config.endSessionEndpoint)
        XCTAssertNil(config.registrationEndpoint)
    }

    func testExplicitEndpointResolveRejectsMalformedUrls() async {
        let source = ConfigSource.explicit(
            authorizationEndpoint: "",
            tokenEndpoint: "https://auth.example.com/token",
            endSessionEndpoint: nil,
            registrationEndpoint: nil
        )
        do {
            _ = try await source.resolve()
            XCTFail("expected throw")
        } catch let AppAuthBridgeError.invalidRequest(message) {
            XCTAssertTrue(message.contains("invalid endpoint URL"), "unexpected message: \(message)")
        } catch {
            XCTFail("expected AppAuthBridgeError.invalidRequest, got \(error)")
        }
    }

    func testDiscoveryKindResolveRejectsMalformedIssuer() async {
        let source = ConfigSource.discovery(issuer: "")
        do {
            _ = try await source.resolve()
            XCTFail("expected throw")
        } catch let AppAuthBridgeError.invalidRequest(message) {
            XCTAssertTrue(message.contains("invalid issuer URL"), "unexpected message: \(message)")
        } catch {
            XCTFail("expected AppAuthBridgeError.invalidRequest, got \(error)")
        }
    }

    // MARK: - Helpers

    private func decode(_ json: String) throws -> ConfigSource {
        let data = Data(json.utf8)
        return try decoder.decode(ConfigSource.self, from: data)
    }
}
