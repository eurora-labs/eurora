#!/usr/bin/env bash
#
# Build the production be-monolith Docker image locally. Useful for testing
# the image CI publishes; *not* needed for normal development — `just dev`
# runs the backend natively for fast iteration.
#
# Usage:
#   ./scripts/build-monolith-image.sh
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_NAME="ghcr.io/eurora-labs/eurora/be-monolith:latest"
DOCKERFILE="$PROJECT_ROOT/crates/backend/be-monolith/Dockerfile"

echo "Building be-monolith binary (release)..."
cargo build --release --package be-monolith --manifest-path "$PROJECT_ROOT/Cargo.toml"

echo "Building Docker image: $IMAGE_NAME"
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE" "$PROJECT_ROOT"

echo "Done. Image tagged as: $IMAGE_NAME"
