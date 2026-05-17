// SPDX-License-Identifier: Apache-2.0

import AuthenticationServices
import UIKit

// `Tauri.Invoke` and AppAuth's response objects are non-Sendable @objc types
// without strict-concurrency annotations. Our flow captures them across the
// IPC-queue → main-actor boundary on purpose (each `Invoke` is resolved by
// exactly one callback path), so we silence the cross-module Sendable
// warnings here rather than scatter `nonisolated(unsafe)` through the file.
@preconcurrency import AppAuth
@preconcurrency import Tauri

/// Tauri 2 mobile plugin that bridges OAuth 2.0 / OIDC flows to AppAuth-iOS.
///
/// AppAuth owns PKCE (S256), `state`/`nonce` validation, discovery, code-for-token
/// exchange, refresh, and end-session. This class is glue: parse the JS payload,
/// run AppAuth on the main actor, and translate the AppAuth callback shape into
/// Tauri's `Invoke` resolution / rejection model.
///
/// Concurrency: the class is `@MainActor`, so `currentSession`,
/// `currentBrowserSession`, `currentBrowserPresentationContext`, and
/// `eventChannel` are accessed from a single isolation domain — no manual
/// queue management is needed for instance state. Tauri dispatches `@objc`
/// commands on its own IPC queue; each entry point is `nonisolated` and hops
/// to main via `Task { @MainActor in … }` before touching `self`. AppAuth's
/// network callbacks fire on URLSession's delegate queue (background), so
/// each callback body re-enters the main actor with the same pattern.
///
/// Long-running flows (`authorize`, `authorizeBrowserOnly`, `endSession`) hold
/// their session on `self` so the underlying browser process is not deallocated
/// while the user is still interacting with it. Starting a new flow cancels the
/// in-flight one — see `resetCurrentSession()`.
@MainActor
class AppAuthPlugin: Plugin {

    /// Active AppAuth-managed user-agent session for `authorize` / `endSession`.
    /// Retained so AppAuth can drive the browser sheet to completion or cancel.
    private var currentSession: OIDExternalUserAgentSession?

    /// `ASWebAuthenticationSession` used by the bare `authorizeBrowserOnly` flow.
    /// We use ASWeb directly here — there is no PKCE/state/nonce/token-exchange
    /// to drive, so AppAuth's full state machine is not the right primitive.
    private var currentBrowserSession: ASWebAuthenticationSession?

    /// Strong reference to the per-session presentation-context provider.
    /// `ASWebAuthenticationSession.presentationContextProvider` is `weak`, so
    /// the provider must outlive the session somewhere — that's here.
    private var currentBrowserPresentationContext: BrowserPresentationContext?

    /// Channel registered via `subscribeEvents`. Diagnostic events (browser
    /// opened, redirect intercepted, token-exchange progress) are emitted here.
    private var eventChannel: Channel?

    // MARK: - subscribeEvents

    @objc public nonisolated func subscribeEvents(_ invoke: Invoke) {
        Task { @MainActor in
            struct Args: Decodable { let channel: Channel }
            do {
                let args = try invoke.parseArgs(Args.self)
                self.eventChannel = args.channel
                invoke.resolve()
            } catch {
                invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
            }
        }
    }

    // MARK: - discover

    @objc public nonisolated func discover(_ invoke: Invoke) {
        Task { @MainActor in
            let args: DiscoverRequest
            do {
                args = try invoke.parseArgs(DiscoverRequest.self)
            } catch {
                invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
                return
            }
            guard let issuerURL = URL(string: args.issuer) else {
                invoke.reject("invalid issuer URL: \(args.issuer)", code: ErrorMapping.codeInvalidRequest)
                return
            }
            do {
                let config = try await ConfigSource.discover(issuerURL: issuerURL)
                invoke.resolve(ServiceConfigurationResponse(from: config))
            } catch {
                ErrorMapping.reject(invoke, error: error)
            }
        }
    }

    // MARK: - authorize

    @objc public nonisolated func authorize(_ invoke: Invoke) {
        Task { @MainActor in
            await self.handleAuthorize(invoke)
        }
    }

    private func handleAuthorize(_ invoke: Invoke) async {
        let args: AuthorizeRequest
        do {
            args = try invoke.parseArgs(AuthorizeRequest.self)
        } catch {
            invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
            return
        }

        let redirectURL: URL
        do {
            redirectURL = try validateRedirect(args.redirectUri)
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }

        let config: OIDServiceConfiguration
        do {
            config = try await args.config.resolve()
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }

        startAuthorize(invoke: invoke, args: args, config: config, redirectURL: redirectURL)
    }

