#!/bin/bash
# Stop headless Wayland display using saved PID

set -euo pipefail

PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/sway-headless.pid"

if [ ! -f "$PID_FILE" ]; then
    echo "No PID file found at $PID_FILE"
    echo "Headless Wayland session may not be running or was started differently"
    exit 1
fi

SWAY_PID=$(cat "$PID_FILE")

if [ -z "$SWAY_PID" ]; then
    echo "PID file is empty"
    rm -f "$PID_FILE"
    exit 1
fi

# Check if the process is still running
if ! kill -0 "$SWAY_PID" 2>/dev/null; then
    echo "Process $SWAY_PID is not running"
    rm -f "$PID_FILE"
    exit 1
fi

# Check if it's actually a sway process
if ! ps -p "$SWAY_PID" -o comm= | grep -q "sway"; then
    echo "Process $SWAY_PID is not a sway process"
    rm -f "$PID_FILE"
    exit 1
fi

echo "Stopping headless Wayland session (PID: $SWAY_PID)..."
kill "$SWAY_PID"

# Wait for the process to terminate
for i in {1..10}; do
    if ! kill -0 "$SWAY_PID" 2>/dev/null; then
        echo "Headless Wayland session stopped successfully"
        rm -f "$PID_FILE"
        exit 0
    fi
    sleep 1
done

# If it's still running, force kill
echo "Process didn't terminate gracefully, force killing..."
kill -9 "$SWAY_PID" 2>/dev/null || true
rm -f "$PID_FILE"
echo "Headless Wayland session force stopped"
