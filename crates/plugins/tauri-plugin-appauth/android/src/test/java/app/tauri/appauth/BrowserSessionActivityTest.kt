// SPDX-License-Identifier: Apache-2.0

package app.tauri.appauth

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

/// `BrowserSessionActivity` runs the `authorizeBrowserOnly` round-trip:
///   * `onCreate` reads `EXTRA_AUTH_URI`,
///   * the first `onResume` launches Custom Tabs,
///   * the OS routes the redirect back via `onNewIntent` + `singleTask`,
///   * the second `onResume` finishes with the redirect URI as the result.
///
/// These tests pin every leg of that handshake — they regress Phase 1.2 (no
/// `FLAG_ACTIVITY_NEW_TASK` — the activity must end up in the host's task so
/// `startActivityForResult` can deliver the result) and Phase 3.5 (lifecycle
/// simplification: a single `return` on the missing-extra path, no duplicate
/// null guard in `onResume`).
@RunWith(RobolectricTestRunner::class)
class BrowserSessionActivityTest {

    private val authUri: Uri = Uri.parse("https://issuer.example.com/oauth/authorize?client_id=abc")
    private val redirectUri: Uri = Uri.parse("tauri.appauth.test:/oauth/callback?code=AUTHCODE")

    @Test
    fun newIntentBuildsLaunchIntentWithoutNewTaskFlag() {
        // Regression for Phase 1.2: `FLAG_ACTIVITY_NEW_TASK` would push the
        // activity into a different task, and `startActivityForResult` would
        // immediately deliver `RESULT_CANCELED` to the plugin caller before
        // the redirect ever arrived.
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val intent = BrowserSessionActivity.newIntent(context, authUri)
        assertEquals(0, intent.flags and Intent.FLAG_ACTIVITY_NEW_TASK)
        assertEquals(authUri.toString(), intent.getStringExtra(BrowserSessionActivity.EXTRA_AUTH_URI))
    }

    @Test
    fun happyPathDeliversRedirectUriToCaller() {
        val launchIntent = BrowserSessionActivity.newIntent(
            ApplicationProvider.getApplicationContext(),
            authUri,
        )
        val controller = Robolectric.buildActivity(BrowserSessionActivity::class.java, launchIntent)
            .create()
            .start()
            .resume()

        // First foreground pass: Custom Tabs launched, no result yet.
        val activity = controller.get()
        val customTabsIntent = shadowOf(activity).peekNextStartedActivity()
        assertNotNull("expected Custom Tabs to be launched on first onResume", customTabsIntent)
        assertEquals(Intent.ACTION_VIEW, customTabsIntent.action)
        assertEquals(authUri, customTabsIntent.data)
        assertFalse(activity.isFinishing)

        // OS routes the redirect through `singleTask` re-delivery.
        val redirect = Intent(Intent.ACTION_VIEW, redirectUri)
        controller.newIntent(redirect)
        controller.resume()

        assertTrue("activity must finish after redirect arrives", activity.isFinishing)
        assertEquals(Activity.RESULT_OK, shadowOf(activity).resultCode)
        assertEquals(redirectUri, shadowOf(activity).resultIntent.data)
    }

    @Test
    fun missingAuthUriExtraReturnsCanceledImmediately() {
        // No `EXTRA_AUTH_URI` extra at all — the activity must reject in
        // `onCreate` and never reach `onResume` / Custom Tabs.
        val launchIntent = Intent(
            ApplicationProvider.getApplicationContext(),
            BrowserSessionActivity::class.java,
        )
        val controller = Robolectric.buildActivity(BrowserSessionActivity::class.java, launchIntent)
            .create()

        val activity = controller.get()
        assertTrue("activity must finish in onCreate when extra is missing", activity.isFinishing)
        assertEquals(Activity.RESULT_CANCELED, shadowOf(activity).resultCode)
        assertNull(
            "no Custom Tabs launch should occur",
            shadowOf(activity).peekNextStartedActivity(),
        )
    }

    @Test
    fun browserNotAvailableSurfacesThroughResultCode() {
        // `CustomTabsIntent.launchUrl` calls `ContextCompat.startActivity`,
        // which delegates to `Activity.startActivity(Intent, Bundle)` and
        // raises `ActivityNotFoundException` when no browser handles the
        // implicit VIEW intent. The activity must convert that into the
        // `RESULT_BROWSER_NOT_AVAILABLE` sentinel (with the exception's
        // message in `EXTRA_ERROR_MESSAGE`) so the plugin can surface
        // `BROWSER_NOT_AVAILABLE` to the JS layer.
        val launchIntent = BrowserSessionActivity.newIntent(
            ApplicationProvider.getApplicationContext(),
            authUri,
        )
        val controller = Robolectric.buildActivity(
            FailingBrowserSessionActivity::class.java,
            launchIntent,
        )
            .create()
            .start()
            .resume()

        val activity = controller.get()
        assertTrue(activity.isFinishing)
        assertEquals(
            BrowserSessionActivity.RESULT_BROWSER_NOT_AVAILABLE,
            shadowOf(activity).resultCode,
        )
        val resultIntent = shadowOf(activity).resultIntent
        assertNotNull("error result must carry an intent with the message extra", resultIntent)
        assertEquals(
            "no compatible browser installed",
            resultIntent.getStringExtra(BrowserSessionActivity.EXTRA_ERROR_MESSAGE),
        )
    }

    /// Stubs out the `startActivity(Intent, Bundle?)` overload that
    /// `CustomTabsIntent.launchUrl` ultimately calls, so we can exercise the
    /// `BROWSER_NOT_AVAILABLE` failure mode without manipulating Robolectric's
    /// global package-manager state.
    class FailingBrowserSessionActivity : BrowserSessionActivity() {
        override fun startActivity(intent: Intent, options: Bundle?) {
            throw ActivityNotFoundException("no compatible browser installed")
        }
    }
}
