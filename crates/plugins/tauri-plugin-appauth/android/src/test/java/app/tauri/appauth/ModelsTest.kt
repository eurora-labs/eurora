// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import com.fasterxml.jackson.annotation.JsonAutoDetect
import com.fasterxml.jackson.annotation.PropertyAccessor
import com.fasterxml.jackson.databind.DeserializationFeature
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

/// `Models.kt` is the wire shape between the Rust crate (via Tauri's IPC),
/// the JS layer, and AppAuth-Android. These tests pin the shapes the Rust
/// side actually emits so a Kotlin refactor cannot silently break the bridge.
///
/// The mapper configuration mirrors `PluginManager.jsonMapper` (the runtime
/// instance the plugin actually receives) so test behaviour matches prod
/// behaviour — most importantly `FAIL_ON_UNKNOWN_PROPERTIES = false`, on
/// which the deletion of `prefersEphemeralSession` from the Android arg
/// classes (Phase 2.4) relies.
class ModelsTest {

    private val mapper = ObjectMapper()
        .disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES)
        .enable(DeserializationFeature.FAIL_ON_NULL_FOR_PRIMITIVES)
        .setVisibility(PropertyAccessor.FIELD, JsonAutoDetect.Visibility.ANY)

    // MARK: - ConfigSource polymorphic dispatch

    @Test
    fun configSourceDiscoveryDeserializes() {
        val json = """{"kind":"discovery","issuer":"https://issuer.example.com"}"""
        val source = mapper.readValue(json, ConfigSource::class.java)
        val discovery = source as ConfigSource.Discovery
        assertEquals("https://issuer.example.com", discovery.issuer)
    }

    @Test
    fun configSourceExplicitWithAllFieldsDeserializes() {
        val json = """
            {
                "kind": "explicit",
                "authorizationEndpoint": "https://auth.example.com/oauth/authorize",
                "tokenEndpoint": "https://auth.example.com/oauth/token",
                "endSessionEndpoint": "https://auth.example.com/oauth/logout",
                "registrationEndpoint": "https://auth.example.com/oauth/register"
            }
        """.trimIndent()
        val source = mapper.readValue(json, ConfigSource::class.java)
        val explicit = source as ConfigSource.Explicit
        assertEquals("https://auth.example.com/oauth/authorize", explicit.authorizationEndpoint)
        assertEquals("https://auth.example.com/oauth/token", explicit.tokenEndpoint)
        assertEquals("https://auth.example.com/oauth/logout", explicit.endSessionEndpoint)
        assertEquals("https://auth.example.com/oauth/register", explicit.registrationEndpoint)
    }

    @Test
    fun configSourceExplicitWithoutOptionalFields() {
        val json = """
            {
                "kind": "explicit",
                "authorizationEndpoint": "https://auth.example.com/oauth/authorize",
                "tokenEndpoint": "https://auth.example.com/oauth/token"
            }
        """.trimIndent()
        val source = mapper.readValue(json, ConfigSource::class.java)
        val explicit = source as ConfigSource.Explicit
        assertNull(explicit.endSessionEndpoint)
        assertNull(explicit.registrationEndpoint)
    }

    @Test
    fun configSourceUnknownKindThrows() {
        val json = """{"kind":"telepathy","issuer":"https://issuer.example.com"}"""
        assertThrows(com.fasterxml.jackson.databind.exc.InvalidTypeIdException::class.java) {
            mapper.readValue(json, ConfigSource::class.java)
        }
    }

    @Test
    fun configSourceMissingKindThrows() {
        val json = """{"issuer":"https://issuer.example.com"}"""
        assertThrows(com.fasterxml.jackson.databind.exc.InvalidTypeIdException::class.java) {
            mapper.readValue(json, ConfigSource::class.java)
        }
    }

    // MARK: - Prompt

    @Test
    fun promptFromStringResolvesEveryDefinedValue() {
        for (variant in Prompt.entries) {
            assertEquals(variant, Prompt.fromString(variant.value))
        }
    }

    @Test
    fun promptFromStringReturnsNullForUnknown() {
        assertNull(Prompt.fromString("nope"))
        assertNull(Prompt.fromString(""))
    }

    @Test
    fun promptDeserializesFromString() {
        // `@JsonCreator` on `fromString` drives Jackson's String → enum mapping.
        val prompt = mapper.readValue(""""select_account"""", Prompt::class.java)
        assertEquals(Prompt.SELECT_ACCOUNT, prompt)
    }

    @Test
    fun promptSerializesToString() {
        // `@JsonValue` on `toValue()` makes the encoded form a bare string,
        // matching the Rust `#[serde(rename_all = "snake_case")]` shape.
        assertEquals(""""consent"""", mapper.writeValueAsString(Prompt.CONSENT))
    }

    // MARK: - AuthEvent wire shape

    @Test
    fun authEventBrowserOpenedSerializes() {
        assertEquals(
            """{"kind":"browserOpened"}""",
            mapper.writeValueAsString(AuthEvent.BROWSER_OPENED),
        )
    }

    @Test
    fun authEventRedirectInterceptedSerializes() {
        assertEquals(
            """{"kind":"redirectIntercepted"}""",
            mapper.writeValueAsString(AuthEvent.REDIRECT_INTERCEPTED),
        )
    }

    @Test
    fun authEventTokenExchangeStartedSerializes() {
        assertEquals(
            """{"kind":"tokenExchangeStarted"}""",
            mapper.writeValueAsString(AuthEvent.TOKEN_EXCHANGE_STARTED),
        )
    }

    @Test
    fun authEventTokenExchangeCompletedSerializes() {
        assertEquals(
            """{"kind":"tokenExchangeCompleted"}""",
            mapper.writeValueAsString(AuthEvent.TOKEN_EXCHANGE_COMPLETED),
        )
    }

    // MARK: - Lenient unknown-property handling

    @Test
    fun authorizeArgsIgnoresUnknownFields() {
        // `prefersEphemeralSession` was deleted from the Android arg classes
        // (Phase 2.4); the JS layer still emits it on every call, so unknown-
        // property leniency is load-bearing.
        val json = """
            {
                "config": {"kind": "discovery", "issuer": "https://issuer.example.com"},
                "clientId": "client-123",
                "redirectUri": "com.example.app:/oauth/callback",
                "prefersEphemeralSession": true,
                "futureFieldFromANewerJsRuntime": "ignore me"
            }
        """.trimIndent()
        val args = mapper.readValue(json, AuthorizeArgs::class.java)
        assertEquals("client-123", args.clientId)
        assertEquals("com.example.app:/oauth/callback", args.redirectUri)
    }
}
