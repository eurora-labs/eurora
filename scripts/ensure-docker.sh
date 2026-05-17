#!/usr/bin/env bash
#
# Ensure the Docker daemon is reachable before `just dev` runs the
# doctor. Doctor itself is side-effect-free by contract — this script
# is the place where we're allowed to *act*.
#
# Behavior:
#   - daemon already up    → exit 0 immediately, no output
#   - macOS, daemon down   → `open -a Docker`, then poll until ready
#   - Linux, daemon down   → exit 0 (starting it needs sudo; doctor
#                            will surface the failure with a hint)
#   - docker not installed → exit 0 (doctor will report it)
#
# Idempotent. Cheap on the happy path (one `docker info` call).

set -uo pipefail

command -v docker >/dev/null 2>&1 || exit 0
docker info >/dev/null 2>&1 && exit 0

case "$(uname -s)" in
    Darwin)
        if ! command -v open >/dev/null 2>&1; then
            exit 0
        fi
        echo "Docker daemon not running — starting Docker Desktop…"
        open -a Docker
        ;;
    *)
        exit 0
        ;;
esac

DEADLINE=${EURORA_DOCKER_TIMEOUT_SECS:-90}
start=$SECONDS
until docker info >/dev/null 2>&1; do
    if [ $((SECONDS - start)) -ge "$DEADLINE" ]; then
        echo "Docker did not become ready within ${DEADLINE}s." >&2
        exit 1
    fi
    sleep 1
done
echo "Docker is ready."
