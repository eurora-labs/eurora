// SPDX-License-Identifier: Apache-2.0

import AuthenticationServices
import CryptoKit
import Foundation
import UIKit

// `Tauri.Invoke` is a non-Sendable `@objc` type without strict-concurrency
// annotations. We capture invokes across the IPC-queue → main-actor boundary
// on purpose (each `Invoke` is resolved by exactly one callback path), so we
// silence the cross-module Sendable warnings here rather than scatter
// `nonisolated(unsafe)` through the file.
@preconcurrency import Tauri

/// Tauri 2 mobile plugin that bridges Sign in with Apple
/// (`ASAuthorizationController`) to Tauri's `Invoke` resolution model.
///
/// Concurrency: the class is `@MainActor`, so `currentController`,
/// `currentDelegate`, and `currentPresentationProvider` are accessed from a
/// single isolation domain. The `@objc` entry points are `nonisolated` and
/// hop to main via `Task { @MainActor in … }` before touching state.
/// `ASAuthorizationController`'s delegate methods are documented to fire on
/// the main queue, so the delegate body re-enters `@MainActor` for free.
///
/// Long-running flow: the in-flight `ASAuthorizationController` is retained on
/// `self` so the underlying sheet is not deallocated while the user is
/// interacting with it. Starting a new flow cancels the prior one — see
/// `resetCurrentFlow()`.
@MainActor
class AppleAuthPlugin: Plugin {

    /// Active `ASAuthorizationController` for the current sign-in attempt.
    /// Retained for the lifetime of the sheet; cleared in the delegate
    /// callbacks once the user has resolved (success / cancel / failure).
    private var currentController: ASAuthorizationController?

    /// Strong reference to the per-flow delegate. Apple holds it weakly on the
    /// controller, so the delegate must outlive the controller somewhere —
    /// that's here.
    private var currentDelegate: AppleAuthDelegate?

    /// Strong reference to the per-flow presentation-context provider.
    /// `ASAuthorizationController.presentationContextProvider` is `weak`, so
    /// the provider must outlive the controller somewhere — that's here.
    private var currentPresentationProvider: PresentationProvider?

    // MARK: - signInWithApple

    @objc public nonisolated func signInWithApple(_ invoke: Invoke) {
        Task { @MainActor in
            self.handleSignIn(invoke)
        }
    }

    private func handleSignIn(_ invoke: Invoke) {
        let args: SignInRequest
        do {
            args = try invoke.parseArgs(SignInRequest.self)
        } catch {
            invoke.reject(
                "invalid request: \(error.localizedDescription)",
                code: "INVALID_REQUEST"
            )
            return
        }

        guard !args.rawNonce.isEmpty else {
            invoke.reject("rawNonce must not be empty", code: "INVALID_REQUEST")
            return
        }

        // Apple echoes whatever the client puts in `request.nonce` into the
        // ID token's `nonce` claim verbatim. Hashing here means the unhashed
        // value never leaves the caller and the backend has a single,
        // canonical comparison form (`base64url(sha256(rawNonce))`).
        let hashedNonce = sha256Base64URL(args.rawNonce)

        let provider = ASAuthorizationAppleIDProvider()
        let request = provider.createRequest()
        request.requestedScopes = [.fullName, .email]
        request.nonce = hashedNonce

        // Pick a presentation anchor before constructing the controller —
        // Apple insists on one for sheet presentation, and erroring up front
        // is clearer than letting `ASAuthorizationController` fail an
        // internal assertion.
        guard let anchor = resolvePresentationAnchor() else {
            invoke.reject(
                "no presentation anchor available",
                code: "NATIVE_UNAVAILABLE"
            )
            return
        }

        // Cancel any in-flight controller so its delegate fires with
        // `.canceled` and the prior invoke resolves rather than dangling.
        resetCurrentFlow()

        let controller = ASAuthorizationController(authorizationRequests: [request])
        let delegate = AppleAuthDelegate { [weak self] outcome in
            Task { @MainActor in
                self?.resetCurrentFlow()
                invoke.resolve(outcome)
            }
        }
        let presentation = PresentationProvider(anchor: anchor)

        controller.delegate = delegate
        controller.presentationContextProvider = presentation

        self.currentController = controller
        self.currentDelegate = delegate
        self.currentPresentationProvider = presentation

        controller.performRequests()
    }

    // MARK: - Internal helpers

    private func resetCurrentFlow() {
        currentController = nil
        currentDelegate = nil
        currentPresentationProvider = nil
    }

    /// Best-effort source for the `ASAuthorizationController` presentation
    /// anchor. Returns `nil` rather than a placeholder window, since
    /// `ASAuthorizationController` rejects unattached anchors.
    private func resolvePresentationAnchor() -> ASPresentationAnchor? {
        if let window = manager.viewController?.view.window {
            return window
        }
        return foregroundKeyWindow()
    }

    private func foregroundKeyWindow() -> UIWindow? {
        return UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .filter { $0.activationState == .foregroundActive }
            .flatMap { $0.windows }
            .first(where: { $0.isKeyWindow })
    }
}

// MARK: - Request decoding

private struct SignInRequest: Decodable {
    let rawNonce: String
}

