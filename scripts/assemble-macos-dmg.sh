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

SIGN_IDENTITY="Developer ID Application: Eurora Labs (${APPLE_TEAM_ID})"

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
cp -R "$TAURI_APP" "$RESOURCES_DIR/$TAURI_APP_NAME"
echo "  Embedded as: $RESOURCES_DIR/$TAURI_APP_NAME"

# 4. Re-sign the entire bundle recursively
# Sign inner components first (most deeply nested first), then the outer app.
echo "--- Code signing ---"

# Sign the embedded Tauri app and its contents
codesign --deep --force --options runtime --timestamp \
    --sign "$SIGN_IDENTITY" \
    "$RESOURCES_DIR/$TAURI_APP_NAME"

# Sign the Safari extension appex if present
APPEX=$(find "assembled/Eurora.app/Contents/PlugIns" -name '*.appex' -type d 2>/dev/null | head -1)
if [ -n "$APPEX" ]; then
    codesign --deep --force --options runtime --timestamp \
        --sign "$SIGN_IDENTITY" \
        "$APPEX"
    echo "  Signed extension: $APPEX"
fi

# Sign the outer app (covers launcher binary, frameworks, etc.)
codesign --deep --force --options runtime --timestamp \
    --sign "$SIGN_IDENTITY" \
    "assembled/Eurora.app"

echo "  Signing complete"

# 5. Verify the signature
echo "--- Verifying signature ---"
codesign --verify --deep --strict "assembled/Eurora.app"
echo "  Signature verified"

# 6. Prepare release directory with DMG and updater artifacts
echo "--- Preparing release artifacts ---"
RELEASE_DIR="release/darwin/${ARCH_DIR}"
mkdir -p "$RELEASE_DIR"

# Create DMG
DMG_NAME="Eurora_${VERSION}_${ARCH}.dmg"
hdiutil create \
    -volname "Eurora" \
    -srcfolder "assembled/Eurora.app" \
    -ov \
    -format UDZO \
    "$RELEASE_DIR/$DMG_NAME"
echo "  DMG created: $RELEASE_DIR/$DMG_NAME"

# Copy Tauri updater artifacts (tar.gz + signature) for the update service
cp "tauri-release/darwin/${ARCH_DIR}/"*.tar.gz "$RELEASE_DIR/" 2>/dev/null || true
cp "tauri-release/darwin/${ARCH_DIR}/"*.tar.gz.sig "$RELEASE_DIR/" 2>/dev/null || true
echo "  Updater artifacts copied"

echo "=== Assembly complete ==="
echo "  DMG: $RELEASE_DIR/$DMG_NAME"
ls -la "$RELEASE_DIR/"
