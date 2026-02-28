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

# Ensure license file exists for bundling (CI generates it via generate-licenses)
LICENSES_FILE="$PWD/../crates/app/euro-tauri/LICENSES-THIRD-PARTY.md"
if [ ! -f "$LICENSES_FILE" ]; then
	info "LICENSES-THIRD-PARTY.md not found, creating placeholder"
	echo "# Third-Party Licenses" > "$LICENSES_FILE"
fi

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

# Export hosts directory for the WiX fragment (Tauri propagates TAURI_-prefixed env vars to candle/light)
if [ "$OS" = "windows" ]; then
	export TAURI_HOSTS_DIR
	TAURI_HOSTS_DIR="$(readlink -f "$PWD/../crates/app/euro-tauri/hosts")"
fi

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
	elif [ "$OS" = "darwin" ]; then
		ditto "$PWD/../target/$TARGET/release/euro-native-messaging" "$BINARIES_DIR/euro-native-messaging-$TARGET"
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
	elif [ "$OS" = "darwin" ]; then
		ditto "$PWD/../target/release/euro-native-messaging" "$BINARIES_DIR/euro-native-messaging-$DEFAULT_TARGET"
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
	# The final DMG is produced by the assemble-macos-dmg step in CI,
	# which embeds this Tauri .app inside the unified Eurora.app wrapper.
	# Here we only copy the updater artifacts (tar.gz + signature).
	MACOS_UPDATER="$(find "$BUNDLE_DIR/macos" -depth 1 -type f -name "*.tar.gz")"
	MACOS_UPDATER_SIG="$(find "$BUNDLE_DIR/macos" -depth 1 -type f -name "*.tar.gz.sig")"

	ditto "$MACOS_UPDATER" "$RELEASE_DIR/$(basename "$MACOS_UPDATER")"
	ditto "$MACOS_UPDATER_SIG" "$RELEASE_DIR/$(basename "$MACOS_UPDATER_SIG")"

	info "built:"
	info "	- $RELEASE_DIR/$(basename "$MACOS_UPDATER")"
	info "	- $RELEASE_DIR/$(basename "$MACOS_UPDATER_SIG")"
elif [ "$OS" = "linux" ]; then
	# With createUpdaterArtifacts: true, the AppImage itself is the updater
	# artifact (no .tar.gz wrapper). Signature is .AppImage.sig.
	APPIMAGE="$(find "$BUNDLE_DIR/appimage" -name '*.AppImage' -not -name '*.sig')"
	APPIMAGE_SIG="$(find "$BUNDLE_DIR/appimage" -name '*.AppImage.sig')"
	DEB="$(find "$BUNDLE_DIR/deb" -name '*.deb')"
	RPM="$(find "$BUNDLE_DIR/rpm" -name '*.rpm')"

	# Sign .deb and .rpm so the updater can serve them to deb/rpm-installed clients.
	# Tauri only generates .AppImage.sig by default; without these signatures the
	# update service would have to fall back to serving AppImage to all Linux users.
	info "Signing .deb package..."
	tauri signer sign "$DEB"
	DEB_SIG="${DEB}.sig"

	info "Signing .rpm package..."
	tauri signer sign "$RPM"
	RPM_SIG="${RPM}.sig"

	cp "$APPIMAGE" "$RELEASE_DIR"
	cp "$APPIMAGE_SIG" "$RELEASE_DIR"
	cp "$DEB" "$RELEASE_DIR"
	cp "$DEB_SIG" "$RELEASE_DIR"
	cp "$RPM" "$RELEASE_DIR"
	cp "$RPM_SIG" "$RELEASE_DIR"

	info "built:"
	info "	- $RELEASE_DIR/$(basename "$APPIMAGE")"
	info "	- $RELEASE_DIR/$(basename "$APPIMAGE_SIG")"
	info "	- $RELEASE_DIR/$(basename "$DEB")"
	info "	- $RELEASE_DIR/$(basename "$DEB_SIG")"
	info "	- $RELEASE_DIR/$(basename "$RPM")"
	info "	- $RELEASE_DIR/$(basename "$RPM_SIG")"
elif [ "$OS" = "windows" ]; then
	# With createUpdaterArtifacts: true, the NSIS installer itself is the
	# updater artifact (no .zip wrapper). Signature is .exe.sig.
	WINDOWS_EXE="$(find "$BUNDLE_DIR/nsis" -name '*.exe' -not -name '*.sig')"
	WINDOWS_EXE_SIG="$(find "$BUNDLE_DIR/nsis" -name '*.exe.sig')"

	cp "$WINDOWS_EXE" "$RELEASE_DIR"
	cp "$WINDOWS_EXE_SIG" "$RELEASE_DIR"

	info "built:"
	info "	- $RELEASE_DIR/$(basename "$WINDOWS_EXE")"
	info "	- $RELEASE_DIR/$(basename "$WINDOWS_EXE_SIG")"
else
	error "unsupported os: $OS"
fi

info "done! bye!"
