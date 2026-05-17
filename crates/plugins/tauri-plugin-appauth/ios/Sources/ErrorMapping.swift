// SPDX-License-Identifier: Apache-2.0

import AppAuth
import Foundation
import Tauri

/// Maps `NSError`s emitted by AppAuth (and by `URLSession` indirectly) onto the
/// stable error codes the Rust crate publishes via `Error::code()`.
///
/// AppAuth surfaces three OAuth-error sub-domains
/// (`OIDOAuthAuthorizationErrorDomain`, `OIDOAuthTokenErrorDomain`,
/// `OIDOAuthRegistrationErrorDomain`); for each we extract the OAuth `error`
/// and `error_description` strings from `userInfo[OIDOAuthErrorResponseErrorKey]`
/// so the JS layer doesn't have to parse free-form messages.
enum ErrorMapping {

    struct Mapping {
        let code: String
        let message: String
        let oauthError: String?
        let oauthErrorDescription: String?
    }

    /// Reject `invoke` with the appropriate code/message/oauth-context combo.
    static func reject(_ invoke: Invoke, error: Error) {
        let mapping = map(error)
        var data: JsonObject = [:]
        if let oauth = mapping.oauthError {
            data["oauthError"] = oauth
        }
        if let desc = mapping.oauthErrorDescription {
            data["oauthErrorDescription"] = desc
        }
        let payload: JsonValue? = data.isEmpty ? nil : .dictionary(data)
        invoke.reject(mapping.message, code: mapping.code, data: payload)
    }

    static func map(_ error: Error) -> Mapping {
        let nsError = error as NSError

        if nsError.domain == OIDOAuthAuthorizationErrorDomain {
            return mapOAuth(nsError, defaultCode: codeAuthorizationFailed)
        }
        if nsError.domain == OIDOAuthTokenErrorDomain {
            return mapOAuth(nsError, defaultCode: codeTokenExchangeFailed)
        }
        if nsError.domain == OIDOAuthRegistrationErrorDomain {
            return mapOAuth(nsError, defaultCode: codeInvalidRegistrationResponse)
        }
        if nsError.domain == OIDGeneralErrorDomain {
            return mapGeneral(nsError)
        }
        if nsError.domain == NSURLErrorDomain {
            return Mapping(
                code: codeNetworkError,
                message: nsError.localizedDescription,
                oauthError: nil,
                oauthErrorDescription: nil
            )
        }
        if let bridge = error as? AppAuthBridgeError {
            switch bridge {
            case .invalidRequest(let message):
                return Mapping(
                    code: codeInvalidRequest,
                    message: message,
                    oauthError: nil,
                    oauthErrorDescription: nil
                )
            case .serverError(let message):
                return Mapping(
                    code: codeServerError,
                    message: message,
                    oauthError: nil,
                    oauthErrorDescription: nil
                )
            }
        }
        return Mapping(
            code: codeAuthorizationFailed,
            message: nsError.localizedDescription,
            oauthError: nil,
            oauthErrorDescription: nil
        )
    }

    // MARK: - Sub-mappers

    /// `OIDGeneralErrorDomain` covers AppAuth's library-internal errors. The
    /// integer codes are stable per AppAuth's public API (see `OIDError.h`);
    /// we compare on `rawValue` rather than enum cases to sidestep Swift's
    /// inconsistent acronym-import rules for `OIDErrorCodeIDToken*`.
    private static func mapGeneral(_ nsError: NSError) -> Mapping {
        let message = nsError.localizedDescription
        switch nsError.code {
        case OIDErrorCode.userCanceledAuthorizationFlow.rawValue,
             OIDErrorCode.programCanceledAuthorizationFlow.rawValue:
            return Mapping(code: codeUserCanceled, message: message, oauthError: nil, oauthErrorDescription: nil)

        case OIDErrorCode.networkError.rawValue:
            return Mapping(code: codeNetworkError, message: message, oauthError: nil, oauthErrorDescription: nil)

        case OIDErrorCode.serverError.rawValue,
             OIDErrorCode.invalidDiscoveryDocument.rawValue,
             OIDErrorCode.jsonDeserializationError.rawValue,
             OIDErrorCode.jsonSerializationError.rawValue:
            return Mapping(code: codeServerError, message: message, oauthError: nil, oauthErrorDescription: nil)

        case OIDErrorCode.tokenResponseConstructionError.rawValue,
             OIDErrorCode.tokenRefreshError.rawValue:
            return Mapping(code: codeTokenExchangeFailed, message: message, oauthError: nil, oauthErrorDescription: nil)

        case OIDErrorCode.registrationResponseConstructionError.rawValue:
            return Mapping(code: codeInvalidRegistrationResponse, message: message, oauthError: nil, oauthErrorDescription: nil)

        case oidErrorCodeIDTokenParsing,
             oidErrorCodeIDTokenFailedValidation:
            return Mapping(code: codeIdTokenValidationFailed, message: message, oauthError: nil, oauthErrorDescription: nil)

        case OIDErrorCode.safariOpenError.rawValue,
             OIDErrorCode.browserOpenError.rawValue:
            return Mapping(code: codeBrowserNotAvailable, message: message, oauthError: nil, oauthErrorDescription: nil)

        default:
            return Mapping(code: codeAuthorizationFailed, message: message, oauthError: nil, oauthErrorDescription: nil)
        }
    }

    /// Pull `error` / `error_description` out of the OAuth response dict that
    /// AppAuth attaches to `userInfo[OIDOAuthErrorResponseErrorKey]`.
    private static func mapOAuth(_ nsError: NSError, defaultCode: String) -> Mapping {
        let response = nsError.userInfo[OIDOAuthErrorResponseErrorKey] as? [String: Any]
        let oauthError = response?[OIDOAuthErrorFieldError] as? String
        let oauthErrorDescription = response?[OIDOAuthErrorFieldErrorDescription] as? String
        return Mapping(
            code: defaultCode,
            message: nsError.localizedDescription,
            oauthError: oauthError,
            oauthErrorDescription: oauthErrorDescription
        )
    }

    // MARK: - Code constants (kept in sync with Rust `Error::code()`)

    static let codeUserCanceled = "USER_CANCELED"
    static let codeAuthorizationFailed = "AUTHORIZATION_FAILED"
    static let codeTokenExchangeFailed = "TOKEN_EXCHANGE_FAILED"
    static let codeNetworkError = "NETWORK_ERROR"
    static let codeInvalidRegistrationResponse = "INVALID_REGISTRATION_RESPONSE"
    static let codeIdTokenValidationFailed = "ID_TOKEN_VALIDATION_FAILED"
    static let codeBrowserNotAvailable = "BROWSER_NOT_AVAILABLE"
    static let codeInvalidRequest = "INVALID_REQUEST"
    static let codeServerError = "SERVER_ERROR"

    // ID Token error codes from `OIDError.h` (-14 / -15). Hard-coded because
    // Swift's import of `OIDErrorCodeIDTokenParsingError` produces a case name
    // (`iDTokenParsingError`) that is unstable across compiler versions.
    private static let oidErrorCodeIDTokenParsing: Int = -14
    private static let oidErrorCodeIDTokenFailedValidation: Int = -15
}
