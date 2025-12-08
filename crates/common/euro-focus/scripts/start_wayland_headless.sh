#!/bin/bash
# Start headless Wayland display for testing

set -euo pipefail

echo "Starting headless Wayland display with Sway"

# Check if our specific Wayland display is already in use
if [ -n "${WAYLAND_DISPLAY:-}" ] && [ "$WAYLAND_DISPLAY" = "wayland-test" ]; then
    echo "Warning: WAYLAND_DISPLAY is already set to wayland-test"
fi

# Create a minimal sway config for headless mode
SWAY_CONFIG=$(mktemp)
cat > "$SWAY_CONFIG" << 'EOF'
# Minimal sway config for headless testing
output * {
    mode 1024x768
    position 0 0
}

# Basic key bindings for testing
bindsym Mod4+Return exec alacritty
bindsym Mod4+Shift+q kill
bindsym Mod4+Shift+e exit

# Disable idle
exec swayidle -w timeout 300 'swaymsg "output * power off"' resume 'swaymsg "output * power on"' &
EOF

echo "Using temporary sway config: $SWAY_CONFIG"

# Set environment variables for headless mode
export WLR_BACKENDS=headless
export WLR_LIBINPUT_NO_DEVICES=1
export WAYLAND_DISPLAY=wayland-test

# Check if socket already exists and clean it up
WAYLAND_SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/wayland-test"
if [ -S "$WAYLAND_SOCKET_PATH" ]; then
    echo "Warning: Wayland socket $WAYLAND_SOCKET_PATH already exists, removing it"
    rm -f "$WAYLAND_SOCKET_PATH"
fi

# Start sway in headless mode
sway --config "$SWAY_CONFIG" --unsupported-gpu &
SWAY_PID=$!

# Store PID for potential external cleanup
echo "$SWAY_PID" > "${XDG_RUNTIME_DIR:-/tmp}/sway-headless.pid"

# Wait for sway to start
sleep 3

echo "Wayland headless display started:"
echo "  Sway PID: $SWAY_PID"
echo "  Wayland Display: $WAYLAND_DISPLAY"
echo "  Export with: export WAYLAND_DISPLAY=$WAYLAND_DISPLAY"

# Create a cleanup function
cleanup() {
    echo "Cleaning up Wayland headless display..."
    kill $SWAY_PID 2>/dev/null || true
    wait $SWAY_PID 2>/dev/null || true
    rm -f "$SWAY_CONFIG"
    rm -f "${XDG_RUNTIME_DIR:-/tmp}/sway-headless.pid"
    echo "Cleanup complete"
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Keep the script running
if [ "${1:-}" = "--daemon" ]; then
    # Run in background
    echo "Running in daemon mode"
    wait
else
    # Interactive mode - wait for user input
    echo "Press Enter to stop the headless display..."
    read
fi