// MARK: - Outcome encoding

/// Mirrors `crate::AppleSignInOutcome` (`#[serde(tag = "kind",
/// rename_all = "snake_case")]`).
private enum SignInOutcome: Encodable {
    case success(SignInResponse)
    case cancelled
    case rejected(reason: String)
    case nativeUnavailable

    private enum CodingKeys: String, CodingKey {
        case kind
        case idToken
        case authorizationCode
        case user
        case reason
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .success(let response):
            try container.encode("success", forKey: .kind)
            try container.encode(response.idToken, forKey: .idToken)
            if let code = response.authorizationCode {
                try container.encode(code, forKey: .authorizationCode)
            }
            if let user = response.user {
                try container.encode(user, forKey: .user)
            }
        case .cancelled:
            try container.encode("cancelled", forKey: .kind)
        case .rejected(let reason):
            try container.encode("rejected", forKey: .kind)
            try container.encode(reason, forKey: .reason)
        case .nativeUnavailable:
            try container.encode("native_unavailable", forKey: .kind)
        }
    }
}

private struct SignInResponse {
    let idToken: String
    let authorizationCode: String?
    let user: AppleNativeUser?
}

/// Mirrors `crate::AppleNativeUser`. Both fields are camelCase on the wire
/// to match the Rust models' `#[serde(rename_all = "camelCase")]`.
private struct AppleNativeUser: Encodable {
    let firstName: String?
    let lastName: String?
}

// MARK: - Delegate

/// `ASAuthorizationControllerDelegate` adapter that funnels every terminal
/// callback into a single `(SignInOutcome) -> Void` continuation. Keeps the
/// plugin class itself free of Apple-specific delegate noise.
private final class AppleAuthDelegate: NSObject, ASAuthorizationControllerDelegate {

    typealias Resolve = (SignInOutcome) -> Void

    private let resolve: Resolve

    init(resolve: @escaping Resolve) {
        self.resolve = resolve
        super.init()
    }

    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithAuthorization authorization: ASAuthorization
    ) {
        guard let credential = authorization.credential as? ASAuthorizationAppleIDCredential else {
            resolve(.rejected(reason: "unexpected credential type"))
            return
        }
        guard
            let idTokenData = credential.identityToken,
            let idToken = String(data: idTokenData, encoding: .utf8),
            !idToken.isEmpty
        else {
            resolve(.rejected(reason: "missing id_token"))
            return
        }

        let authorizationCode: String? = credential.authorizationCode
            .flatMap { String(data: $0, encoding: .utf8) }
            .flatMap { $0.isEmpty ? nil : $0 }

        // `fullName` is a `PersonNameComponents?`; Apple only populates it on
        // the very first sign-in for a given user. Subsequent flows leave it
        // nil, and the backend's display-name guard ensures we never
        // overwrite an existing name even if a malicious client fabricates
        // one here.
        let user: AppleNativeUser? = credential.fullName.map { name in
            AppleNativeUser(firstName: name.givenName, lastName: name.familyName)
        }.flatMap { native -> AppleNativeUser? in
            if native.firstName == nil && native.lastName == nil {
                return nil
            }
            return native
        }

        resolve(.success(SignInResponse(
            idToken: idToken,
            authorizationCode: authorizationCode,
            user: user
        )))
    }

    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithError error: Error
    ) {
        let nsError = error as NSError
        if nsError.domain == ASAuthorizationErrorDomain,
           let code = ASAuthorizationError.Code(rawValue: nsError.code) {
            switch code {
            case .canceled:
                resolve(.cancelled)
                return
            case .notHandled, .failed, .invalidResponse, .unknown, .notInteractive:
                resolve(.rejected(reason: nsError.localizedDescription))
                return
            @unknown default:
                resolve(.rejected(reason: nsError.localizedDescription))
                return
            }
        }
        resolve(.rejected(reason: nsError.localizedDescription))
    }
}

// MARK: - Presentation anchor

private final class PresentationProvider: NSObject, ASAuthorizationControllerPresentationContextProviding {
    private let anchor: ASPresentationAnchor

    init(anchor: ASPresentationAnchor) {
        self.anchor = anchor
        super.init()
    }

    func presentationAnchor(
        for controller: ASAuthorizationController
    ) -> ASPresentationAnchor {
        return anchor
    }
}

// MARK: - Nonce hashing

/// Compute `base64url(sha256(input_utf8))` with **no padding** — matches
/// `crates/backend/be-auth-service/src/oauth_flow.rs::expected_apple_native_nonce`
/// so the byte-equal comparison on the backend succeeds.
private func sha256Base64URL(_ input: String) -> String {
    let digest = SHA256.hash(data: Data(input.utf8))
    let bytes = Data(digest)
    let base64 = bytes.base64EncodedString()
    // Standard base64 → base64url, then strip padding.
    return base64
        .replacingOccurrences(of: "+", with: "-")
        .replacingOccurrences(of: "/", with: "_")
        .replacingOccurrences(of: "=", with: "")
}

@_cdecl("init_plugin_apple_auth")
func initPlugin() -> Plugin {
    return AppleAuthPlugin()
}
