#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY_NAME="euro-native-messaging"

echo "Building $BINARY_NAME..."
cargo build --package "$BINARY_NAME"

OS="$(uname -s)"
REPLACED=0

replace_binary() {
	local target="$1"
	local source="$2"
	if [[ -f "$target" ]]; then
		if ! cp "$source" "$target" 2>/dev/null; then
			echo "  Error: It appears some browser extensions are using the target binary, disable them first" >&2
			exit 1
		fi
		chmod 755 "$target"
		echo "  Replaced: $target"
		REPLACED=$((REPLACED + 1))
	fi
}

case "$OS" in
	Linux)
		SOURCE="$REPO_ROOT/target/debug/$BINARY_NAME"

		echo ""
		echo "Scanning for installed $BINARY_NAME binaries..."

		replace_binary "$HOME/.eurora/native-messaging/$BINARY_NAME" "$SOURCE"
		;;

	Darwin)
		SOURCE="$REPO_ROOT/target/debug/$BINARY_NAME"

		echo ""
		echo "Scanning for installed $BINARY_NAME binaries..."

		# Nightly
		replace_binary "/Applications/Eurora Nightly.app/Contents/MacOS/$BINARY_NAME" "$SOURCE"
		# Release
		replace_binary "/Applications/Eurora.app/Contents/MacOS/$BINARY_NAME" "$SOURCE"
		# Runtime copy (used by manifests on both Linux and macOS)
		replace_binary "$HOME/.eurora/native-messaging/$BINARY_NAME" "$SOURCE"
		;;

	MINGW*|MSYS*|CYGWIN*|Windows_NT)
		SOURCE="$REPO_ROOT/target/debug/$BINARY_NAME.exe"

		echo ""
		echo "Scanning for installed $BINARY_NAME.exe binaries..."

		LOCALAPPDATA_UNIX="$(cygpath "$LOCALAPPDATA" 2>/dev/null || echo "$LOCALAPPDATA")"

		for APP_DIR in "Eurora Nightly" "Eurora"; do
			INSTALL_DIR="$LOCALAPPDATA_UNIX/$APP_DIR"
			replace_binary "$INSTALL_DIR/$BINARY_NAME.exe" "$SOURCE"
			for BROWSER in chrome edge firefox; do
				replace_binary "$INSTALL_DIR/native-messaging/$BROWSER/$BINARY_NAME.exe" "$SOURCE"
			done
		done
		;;

	*)
		echo "Error: Unsupported platform '$OS'."
		exit 1
		;;
esac

echo ""
if [[ $REPLACED -eq 0 ]]; then
	echo "No installed $BINARY_NAME binaries found to replace."
else
	echo "Done. Replaced $REPLACED binary(ies)."
fi
