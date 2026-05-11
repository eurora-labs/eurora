// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import com.fasterxml.jackson.databind.ObjectMapper

/// Captures the JSON payload that an `Invoke` would send back to the JS layer.
///
/// `Invoke.resolve(...)` and `Invoke.reject(...)` ultimately call a
/// `sendResponse: (callback: Long, data: String) -> Unit` lambda passed in at
/// construction. The Tauri runtime supplies that lambda to bridge to native;
/// here we capture the (callback id, payload) pair so tests can assert on the
/// rejection code, message, and data without spinning up a real plugin host.
internal class RecordingInvoke(argsJson: String = "{}") {

    private val responses = mutableListOf<Response>()

    val invoke: Invoke = Invoke(
        id = 1L,
        command = "test",
        callback = CALLBACK,
        error = ERROR,
        sendResponse = { id, data -> responses += Response(id, data) },
        argsJson = argsJson,
        jsonMapper = ObjectMapper(),
    )

    /// Single recorded rejection. Throws if there isn't exactly one response or
    /// if it was sent to the resolve callback rather than the error callback.
    fun rejection(): JSObject {
        check(responses.size == 1) { "expected exactly one response, got ${responses.size}" }
        val response = responses.single()
        check(response.callbackId == ERROR) {
            "expected response on error callback (was on $response.callbackId — likely a resolve)"
        }
        return JSObject(response.payload)
    }

    private data class Response(val callbackId: Long, val payload: String)

    private companion object {
        const val CALLBACK = 10L
        const val ERROR = 20L
    }
}
