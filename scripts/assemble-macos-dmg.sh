#!/bin/bash
#
# Required env: ARCH, APPLE_TEAM_ID, VERSION, CHANNEL
# Expected dirs: tauri-release/, launcher-release/

set -o errexit
set -o nounset
set -o pipefail

ARCH="${ARCH:?ARCH is required (arm64 or x86_64)}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:?APPLE_TEAM_ID is required}"
VERSION="${VERSION:?VERSION is required}"
CHANNEL="${CHANNEL:-release}"

if [ "$ARCH" = "arm64" ]; then
    ARCH_DIR="aarch64"
else
    ARCH_DIR="x86_64"
fi

if [ "$CHANNEL" = "nightly" ]; then
    APP_DISPLAY_NAME="Eurora Nightly"
    EXTENSION_DISPLAY_NAME="Eurora Nightly Safari Extension"
else
    APP_DISPLAY_NAME="Eurora"
    EXTENSION_DISPLAY_NAME="Eurora Safari Extension"
fi

APP_NAME="${APP_DISPLAY_NAME}.app"
ASSEMBLED_APP="assembled/${APP_NAME}"

echo "=== Available codesigning identities ==="
security find-identity -v -p codesigning

SIGN_IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID Application" | grep "$APPLE_TEAM_ID" | head -1 | sed 's/.*"\(.*\)"/\1/')

if [ -z "$SIGN_IDENTITY" ]; then
    echo "ERROR: No 'Developer ID Application' identity found for team $APPLE_TEAM_ID"
    security find-identity -v -p codesigning
    exit 1
fi

echo "=== Assembling unified macOS app ==="
echo "  channel:  $CHANNEL"
echo "  app name: $APP_DISPLAY_NAME"
echo "  arch:     $ARCH ($ARCH_DIR)"
echo "  version:  $VERSION"
echo "  identity: $SIGN_IDENTITY"

echo "--- Extracting launcher app ---"
mkdir -p assembled
LAUNCHER_ZIP="launcher-release/darwin/${ARCH_DIR}/EuroraLauncher.app.zip"
if [ ! -f "$LAUNCHER_ZIP" ]; then
    echo "ERROR: Launcher artifact not found at $LAUNCHER_ZIP"
    exit 1
fi
ditto -x -k "$LAUNCHER_ZIP" assembled/
if [ ! -d "assembled/Eurora.app" ]; then
    echo "ERROR: Eurora.app not found after extracting launcher"
    ls -la assembled/
    exit 1
fi

if [ "$APP_NAME" != "Eurora.app" ]; then
    mv "assembled/Eurora.app" "$ASSEMBLED_APP"
    echo "  Renamed launcher to: $APP_NAME"
fi
echo "  Launcher extracted: $ASSEMBLED_APP"

echo "--- Applying channel branding ($CHANNEL) ---"
APP_PLIST="$ASSEMBLED_APP/Contents/Info.plist"

/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName $APP_DISPLAY_NAME" "$APP_PLIST" 2>/dev/null \
    || /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string $APP_DISPLAY_NAME" "$APP_PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleName $APP_DISPLAY_NAME" "$APP_PLIST" 2>/dev/null \
    || /usr/libexec/PlistBuddy -c "Add :CFBundleName string $APP_DISPLAY_NAME" "$APP_PLIST"
echo "  App display name → $APP_DISPLAY_NAME"

APPEX_PLIST_PATH=$(find "$ASSEMBLED_APP/Contents/PlugIns" -name 'Info.plist' -path '*.appex/*' 2>/dev/null | head -1)
if [ -n "$APPEX_PLIST_PATH" ]; then
    /usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName $EXTENSION_DISPLAY_NAME" "$APPEX_PLIST_PATH" 2>/dev/null \
        || /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string $EXTENSION_DISPLAY_NAME" "$APPEX_PLIST_PATH"
    echo "  Extension display name → $EXTENSION_DISPLAY_NAME"
fi

