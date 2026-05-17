// SPDX-License-Identifier: Apache-2.0

import AppAuth
import Foundation

// MARK: - Inputs decoded from JS payloads

struct DiscoverRequest: Decodable {
    let issuer: String
}

/// Mirrors the Rust `ConfigSource` tagged union (`kind: "discovery"|"explicit"`).
enum ConfigSource: Decodable {
    case discovery(issuer: String)
    case explicit(
        authorizationEndpoint: String,
        tokenEndpoint: String,
        endSessionEndpoint: String?,
        registrationEndpoint: String?
    )

    private enum CodingKeys: String, CodingKey {
        case kind
        case issuer
        case authorizationEndpoint
        case tokenEndpoint
        case endSessionEndpoint
        case registrationEndpoint
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .kind)
        switch kind {
        case "discovery":
            self = .discovery(issuer: try container.decode(String.self, forKey: .issuer))
        case "explicit":
            self = .explicit(
                authorizationEndpoint: try container.decode(String.self, forKey: .authorizationEndpoint),
                tokenEndpoint: try container.decode(String.self, forKey: .tokenEndpoint),
                endSessionEndpoint: try container.decodeIfPresent(String.self, forKey: .endSessionEndpoint),
                registrationEndpoint: try container.decodeIfPresent(String.self, forKey: .registrationEndpoint)
            )
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .kind,
                in: container,
                debugDescription: "unknown ConfigSource kind: \(kind)"
            )
        }
    }

    /// Resolve to an `OIDServiceConfiguration`, hitting the discovery endpoint
    /// when needed.
    ///
    /// Async wrapper around AppAuth's callback-based discovery: the suspension
    /// resumes on the caller's actor, so `@MainActor` callers stay on main
    /// without having to dispatch defensively.
    func resolve() async throws -> OIDServiceConfiguration {
        switch self {
        case .discovery(let issuer):
            guard let issuerURL = URL(string: issuer) else {
                throw AppAuthBridgeError.invalidRequest("invalid issuer URL: \(issuer)")
            }
            return try await Self.discover(issuerURL: issuerURL)
        case .explicit(let authEndpoint, let tokenEndpoint, let endSessionEndpoint, let registrationEndpoint):
            guard
                let authURL = URL(string: authEndpoint),
                let tokenURL = URL(string: tokenEndpoint)
            else {
                throw AppAuthBridgeError.invalidRequest("invalid endpoint URL")
            }
            return OIDServiceConfiguration(
                authorizationEndpoint: authURL,
                tokenEndpoint: tokenURL,
                issuer: nil,
                registrationEndpoint: registrationEndpoint.flatMap(URL.init(string:)),
                endSessionEndpoint: endSessionEndpoint.flatMap(URL.init(string:))
            )
        }
    }

    /// Bridge `OIDAuthorizationService.discoverConfiguration` to `async throws`.
    /// Public so `discover` can hit the same path without round-tripping through
    /// a `ConfigSource` value.
    static func discover(issuerURL: URL) async throws -> OIDServiceConfiguration {
        return try await withCheckedThrowingContinuation { continuation in
            OIDAuthorizationService.discoverConfiguration(forIssuer: issuerURL) { config, error in
                if let error = error {
                    continuation.resume(throwing: error)
                    return
                }
                guard let config = config else {
                    continuation.resume(throwing: AppAuthBridgeError.serverError("discovery returned no configuration"))
                    return
                }
                continuation.resume(returning: config)
            }
        }
    }
}

/// OIDC `prompt` parameter values. `snake_case` to match the Rust enum.
enum Prompt: String, Decodable {
    case login
    case consent
    case selectAccount = "select_account"
    case none
}

struct AuthorizeRequest: Decodable {
    let config: ConfigSource
    let clientId: String
    let redirectUri: String
    @DefaultEmptyArray<String> var scopes: [String]
    @DefaultEmptyDictionary<String, String> var additionalParameters: [String: String]
    let prompt: Prompt?
    let loginHint: String?
    let uiLocales: [String]?
    @DefaultTrue var prefersEphemeralSession: Bool
    @DefaultTrue var useNonce: Bool
}

struct BrowserOnlyRequest: Decodable {
    let authUrl: String
    let redirectUri: String
    @DefaultTrue var prefersEphemeralSession: Bool
}

struct RefreshRequest: Decodable {
    let config: ConfigSource
    let clientId: String
    let refreshToken: String
    @DefaultEmptyArray<String> var scopes: [String]
    @DefaultEmptyDictionary<String, String> var additionalParameters: [String: String]
}

