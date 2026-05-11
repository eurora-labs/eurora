// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import app.tauri.annotation.InvokeArg
import com.fasterxml.jackson.annotation.JsonCreator
import com.fasterxml.jackson.annotation.JsonIgnoreProperties
import com.fasterxml.jackson.annotation.JsonSubTypes
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonValue
import com.fasterxml.jackson.core.JsonGenerator
import com.fasterxml.jackson.databind.JsonSerializer
import com.fasterxml.jackson.databind.SerializerProvider
import com.fasterxml.jackson.databind.annotation.JsonSerialize

// MARK: - Inputs decoded from JS payloads

@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class DiscoverArgs {
    lateinit var issuer: String
}

/// Mirrors the Rust `ConfigSource` tagged union. Jackson dispatches on the
/// `kind` discriminator the Rust side writes via `#[serde(tag = "kind")]`.
@JsonTypeInfo(
    use = JsonTypeInfo.Id.NAME,
    include = JsonTypeInfo.As.PROPERTY,
    property = "kind"
)
@JsonSubTypes(
    JsonSubTypes.Type(value = ConfigSource.Discovery::class, name = "discovery"),
    JsonSubTypes.Type(value = ConfigSource.Explicit::class, name = "explicit"),
)
sealed class ConfigSource {
    @JsonIgnoreProperties(ignoreUnknown = true)
    @InvokeArg
    class Discovery : ConfigSource() {
        lateinit var issuer: String
    }

    @JsonIgnoreProperties(ignoreUnknown = true)
    @InvokeArg
    class Explicit : ConfigSource() {
        lateinit var authorizationEndpoint: String
        lateinit var tokenEndpoint: String
        var endSessionEndpoint: String? = null
        var registrationEndpoint: String? = null
    }
}

/// OIDC `prompt` parameter values. The Rust enum is `snake_case`; Jackson maps
/// the JSON strings onto these constants via `@JsonValue`-style passthrough.
enum class Prompt(val value: String) {
    LOGIN("login"),
    CONSENT("consent"),
    SELECT_ACCOUNT("select_account"),
    NONE("none");

    @JsonValue
    fun toValue(): String = value

    companion object {
        @JsonCreator
        @JvmStatic
        fun fromString(value: String): Prompt? =
            entries.firstOrNull { it.value == value }
    }
}

/// Custom Tabs has no equivalent of iOS's `prefersEphemeralSession`: it always
/// shares cookies with the user's default browser. The corresponding field
/// from the JS payload is therefore intentionally absent on the Android arg
/// classes — `@JsonIgnoreProperties(ignoreUnknown = true)` makes that contract
/// self-describing instead of relying on the Tauri runtime mapper's lenient
/// global default.
@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class AuthorizeArgs {
    lateinit var config: ConfigSource
    lateinit var clientId: String
    lateinit var redirectUri: String
    var scopes: List<String> = emptyList()
    var additionalParameters: Map<String, String> = emptyMap()
    var prompt: Prompt? = null
    var loginHint: String? = null
    var uiLocales: List<String>? = null
    var useNonce: Boolean = true
}

@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class BrowserOnlyArgs {
    lateinit var authUrl: String
    lateinit var redirectUri: String
}

@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class RefreshArgs {
    lateinit var config: ConfigSource
    lateinit var clientId: String
    lateinit var refreshToken: String
    var scopes: List<String> = emptyList()
    var additionalParameters: Map<String, String> = emptyMap()
}

@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class RegisterArgs {
    lateinit var config: ConfigSource
    lateinit var redirectUris: List<String>
    var clientName: String? = null
    var responseTypes: List<String> = emptyList()
    var grantTypes: List<String> = emptyList()
    var subjectTypes: List<String> = emptyList()
    var tokenEndpointAuthMethod: String? = null
    var additionalParameters: Map<String, String> = emptyMap()
}

@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class EndSessionArgs {
    lateinit var config: ConfigSource
    /// Optional per RFC 8665 / OIDC RP-Initiated Logout: the parameter is
    /// RECOMMENDED, not REQUIRED. Some IdPs accept end-session without it.
    var idTokenHint: String? = null
    lateinit var postLogoutRedirectUri: String
    var state: String? = null
    var additionalParameters: Map<String, String> = emptyMap()
}

@JsonIgnoreProperties(ignoreUnknown = true)
@InvokeArg
class SubscribeEventsArgs {
    lateinit var channel: app.tauri.plugin.Channel
}

// MARK: - Outputs encoded to JS responses

/// Plain `data class` outputs. Jackson serializes them via field access (the
/// `setVisibility(FIELD, ANY)` configured on the shared mapper).
data class ServiceConfigurationResponse(
    val authorizationEndpoint: String,
    val tokenEndpoint: String,
    val endSessionEndpoint: String?,
    val registrationEndpoint: String?,
    val issuer: String?,
    val additionalParameters: Map<String, String>,
)

data class AuthStateResponse(
    val accessToken: String?,
    val accessTokenExpiresAt: Long?,
    val idToken: String?,
    val refreshToken: String?,
    val scope: String?,
    val tokenType: String?,
    val authorizationCode: String?,
    val additionalParameters: Map<String, String>,
)

data class BrowserOnlyResponse(
    val url: String,
)

data class RegistrationResponseModel(
    val clientId: String,
    val clientIdIssuedAt: Long?,
    val clientSecret: String?,
    val clientSecretExpiresAt: Long?,
    val registrationAccessToken: String?,
    val registrationClientUri: String?,
    val tokenEndpointAuthMethod: String?,
    val additionalParameters: Map<String, String>,
)

data class EndSessionResponseModel(
    val url: String,
    val state: String?,
)

/// Diagnostic event mirroring `crate::events::AuthEvent` and the iOS
/// `AuthEvent` enum (`AppAuthPlugin.swift`). Encoded as `{"kind": "<camelCase>"}`
/// to match the Rust serde shape (`#[serde(tag = "kind", rename_all = "camelCase")]`).
@JsonSerialize(using = AuthEvent.Serializer::class)
enum class AuthEvent(val kind: String) {
    BROWSER_OPENED("browserOpened"),
    REDIRECT_INTERCEPTED("redirectIntercepted"),
    TOKEN_EXCHANGE_STARTED("tokenExchangeStarted"),
    TOKEN_EXCHANGE_COMPLETED("tokenExchangeCompleted");

    internal class Serializer : JsonSerializer<AuthEvent>() {
        override fun serialize(
            value: AuthEvent,
            gen: JsonGenerator,
            serializers: SerializerProvider,
        ) {
            gen.writeStartObject()
            gen.writeStringField("kind", value.kind)
            gen.writeEndObject()
        }
    }
}
