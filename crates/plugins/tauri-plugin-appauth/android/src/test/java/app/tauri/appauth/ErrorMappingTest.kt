// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import net.openid.appauth.AuthorizationException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/// `ErrorMapping.reject` is the only authoritative source for the `code`
/// strings the JS layer pattern-matches on. One test per `TYPE_*` branch and
/// per `GeneralErrors` bucket pins those mappings against silent breakage.
///
/// Robolectric supplies the Android runtime dependencies that `Invoke.reject`
/// pulls in — `org.json.JSONObject` (for `JSObject`) and `android.util.Log`
/// (for `Logger.error`). A fake host or hand-rolled mock is not enough.
@RunWith(RobolectricTestRunner::class)
class ErrorMappingTest {

    // MARK: - Type buckets

    @Test
    fun generalUserCanceledMapsToUserCanceled() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.USER_CANCELED_AUTH_FLOW,
        )
        assertEquals(ErrorMapping.CODE_USER_CANCELED, recorder.rejection().getString("code"))
    }

    @Test
    fun generalProgramCanceledMapsToUserCanceled() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.PROGRAM_CANCELED_AUTH_FLOW,
        )
        assertEquals(ErrorMapping.CODE_USER_CANCELED, recorder.rejection().getString("code"))
    }

    @Test
    fun generalNetworkErrorMapsToNetworkError() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.NETWORK_ERROR,
        )
        assertEquals(ErrorMapping.CODE_NETWORK_ERROR, recorder.rejection().getString("code"))
    }

    @Test
    fun generalServerErrorMapsToServerError() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.SERVER_ERROR,
        )
        assertEquals(ErrorMapping.CODE_SERVER_ERROR, recorder.rejection().getString("code"))
    }

    @Test
    fun generalInvalidDiscoveryDocumentMapsToServerError() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.INVALID_DISCOVERY_DOCUMENT,
        )
        assertEquals(ErrorMapping.CODE_SERVER_ERROR, recorder.rejection().getString("code"))
    }

    @Test
    fun generalJsonDeserializationMapsToServerError() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.JSON_DESERIALIZATION_ERROR,
        )
        assertEquals(ErrorMapping.CODE_SERVER_ERROR, recorder.rejection().getString("code"))
    }

    @Test
    fun generalIdTokenParsingMapsToIdTokenValidationFailed() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.ID_TOKEN_PARSING_ERROR,
        )
        assertEquals(
            ErrorMapping.CODE_ID_TOKEN_VALIDATION_FAILED,
            recorder.rejection().getString("code"),
        )
    }

    @Test
    fun generalIdTokenValidationMapsToIdTokenValidationFailed() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException.GeneralErrors.ID_TOKEN_VALIDATION_ERROR,
        )
        assertEquals(
            ErrorMapping.CODE_ID_TOKEN_VALIDATION_FAILED,
            recorder.rejection().getString("code"),
        )
    }

    @Test
    fun unknownGeneralCodeFallsBackToAuthorizationFailed() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_GENERAL_ERROR,
                /* code = */ -9999,
                /* error = */ null,
                /* errorDescription = */ "future AppAuth code",
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        val rejection = recorder.rejection()
        assertEquals(ErrorMapping.CODE_AUTHORIZATION_FAILED, rejection.getString("code"))
        assertEquals("future AppAuth code", rejection.getString("message"))
    }

    @Test
    fun oauthAuthorizationErrorMapsToAuthorizationFailed() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_OAUTH_AUTHORIZATION_ERROR,
                /* code = */ 1,
                /* error = */ "invalid_request",
                /* errorDescription = */ "missing client_id",
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        assertEquals(
            ErrorMapping.CODE_AUTHORIZATION_FAILED,
            recorder.rejection().getString("code"),
        )
    }

    @Test
    fun oauthTokenErrorMapsToTokenExchangeFailed() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_OAUTH_TOKEN_ERROR,
                /* code = */ 2,
                /* error = */ "invalid_grant",
                /* errorDescription = */ "code expired",
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        assertEquals(
            ErrorMapping.CODE_TOKEN_EXCHANGE_FAILED,
            recorder.rejection().getString("code"),
        )
    }

    @Test
    fun oauthRegistrationErrorMapsToInvalidRegistrationResponse() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_OAUTH_REGISTRATION_ERROR,
                /* code = */ 3,
                /* error = */ "invalid_redirect_uri",
                /* errorDescription = */ "scheme not registered",
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        assertEquals(
            ErrorMapping.CODE_INVALID_REGISTRATION_RESPONSE,
            recorder.rejection().getString("code"),
        )
    }

    @Test
    fun resourceServerAuthorizationErrorMapsToAuthorizationFailed() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_RESOURCE_SERVER_AUTHORIZATION_ERROR,
                /* code = */ 4,
                /* error = */ "insufficient_scope",
                /* errorDescription = */ null,
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        assertEquals(
            ErrorMapping.CODE_AUTHORIZATION_FAILED,
            recorder.rejection().getString("code"),
        )
    }

    // MARK: - access_denied promotion

    @Test
    fun accessDeniedPromotesToUserCanceled() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_OAUTH_AUTHORIZATION_ERROR,
                /* code = */ 1,
                /* error = */ "access_denied",
                /* errorDescription = */ "user dismissed the consent screen",
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        assertEquals(ErrorMapping.CODE_USER_CANCELED, recorder.rejection().getString("code"))
    }

    // MARK: - OAuth field propagation

    @Test
    fun rejectionPropagatesOauthErrorAndDescription() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_OAUTH_TOKEN_ERROR,
                /* code = */ 2,
                /* error = */ "invalid_client",
                /* errorDescription = */ "client authentication failed",
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        val rejection = recorder.rejection()
        assertEquals("client authentication failed", rejection.getString("message"))
        val data = rejection.getJSObject("data")!!
        assertEquals("invalid_client", data.getString("oauthError"))
        assertEquals("client authentication failed", data.getString("oauthErrorDescription"))
    }

    @Test
    fun rejectionMessageFallsBackToErrorWhenDescriptionMissing() {
        val recorder = RecordingInvoke()
        ErrorMapping.reject(
            recorder.invoke,
            AuthorizationException(
                AuthorizationException.TYPE_OAUTH_TOKEN_ERROR,
                /* code = */ 2,
                /* error = */ "unsupported_grant_type",
                /* errorDescription = */ null,
                /* errorUri = */ null,
                /* rootCause = */ null,
            ),
        )
        val rejection = recorder.rejection()
        assertEquals("unsupported_grant_type", rejection.getString("message"))
        val data = rejection.getJSObject("data")!!
        assertEquals("unsupported_grant_type", data.getString("oauthError"))
        assertFalse(
            "no description means the data must not carry oauthErrorDescription",
            data.has("oauthErrorDescription"),
        )
    }
}