struct RegisterRequest: Decodable {
    let config: ConfigSource
    let redirectUris: [String]
    let clientName: String?
    @DefaultEmptyArray<String> var responseTypes: [String]
    @DefaultEmptyArray<String> var grantTypes: [String]
    @DefaultEmptyArray<String> var subjectTypes: [String]
    let tokenEndpointAuthMethod: String?
    @DefaultEmptyDictionary<String, String> var additionalParameters: [String: String]
}

struct EndSessionRequest: Decodable {
    let config: ConfigSource
    /// Optional per RFC 8665 / OIDC RP-Initiated Logout: the parameter is
    /// RECOMMENDED, not REQUIRED, and some IdPs accept end-session without it.
    let idTokenHint: String?
    let postLogoutRedirectUri: String
    let state: String?
    @DefaultEmptyDictionary<String, String> var additionalParameters: [String: String]
    @DefaultTrue var prefersEphemeralSession: Bool
}

// MARK: - Outputs encoded to JS responses

struct ServiceConfigurationResponse: Encodable {
    let authorizationEndpoint: String
    let tokenEndpoint: String
    let endSessionEndpoint: String?
    let registrationEndpoint: String?
    let issuer: String?
    let additionalParameters: [String: String]

    init(from config: OIDServiceConfiguration) {
        authorizationEndpoint = config.authorizationEndpoint.absoluteString
        tokenEndpoint = config.tokenEndpoint.absoluteString
        endSessionEndpoint = config.endSessionEndpoint?.absoluteString
        registrationEndpoint = config.registrationEndpoint?.absoluteString
        issuer = config.issuer?.absoluteString
        // Surface the raw discovery doc fields beyond the typed five so callers
        // can pick up provider-specific extensions (e.g. `userinfo_endpoint`).
        // Stringify nested arrays / objects to keep the wire shape uniform; the
        // typed fields above remain the primary source of truth.
        additionalParameters = stringifyDictionary(config.discoveryDocument?.discoveryDictionary)
    }
}

struct AuthStateResponse: Encodable {
    let accessToken: String?
    let accessTokenExpiresAt: Int64?
    let idToken: String?
    let refreshToken: String?
    let scope: String?
    let tokenType: String?
    let authorizationCode: String?
    let additionalParameters: [String: String]

    /// Construct from the merged `OIDAuthState` produced by the full
    /// `authorize` flow. Token-endpoint values take precedence over the
    /// authorization-endpoint snapshots they replaced.
    init(from authState: OIDAuthState) {
        let tokenResponse = authState.lastTokenResponse
        let authResponse = authState.lastAuthorizationResponse
        accessToken = tokenResponse?.accessToken ?? authResponse.accessToken
        accessTokenExpiresAt = (tokenResponse?.accessTokenExpirationDate
            ?? authResponse.accessTokenExpirationDate)
            .map { Int64($0.timeIntervalSince1970) }
        idToken = tokenResponse?.idToken ?? authResponse.idToken
        refreshToken = authState.refreshToken
        scope = authState.scope ?? tokenResponse?.scope ?? authResponse.scope
        tokenType = tokenResponse?.tokenType ?? authResponse.tokenType
        authorizationCode = authResponse.authorizationCode
        additionalParameters = stringifyDictionary(tokenResponse?.additionalParameters)
    }

    /// Construct from a bare `OIDTokenResponse` (e.g. refresh flows where there
    /// is no preceding authorization response).
    ///
    /// The refresh-token fallback to `tokenResponse.request.refreshToken` is
    /// required by RFC 6749 §6: the authorization server MAY but is not
    /// required to issue a new refresh token in a refresh response. When it
    /// doesn't, callers must continue using the original refresh token, so we
    /// echo it back. Removing the fallback would silently break the next
    /// refresh after a server that opts out of rotation.
    init(from tokenResponse: OIDTokenResponse) {
        accessToken = tokenResponse.accessToken
        accessTokenExpiresAt = tokenResponse.accessTokenExpirationDate.map { Int64($0.timeIntervalSince1970) }
        idToken = tokenResponse.idToken
        refreshToken = tokenResponse.refreshToken ?? tokenResponse.request.refreshToken
        scope = tokenResponse.scope
        tokenType = tokenResponse.tokenType
        authorizationCode = nil
        additionalParameters = stringifyDictionary(tokenResponse.additionalParameters)
    }
}

struct BrowserOnlyResponse: Encodable {
    let url: String
}

struct RegistrationResponseModel: Encodable {
    let clientId: String
    let clientIdIssuedAt: Int64?
    let clientSecret: String?
    let clientSecretExpiresAt: Int64?
    let registrationAccessToken: String?
    let registrationClientUri: String?
    let tokenEndpointAuthMethod: String?
    let additionalParameters: [String: String]

