#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

PWD="$(dirname "$(readlink -f -- "$0")")"

CHANNEL=""
DO_SIGN="false"
VERSION=""
TARGET="${CARGO_BUILD_TARGET:-}"

function help() {
	local to
	to="$1"

	echo "Usage: $0 <flags>" 1>&"$to"
	echo 1>&"$to"
	echo "flags:" 1>&"$to"
	echo "	--version											release version." 1>&"$to"
	echo "	--dist												path to store artifacts in." 1>&"$to"
	echo "	--sign												if set, will sign the app." 1>&"$to"
	echo "	--channel											the channel to use for the release (release | nightly)." 1>&"$to"
	echo "	--help												display this message." 1>&"$to"
}

function error() {
	echo "error: $*" 1>&2
	echo 1>&2
	help 2
	exit 1
}

function info() {
	echo "$@"
}

function os() {
	local os
	os="$(uname -s)"
	case "$os" in
	Darwin)
		echo "darwin"
		;;
	Linux)
		echo "linux"
		;;
	Windows | MSYS* | MINGW*)
		echo "windows"
		;;
	*)
		error "$os: unsupported"
		;;
	esac
}

function arch() {
	local arch

	# If TARGET is specified, extract architecture from it
	if [ -n "${TARGET:-}" ]; then
		case "$TARGET" in
		*aarch64* | *arm64*)
			echo "aarch64"
			return
			;;
		*x86_64* | *amd64*)
			echo "x86_64"
			return
			;;
		esac
	fi

	# Otherwise, detect from system
	arch="$(uname -m)"
	case "$arch" in
	arm64 | aarch64)
		echo "aarch64"
		;;
	x86_64)
		echo "x86_64"
		;;
	*)
		error "$arch: unsupported architecture"
		;;
	esac
}

ARCH="$(arch)"
OS="$(os)"
DIST="release"

function tauri() {
	(cd "$PWD/.." && pnpm tauri "$@")
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--help)
		help 1
		exit 1
		;;
	--version)
		VERSION="$2"
		shift
		shift
		;;
	--dist)
		DIST="$2"
		shift
		shift
		;;
	--sign)
		DO_SIGN="true"
		shift
		;;
	--channel)
		CHANNEL="$2"
		shift
		shift
		;;
	*)
		error "unknown flag $1"
		;;
	esac
done

# Recalculate ARCH after TARGET is set
ARCH="$(arch)"

[ -z "${VERSION-}" ] && error "--version is not set"

[ -z "${TAURI_SIGNING_PRIVATE_KEY-}" ] && error "$TAURI_SIGNING_PRIVATE_KEY is not set"
[ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD-}" ] && error "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not set"

if [ "$CHANNEL" != "release" ] && [ "$CHANNEL" != "nightly" ]; then
	error "--channel must be either 'release' or 'nightly'"
fi

if [ "$DO_SIGN" = "true" ]; then
	if [ "$OS" = "darwin" ]; then
		[ -z "${APPLE_CERTIFICATE-}" ] && error "$APPLE_CERTIFICATE is not set"
		[ -z "${APPLE_CERTIFICATE_PASSWORD-}" ] && error "$APPLE_CERTIFICATE_PASSWORD is not set"
		[ -z "${APPLE_ID-}" ] && error "$APPLE_ID is not set"
		[ -z "${APPLE_TEAM_ID-}" ] && error "$APPLE_TEAM_ID is not set"
		[ -z "${APPLE_PASSWORD-}" ] && error "$APPLE_PASSWORD is not set"
		export APPLE_CERTIFICATE="$APPLE_CERTIFICATE"
		export APPLE_CERTIFICATE_PASSWORD="$APPLE_CERTIFICATE_PASSWORD"
		export APPLE_ID="$APPLE_ID"
		export APPLE_TEAM_ID="$APPLE_TEAM_ID"
		export APPLE_PASSWORD="$APPLE_PASSWORD"
	elif [ "$OS" == "linux" ]; then
		[ -z "${APPIMAGE_KEY_ID-}" ] && error "$APPIMAGE_KEY_ID is not set"
		[ -z "${APPIMAGE_KEY_PASSPHRASE-}" ] && error "$APPIMAGE_KEY_PASSPHRASE is not set"
		export SIGN=1
		export SIGN_KEY="$APPIMAGE_KEY_ID"
		export APPIMAGETOOL_SIGN_PASSPHRASE="$APPIMAGE_KEY_PASSPHRASE"
	elif [ "$OS" == "windows" ]; then
		# Nothing to do on windows
		export OS
	else
		error "signing is not supported on $(uname -s)"
	fi