if [ "$CHANNEL" = "nightly" ]; then
    NIGHTLY_ICNS="crates/app/euro-tauri/icons/nightly/icon.icns"
    if [ -f "$NIGHTLY_ICNS" ]; then
        ICON_FILE_NAME="AppIcon-Nightly.icns"
        ditto "$NIGHTLY_ICNS" "$ASSEMBLED_APP/Contents/Resources/$ICON_FILE_NAME"
        /usr/libexec/PlistBuddy -c "Set :CFBundleIconFile $ICON_FILE_NAME" "$APP_PLIST" 2>/dev/null \
            || /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string $ICON_FILE_NAME" "$APP_PLIST"
        # macOS prefers CFBundleIconName (asset catalog) over CFBundleIconFile; remove it to force our .icns
        /usr/libexec/PlistBuddy -c "Delete :CFBundleIconName" "$APP_PLIST" 2>/dev/null || true
        echo "  Icon → nightly ($ICON_FILE_NAME)"
    else
        echo "  WARNING: Nightly icon not found at $NIGHTLY_ICNS — keeping default"
    fi
fi

echo "--- Extracting Tauri app ---"
mkdir -p tauri-extracted
TAURI_TARGZ=$(find "tauri-release/darwin/${ARCH_DIR}" -name '*.tar.gz' -not -name '*.sig' | head -1)
if [ -z "$TAURI_TARGZ" ]; then
    echo "ERROR: Tauri .tar.gz not found in tauri-release/darwin/${ARCH_DIR}/"
    ls -la "tauri-release/darwin/${ARCH_DIR}/" || true
    exit 1
fi
tar xzf "$TAURI_TARGZ" -C tauri-extracted
TAURI_APP=$(find tauri-extracted -maxdepth 1 -name '*.app' -type d | head -1)
if [ -z "$TAURI_APP" ]; then
    echo "ERROR: No .app found after extracting $TAURI_TARGZ"
    ls -la tauri-extracted/
    exit 1
fi
echo "  Tauri app extracted: $TAURI_APP"

echo "--- Embedding Tauri app into launcher ---"
RESOURCES_DIR="$ASSEMBLED_APP/Contents/Resources"
mkdir -p "$RESOURCES_DIR"
TAURI_APP_NAME=$(basename "$TAURI_APP")
ditto "$TAURI_APP" "$RESOURCES_DIR/$TAURI_APP_NAME"
echo "  Embedded as: $RESOURCES_DIR/$TAURI_APP_NAME"

# codesign --force strips entitlements injected by Xcode/Tauri at build time,
# so we extract them first and re-apply during re-signing.
echo "--- Extracting entitlements from original binaries ---"

ENTITLEMENTS_DIR=$(mktemp -d)
trap 'rm -rf "$ENTITLEMENTS_DIR"' EXIT

TAURI_MAIN_BIN="$RESOURCES_DIR/$TAURI_APP_NAME/Contents/MacOS/$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$RESOURCES_DIR/$TAURI_APP_NAME/Contents/Info.plist")"
if codesign -d --entitlements :- "$TAURI_MAIN_BIN" > "$ENTITLEMENTS_DIR/tauri.plist" 2>/dev/null && [ -s "$ENTITLEMENTS_DIR/tauri.plist" ]; then
    echo "  Extracted Tauri app entitlements"
else
    rm -f "$ENTITLEMENTS_DIR/tauri.plist"
fi

APPEX=$(find "$ASSEMBLED_APP/Contents/PlugIns" -name '*.appex' -type d 2>/dev/null | head -1)
if [ -n "$APPEX" ]; then
    APPEX_BIN="$APPEX/Contents/MacOS/$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$APPEX/Contents/Info.plist")"
    if codesign -d --entitlements :- "$APPEX_BIN" > "$ENTITLEMENTS_DIR/appex.plist" 2>/dev/null && [ -s "$ENTITLEMENTS_DIR/appex.plist" ]; then
        echo "  Extracted appex entitlements"
    else
        rm -f "$ENTITLEMENTS_DIR/appex.plist"
    fi
fi

