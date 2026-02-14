#!/bin/bash
# assemble-macos-dmg.sh
#
# Assembles the unified macOS Eurora.app by embedding the Tauri desktop app
# (EuroraDesktop.app) inside the Swift launcher (Eurora.app), then re-signs
# the entire bundle and creates a DMG.
#
# Required environment variables:
#   ARCH              - "arm64" or "x86_64"
#   APPLE_TEAM_ID     - Apple Developer Team ID (for codesign identity)
#   VERSION           - Release version string (for DMG naming)
#
# Expected directory layout (CI artifacts downloaded before this script runs):
#   tauri-release/    - Tauri build artifacts (contains darwin/{aarch64|x86_64}/*.tar.gz)
#   launcher-release/ - Launcher build artifacts (contains darwin/{aarch64|x86_64}/EuroraLauncher.app.zip)

set -o errexit
set -o nounset
set -o pipefail

ARCH="${ARCH:?ARCH is required (arm64 or x86_64)}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:?APPLE_TEAM_ID is required}"
VERSION="${VERSION:?VERSION is required}"

if [ "$ARCH" = "arm64" ]; then
    ARCH_DIR="aarch64"
else
    ARCH_DIR="x86_64"
fi

# Discover the signing identity dynamically from the keychain
# instead of hardcoding the org name, which may not match exactly.
echo "=== Available codesigning identities ==="
security find-identity -v -p codesigning

SIGN_IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID Application" | grep "$APPLE_TEAM_ID" | head -1 | sed 's/.*"\(.*\)"/\1/')

if [ -z "$SIGN_IDENTITY" ]; then
    echo "ERROR: No 'Developer ID Application' identity found for team $APPLE_TEAM_ID"
    echo "Available identities:"
    security find-identity -v -p codesigning
    exit 1
fi

echo "=== Assembling unified macOS app ==="
echo "  arch:     $ARCH ($ARCH_DIR)"
echo "  version:  $VERSION"
echo "  identity: $SIGN_IDENTITY"

# 1. Extract the launcher app (Eurora.app with Safari extension)
echo "--- Extracting launcher app ---"
mkdir -p assembled
LAUNCHER_ZIP="launcher-release/darwin/${ARCH_DIR}/EuroraLauncher.app.zip"
if [ ! -f "$LAUNCHER_ZIP" ]; then
    echo "ERROR: Launcher artifact not found at $LAUNCHER_ZIP"
    exit 1
fi
ditto -x -k "$LAUNCHER_ZIP" assembled/
# The zip contains Eurora.app (after PRODUCT_NAME change)
if [ ! -d "assembled/Eurora.app" ]; then
    echo "ERROR: Eurora.app not found after extracting launcher"
    ls -la assembled/
    exit 1
fi
echo "  Launcher extracted: assembled/Eurora.app"

# 2. Extract the Tauri app from the updater tarball
echo "--- Extracting Tauri app ---"
mkdir -p tauri-extracted
TAURI_TARGZ=$(find "tauri-release/darwin/${ARCH_DIR}" -name '*.tar.gz' -not -name '*.sig' | head -1)
if [ -z "$TAURI_TARGZ" ]; then
    echo "ERROR: Tauri .tar.gz not found in tauri-release/darwin/${ARCH_DIR}/"
    ls -la "tauri-release/darwin/${ARCH_DIR}/" || true
    exit 1
fi
tar xzf "$TAURI_TARGZ" -C tauri-extracted
# Find the extracted .app (e.g., EuroraDesktop.app or EuroraDesktop Nightly.app)
TAURI_APP=$(find tauri-extracted -maxdepth 1 -name '*.app' -type d | head -1)
if [ -z "$TAURI_APP" ]; then
    echo "ERROR: No .app found after extracting $TAURI_TARGZ"
    ls -la tauri-extracted/
    exit 1
fi
echo "  Tauri app extracted: $TAURI_APP"

# 3. Embed the Tauri app inside the launcher's Resources
echo "--- Embedding Tauri app into launcher ---"
RESOURCES_DIR="assembled/Eurora.app/Contents/Resources"
mkdir -p "$RESOURCES_DIR"
# Use the basename from the tarball (preserves the exact product name)
TAURI_APP_NAME=$(basename "$TAURI_APP")
ditto "$TAURI_APP" "$RESOURCES_DIR/$TAURI_APP_NAME"
echo "  Embedded as: $RESOURCES_DIR/$TAURI_APP_NAME"

