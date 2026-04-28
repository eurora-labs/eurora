#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

CHANNEL=""
VERSION=""
BUILD_NUMBER=""
DIST=""
DO_UPLOAD="false"

function help() {
	local to=$1
	echo "Usage: $0 <flags>" 1>&"$to"
	echo 1>&"$to"
	echo "flags:" 1>&"$to"
	echo "	--channel        release channel (release | nightly)" 1>&"$to"
	echo "	--version        marketing version (e.g. 1.2.3)" 1>&"$to"
	echo "	--build-number   build number (monotonic integer)" 1>&"$to"
	echo "	--dist           path to copy final .ipa into" 1>&"$to"
	echo "	--upload         if set, upload .ipa to TestFlight via altool" 1>&"$to"
	echo "	--help           display this message" 1>&"$to"
}

function error() {
	echo "error: $*" 1>&2
	help 2
	exit 1
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--channel)
		CHANNEL="$2"
		shift 2
		;;
	--version)
		VERSION="$2"
		shift 2
		;;
	--build-number)
		BUILD_NUMBER="$2"
		shift 2
		;;
	--dist)
		DIST="$2"
		shift 2
		;;
	--upload)
		DO_UPLOAD="true"
		shift
		;;
	--help)
		help 1
		exit 0
		;;
	*)
		error "unknown flag: $1"
		;;
	esac
done

[ -z "$CHANNEL" ] && error "--channel is required"
[ -z "$VERSION" ] && error "--version is required"
[ -z "$BUILD_NUMBER" ] && error "--build-number is required"
[ -z "$DIST" ] && error "--dist is required"

# Resolve --dist to an absolute path before we start cd'ing around.
mkdir -p "$DIST"
DIST="$(cd "$DIST" && pwd -P)"

case "$CHANNEL" in
release)
	BUNDLE_ID="com.eurora-labs.eurora"
	PRODUCT_NAME="Eurora"
	PROFILE_BASE64="${APPLE_IOS_PROVISIONING_PROFILE_RELEASE:-}"
	;;
nightly)
	BUNDLE_ID="com.eurora-labs.eurora.nightly"
	PRODUCT_NAME="Eurora Nightly"
	PROFILE_BASE64="${APPLE_IOS_PROVISIONING_PROFILE_NIGHTLY:-}"
	;;
*)
	error "unsupported channel: $CHANNEL"
	;;
esac

: "${APPLE_TEAM_ID:?APPLE_TEAM_ID is required}"
: "${APPLE_IOS_CERTIFICATE:?APPLE_IOS_CERTIFICATE is required}"
: "${APPLE_IOS_CERTIFICATE_PASSWORD:?APPLE_IOS_CERTIFICATE_PASSWORD is required}"
[ -z "$PROFILE_BASE64" ] && error "provisioning profile secret is empty for channel $CHANNEL"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
MOBILE_DIR="$REPO_ROOT/crates/app/euro-mobile"
APPLE_DIR="$MOBILE_DIR/gen/apple"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

CERT_PATH="$WORK_DIR/ios_cert.p12"
KEYCHAIN_PATH="$WORK_DIR/ios.keychain-db"
KEYCHAIN_PASSWORD="$(openssl rand -base64 32)"
PROFILE_PATH="$WORK_DIR/profile.mobileprovision"
EXPORT_OPTIONS="$WORK_DIR/ExportOptions.plist"
ARCHIVE_PATH="$WORK_DIR/euro-mobile_iOS.xcarchive"
EXPORT_DIR="$WORK_DIR/export"

echo "==> Importing iOS distribution certificate"
echo -n "$APPLE_IOS_CERTIFICATE" | base64 --decode -o "$CERT_PATH"
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security import "$CERT_PATH" -P "$APPLE_IOS_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$KEYCHAIN_PATH"
security set-key-partition-list -S apple-tool:,apple: -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security list-keychain -d user -s "$KEYCHAIN_PATH" "$(security list-keychains -d user | tr -d '"' | tr '\n' ' ')"