LAUNCHER_BIN="$ASSEMBLED_APP/Contents/MacOS/$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$APP_PLIST")"
if codesign -d --entitlements :- "$LAUNCHER_BIN" > "$ENTITLEMENTS_DIR/launcher.plist" 2>/dev/null && [ -s "$ENTITLEMENTS_DIR/launcher.plist" ]; then
    echo "  Extracted launcher entitlements"
else
    rm -f "$ENTITLEMENTS_DIR/launcher.plist"
fi

# Apple discourages --deep because it can't apply per-component entitlements.
# We sign innermost components first, then outer.
echo "--- Code signing ---"

# Sort deepest-path-first so nested frameworks inside other frameworks are signed
# before their parents (required by Apple's code signing validation).
sign_frameworks() {
    local search_dir="$1"
    if [ -d "$search_dir" ]; then
        find "$search_dir" \( -name '*.dylib' -o -name '*.framework' \) -print0 \
            | sort -z -r \
            | xargs -0 -I {} codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "{}"
    fi
}

sign_frameworks "$RESOURCES_DIR/$TAURI_APP_NAME/Contents/Frameworks"

TAURI_MACOS_DIR="$RESOURCES_DIR/$TAURI_APP_NAME/Contents/MacOS"
TAURI_MAIN_NAME=$(basename "$TAURI_MAIN_BIN")
if [ -d "$TAURI_MACOS_DIR" ]; then
    # Main binary is signed as part of the .app bundle below
    find "$TAURI_MACOS_DIR" -type f -perm +0111 ! -name "$TAURI_MAIN_NAME" | while read -r item; do
        echo "  Signing helper executable: $(basename "$item")"
        codesign --force --options runtime --timestamp \
            --sign "$SIGN_IDENTITY" \
            "$item"
    done
fi

TAURI_SIGN_ARGS=(--force --options runtime --timestamp --sign "$SIGN_IDENTITY")
if [ -f "$ENTITLEMENTS_DIR/tauri.plist" ]; then
    TAURI_SIGN_ARGS+=(--entitlements "$ENTITLEMENTS_DIR/tauri.plist")
fi
codesign "${TAURI_SIGN_ARGS[@]}" "$RESOURCES_DIR/$TAURI_APP_NAME"
echo "  Signed Tauri app: $RESOURCES_DIR/$TAURI_APP_NAME"

if [ -n "$APPEX" ]; then
    APPEX_SIGN_ARGS=(--force --options runtime --timestamp --sign "$SIGN_IDENTITY")
    if [ -f "$ENTITLEMENTS_DIR/appex.plist" ]; then
        APPEX_SIGN_ARGS+=(--entitlements "$ENTITLEMENTS_DIR/appex.plist")
    fi
    codesign "${APPEX_SIGN_ARGS[@]}" "$APPEX"
    echo "  Signed extension: $APPEX"
fi

sign_frameworks "$ASSEMBLED_APP/Contents/Frameworks"

LAUNCHER_MACOS_DIR="$ASSEMBLED_APP/Contents/MacOS"
LAUNCHER_MAIN_NAME=$(basename "$LAUNCHER_BIN")
if [ -d "$LAUNCHER_MACOS_DIR" ]; then
    find "$LAUNCHER_MACOS_DIR" -type f -perm +0111 ! -name "$LAUNCHER_MAIN_NAME" | while read -r item; do
        echo "  Signing helper executable: $(basename "$item")"
        codesign --force --options runtime --timestamp \
            --sign "$SIGN_IDENTITY" \
            "$item"
    done
fi

LAUNCHER_SIGN_ARGS=(--force --options runtime --timestamp --sign "$SIGN_IDENTITY")
if [ -f "$ENTITLEMENTS_DIR/launcher.plist" ]; then
    LAUNCHER_SIGN_ARGS+=(--entitlements "$ENTITLEMENTS_DIR/launcher.plist")
fi
codesign "${LAUNCHER_SIGN_ARGS[@]}" "$ASSEMBLED_APP"

echo "  Signing complete"

echo "--- Verifying signature ---"
codesign --verify --deep --strict "$ASSEMBLED_APP"
echo "  Signature verified"

echo "=== Assembly complete ==="
