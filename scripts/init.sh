#!/usr/bin/env bash
#
# First-run setup: copy .env.example to .env if .env doesn't exist yet.
# Idempotent — safe to re-run.

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$REPO_ROOT"

if [ ! -f .env ]; then
    cp .env.example .env
    echo ".env created from .env.example — open it and set OPENAI_API_KEY."
else
    echo ".env already exists — leaving it alone."
fi