    init(from response: OIDRegistrationResponse) {
        clientId = response.clientID
        clientIdIssuedAt = response.clientIDIssuedAt.map { Int64($0.timeIntervalSince1970) }
        clientSecret = response.clientSecret
        clientSecretExpiresAt = response.clientSecretExpiresAt.map { Int64($0.timeIntervalSince1970) }
        registrationAccessToken = response.registrationAccessToken
        registrationClientUri = response.registrationClientURI?.absoluteString
        tokenEndpointAuthMethod = response.tokenEndpointAuthenticationMethod
        additionalParameters = stringifyDictionary(response.additionalParameters)
    }
}

struct EndSessionResponseModel: Encodable {
    let url: String
    let state: String?
}

// MARK: - Internal helpers

/// Errors raised by the Swift bridge before reaching AppAuth (e.g. malformed
/// inputs). Carries the same shape as AppAuth `NSError`s so the unified error
/// mapper can handle them.
enum AppAuthBridgeError: LocalizedError {
    case invalidRequest(String)
    case serverError(String)

    var errorDescription: String? {
        switch self {
        case .invalidRequest(let message), .serverError(let message): return message
        }
    }
}

// MARK: - Redirect URI parsing

/// How `ASWebAuthenticationSession` should intercept the redirect.
///
/// Custom-scheme URIs (`com.example.app:/oauth/callback`) use the legacy
/// `callbackURLScheme:` initializer. HTTPS Universal Links use the iOS 17.4+
/// `Callback.https(host:path:)` initializer; older iOS versions reject before
/// constructing the session.
enum RedirectKind: Equatable {
    case customScheme(String)
    case universalLink(host: String, path: String)
}

/// Syntactic validation for redirect URIs.
///
/// A valid URI must have a non-empty scheme and at least one of: a non-empty
/// host or a path beginning with `/`. This rejects bare-scheme strings like
/// `com.example:` that would otherwise reach AppAuth and fail late with a
/// confusing error.
func validateRedirect(_ uri: String) throws -> URL {
    let trimmed = uri.trimmingCharacters(in: .whitespacesAndNewlines)
    guard
        !trimmed.isEmpty,
        let url = URL(string: trimmed),
        let components = URLComponents(string: trimmed),
        let scheme = components.scheme, !scheme.isEmpty
    else {
        throw AppAuthBridgeError.invalidRequest("invalid redirect URI: \(uri)")
    }
    let hasHost = !(components.host?.isEmpty ?? true)
    let hasPath = components.path.hasPrefix("/")
    guard hasHost || hasPath else {
        throw AppAuthBridgeError.invalidRequest(
            "redirect URI must include a host or a path beginning with '/': \(uri)"
        )
    }
    return url
}

/// Classify a previously-validated redirect URL for `ASWebAuthenticationSession`.
///
/// HTTPS URLs (Universal Links) require a host; custom schemes carry their
/// scheme verbatim. Returns `nil` only if `validateRedirect` would have thrown.
func redirectKind(for url: URL) -> RedirectKind? {
    guard
        let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
        let scheme = components.scheme, !scheme.isEmpty
    else {
        return nil
    }
    if scheme.lowercased() == "https" {
        guard let host = components.host, !host.isEmpty else {
            return nil
        }
        let path = components.path.isEmpty ? "/" : components.path
        return .universalLink(host: host, path: path)
    }
    return .customScheme(scheme)
}

/// Coerce AppAuth's additional-parameter dictionaries to a JSON-encodable
/// `[String: String]`. AppAuth surfaces both `[String: NSObject<NSCopying>]`
/// (token / registration responses) and `[String: Any]`
/// (`OIDServiceDiscovery.discoveryDictionary`) — handling `Any` covers both.
/// OAuth/OIDC additional parameters are strings or numbers in practice;
/// nested arrays / objects are JSON-serialized so values stay round-trippable.
func stringifyDictionary(_ source: [String: Any]?) -> [String: String] {
    guard let source = source else { return [:] }
    var out: [String: String] = [:]
    out.reserveCapacity(source.count)
    for (key, value) in source {
        out[key] = stringify(value)
    }
    return out
}

private func stringify(_ value: Any) -> String {
    if let s = value as? String { return s }
    if let n = value as? NSNumber { return n.stringValue }
    if JSONSerialization.isValidJSONObject(value),
       let data = try? JSONSerialization.data(withJSONObject: value),
       let s = String(data: data, encoding: .utf8)
    {
        return s
    }
    return String(describing: value)
}
