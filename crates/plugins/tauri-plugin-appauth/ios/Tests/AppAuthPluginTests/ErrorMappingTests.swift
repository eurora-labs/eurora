// SPDX-License-Identifier: Apache-2.0

import AppAuth
import XCTest
@testable import tauri_plugin_appauth

/// `ErrorMapping.map` is the bridge's only authoritative source for the
/// `code` strings the JS layer pattern-matches on; if a bucket changes,
/// callers break silently. One test per bucket keeps that contract honest.
final class ErrorMappingTests: XCTestCase {

    // MARK: - OAuth sub-domains

    func testAuthorizationErrorDomainExtractsOAuthFields() {
        let mapping = ErrorMapping.map(makeOAuthError(
            domain: OIDOAuthAuthorizationErrorDomain,
            code: -1,
            oauthError: "invalid_grant",
            oauthErrorDescription: "code expired"
        ))
        XCTAssertEqual(mapping.code, ErrorMapping.codeAuthorizationFailed)
        XCTAssertEqual(mapping.oauthError, "invalid_grant")
        XCTAssertEqual(mapping.oauthErrorDescription, "code expired")
    }

    func testTokenErrorDomainMapsToTokenExchangeFailed() {
        let mapping = ErrorMapping.map(makeOAuthError(
            domain: OIDOAuthTokenErrorDomain,
            code: -2,
            oauthError: "invalid_client",
            oauthErrorDescription: nil
        ))
        XCTAssertEqual(mapping.code, ErrorMapping.codeTokenExchangeFailed)
        XCTAssertEqual(mapping.oauthError, "invalid_client")
        XCTAssertNil(mapping.oauthErrorDescription)
    }

    func testRegistrationErrorDomainMapsToInvalidRegistrationResponse() {
        let mapping = ErrorMapping.map(makeOAuthError(
            domain: OIDOAuthRegistrationErrorDomain,
            code: -3,
            oauthError: "invalid_redirect_uri",
            oauthErrorDescription: "scheme not registered"
        ))
        XCTAssertEqual(mapping.code, ErrorMapping.codeInvalidRegistrationResponse)
        XCTAssertEqual(mapping.oauthError, "invalid_redirect_uri")
    }

    // MARK: - General error domain (OIDError.h codes)

    func testUserCanceledMapsToUserCanceled() {
        let mapping = ErrorMapping.map(makeGeneralError(.userCanceledAuthorizationFlow))
        XCTAssertEqual(mapping.code, ErrorMapping.codeUserCanceled)
        XCTAssertNil(mapping.oauthError)
    }

    func testProgramCanceledMapsToUserCanceled() {
        let mapping = ErrorMapping.map(makeGeneralError(.programCanceledAuthorizationFlow))
        XCTAssertEqual(mapping.code, ErrorMapping.codeUserCanceled)
    }

    func testGeneralNetworkErrorMapsToNetworkError() {
        let mapping = ErrorMapping.map(makeGeneralError(.networkError))
        XCTAssertEqual(mapping.code, ErrorMapping.codeNetworkError)
    }

    func testGeneralServerErrorMapsToServerError() {
        let mapping = ErrorMapping.map(makeGeneralError(.serverError))
        XCTAssertEqual(mapping.code, ErrorMapping.codeServerError)
    }

    func testInvalidDiscoveryDocumentMapsToServerError() {
        let mapping = ErrorMapping.map(makeGeneralError(.invalidDiscoveryDocument))
        XCTAssertEqual(mapping.code, ErrorMapping.codeServerError)
    }

    func testTokenRefreshErrorMapsToTokenExchangeFailed() {
        let mapping = ErrorMapping.map(makeGeneralError(.tokenRefreshError))
        XCTAssertEqual(mapping.code, ErrorMapping.codeTokenExchangeFailed)
    }

    func testRegistrationResponseConstructionMapsToInvalidRegistrationResponse() {
        let mapping = ErrorMapping.map(makeGeneralError(.registrationResponseConstructionError))
        XCTAssertEqual(mapping.code, ErrorMapping.codeInvalidRegistrationResponse)
    }

