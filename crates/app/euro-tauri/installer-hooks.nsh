; Native messaging host installation hooks for NSIS installer.
; Installs browser-specific JSON manifests and the native messaging binary,
; then registers them via HKCU registry entries so no admin is required.

; Resolve build-time environment variables (set by scripts/release.sh)
!define TAURI_HOSTS_DIR "$%TAURI_HOSTS_DIR%"
!define TAURI_NATIVE_MESSAGING_BIN "$%TAURI_NATIVE_MESSAGING_BIN%"

!macro NSIS_HOOK_POSTINSTALL
  ; --- Chrome ---
  CreateDirectory "$INSTDIR\native-messaging\chrome"
  SetOutPath "$INSTDIR\native-messaging\chrome"
  File /oname=com.eurora.app.json "${TAURI_HOSTS_DIR}\windows.chromium.native-messaging.json"
  File /oname=euro-native-messaging.exe "${TAURI_NATIVE_MESSAGING_BIN}"

  ; --- Edge ---
  CreateDirectory "$INSTDIR\native-messaging\edge"
  SetOutPath "$INSTDIR\native-messaging\edge"
  File /oname=com.eurora.app.json "${TAURI_HOSTS_DIR}\windows.edge.native-messaging.json"
  File /oname=euro-native-messaging.exe "${TAURI_NATIVE_MESSAGING_BIN}"

  ; --- Firefox ---
  CreateDirectory "$INSTDIR\native-messaging\firefox"
  SetOutPath "$INSTDIR\native-messaging\firefox"
  File /oname=com.eurora.app.json "${TAURI_HOSTS_DIR}\windows.firefox.native-messaging.json"
  File /oname=euro-native-messaging.exe "${TAURI_NATIVE_MESSAGING_BIN}"

  ; Reset output path
  SetOutPath "$INSTDIR"

  ; Registry: point each browser to its manifest
  WriteRegStr HKCU "Software\Google\Chrome\NativeMessagingHosts\com.eurora.app" "" "$INSTDIR\native-messaging\chrome\com.eurora.app.json"
  WriteRegStr HKCU "Software\Microsoft\Edge\NativeMessagingHosts\com.eurora.app" "" "$INSTDIR\native-messaging\edge\com.eurora.app.json"
  WriteRegStr HKCU "Software\Mozilla\NativeMessagingHosts\com.eurora.app" "" "$INSTDIR\native-messaging\firefox\com.eurora.app.json"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove native messaging registry entries
  DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\com.eurora.app"
  DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\com.eurora.app"
  DeleteRegKey HKCU "Software\Mozilla\NativeMessagingHosts\com.eurora.app"

  ; Remove native messaging files
  RMDir /r "$INSTDIR\native-messaging"
!macroend
