#!/usr/bin/env bash
set -euo pipefail

# Install com.eurora.dev.json native messaging host manifests for Chrome and
# Firefox so that the dev build of the browser extension can talk to the
# locally-built euro-native-messaging binary.

HOST_NAME="com.eurora.dev"
MANIFEST_FILE="${HOST_NAME}.json"

# ── usage ──────────────────────────────────────────────────────────────────
usage() {
	echo "Usage: $0 <extension-id>"
	echo ""
	echo "  <extension-id>  Chrome extension ID (e.g. abcdefghijklmnopqrstuvwxyz)"
	echo ""
	echo "The script auto-detects the platform (Linux / macOS) and writes"
	echo "native messaging host manifests for Chrome and Firefox into the"
	echo "per-user directories."
	exit 1
}

if [[ $# -lt 1 ]]; then
	usage
fi

EXTENSION_ID="$1"

# ── resolve the native-messaging binary ────────────────────────────────────
# Prefer the cargo build output; fall back to the well-known dev location.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BINARY_NAME="euro-native-messaging"
CARGO_BIN="$REPO_ROOT/target/debug/$BINARY_NAME"

if [[ -x "$CARGO_BIN" ]]; then
	BINARY_PATH="$CARGO_BIN"
else
	echo "Warning: $CARGO_BIN not found – building $BINARY_NAME ..."
	cargo build --package "$BINARY_NAME" --manifest-path "$REPO_ROOT/Cargo.toml"
	BINARY_PATH="$CARGO_BIN"
fi

# The manifest requires an absolute path.
BINARY_PATH="$(realpath "$BINARY_PATH")"
echo "Using binary: $BINARY_PATH"

# ── platform-specific target directories ───────────────────────────────────
OS="$(uname -s)"

case "$OS" in
	Linux)
		CHROME_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"
		CHROMIUM_DIR="$HOME/.config/chromium/NativeMessagingHosts"
		FIREFOX_DIR="$HOME/.mozilla/native-messaging-hosts"
		;;
	Darwin)
		CHROME_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
		CHROMIUM_DIR="$HOME/Library/Application Support/Chromium/NativeMessagingHosts"
		FIREFOX_DIR="$HOME/Library/Application Support/Mozilla/NativeMessagingHosts"
		;;
	*)
		echo "Error: Unsupported platform '$OS'."
		echo "This script supports Linux and macOS."
		exit 1
		;;
esac

# ── manifest contents ─────────────────────────────────────────────────────
CHROME_MANIFEST=$(cat <<EOF
{
	"name": "${HOST_NAME}",
	"description": "Eurora Native Messaging Host (dev)",
	"path": "${BINARY_PATH}",
	"type": "stdio",
	"allowed_origins": ["chrome-extension://${EXTENSION_ID}/"]
}
EOF
)

FIREFOX_MANIFEST=$(cat <<EOF
{
	"name": "${HOST_NAME}",
	"description": "Eurora Native Messaging Host (dev)",
	"path": "${BINARY_PATH}",
	"type": "stdio",
	"allowed_extensions": ["${EXTENSION_ID}"]
}
EOF
)

# ── write manifests ────────────────────────────────────────────────────────
install_manifest() {
	local dir="$1"
	local content="$2"
	local label="$3"

	mkdir -p "$dir"
	echo "$content" > "$dir/$MANIFEST_FILE"
	echo "Installed $label manifest -> $dir/$MANIFEST_FILE"
}

install_manifest "$CHROME_DIR"   "$CHROME_MANIFEST"   "Chrome"
install_manifest "$CHROMIUM_DIR" "$CHROME_MANIFEST"    "Chromium"
install_manifest "$FIREFOX_DIR"  "$FIREFOX_MANIFEST"   "Firefox"

echo ""
echo "Done. Native host '${HOST_NAME}' registered for extension '${EXTENSION_ID}'."