# 4. Extract entitlements from original binaries before re-signing
# codesign --force without --entitlements strips the embedded entitlements that
# Xcode and Tauri injected at build time (sandbox, network, JIT, etc.).
# We extract them first, then re-apply during re-signing.
echo "--- Extracting entitlements from original binaries ---"

ENTITLEMENTS_DIR=$(mktemp -d)
trap 'rm -rf "$ENTITLEMENTS_DIR"' EXIT

# Extract entitlements from the embedded Tauri app's main executable
TAURI_MAIN_BIN="$RESOURCES_DIR/$TAURI_APP_NAME/Contents/MacOS/$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$RESOURCES_DIR/$TAURI_APP_NAME/Contents/Info.plist")"
if codesign -d --entitlements :- "$TAURI_MAIN_BIN" > "$ENTITLEMENTS_DIR/tauri.plist" 2>/dev/null && [ -s "$ENTITLEMENTS_DIR/tauri.plist" ]; then
    echo "  Extracted Tauri app entitlements"
else
    echo "  No entitlements found on Tauri app (will re-sign without)"
    rm -f "$ENTITLEMENTS_DIR/tauri.plist"
fi

# Extract entitlements from the Safari extension appex
APPEX=$(find "assembled/Eurora.app/Contents/PlugIns" -name '*.appex' -type d 2>/dev/null | head -1)
if [ -n "$APPEX" ]; then
    APPEX_BIN="$APPEX/Contents/MacOS/$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$APPEX/Contents/Info.plist")"
    if codesign -d --entitlements :- "$APPEX_BIN" > "$ENTITLEMENTS_DIR/appex.plist" 2>/dev/null && [ -s "$ENTITLEMENTS_DIR/appex.plist" ]; then
        echo "  Extracted appex entitlements"
    else
        echo "  No entitlements found on appex (will re-sign without)"
        rm -f "$ENTITLEMENTS_DIR/appex.plist"
    fi
fi

# Extract entitlements from the outer launcher app
LAUNCHER_BIN="assembled/Eurora.app/Contents/MacOS/$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "assembled/Eurora.app/Contents/Info.plist")"
if codesign -d --entitlements :- "$LAUNCHER_BIN" > "$ENTITLEMENTS_DIR/launcher.plist" 2>/dev/null && [ -s "$ENTITLEMENTS_DIR/launcher.plist" ]; then
    echo "  Extracted launcher entitlements"
else
    echo "  No entitlements found on launcher (will re-sign without)"
    rm -f "$ENTITLEMENTS_DIR/launcher.plist"
fi

# 5. Re-sign each component individually with its original entitlements
# Apple discourages --deep for production signing because it cannot apply
# per-component entitlements. We sign innermost components first, then outer.
echo "--- Code signing ---"

# Sign all nested frameworks/dylibs inside the Tauri app (no entitlements needed for these)
if [ -d "$RESOURCES_DIR/$TAURI_APP_NAME/Contents/Frameworks" ]; then
    find "$RESOURCES_DIR/$TAURI_APP_NAME/Contents/Frameworks" \
        \( -name '*.dylib' -o -name '*.framework' \) | while read -r item; do
        codesign --force --options runtime --timestamp \
            --sign "$SIGN_IDENTITY" \
            "$item"
    done
fi

# Sign helper executables in the Tauri app's MacOS directory (e.g., euro-native-messaging).
# The main binary is signed when the .app bundle is signed below, so we skip it here.
TAURI_MACOS_DIR="$RESOURCES_DIR/$TAURI_APP_NAME/Contents/MacOS"
TAURI_MAIN_NAME=$(basename "$TAURI_MAIN_BIN")
if [ -d "$TAURI_MACOS_DIR" ]; then
    find "$TAURI_MACOS_DIR" -type f -perm +0111 ! -name "$TAURI_MAIN_NAME" | while read -r item; do
        echo "  Signing helper executable: $(basename "$item")"
        codesign --force --options runtime --timestamp \
            --sign "$SIGN_IDENTITY" \
            "$item"
    done
fi

# Sign the embedded Tauri app's main executable with its entitlements
TAURI_SIGN_ARGS=(--force --options runtime --timestamp --sign "$SIGN_IDENTITY")
if [ -f "$ENTITLEMENTS_DIR/tauri.plist" ]; then
    TAURI_SIGN_ARGS+=(--entitlements "$ENTITLEMENTS_DIR/tauri.plist")
