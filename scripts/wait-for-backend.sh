#!/usr/bin/env bash
#
# Block until the backend's /health endpoint responds, with a 120s ceiling
# to cover a slow first-time debug compile.
#
# Used by `just dev` to delay web / desktop startup until the backend has
# bound its port — without this the Vite dev server tries to call /llm/info
# before the backend exists and the desktop app surfaces a connection error
# on boot.
#
# Implementation notes:
#   - Uses bash's $SECONDS for the deadline instead of GNU `timeout`, which
#     isn't on a vanilla macOS install.
#   - Exit code 0 on success, 1 on timeout — matches the contract the
#     justfile expects.

set -uo pipefail

URL=${EURORA_HEALTH_URL:-http://localhost:3000/health}
DEADLINE=${EURORA_HEALTH_TIMEOUT_SECS:-120}

start=$SECONDS
until curl -fsS "$URL" >/dev/null 2>&1; do
    if [ $((SECONDS - start)) -ge "$DEADLINE" ]; then
        echo "Backend did not become ready within ${DEADLINE}s." >&2
        exit 1
    fi
    sleep 0.5
done

echo "Backend is ready."