    private func startAuthorize(
        invoke: Invoke,
        args: AuthorizeRequest,
        config: OIDServiceConfiguration,
        redirectURL: URL
    ) {
        let additionalParameters = buildAuthorizationParameters(args)

        // The two-arg `nonce` convenience initializer keeps AppAuth's auto
        // state-generation and PKCE while letting callers opt out of the OIDC
        // nonce (some non-OIDC providers reject it). When `useNonce` is true we
        // let AppAuth generate one for us.
        let request: OIDAuthorizationRequest
        if args.useNonce {
            request = OIDAuthorizationRequest(
                configuration: config,
                clientId: args.clientId,
                scopes: args.scopes.isEmpty ? nil : args.scopes,
                redirectURL: redirectURL,
                responseType: OIDResponseTypeCode,
                additionalParameters: additionalParameters
            )
        } else {
            request = OIDAuthorizationRequest(
                configuration: config,
                clientId: args.clientId,
                scopes: args.scopes.isEmpty ? nil : args.scopes,
                redirectURL: redirectURL,
                responseType: OIDResponseTypeCode,
                nonce: nil,
                additionalParameters: additionalParameters
            )
        }

        guard let presenter = self.presentationViewController() else {
            invoke.reject("no presenting view controller available", code: ErrorMapping.codeBrowserNotAvailable)
            return
        }
        // Construct the external user agent ourselves rather than going
        // through `OIDAuthState.authState(byPresenting:presenting:prefersEphemeralSession:callback:)`.
        // That convenience method lives in the iOS-specific `OIDAuthState
        // (IOS)` Objective-C category, and Tauri statically links AppAuth
        // into `libapp.a`. The Mach-O linker only pulls `.o` files from a
        // static archive when one of their non-category symbols is
        // referenced, so the category file gets dropped and the selector
        // goes missing at runtime (`NSInvalidArgumentException:
        // unrecognized selector sent to class`). Going through the regular
        // class methods on `OIDExternalUserAgentIOS` and
        // `OIDAuthState.authState(byPresenting:externalUserAgent:callback:)`
        // hits only non-category symbols, which the linker keeps without
        // any `-ObjC` / `-force_load` workarounds in the host app.
        guard let userAgent = OIDExternalUserAgentIOS(
            presenting: presenter,
            prefersEphemeralSession: args.prefersEphemeralSession
        ) else {
            invoke.reject(
                "could not initialize external user agent",
                code: ErrorMapping.codeBrowserNotAvailable
            )
            return
        }

        // Cancel any in-flight AppAuth session so its callback fires with
        // `programCanceledAuthorizationFlow` (mapped to `USER_CANCELED`)
        // and the prior `Invoke` resolves rather than dangling forever.
        self.resetCurrentSession()

        self.emit(.browserOpened)

        self.currentSession = OIDAuthState.authState(
            byPresenting: request,
            externalUserAgent: userAgent
        ) { authState, error in
            // AppAuth's network callback fires on URLSession's delegate queue;
            // hop back to the main actor before touching `self` or `invoke`.
            Task { @MainActor in
                if let error = error {
                    ErrorMapping.reject(invoke, error: error)
                    return
                }
                guard let authState = authState else {
                    invoke.reject(
                        "authorization completed without a state",
                        code: ErrorMapping.codeAuthorizationFailed
                    )
                    return
                }
                self.emit(.tokenExchangeCompleted)
                invoke.resolve(AuthStateResponse(from: authState))
            }
        }
    }

    private func buildAuthorizationParameters(_ args: AuthorizeRequest) -> [String: String] {
        var parameters = args.additionalParameters
        if let prompt = args.prompt {
            parameters["prompt"] = prompt.rawValue
        }
        if let loginHint = args.loginHint {
            parameters["login_hint"] = loginHint
        }
        if let uiLocales = args.uiLocales, !uiLocales.isEmpty {
            parameters["ui_locales"] = uiLocales.joined(separator: " ")
        }
        return parameters
    }

    // MARK: - authorizeBrowserOnly

    @objc public nonisolated func authorizeBrowserOnly(_ invoke: Invoke) {
        Task { @MainActor in
            self.handleAuthorizeBrowserOnly(invoke)
        }
    }