fi
codesign "${TAURI_SIGN_ARGS[@]}" "$RESOURCES_DIR/$TAURI_APP_NAME"
echo "  Signed Tauri app: $RESOURCES_DIR/$TAURI_APP_NAME"

# Sign the Safari extension appex if present
if [ -n "$APPEX" ]; then
    APPEX_SIGN_ARGS=(--force --options runtime --timestamp --sign "$SIGN_IDENTITY")
    if [ -f "$ENTITLEMENTS_DIR/appex.plist" ]; then
        APPEX_SIGN_ARGS+=(--entitlements "$ENTITLEMENTS_DIR/appex.plist")
    fi
    codesign "${APPEX_SIGN_ARGS[@]}" "$APPEX"
    echo "  Signed extension: $APPEX"
fi

# Sign any frameworks/dylibs in the outer launcher
if [ -d "assembled/Eurora.app/Contents/Frameworks" ]; then
    find "assembled/Eurora.app/Contents/Frameworks" \
        \( -name '*.dylib' -o -name '*.framework' \) | while read -r item; do
        codesign --force --options runtime --timestamp \
            --sign "$SIGN_IDENTITY" \
            "$item"
    done
fi

# Sign helper executables in the launcher's MacOS directory (excluding main binary)
LAUNCHER_MACOS_DIR="assembled/Eurora.app/Contents/MacOS"
LAUNCHER_MAIN_NAME=$(basename "$LAUNCHER_BIN")
if [ -d "$LAUNCHER_MACOS_DIR" ]; then
    find "$LAUNCHER_MACOS_DIR" -type f -perm +0111 ! -name "$LAUNCHER_MAIN_NAME" | while read -r item; do
        echo "  Signing helper executable: $(basename "$item")"
        codesign --force --options runtime --timestamp \
            --sign "$SIGN_IDENTITY" \
            "$item"
    done
fi

# Sign the outer launcher app with its entitlements (covers the launcher binary)
LAUNCHER_SIGN_ARGS=(--force --options runtime --timestamp --sign "$SIGN_IDENTITY")
if [ -f "$ENTITLEMENTS_DIR/launcher.plist" ]; then
    LAUNCHER_SIGN_ARGS+=(--entitlements "$ENTITLEMENTS_DIR/launcher.plist")
fi
codesign "${LAUNCHER_SIGN_ARGS[@]}" "assembled/Eurora.app"

echo "  Signing complete"

# 6. Verify the signature
echo "--- Verifying signature ---"
codesign --verify --deep --strict "assembled/Eurora.app"
echo "  Signature verified"

# 7. Prepare release directory with DMG and updater artifacts
echo "--- Preparing release artifacts ---"
RELEASE_DIR="release/darwin/${ARCH_DIR}"
mkdir -p "$RELEASE_DIR"

# Create DMG with an Applications symlink for the standard drag-to-install UX
DMG_NAME="Eurora_${VERSION}_${ARCH}.dmg"
DMG_STAGING="$(mktemp -d)"
ditto "assembled/Eurora.app" "$DMG_STAGING/Eurora.app"
ln -s /Applications "$DMG_STAGING/Applications"
hdiutil create \
    -volname "Eurora" \
    -srcfolder "$DMG_STAGING" \
    -ov \
    -format UDZO \
    "$RELEASE_DIR/$DMG_NAME"
rm -rf "$DMG_STAGING"
echo "  DMG created: $RELEASE_DIR/$DMG_NAME"

# Sign the DMG itself (Apple recommends signing disk images for distribution)
codesign --force --sign "$SIGN_IDENTITY" "$RELEASE_DIR/$DMG_NAME"
echo "  DMG signed: $RELEASE_DIR/$DMG_NAME"

# NOTE: The updater tar.gz is NO LONGER copied from the raw Tauri build here.
# The CI workflow creates it from the fully assembled + notarized Eurora.app
# (see the "Create updater tar.gz" step in publish.yaml) so that auto-updates
# replace the entire bundle — preserving the Safari extension and code signature.

echo "=== Assembly complete ==="
echo "  DMG: $RELEASE_DIR/$DMG_NAME"
ls -la "$RELEASE_DIR/"
