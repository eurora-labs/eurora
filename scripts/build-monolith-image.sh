#!/usr/bin/env bash
#
# Build the be-monolith Docker image locally and optionally restart the
# docker-compose backend service.
#
# Usage:
#   ./scripts/build-monolith-image.sh              # build only
#   ./scripts/build-monolith-image.sh --restart     # build + restart compose backend
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_NAME="ghcr.io/eurora-labs/eurora/be-monolith:latest"
DOCKERFILE="$PROJECT_ROOT/crates/backend/be-monolith/Dockerfile"
COMPOSE_FILE="$PROJECT_ROOT/crates/app/euro-tauri/docker-compose.yml"

RESTART=false
for arg in "$@"; do
    case "$arg" in
        --restart) RESTART=true ;;
    esac
done

echo "Building be-monolith binary (release)..."
cargo build --release --package be-monolith --manifest-path "$PROJECT_ROOT/Cargo.toml"

echo "Building Docker image: $IMAGE_NAME"
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE" "$PROJECT_ROOT"

echo "Done. Image tagged as: $IMAGE_NAME"

if [ "$RESTART" = true ]; then
    echo "Restarting backend service..."
    docker compose -f "$COMPOSE_FILE" up -d backend
fi