    func testIDTokenParsingCodeMapsToIdTokenValidationFailed() {
        // OIDErrorCodeIDTokenParsingError = -14 in OIDError.h; spelled directly
        // because Swift's import of the enum case name is unstable.
        let error = NSError(
            domain: OIDGeneralErrorDomain,
            code: -14,
            userInfo: [NSLocalizedDescriptionKey: "id token malformed"]
        )
        let mapping = ErrorMapping.map(error)
        XCTAssertEqual(mapping.code, ErrorMapping.codeIdTokenValidationFailed)
    }

    func testIDTokenValidationCodeMapsToIdTokenValidationFailed() {
        let error = NSError(
            domain: OIDGeneralErrorDomain,
            code: -15,
            userInfo: [NSLocalizedDescriptionKey: "id token signature mismatch"]
        )
        let mapping = ErrorMapping.map(error)
        XCTAssertEqual(mapping.code, ErrorMapping.codeIdTokenValidationFailed)
    }

    func testSafariOpenErrorMapsToBrowserNotAvailable() {
        let mapping = ErrorMapping.map(makeGeneralError(.safariOpenError))
        XCTAssertEqual(mapping.code, ErrorMapping.codeBrowserNotAvailable)
    }

    func testUnknownGeneralCodeFallsBackToAuthorizationFailed() {
        let error = NSError(
            domain: OIDGeneralErrorDomain,
            code: -9999,
            userInfo: [NSLocalizedDescriptionKey: "future AppAuth code"]
        )
        let mapping = ErrorMapping.map(error)
        XCTAssertEqual(mapping.code, ErrorMapping.codeAuthorizationFailed)
        XCTAssertEqual(mapping.message, "future AppAuth code")
    }

    // MARK: - URL error domain

    func testNSURLErrorMapsToNetworkError() {
        let error = NSError(
            domain: NSURLErrorDomain,
            code: NSURLErrorTimedOut,
            userInfo: [NSLocalizedDescriptionKey: "request timed out"]
        )
        let mapping = ErrorMapping.map(error)
        XCTAssertEqual(mapping.code, ErrorMapping.codeNetworkError)
        XCTAssertEqual(mapping.message, "request timed out")
        XCTAssertNil(mapping.oauthError)
    }

    // MARK: - Bridge errors

    func testBridgeInvalidRequestMapsToInvalidRequest() {
        let mapping = ErrorMapping.map(AppAuthBridgeError.invalidRequest("bad redirect"))
        XCTAssertEqual(mapping.code, ErrorMapping.codeInvalidRequest)
        XCTAssertEqual(mapping.message, "bad redirect")
    }

    func testBridgeServerErrorMapsToServerError() {
        let mapping = ErrorMapping.map(AppAuthBridgeError.serverError("discovery returned no configuration"))
        XCTAssertEqual(mapping.code, ErrorMapping.codeServerError)
        XCTAssertEqual(mapping.message, "discovery returned no configuration")
    }

    // MARK: - Catch-all

    func testUnknownDomainFallsBackToAuthorizationFailed() {
        let error = NSError(
            domain: "com.example.unknown",
            code: 42,
            userInfo: [NSLocalizedDescriptionKey: "out of nowhere"]
        )
        let mapping = ErrorMapping.map(error)
        XCTAssertEqual(mapping.code, ErrorMapping.codeAuthorizationFailed)
        XCTAssertEqual(mapping.message, "out of nowhere")
    }

    // MARK: - Helpers

    private func makeOAuthError(
        domain: String,
        code: Int,
        oauthError: String?,
        oauthErrorDescription: String?
    ) -> NSError {
        var response: [String: Any] = [:]
        if let oauthError = oauthError {
            response[OIDOAuthErrorFieldError] = oauthError
        }
        if let oauthErrorDescription = oauthErrorDescription {
            response[OIDOAuthErrorFieldErrorDescription] = oauthErrorDescription
        }
        return NSError(
            domain: domain,
            code: code,
            userInfo: [
                NSLocalizedDescriptionKey: "oauth failure",
                OIDOAuthErrorResponseErrorKey: response
            ]
        )
    }

    private func makeGeneralError(_ code: OIDErrorCode) -> NSError {
        NSError(
            domain: OIDGeneralErrorDomain,
            code: code.rawValue,
            userInfo: [NSLocalizedDescriptionKey: "general error \(code.rawValue)"]
        )
    }
}