    private func handleAuthorizeBrowserOnly(_ invoke: Invoke) {
        let args: BrowserOnlyRequest
        do {
            args = try invoke.parseArgs(BrowserOnlyRequest.self)
        } catch {
            invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
            return
        }
        guard let authURL = URL(string: args.authUrl) else {
            invoke.reject("invalid auth URL: \(args.authUrl)", code: ErrorMapping.codeInvalidRequest)
            return
        }

        let redirectURL: URL
        do {
            redirectURL = try validateRedirect(args.redirectUri)
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }
        guard let kind = redirectKind(for: redirectURL) else {
            invoke.reject(
                "redirect URI is not a custom scheme or HTTPS Universal Link: \(args.redirectUri)",
                code: ErrorMapping.codeInvalidRequest
            )
            return
        }

        // Cancel any previous browser session before starting a new one so
        // the system doesn't reject the second call with `.canceledLogin`.
        self.resetCurrentBrowserSession()

        guard let anchor = self.resolvePresentationAnchor() else {
            invoke.reject(
                "no presentation anchor available for the browser session",
                code: ErrorMapping.codeBrowserNotAvailable
            )
            return
        }

        // Capture `self` strongly inside the completion handlers below: the
        // session retains the closure, the closure retains `self`, and the
        // cycle breaks when the callback fires. This guarantees `invoke`
        // always resolves — using `[weak self]` here would re-introduce
        // the orphan-`Invoke` bug that 1.1 closes.
        let session: ASWebAuthenticationSession
        switch kind {
        case .customScheme(let scheme):
            session = ASWebAuthenticationSession(
                url: authURL,
                callbackURLScheme: scheme
            ) { callbackURL, error in
                Task { @MainActor in
                    self.handleBrowserOnlyCompletion(invoke: invoke, callbackURL: callbackURL, error: error)
                }
            }

        case .universalLink(let host, let path):
            guard #available(iOS 17.4, *) else {
                invoke.reject(
                    "HTTPS Universal Link redirects require iOS 17.4 or later",
                    code: ErrorMapping.codeInvalidRequest
                )
                return
            }
            session = ASWebAuthenticationSession(
                url: authURL,
                callback: .https(host: host, path: path)
            ) { callbackURL, error in
                Task { @MainActor in
                    self.handleBrowserOnlyCompletion(invoke: invoke, callbackURL: callbackURL, error: error)
                }
            }
        }

        let presentationContext = BrowserPresentationContext(anchor: anchor)
        session.presentationContextProvider = presentationContext
        session.prefersEphemeralWebBrowserSession = args.prefersEphemeralSession

        self.currentBrowserSession = session
        self.currentBrowserPresentationContext = presentationContext

