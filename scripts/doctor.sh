#!/usr/bin/env bash
#
# Pre-flight check for `just dev`. Validates that the developer's machine
# has the tools and configuration we need before we try to bring the stack
# up. Exit code is the number of failed checks (capped at 1) so the script
# fits cleanly into CI gates and `just dev: doctor` dependencies.
#
# Side-effect-free by design: nothing is installed or written. Failures
# carry a one-line remediation hint pointing at the exact command to run.
#
# Usage:
#   just doctor
#   ./scripts/doctor.sh         # equivalent

set -uo pipefail

# Colors only when stdout is a TTY. CI logs stay readable either way.
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    YELLOW='\033[0;33m'
    BOLD='\033[1m'
    DIM='\033[2m'
    RESET='\033[0m'
else
    GREEN=''
    RED=''
    YELLOW=''
    BOLD=''
    DIM=''
    RESET=''
fi

FAILED=0

# ─── Output helpers ────────────────────────────────────────────────────────

# Width of the check name column. Tuned to fit the longest label without
# wrapping at 80 cols.
COL=18

pass() {
    local label=$1 detail=${2:-}
    printf "  ${GREEN}✓${RESET} %-${COL}s ${DIM}%s${RESET}\n" "$label" "$detail"
}

fail() {
    local label=$1 detail=${2:-}
    printf "  ${RED}✗${RESET} %-${COL}s ${RED}%s${RESET}\n" "$label" "$detail"
    FAILED=$((FAILED + 1))
}

hint() {
    local message=$1
    printf "    ${DIM}↳ %s${RESET}\n" "$message"
}

# ─── Individual checks ─────────────────────────────────────────────────────

check_command() {
    local label=$1 cmd=$2 install_hint=$3
    if version=$($cmd --version 2>&1 | head -n1); then
        pass "$label" "$version"
        return 0
    fi
    fail "$label" "not installed"
    hint "$install_hint"
    return 1
}

check_docker_daemon() {
    if docker info >/dev/null 2>&1; then
        pass "docker daemon" "running"
        return 0
    fi
    fail "docker daemon" "not reachable"
    hint "Start Docker Desktop, or run: sudo systemctl start docker"
    return 1
}

# Returns 0 if `port` is free, 1 if it's in use. Uses a portable
# /dev/tcp probe so we don't depend on `nc` / `lsof` which vary by
# distribution.
port_in_use() {
    local port=$1
    (timeout 1 bash -c "</dev/tcp/127.0.0.1/$port") >/dev/null 2>&1
}

# True iff the host port $1 is bound by our docker-compose Postgres
# container. `docker compose port` resolves the publish mapping directly
# (e.g. "0.0.0.0:5434"), which is more robust than scraping `ps`.
port_owned_by_eurora_postgres() {
    local port=$1
    local binding
    binding=$(docker compose port postgres 5432 2>/dev/null) || return 1
    [ -n "$binding" ] && [ "${binding##*:}" = "$port" ]
}

check_port_free() {
    local label=$1 port=$2 hint_msg=$3
    if port_in_use "$port"; then
        fail "$label" "in use (port $port)"
        hint "$hint_msg"
        return 1
    fi
    pass "$label" "free (port $port)"
    return 0
}

check_postgres_port() {
    local port=$1
    if ! port_in_use "$port"; then
        pass "port $port" "free"
        return 0
    fi
    if port_owned_by_eurora_postgres "$port"; then
        pass "port $port" "in use by Eurora postgres container"
        return 0
    fi
    fail "port $port" "in use by something else"
    hint "Stop the conflicting process or set EURORA_POSTGRES_PORT in .env."
    return 1
}

check_env_file() {
    if [ -f "$REPO_ROOT/.env" ]; then
        pass ".env" "exists"
        return 0
    fi
    fail ".env" "not found"
    hint "Run: just bootstrap"
    return 1
}

check_openai_key() {
    local env_file=$REPO_ROOT/.env
    if [ ! -f "$env_file" ]; then
        # Already reported by `check_env_file`. Skip silently to keep
        # the output uncluttered.
        return 1
    fi
    local value
    value=$(awk -F= '/^OPENAI_API_KEY=/{ sub(/^OPENAI_API_KEY=/, ""); print; exit }' "$env_file")
    if [ -z "$value" ]; then
        fail "OPENAI_API_KEY" "unset in .env"
        hint "Get a key from https://platform.openai.com/api-keys and add it to .env."
        return 1
    fi
    if [[ "$value" == "sk-..." ]] || [[ "$value" == "sk_test" ]]; then
        fail "OPENAI_API_KEY" "still set to a placeholder"
        hint "Replace the placeholder in .env with a real key from https://platform.openai.com/api-keys."
        return 1
    fi
    pass "OPENAI_API_KEY" "set"
    return 0
}

# ─── Main ──────────────────────────────────────────────────────────────────

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

printf "${BOLD}Eurora dev environment doctor${RESET}\n"
printf "${DIM}─────────────────────────────${RESET}\n"

check_command "docker"      "docker"      "Install Docker Desktop or docker-engine: https://docs.docker.com/get-docker/" || true
if command -v docker >/dev/null 2>&1; then
    check_docker_daemon || true
    if docker compose version >/dev/null 2>&1; then
        pass "docker compose" "$(docker compose version --short 2>/dev/null || echo 'v2')"
    else
        fail "docker compose" "v2 not found"
        hint "Update Docker; v1 'docker-compose' is unsupported."
    fi
fi

check_command "cargo"       "cargo"       "Install Rust via https://rustup.rs" || true
check_command "cargo-watch" "cargo-watch" "Install with: cargo install cargo-watch" || true
check_command "pnpm"        "pnpm"        "Install with: corepack enable" || true
check_command "just"        "just"        "Install with: cargo install just" || true

# Port checks. Postgres has a special case because we run our own postgres,
# and the host port is overridable via EURORA_POSTGRES_PORT (set by the user
# when their host already has a Postgres on 5432). When invoked through
# `just doctor`, the env var is already exported via dotenv-load; when run
# standalone we fall back to grepping .env so the standalone path agrees.
postgres_port=${EURORA_POSTGRES_PORT:-}
if [ -z "$postgres_port" ] && [ -f "$REPO_ROOT/.env" ]; then
    postgres_port=$(awk -F= '/^EURORA_POSTGRES_PORT=/{ sub(/^EURORA_POSTGRES_PORT=/, ""); print; exit }' "$REPO_ROOT/.env")
fi
postgres_port=${postgres_port:-5432}

check_port_free "port 3000" 3000 "Stop the conflicting process or set HTTP_ADDR." || true
check_postgres_port "$postgres_port"                                              || true
check_port_free "port 5173" 5173 "Stop the conflicting process or move the web dev server." || true

check_env_file    || true
check_openai_key  || true

printf "\n"
if [ "$FAILED" -gt 0 ]; then
    printf "${RED}${BOLD}%d check(s) failed.${RESET}\n" "$FAILED"
    exit 1
fi
printf "${GREEN}${BOLD}All checks passed.${RESET}\n"
exit 0
