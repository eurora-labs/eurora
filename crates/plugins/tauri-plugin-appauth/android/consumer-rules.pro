# Keep the plugin class, the browser-redirect activity, and @Command /
# @ActivityCallback entry points; the Tauri host resolves all of these by
# name via reflection.
-keep class app.tauri.appauth.AppAuthPlugin { *; }
-keep class app.tauri.appauth.BrowserSessionActivity { *; }
-keepclassmembers class app.tauri.appauth.** {
    @app.tauri.annotation.Command *;
    @app.tauri.annotation.ActivityCallback *;
}