        if session.start() {
            self.emit(.browserOpened)
        } else {
            self.currentBrowserSession = nil
            self.currentBrowserPresentationContext = nil
            invoke.reject(
                "could not start an authentication session",
                code: ErrorMapping.codeBrowserNotAvailable
            )
        }
    }

    private func handleBrowserOnlyCompletion(invoke: Invoke, callbackURL: URL?, error: Error?) {
        if let error = error {
            handleBrowserOnlyError(invoke: invoke, error: error)
            return
        }
        guard let callbackURL = callbackURL else {
            invoke.reject(
                "browser session ended without a redirect",
                code: ErrorMapping.codeAuthorizationFailed
            )
            return
        }
        emit(.redirectIntercepted)
        invoke.resolve(BrowserOnlyResponse(url: callbackURL.absoluteString))
    }

    private func handleBrowserOnlyError(invoke: Invoke, error: Error) {
        let nsError = error as NSError
        if nsError.domain == ASWebAuthenticationSessionErrorDomain
            && nsError.code == ASWebAuthenticationSessionError.canceledLogin.rawValue
        {
            invoke.reject(error.localizedDescription, code: ErrorMapping.codeUserCanceled)
            return
        }
        ErrorMapping.reject(invoke, error: error)
    }

    // MARK: - refresh

    @objc public nonisolated func refresh(_ invoke: Invoke) {
        Task { @MainActor in
            await self.handleRefresh(invoke)
        }
    }

    private func handleRefresh(_ invoke: Invoke) async {
        let args: RefreshRequest
        do {
            args = try invoke.parseArgs(RefreshRequest.self)
        } catch {
            invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
            return
        }

        let config: OIDServiceConfiguration
        do {
            config = try await args.config.resolve()
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }

        let scope = args.scopes.isEmpty ? nil : args.scopes.joined(separator: " ")
        let tokenRequest = OIDTokenRequest(
            configuration: config,
            grantType: OIDGrantTypeRefreshToken,
            authorizationCode: nil,
            redirectURL: nil,
            clientID: args.clientId,
            clientSecret: nil,
            scope: scope,
            refreshToken: args.refreshToken,
            codeVerifier: nil,
            additionalParameters: args.additionalParameters
        )

        self.emit(.tokenExchangeStarted)

        OIDAuthorizationService.perform(tokenRequest) { [weak self] response, error in
            Task { @MainActor in
                if let error = error {
                    ErrorMapping.reject(invoke, error: error)
                    return
                }
                guard let response = response else {
                    invoke.reject(
                        "token endpoint returned no response",
                        code: ErrorMapping.codeTokenExchangeFailed
                    )
                    return
                }
                self?.emit(.tokenExchangeCompleted)
                invoke.resolve(AuthStateResponse(from: response))
            }
        }
    }

    // MARK: - endSession

    @objc public nonisolated func endSession(_ invoke: Invoke) {
        Task { @MainActor in
            await self.handleEndSession(invoke)
        }
    }

    private func handleEndSession(_ invoke: Invoke) async {
        let args: EndSessionRequest
        do {
            args = try invoke.parseArgs(EndSessionRequest.self)
        } catch {
            invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
            return
        }

        let postLogoutURL: URL
        do {
            postLogoutURL = try validateRedirect(args.postLogoutRedirectUri)
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }

        let config: OIDServiceConfiguration
        do {
            config = try await args.config.resolve()
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }

        startEndSession(
            invoke: invoke,
            args: args,
            config: config,
            postLogoutURL: postLogoutURL
        )
    }

    private func startEndSession(
        invoke: Invoke,
        args: EndSessionRequest,
        config: OIDServiceConfiguration,
        postLogoutURL: URL
    ) {
        // AppAuth-iOS annotates `idTokenHint:` as non-nullable in Swift even
        // though the Objective-C implementation accepts nil and only emits
        // `id_token_hint` to the URL when its backing ivar is non-nil. Pass a
        // placeholder when absent and immediately clear the ivar through KVC
        // so RFC 8665 RP-Initiated Logout works for IdPs that accept the
        // request without an ID token hint.
        let hintForInit = args.idTokenHint ?? ""
        let request: OIDEndSessionRequest
        if let state = args.state {
            request = OIDEndSessionRequest(
                configuration: config,
                idTokenHint: hintForInit,
                postLogoutRedirectURL: postLogoutURL,
                state: state,
                additionalParameters: args.additionalParameters
            )
        } else {
            request = OIDEndSessionRequest(
                configuration: config,
                idTokenHint: hintForInit,
                postLogoutRedirectURL: postLogoutURL,
                additionalParameters: args.additionalParameters
            )
        }
        if args.idTokenHint == nil {
            request.setValue(nil, forKey: "idTokenHint")
        }

        guard let presenter = self.presentationViewController() else {
            invoke.reject("no presenting view controller available", code: ErrorMapping.codeBrowserNotAvailable)
            return
        }
        guard let userAgent = OIDExternalUserAgentIOS(
            presenting: presenter,
            prefersEphemeralSession: args.prefersEphemeralSession
        ) else {
            invoke.reject("could not initialize external user agent", code: ErrorMapping.codeBrowserNotAvailable)
            return
        }

        self.resetCurrentSession()

        self.emit(.browserOpened)

        self.currentSession = OIDAuthorizationService.present(
            request,
            externalUserAgent: userAgent
        ) { response, error in
            Task { @MainActor in
                if let error = error {
                    ErrorMapping.reject(invoke, error: error)
                    return
                }

                self.emit(.redirectIntercepted)
                // `OIDEndSessionResponse` does not surface the raw redirect URL;
                // mirror the configured post-logout URL so callers can confirm
                // the round-trip without parsing free-form messages.
                invoke.resolve(EndSessionResponseModel(
                    url: postLogoutURL.absoluteString,
                    state: response?.state
                ))
            }
        }
    }

    // MARK: - register (RFC 7591 dynamic client registration)

    @objc public nonisolated func register(_ invoke: Invoke) {
        Task { @MainActor in
            await self.handleRegister(invoke)
        }
    }

    private func handleRegister(_ invoke: Invoke) async {
        let args: RegisterRequest
        do {
            args = try invoke.parseArgs(RegisterRequest.self)
        } catch {
            invoke.reject("invalid request: \(error.localizedDescription)", code: ErrorMapping.codeInvalidRequest)
            return
        }

        let redirectURLs: [URL]
        do {
            redirectURLs = try args.redirectUris.map(validateRedirect)
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }
        if redirectURLs.isEmpty {
            invoke.reject("at least one redirect URI is required", code: ErrorMapping.codeInvalidRequest)
            return
        }

        // OIDC discovery advertises `subject_types_supported` as a list, but a
        // single registration commits to exactly one value — and AppAuth-iOS's
        // `subjectType` is a single string. Reject up front rather than
        // silently dropping later entries, so callers see the contract clearly.
        if args.subjectTypes.count > 1 {
            invoke.reject(
                "subjectTypes accepts at most one value (got \(args.subjectTypes.count)); a single registration commits to one subject type",
                code: ErrorMapping.codeInvalidRequest
            )
            return
        }

        let config: OIDServiceConfiguration
        do {
            config = try await args.config.resolve()
        } catch {
            ErrorMapping.reject(invoke, error: error)
            return
        }

        let request = OIDRegistrationRequest(
            configuration: config,
            redirectURIs: redirectURLs,
            responseTypes: args.responseTypes.isEmpty ? nil : args.responseTypes,
            grantTypes: args.grantTypes.isEmpty ? nil : args.grantTypes,
            subjectType: args.subjectTypes.first,
            tokenEndpointAuthMethod: args.tokenEndpointAuthMethod,
            additionalParameters: args.additionalParameters
        )

        OIDAuthorizationService.perform(request) { response, error in
            Task { @MainActor in
                if let error = error {
                    ErrorMapping.reject(invoke, error: error)
                    return
                }
                guard let response = response else {
                    invoke.reject(
                        "registration endpoint returned no response",
                        code: ErrorMapping.codeInvalidRegistrationResponse
                    )
                    return
                }
                invoke.resolve(RegistrationResponseModel(from: response))
            }
        }
    }

    // MARK: - Internal helpers

    /// Cancel any in-flight AppAuth session so its callback fires with
    /// `programCanceledAuthorizationFlow` and the prior `Invoke` is rejected.
    /// Calling `cancel()` on a completed session is a no-op per AppAuth's API.
    private func resetCurrentSession() {
        currentSession?.cancel()
        currentSession = nil
    }

    /// Cancel any in-flight `ASWebAuthenticationSession`. Mirrors
    /// `resetCurrentSession()` for the browser-only flow.
    private func resetCurrentBrowserSession() {
        currentBrowserSession?.cancel()
        currentBrowserSession = nil
        currentBrowserPresentationContext = nil
    }

    /// Best-effort source for the AppAuth presentation view controller.
    ///
    /// The Tauri `PluginManager` populates `viewController` when the webview is
    /// created; in the rare case that the anchor is unavailable (background
    /// launches, scene transitions) we fall back to the active key window's
    /// root view controller. iPad multitasking note: the chosen window is the
    /// one currently in the foreground active state, which matches the
    /// expected user-visible session.
    private func presentationViewController() -> UIViewController? {
        if let viewController = manager.viewController {
            return viewController
        }
        return foregroundKeyWindow()?.rootViewController
    }

    /// Best-effort source for the `ASWebAuthenticationSession` presentation
    /// anchor. Returns `nil` rather than a placeholder `ASPresentationAnchor`,
    /// since `ASWebAuthenticationSession`'s behavior with an unattached window
    /// is undefined.
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

    private func emit(_ event: AuthEvent) {
        guard let channel = eventChannel else { return }
        do {
            try channel.send(event)
        } catch {
            // Diagnostic events are best-effort; never let serialization
            // failures break the underlying flow. Surface them in DEBUG so
            // problems are visible during development without leaking to
            // production logs.
            #if DEBUG
            print("AppAuthPlugin: event emission failed for \(event.rawValue): \(error)")
            #endif
        }
    }
}

// MARK: - Diagnostic events

/// Mirrors `crate::events::AuthEvent` (tagged on `kind`, camelCase).
enum AuthEvent: String, Encodable {
    case browserOpened
    case redirectIntercepted
    case tokenExchangeStarted
    case tokenExchangeCompleted

    private enum CodingKeys: String, CodingKey { case kind }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(rawValue, forKey: .kind)
    }
}

// MARK: - ASWebAuthenticationSession presentation anchor (iOS 13+)

/// Owns the `ASPresentationAnchor` for a single `ASWebAuthenticationSession`.
/// Built per session so we never fall back to a placeholder anchor — the plugin
/// rejects up front when no real anchor can be resolved.
private final class BrowserPresentationContext: NSObject, ASWebAuthenticationPresentationContextProviding {
    private let anchor: ASPresentationAnchor

    init(anchor: ASPresentationAnchor) {
        self.anchor = anchor
        super.init()
    }

    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        return anchor
    }
}

@_cdecl("init_plugin_appauth")
func initPlugin() -> Plugin {
    return AppAuthPlugin()
}
