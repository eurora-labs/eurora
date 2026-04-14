#!/usr/bin/env bash
set -euo pipefail

# Install and configure everything needed for Tauri Android development
# on macOS Apple Silicon. Safe to re-run — each step is idempotent.

# ── helpers ───────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${GREEN}[✓]${RESET} $*"; }
warn()  { echo -e "${YELLOW}[!]${RESET} $*"; }
error() { echo -e "${RED}[✗]${RESET} $*" >&2; exit 1; }

# ── platform guard ────────────────────────────────────────────────────────

[[ "$(uname -s)" == "Darwin" ]] || error "This script only supports macOS."
[[ "$(uname -m)" == "arm64"  ]] || error "This script only supports Apple Silicon (arm64)."

# ── homebrew ──────────────────────────────────────────────────────────────

command -v brew &>/dev/null || error "Homebrew is required. Install it from https://brew.sh"
info "Homebrew found"

# ── JDK 17 (Azul Zulu — native Apple Silicon build) ──────────────────────

if /usr/libexec/java_home -v 17 &>/dev/null; then
	info "JDK 17 already installed"
else
	warn "Installing JDK 17 (zulu@17)…"
	brew install --cask zulu@17
fi

JAVA_HOME_VAL="$(/usr/libexec/java_home -v 17)"
info "JAVA_HOME → $JAVA_HOME_VAL"

# ── Android command-line tools ────────────────────────────────────────────

ANDROID_HOME_VAL="$HOME/Library/Android/sdk"

if [[ -d "$ANDROID_HOME_VAL/cmdline-tools/latest/bin" ]]; then
	info "Android command-line tools already installed"
else
	if brew list --cask android-commandlinetools &>/dev/null; then
		info "android-commandlinetools cask present, linking SDK…"
	else
		warn "Installing Android command-line tools…"
		brew install --cask android-commandlinetools
	fi

	# Homebrew installs cmdline-tools to its own prefix. Bootstrap the
	# canonical SDK location so sdkmanager writes to ~/Library/Android/sdk.
	BREW_CMDLINE="$(brew --prefix)/share/android-commandlinetools"
	mkdir -p "$ANDROID_HOME_VAL"
	if [[ -d "$BREW_CMDLINE" ]]; then
		JAVA_HOME="$JAVA_HOME_VAL" "$BREW_CMDLINE/cmdline-tools/latest/bin/sdkmanager" \
			--sdk_root="$ANDROID_HOME_VAL" "cmdline-tools;latest"
	fi
fi

SDKMANAGER="$ANDROID_HOME_VAL/cmdline-tools/latest/bin/sdkmanager"
[[ -x "$SDKMANAGER" ]] || error "sdkmanager not found at $SDKMANAGER"

# ── Android SDK components ────────────────────────────────────────────────
# Versions match crates/app/euro-mobile/gen/android (compileSdk=36, targetSdk=36)

SDK_PACKAGES=(
	"platform-tools"
	"platforms;android-36"
	"build-tools;36.0.0"
	"ndk;28.0.12674087"
)

warn "Installing Android SDK components (you may be prompted to accept licenses)…"
JAVA_HOME="$JAVA_HOME_VAL" "$SDKMANAGER" --sdk_root="$ANDROID_HOME_VAL" "${SDK_PACKAGES[@]}"
info "Android SDK components installed"

NDK_HOME_VAL="$ANDROID_HOME_VAL/ndk/28.0.12674087"

# ── Android Studio ────────────────────────────────────────────────────────
# Required by `tauri android dev` to build, deploy, and launch the APK.

if [[ -d "/Applications/Android Studio.app" ]]; then
	info "Android Studio already installed"
else
	warn "Installing Android Studio…"
	brew install --cask android-studio
fi

# ── Rust Android targets ─────────────────────────────────────────────────

RUST_TARGETS=(
	aarch64-linux-android
	armv7-linux-androideabi
	i686-linux-android
	x86_64-linux-android
)

for target in "${RUST_TARGETS[@]}"; do
	if rustup target list --installed | grep -q "^${target}$"; then
		info "Rust target ${target} already installed"
	else
		warn "Adding Rust target ${target}…"
		rustup target add "$target"
	fi
done

# ── environment variables ─────────────────────────────────────────────────

ENV_MARKER="# eurora-android-dev"

# Detect the active shell rc file
if [[ -n "${ZSH_VERSION:-}" ]] || [[ "$SHELL" == */zsh ]]; then
	SHELL_RC="$HOME/.zshrc"
elif [[ -n "${BASH_VERSION:-}" ]] || [[ "$SHELL" == */bash ]]; then
	SHELL_RC="$HOME/.bashrc"
elif [[ "$SHELL" == */fish ]]; then
	SHELL_RC="$HOME/.config/fish/config.fish"
else
	SHELL_RC="$HOME/.zshrc"
	warn "Unknown shell, falling back to $SHELL_RC"
fi

write_env_vars() {
	local NDK_TOOLCHAIN="$NDK_HOME_VAL/toolchains/llvm/prebuilt/darwin-x86_64/bin"

	if [[ "$SHELL_RC" == *.fish ]]; then
		cat <<-EOF

		$ENV_MARKER
		set -gx JAVA_HOME "$JAVA_HOME_VAL"
		set -gx ANDROID_HOME "$ANDROID_HOME_VAL"
		set -gx NDK_HOME "$NDK_HOME_VAL"
		fish_add_path "$NDK_TOOLCHAIN"
		fish_add_path "\$ANDROID_HOME/cmdline-tools/latest/bin"
		fish_add_path "\$ANDROID_HOME/platform-tools"
		fish_add_path "\$ANDROID_HOME/emulator"
		EOF
	else
		cat <<-EOF

		$ENV_MARKER
		export JAVA_HOME="$JAVA_HOME_VAL"
		export ANDROID_HOME="$ANDROID_HOME_VAL"
		export NDK_HOME="$NDK_HOME_VAL"
		export PATH="$NDK_TOOLCHAIN:\$ANDROID_HOME/cmdline-tools/latest/bin:\$ANDROID_HOME/platform-tools:\$ANDROID_HOME/emulator:\$PATH"
		EOF
	fi
}

if grep -qF "$ENV_MARKER" "$SHELL_RC" 2>/dev/null; then
	info "Environment variables already present in $SHELL_RC"
else
	write_env_vars >> "$SHELL_RC"
	info "Environment variables appended to $SHELL_RC"
fi

# ── summary ───────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}Android dev environment ready!${RESET}"
echo ""
echo "  JAVA_HOME     $JAVA_HOME_VAL"
echo "  ANDROID_HOME  $ANDROID_HOME_VAL"
echo "  NDK_HOME      $NDK_HOME_VAL"
echo ""
echo "  Rust targets  ${RUST_TARGETS[*]}"
echo ""
echo -e "${YELLOW}Restart your shell (or run 'source $SHELL_RC') to pick up the new environment variables.${RESET}"
echo ""
echo "To verify, run:"
echo "  pnpm tauri android init    # (if not already initialized)"
echo "  pnpm tauri android dev     # start the dev server on a connected device/emulator"