fi

info "building:"
info "	channel: $CHANNEL"
info "	version: $VERSION"
info "	os: $OS"
info "	arch: $ARCH"
info "	dist: $DIST"
info "	sign: $DO_SIGN"
info "	target: ${TARGET:-default}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' exit

CONFIG_PATH=$(readlink -f "$PWD/../crates/app/euro-tauri/tauri.conf.$CHANNEL.json")

if [ "$OS" = "windows" ]; then
	FEATURES="windows"
else
	FEATURES=""
fi

# Note: OS values are: darwin, linux, windows

# update the version in the tauri release config
jq '.version="'"$VERSION"'"' "$CONFIG_PATH" >"$TMP_DIR/tauri.conf.json"

# Useful for understanding exactly what goes into the tauri build/bundle.
cat "$TMP_DIR/tauri.conf.json"

# set the VERSION and CHANNEL as an environment variables
export VERSION
export CHANNEL

# Create binaries directory for externalBin
BINARIES_DIR="$PWD/../crates/app/euro-tauri/binaries"
mkdir -p "$BINARIES_DIR"

# Build the app with release config
if [ -n "$TARGET" ]; then
	# Export TARGET for cargo to use
	export CARGO_BUILD_TARGET="$TARGET"

	# Build native messaging with the same target
	info "Building native messaging for target: $TARGET"
	cargo build --package euro-native-messaging --release --target "$TARGET"

	# Copy the binary with the target-triple suffix for Tauri's externalBin
	# Tauri expects binaries named like: binary-name-<target-triple>[.exe]
	if [ "$OS" = "windows" ]; then
		cp "$PWD/../target/$TARGET/release/euro-native-messaging.exe" "$BINARIES_DIR/euro-native-messaging-$TARGET.exe"
	else
		cp "$PWD/../target/$TARGET/release/euro-native-messaging" "$BINARIES_DIR/euro-native-messaging-$TARGET"
	fi

	info "Copied native messaging binary to: $BINARIES_DIR/euro-native-messaging-$TARGET"

	# Build with specified target
	# Note: passing --target is necessary to let tauri find the binaries,
	# it ignores CARGO_BUILD_TARGET and is more of a hack.
	tauri build \
		--verbose \
		--features "$FEATURES" \
		--config "$TMP_DIR/tauri.conf.json" \
		--target "$TARGET"

	BUNDLE_DIR=$(readlink -f "$PWD/../target/$TARGET/release/bundle")
else
	# Detect the default target triple
	DEFAULT_TARGET=$(rustc -vV | grep host | cut -d' ' -f2)

	# Build native messaging without target (default)
	info "Building native messaging for default target: $DEFAULT_TARGET"
	cargo build --package euro-native-messaging --release

	# Copy the binary with the target-triple suffix for Tauri's externalBin
	if [ "$OS" = "windows" ]; then
		cp "$PWD/../target/release/euro-native-messaging.exe" "$BINARIES_DIR/euro-native-messaging-$DEFAULT_TARGET.exe"
	else
		cp "$PWD/../target/release/euro-native-messaging" "$BINARIES_DIR/euro-native-messaging-$DEFAULT_TARGET"
	fi

	info "Copied native messaging binary to: $BINARIES_DIR/euro-native-messaging-$DEFAULT_TARGET"

	# Build with default target
	tauri build \
		--verbose \
		--features "$FEATURES" \
		--config "$TMP_DIR/tauri.conf.json"

	BUNDLE_DIR=$(readlink -f "$PWD/../target/release/bundle")
fi

RELEASE_DIR="$DIST/$OS/$ARCH"
mkdir -p "$RELEASE_DIR"