echo "==> Installing provisioning profile"
echo -n "$PROFILE_BASE64" | base64 --decode -o "$PROFILE_PATH"
PROFILE_UUID="$(security cms -D -i "$PROFILE_PATH" | plutil -extract UUID raw -)"
PROFILE_NAME="$(security cms -D -i "$PROFILE_PATH" | plutil -extract Name raw -)"
PROFILES_DIR="$HOME/Library/MobileDevice/Provisioning Profiles"
mkdir -p "$PROFILES_DIR"
cp "$PROFILE_PATH" "$PROFILES_DIR/$PROFILE_UUID.mobileprovision"
echo "    profile UUID: $PROFILE_UUID"
echo "    profile name: $PROFILE_NAME"

echo "==> Locating signing identity"
IDENTITY_LINES="$(security find-identity -v -p codesigning "$KEYCHAIN_PATH" | grep -E 'Apple Distribution|iPhone Distribution' || true)"
IDENTITY_COUNT="$(printf '%s' "$IDENTITY_LINES" | grep -cE '"' || true)"
if [ -z "$IDENTITY_LINES" ]; then
	error "no Apple Distribution / iPhone Distribution identity found in keychain"
fi
if [ "$IDENTITY_COUNT" -gt 1 ]; then
	echo "$IDENTITY_LINES" >&2
	error "expected exactly 1 distribution identity in keychain, found $IDENTITY_COUNT"
fi
SIGN_IDENTITY="$(printf '%s' "$IDENTITY_LINES" | sed -E 's/.*"(.*)"/\1/')"
echo "    identity: $SIGN_IDENTITY"

echo "==> Compiling Rust staticlib for iOS (release)"
rustup target add aarch64-apple-ios >/dev/null
cd "$REPO_ROOT"

CARGO_PROFILE_RELEASE_LTO=fat \
	CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
	CARGO_PROFILE_RELEASE_OPT_LEVEL=s \
	cargo build --release --target aarch64-apple-ios -p euro-mobile

mkdir -p "$APPLE_DIR/Externals/arm64/release"
cp "$REPO_ROOT/target/aarch64-apple-ios/release/libeuro_mobile.a" "$APPLE_DIR/Externals/arm64/release/libapp.a"

# `assets/` is a folder reference in project.yml. It can be empty (Tauri-build
# inlines the frontend into libapp.a), but the directory must exist or
# xcodebuild fails the resource-copy step.
mkdir -p "$APPLE_DIR/assets"

echo "==> Regenerating Xcode project from project.yml"
# The committed pbxproj reflects whatever Externals subdirs existed on the
# developer machine at xcodegen time (typically debug). Regenerate so the
# pbxproj only references what we just staged (arm64/release).
command -v xcodegen >/dev/null || error "xcodegen not found on PATH; install it (brew install xcodegen)"
( cd "$APPLE_DIR" && xcodegen generate --spec project.yml --quiet )

echo "==> Archiving Xcode project"
cd "$APPLE_DIR"

# `CI=true` instructs the project's "Build Rust Code" preBuildScript to no-op,
# since the staticlib is already compiled above.
CI=true xcodebuild archive \
	-project euro-mobile.xcodeproj \
	-scheme euro-mobile_iOS \
	-configuration release \
	-destination "generic/platform=iOS" \
	-archivePath "$ARCHIVE_PATH" \
	CODE_SIGN_STYLE=Manual \
	CODE_SIGN_IDENTITY="$SIGN_IDENTITY" \
	PROVISIONING_PROFILE_SPECIFIER="$PROFILE_NAME" \
	DEVELOPMENT_TEAM="$APPLE_TEAM_ID" \
	PRODUCT_BUNDLE_IDENTIFIER="$BUNDLE_ID" \
	PRODUCT_NAME="$PRODUCT_NAME" \
	MARKETING_VERSION="$VERSION" \
	CURRENT_PROJECT_VERSION="$BUILD_NUMBER" \
	OTHER_CODE_SIGN_FLAGS="--keychain $KEYCHAIN_PATH"

