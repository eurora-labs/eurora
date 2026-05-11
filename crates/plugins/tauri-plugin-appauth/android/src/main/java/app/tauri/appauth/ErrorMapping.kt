// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import net.openid.appauth.AuthorizationException

/// Maps AppAuth-Android `AuthorizationException`s onto the stable error codes
/// the Rust crate publishes via `Error::code()`.
///
/// AppAuth groups exceptions into four type categories that mirror the OAuth
/// spec's error surface (general / authorization / token / registration). The
/// `error` and `errorDescription` fields are surfaced verbatim to the JS layer
/// so callers don't have to parse free-form messages.
object ErrorMapping {

    const val CODE_USER_CANCELED = "USER_CANCELED"
    const val CODE_AUTHORIZATION_FAILED = "AUTHORIZATION_FAILED"
    const val CODE_TOKEN_EXCHANGE_FAILED = "TOKEN_EXCHANGE_FAILED"
    const val CODE_NETWORK_ERROR = "NETWORK_ERROR"
    const val CODE_INVALID_REGISTRATION_RESPONSE = "INVALID_REGISTRATION_RESPONSE"
    const val CODE_ID_TOKEN_VALIDATION_FAILED = "ID_TOKEN_VALIDATION_FAILED"
    const val CODE_BROWSER_NOT_AVAILABLE = "BROWSER_NOT_AVAILABLE"
    const val CODE_INVALID_REQUEST = "INVALID_REQUEST"
    const val CODE_SERVER_ERROR = "SERVER_ERROR"

    /// Reject `invoke` with the appropriate code/message/oauth-context combo.
    fun reject(invoke: Invoke, ex: AuthorizationException) {
        val code = when (ex.type) {
            AuthorizationException.TYPE_GENERAL_ERROR -> mapGeneral(ex)
            AuthorizationException.TYPE_OAUTH_AUTHORIZATION_ERROR -> mapAuthorization(ex)
            AuthorizationException.TYPE_OAUTH_TOKEN_ERROR -> CODE_TOKEN_EXCHANGE_FAILED
            AuthorizationException.TYPE_OAUTH_REGISTRATION_ERROR -> CODE_INVALID_REGISTRATION_RESPONSE
            AuthorizationException.TYPE_RESOURCE_SERVER_AUTHORIZATION_ERROR -> CODE_AUTHORIZATION_FAILED
            else -> CODE_AUTHORIZATION_FAILED
        }
        val message = ex.errorDescription ?: ex.error ?: ex.message ?: code
        val data = JSObject().apply {
            ex.error?.let { put("oauthError", it) }
            ex.errorDescription?.let { put("oauthErrorDescription", it) }
        }
        invoke.reject(message, code, ex, if (data.length() > 0) data else null)
    }

    /// `TYPE_GENERAL_ERROR` covers transport, browser, and validation failures.
    /// We split them into the per-cause buckets the JS layer needs.
    private fun mapGeneral(ex: AuthorizationException): String {
        return when (ex.code) {
            AuthorizationException.GeneralErrors.USER_CANCELED_AUTH_FLOW.code,
            AuthorizationException.GeneralErrors.PROGRAM_CANCELED_AUTH_FLOW.code -> CODE_USER_CANCELED

            AuthorizationException.GeneralErrors.NETWORK_ERROR.code -> CODE_NETWORK_ERROR

            AuthorizationException.GeneralErrors.SERVER_ERROR.code,
            AuthorizationException.GeneralErrors.INVALID_DISCOVERY_DOCUMENT.code,
            AuthorizationException.GeneralErrors.JSON_DESERIALIZATION_ERROR.code -> CODE_SERVER_ERROR

            AuthorizationException.GeneralErrors.ID_TOKEN_PARSING_ERROR.code,
            AuthorizationException.GeneralErrors.ID_TOKEN_VALIDATION_ERROR.code -> CODE_ID_TOKEN_VALIDATION_FAILED

            else -> CODE_AUTHORIZATION_FAILED
        }
    }

    private fun mapAuthorization(ex: AuthorizationException): String {
        // `access_denied` (USER_CANCELED equivalent at the OAuth level) is the
        // only authorization-endpoint error we promote out of AUTHORIZATION_FAILED
        // — every other code is a server-side denial that callers handle the
        // same way (display the OAuth error code from `data.oauthError`).
        return if (ex.error == "access_denied") CODE_USER_CANCELED else CODE_AUTHORIZATION_FAILED
    }
}
