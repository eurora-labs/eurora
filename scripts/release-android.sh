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
	echo "	--build-number   build number (monotonic integer, used as versionCode)" 1>&"$to"
	echo "	--dist           path to copy the final .aab / .apk into" 1>&"$to"
	echo "	--upload         if set, upload .aab to Google Play via fastlane supply" 1>&"$to"
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

case "$CHANNEL" in
release)
	PACKAGE_NAME="com.eurora_labs.eurora"
	PRODUCT_NAME="Eurora"
	# Production release goes to the `production` track. Switch to `internal`
	# or `alpha` if you'd rather route the first push through testing first.
	PLAY_TRACK="${ANDROID_PLAY_TRACK:-production}"
	;;
nightly)
	PACKAGE_NAME="com.eurora_labs.eurora.nightly"
	PRODUCT_NAME="Eurora Nightly"
	PLAY_TRACK="${ANDROID_PLAY_TRACK:-internal}"
	;;
*)
	error "unsupported channel: $CHANNEL"
	;;
esac

: "${ANDROID_KEYSTORE_BASE64:?ANDROID_KEYSTORE_BASE64 is required}"
: "${ANDROID_KEYSTORE_PASSWORD:?ANDROID_KEYSTORE_PASSWORD is required}"
: "${ANDROID_KEY_ALIAS:?ANDROID_KEY_ALIAS is required}"
: "${ANDROID_KEY_PASSWORD:?ANDROID_KEY_PASSWORD is required}"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
MOBILE_DIR="$REPO_ROOT/crates/app/euro-mobile"
ANDROID_DIR="$MOBILE_DIR/gen/android"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

KEYSTORE_PATH="$WORK_DIR/upload.keystore"
TAURI_PROPS="$ANDROID_DIR/app/tauri.properties"

echo "==> Decoding upload keystore"
# `base64 -d` is portable across GNU/BSD coreutils; -o is macOS-only.
echo -n "$ANDROID_KEYSTORE_BASE64" | base64 -d > "$KEYSTORE_PATH"
chmod 600 "$KEYSTORE_PATH"

echo "==> Writing tauri.properties (versionCode=$BUILD_NUMBER, versionName=$VERSION)"
cat > "$TAURI_PROPS" <<EOF
tauri.android.versionCode=$BUILD_NUMBER
tauri.android.versionName=$VERSION
EOF

echo "==> Building signed Android App Bundle (release, channel=$CHANNEL)"
cd "$REPO_ROOT"

# These env vars are read by crates/app/euro-mobile/gen/android/app/build.gradle.kts
export ANDROID_CHANNEL="$CHANNEL"
export ANDROID_KEYSTORE_PATH="$KEYSTORE_PATH"
# ANDROID_KEYSTORE_PASSWORD / ANDROID_KEY_ALIAS / ANDROID_KEY_PASSWORD are
# already in the environment from the caller.

# `pnpm tauri android build --aab` produces a signed Android App Bundle at:
#   gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
pnpm tauri android build --aab --config "$MOBILE_DIR/tauri.conf.json"

AAB_PATH="$(find "$ANDROID_DIR/app/build/outputs/bundle" -maxdepth 3 -name '*-release.aab' | head -1)"
[ -z "$AAB_PATH" ] && error "no signed .aab produced under $ANDROID_DIR/app/build/outputs/bundle"

mkdir -p "$DIST"
DEST_AAB="$DIST/${PRODUCT_NAME// /_}_${VERSION}_${BUILD_NUMBER}.aab"
cp "$AAB_PATH" "$DEST_AAB"
echo "==> .aab at $DEST_AAB"

# Stash mapping.txt for Play Console deobfuscation, when minify produced one.
MAPPING_PATH="$(find "$ANDROID_DIR/app/build/outputs/mapping" -maxdepth 3 -name 'mapping.txt' | head -1 || true)"
if [ -n "$MAPPING_PATH" ]; then
	cp "$MAPPING_PATH" "$DIST/${PRODUCT_NAME// /_}_${VERSION}_${BUILD_NUMBER}_mapping.txt"
	echo "==> mapping.txt copied"
fi

if [ "$DO_UPLOAD" = "true" ]; then
	: "${GOOGLE_PLAY_SERVICE_ACCOUNT_JSON:?GOOGLE_PLAY_SERVICE_ACCOUNT_JSON is required for upload}"

	SA_PATH="$WORK_DIR/play-service-account.json"
	# Allow either raw JSON or base64-encoded JSON in the secret.
	if printf '%s' "$GOOGLE_PLAY_SERVICE_ACCOUNT_JSON" | head -c 1 | grep -q '{'; then
		printf '%s' "$GOOGLE_PLAY_SERVICE_ACCOUNT_JSON" > "$SA_PATH"
	else
		printf '%s' "$GOOGLE_PLAY_SERVICE_ACCOUNT_JSON" | base64 -d > "$SA_PATH"
	fi
	chmod 600 "$SA_PATH"

	echo "==> Uploading to Google Play (package=$PACKAGE_NAME, track=$PLAY_TRACK)"
	# fastlane supply works without a Fastfile when all inputs are passed as
	# CLI args. It's a Ruby gem; the workflow installs it before invoking us.
	SUPPLY_ARGS=(
		--aab "$DEST_AAB"
		--json_key "$SA_PATH"
		--package_name "$PACKAGE_NAME"
		--track "$PLAY_TRACK"
		--release_status "${ANDROID_RELEASE_STATUS:-completed}"
		--skip_upload_metadata true
		--skip_upload_changelogs true
		--skip_upload_images true
		--skip_upload_screenshots true
	)
	if [ -n "$MAPPING_PATH" ]; then
		SUPPLY_ARGS+=(--mapping "$MAPPING_PATH")
	fi
	fastlane supply "${SUPPLY_ARGS[@]}"
fi

echo "==> Done"