echo "==> Writing ExportOptions.plist"
APPLE_TEAM_ID="$APPLE_TEAM_ID" \
SIGN_IDENTITY="$SIGN_IDENTITY" \
BUNDLE_ID="$BUNDLE_ID" \
PROFILE_NAME="$PROFILE_NAME" \
EXPORT_OPTIONS="$EXPORT_OPTIONS" \
python3 - <<'PY'
import os, plistlib
with open(os.environ["EXPORT_OPTIONS"], "wb") as f:
    plistlib.dump({
        "method": "app-store-connect",
        "destination": "export",
        "teamID": os.environ["APPLE_TEAM_ID"],
        "signingStyle": "manual",
        "signingCertificate": os.environ["SIGN_IDENTITY"],
        "provisioningProfiles": {
            os.environ["BUNDLE_ID"]: os.environ["PROFILE_NAME"],
        },
        "uploadSymbols": True,
        "stripSwiftSymbols": True,
    }, f)
PY

echo "==> Exporting signed .ipa"
xcodebuild -exportArchive \
	-archivePath "$ARCHIVE_PATH" \
	-exportPath "$EXPORT_DIR" \
	-exportOptionsPlist "$EXPORT_OPTIONS"

IPA_PATH="$(find "$EXPORT_DIR" -maxdepth 2 -name '*.ipa' | head -1)"
[ -z "$IPA_PATH" ] && error "no .ipa produced under $EXPORT_DIR"

DEST_IPA="$DIST/${PRODUCT_NAME// /_}_${VERSION}_${BUILD_NUMBER}.ipa"
cp "$IPA_PATH" "$DEST_IPA"
echo "==> .ipa at $DEST_IPA"

# Stash dSYMs alongside for Sentry / debugging.
DSYM_DIR="$ARCHIVE_PATH/dSYMs"
if [ -d "$DSYM_DIR" ]; then
	(cd "$DSYM_DIR" && zip -qr "$DIST/${PRODUCT_NAME// /_}_${VERSION}_${BUILD_NUMBER}_dSYMs.zip" .)
	echo "==> dSYMs zipped"
fi

if [ "$DO_UPLOAD" = "true" ]; then
	: "${APP_STORE_CONNECT_API_KEY_ID:?APP_STORE_CONNECT_API_KEY_ID is required for upload}"
	: "${APP_STORE_CONNECT_API_ISSUER_ID:?APP_STORE_CONNECT_API_ISSUER_ID is required for upload}"
	: "${APP_STORE_CONNECT_API_KEY_BASE64:?APP_STORE_CONNECT_API_KEY_BASE64 is required for upload}"

	# altool resolves --apiKey by looking for AuthKey_<id>.p8 in one of:
	#   ./private_keys, ~/private_keys, ~/.private_keys, ~/.appstoreconnect/private_keys
	API_KEY_DIR="$HOME/.appstoreconnect/private_keys"
	mkdir -p "$API_KEY_DIR"
	API_KEY_PATH="$API_KEY_DIR/AuthKey_${APP_STORE_CONNECT_API_KEY_ID}.p8"
	echo -n "$APP_STORE_CONNECT_API_KEY_BASE64" | base64 --decode -o "$API_KEY_PATH"
	chmod 600 "$API_KEY_PATH"

	echo "==> Uploading to App Store Connect (TestFlight)"
	xcrun altool --upload-app \
		--type ios \
		--file "$DEST_IPA" \
		--apiKey "$APP_STORE_CONNECT_API_KEY_ID" \
		--apiIssuer "$APP_STORE_CONNECT_API_ISSUER_ID"
fi

echo "==> Done"