if [ "$OS" = "darwin" ]; then
	MACOS_DMG="$(find "$BUNDLE_DIR/dmg" -depth 1 -type f -name "*.dmg")"
	MACOS_UPDATER="$(find "$BUNDLE_DIR/macos" -depth 1 -type f -name "*.tar.gz")"
	MACOS_UPDATER_SIG="$(find "$BUNDLE_DIR/macos" -depth 1 -type f -name "*.tar.gz.sig")"

	cp "$MACOS_DMG" "$RELEASE_DIR"
	cp "$MACOS_UPDATER" "$RELEASE_DIR"
	cp "$MACOS_UPDATER_SIG" "$RELEASE_DIR"

	info "built:"
	info "	- $RELEASE_DIR/$(basename "$MACOS_DMG")"
	info "	- $RELEASE_DIR/$(basename "$MACOS_UPDATER")"
	info "	- $RELEASE_DIR/$(basename "$MACOS_UPDATER_SIG")"
elif [ "$OS" = "linux" ]; then
	APPIMAGE="$(find "$BUNDLE_DIR/appimage" -name \*.AppImage)"
	APPIMAGE_UPDATER="$(find "$BUNDLE_DIR/appimage" -name \*.AppImage.tar.gz)"
	APPIMAGE_UPDATER_SIG="$(find "$BUNDLE_DIR/appimage" -name \*.AppImage.tar.gz.sig)"
	DEB="$(find "$BUNDLE_DIR/deb" -name \*.deb)"
	RPM="$(find "$BUNDLE_DIR/rpm" -name \*.rpm)"

	cp "$APPIMAGE" "$RELEASE_DIR"
	cp "$APPIMAGE_UPDATER" "$RELEASE_DIR"
	cp "$APPIMAGE_UPDATER_SIG" "$RELEASE_DIR"
	cp "$DEB" "$RELEASE_DIR"
	cp "$RPM" "$RELEASE_DIR"

	info "built:"
	info "	- $RELEASE_DIR/$(basename "$APPIMAGE")"
	info "	- $RELEASE_DIR/$(basename "$APPIMAGE_UPDATER")"
	info "	- $RELEASE_DIR/$(basename "$APPIMAGE_UPDATER_SIG")"
	info "	- $RELEASE_DIR/$(basename "$DEB")"
	info "	- $RELEASE_DIR/$(basename "$RPM")"
elif [ "$OS" = "windows" ]; then
	WINDOWS_INSTALLER="$(find "$BUNDLE_DIR/msi" -name \*.msi)"
	WINDOWS_UPDATER="$(find "$BUNDLE_DIR/msi" -name \*.msi.zip)"
	WINDOWS_UPDATER_SIG="$(find "$BUNDLE_DIR/msi" -name \*.msi.zip.sig)"

	cp "$WINDOWS_INSTALLER" "$RELEASE_DIR"
	cp "$WINDOWS_UPDATER" "$RELEASE_DIR"
	cp "$WINDOWS_UPDATER_SIG" "$RELEASE_DIR"

	info "built:"
	info "	- $RELEASE_DIR/$(basename "$WINDOWS_INSTALLER")"
	info "	- $RELEASE_DIR/$(basename "$WINDOWS_UPDATER")"
	info "	- $RELEASE_DIR/$(basename "$WINDOWS_UPDATER_SIG")"
else
	error "unsupported os: $OS"
fi

