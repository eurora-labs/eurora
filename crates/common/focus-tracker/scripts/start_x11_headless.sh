#!/bin/bash
# Start headless X11 display for testing

set -euo pipefail

DISPLAY_NUM=${1:-99}
DISPLAY=":$DISPLAY_NUM"

echo "Starting headless X11 display $DISPLAY"

# Kill any existing Xvfb on this display
pkill -fx "Xvfb $DISPLAY" 2>/dev/null || true

# Start Xvfb
Xvfb $DISPLAY -screen 0 1024x768x24 -ac +extension GLX +render -noreset &
XVFB_PID=$!

# Wait for X server to start
sleep 2

# Export display for other processes
export DISPLAY=$DISPLAY

# Start a lightweight window manager
openbox --config-file /dev/null &
WM_PID=$!

echo "X11 headless display started:"
echo "  Display: $DISPLAY"
echo "  Xvfb PID: $XVFB_PID"
echo "  Window Manager PID: $WM_PID"
echo "  Export with: export DISPLAY=$DISPLAY"

# Create a cleanup function
cleanup() {
    echo "Cleaning up X11 headless display..."
    kill $WM_PID 2>/dev/null || true
    kill $XVFB_PID 2>/dev/null || true
    wait $XVFB_PID 2>/dev/null || true
    wait $WM_PID 2>/dev/null || true
    echo "Cleanup complete"
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Keep the script running
if [ "${2:-}" = "--daemon" ]; then
    # Run in background
    echo "Running in daemon mode"
    wait
else
    # Interactive mode - wait for user input
    echo "Press Enter to stop the headless display..."
    read
fi
