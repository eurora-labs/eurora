// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.browser.customtabs.CustomTabsIntent

/// Hosts the back-channel for `authorizeBrowserOnly`.
///
/// The activity:
///   1. opens Custom Tabs at the URL handed in via `EXTRA_AUTH_URI`,
///   2. waits in the back stack while the user authenticates,
///   3. is reused (`launchMode="singleTask"` in the manifest) when the OS
///      delivers the redirect URL via the registered intent-filter, and
///   4. finishes with the redirect URL as the activity result, or with
///      `RESULT_CANCELED` if the user dismisses the browser.
///
/// Patterned after AppAuth-Android's `AuthorizationManagementActivity` so the
/// state-preservation behaviour around process death matches what users get
/// for the full `authorize` flow. See the `AppAuth-Android` source for the
/// full lifecycle reasoning.
// `open` so unit tests can subclass and stub `startActivity` for the
// browser-not-available path; the activity is otherwise self-contained.
open class BrowserSessionActivity : ComponentActivity() {

    private var browserStarted: Boolean = false
    private var authUri: Uri? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val source = savedInstanceState ?: intent.extras
        authUri = source?.getString(EXTRA_AUTH_URI)?.let(Uri::parse)
        browserStarted = source?.getBoolean(KEY_BROWSER_STARTED, false) ?: false

        if (authUri == null) {
            setResult(RESULT_CANCELED)
            finish()
            return
        }
    }

    override fun onResume() {
        super.onResume()
        // `onCreate` finishes the activity when `authUri` is null, so by the
        // time we reach `onResume` it is guaranteed to be non-null.
        val uri = authUri ?: return

        // First foreground pass: launch Custom Tabs and wait. We come back to
        // `onResume` either via `onNewIntent` (redirect succeeded) or via a
        // user-driven dismissal (`RESULT_CANCELED`).
        if (!browserStarted) {
            try {
                CustomTabsIntent.Builder().build().launchUrl(this, uri)
                browserStarted = true
            } catch (e: Exception) {
                setResult(
                    RESULT_BROWSER_NOT_AVAILABLE,
                    Intent().putExtra(EXTRA_ERROR_MESSAGE, e.message ?: "no compatible browser")
                )
                finish()
            }
            return
        }

        // Second foreground pass.
        val data = intent.data
        if (data != null) {
            setResult(RESULT_OK, Intent().setData(data))
        } else {
            setResult(RESULT_CANCELED)
        }
        finish()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putBoolean(KEY_BROWSER_STARTED, browserStarted)
        outState.putString(EXTRA_AUTH_URI, authUri?.toString())
    }

    companion object {
        const val EXTRA_AUTH_URI = "app.tauri.appauth.AUTH_URI"
        const val EXTRA_ERROR_MESSAGE = "app.tauri.appauth.ERROR_MESSAGE"
        const val RESULT_BROWSER_NOT_AVAILABLE = Activity.RESULT_FIRST_USER + 1

        private const val KEY_BROWSER_STARTED = "app.tauri.appauth.BROWSER_STARTED"

        /// Build the intent the plugin hands to `startActivityForResult`.
        ///
        /// No `FLAG_ACTIVITY_NEW_TASK`: the activity must run inside the host
        /// app's task so `startActivityForResult` can deliver the result back
        /// to the caller. AppAuth-Android's `AuthorizationManagementActivity`
        /// follows the same convention. Cross-task launches return
        /// `RESULT_CANCELED` to the caller before the redirect arrives.
        fun newIntent(context: Context, authUri: Uri): Intent =
            Intent(context, BrowserSessionActivity::class.java)
                .putExtra(EXTRA_AUTH_URI, authUri.toString())
    }
}