# Install Chrome extension native messaging host manifest
function install_native_messaging_host() {
	info "Installing Chrome extension native messaging host manifest..."

	# Path to the native messaging host binary in the release
	local NATIVE_MESSAGING_HOST_BINARY
	local MANIFEST_DIR
	local MANIFEST_PATH
	local MANIFEST_CONTENT

	# Get the path to the native-messaging-host.json template
	# local TEMPLATE_PATH="$PWD/../extensions/chromium/native-messaging-host.json"
	cp "$PWD/../apps/browser/src/native-messaging-host.chromium.json" "$PWD/../apps/browser/src/native-messaging-host.json"
	local TEMPLATE_PATH="$PWD/../apps/browser/src/native-messaging-host.json"

	if [ "$OS" = "darwin" ]; then
		# For macOS (darwin), the binary is inside the .app bundle
		NATIVE_MESSAGING_HOST_BINARY="/Applications/Eurora.app/Contents/MacOS/euro-native-messaging"
		MANIFEST_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"

		# Also support Chromium and other browsers
		mkdir -p "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
		mkdir -p "$HOME/Library/Application Support/Chromium/NativeMessagingHosts"
		mkdir -p "$HOME/Library/Application Support/Microsoft Edge/NativeMessagingHosts"
		mkdir -p "$HOME/Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"

	elif [ "$OS" = "linux" ]; then
		# For Linux, use the installed binary path
		NATIVE_MESSAGING_HOST_BINARY="/usr/bin/euro-native-messaging"
		MANIFEST_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"

		# Also support Chromium and other browsers
		mkdir -p "$HOME/.config/google-chrome/NativeMessagingHosts"
		mkdir -p "$HOME/.config/chromium/NativeMessagingHosts"
		mkdir -p "$HOME/.config/microsoft-edge/NativeMessagingHosts"
		mkdir -p "$HOME/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"

	elif [ "$OS" = "windows" ]; then
		# For Windows, use the installed binary path
		NATIVE_MESSAGING_HOST_BINARY="C:\\Program Files\\Eurora\\euro-native-messaging.exe"
		# Windows uses registry instead of file system for manifest
		MANIFEST_DIR=""
	else
		error "Unsupported OS for native messaging host: $OS"
	fi

	# Create the manifest content with the correct binary path
	MANIFEST_CONTENT=$(cat "$TEMPLATE_PATH" | sed "s|\"path\": \".*\"|\"path\": \"$NATIVE_MESSAGING_HOST_BINARY\"|")

	if [ "$OS" = "darwin" ] || [ "$OS" = "linux" ]; then
		# For darwin (macOS) and Linux, write the manifest to the filesystem
		for browser_dir in "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts" \
                            "$HOME/Library/Application Support/Chromium/NativeMessagingHosts" \
                            "$HOME/Library/Application Support/Microsoft Edge/NativeMessagingHosts" \
                            "$HOME/Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts" \
                            "$HOME/.config/google-chrome/NativeMessagingHosts" \
                            "$HOME/.config/chromium/NativeMessagingHosts" \
                            "$HOME/.config/microsoft-edge/NativeMessagingHosts" \
                            "$HOME/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"; do
			if [ -d "$browser_dir" ]; then
				MANIFEST_PATH="$browser_dir/com.eurora.app.json"
				echo "$MANIFEST_CONTENT" > "$MANIFEST_PATH"
				info "  - Installed manifest to $MANIFEST_PATH"
			fi
		done
	elif [ "$OS" = "windows" ]; then
		# For Windows, we need to create registry entries
		# This would typically be done by the installer, but we'll include the commands here
		# Note: This requires administrative privileges to run
		info "  - On Windows, the native messaging host manifest is installed via registry"
		info "  - The installer should run the following commands:"
		info "    REG ADD \"HKEY_LOCAL_MACHINE\\SOFTWARE\\Google\\Chrome\\NativeMessagingHosts\\com.eurora.app\" /ve /t REG_SZ /d \"C:\\Program Files\\Eurora\\com.eurora.app.json\" /f"
		info "    REG ADD \"HKEY_LOCAL_MACHINE\\SOFTWARE\\Chromium\\NativeMessagingHosts\\com.eurora.app\" /ve /t REG_SZ /d \"C:\\Program Files\\Eurora\\com.eurora.app.json\" /f"
		info "    REG ADD \"HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Edge\\NativeMessagingHosts\\com.eurora.app\" /ve /t REG_SZ /d \"C:\\Program Files\\Eurora\\com.eurora.app.json\" /f"

		# Also create the manifest file in the installation directory
		info "  - The manifest file should be placed at: C:\\Program Files\\Eurora\\com.eurora.app.json"
	fi
}

# Call the function to install the native messaging host
install_native_messaging_host

info "done! bye!"
