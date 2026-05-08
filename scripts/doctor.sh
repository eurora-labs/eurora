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
    case "$(uname -s)" in
        Darwin) hint "Start Docker Desktop: open -a Docker (or 'just dev' — auto-starts it)" ;;
        Linux)  hint "Start Docker: sudo systemctl start docker" ;;
        *)      hint "Start your Docker daemon." ;;
    esac
    return 1
}

# Returns 0 if `port` is in use, non-zero if it's free. Uses bash's
# built-in /dev/tcp redirection — portable across Linux and macOS without
# depending on `nc` / `lsof` (which vary by distribution) or `timeout`
# (GNU coreutils, missing from a vanilla macOS install).
#
# On localhost this is fast: the kernel returns ECONNREFUSED immediately
# when nothing is listening, so no explicit timeout is needed.
port_in_use() {
    local port=$1
    (exec 3<>/dev/tcp/127.0.0.1/$port) >/dev/null 2>&1
    local rc=$?
    exec 3<&- 2>/dev/null || true
    exec 3>&- 2>/dev/null || true
    return $rc
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
    hint "Stop the conflicting process or change the host-side port in docker-compose.yml."
    return 1
}

check_env_file() {
    if [ -f "$REPO_ROOT/.env" ]; then
        pass ".env" "exists"
        return 0
    fi
    fail ".env" "not found"
    hint "Run: just init"
    return 1
}

# Resolve `key`'s effective value. Process env wins (so values
# exported by `set dotenv-load` in the justfile take precedence); we
# fall back to grepping `.env` for the standalone invocation path
# (`./scripts/doctor.sh`). Empty string if neither defines it.
#
# We deliberately do NOT `source .env` — its contents are user-provided
# and can contain command substitutions that would execute under bash.
# Awk-grepping the raw value is the safe equivalent.
resolve_env_value() {
    local key=$1
    local value=${!key:-}
    if [ -n "$value" ]; then
        printf '%s' "$value"
        return 0
    fi
    if [ -f "$REPO_ROOT/.env" ]; then
        awk -F= -v k="$key" '
            $1 == k {
                sub(/^[^=]*=/, "")
                print
                exit
            }
        ' "$REPO_ROOT/.env"
    fi
}

# Print the names of every required env var, sourced from
# `.env.example` (every uncommented `KEY=VALUE` line). `.env.example`
# is the single source of truth — adding a required key means
# uncommenting it there, and doctor picks it up automatically.
#
# OPENAI_API_KEY is excluded here because `check_openai_key` runs a
# more detailed check (placeholder detection) for it specifically.
parse_required_env() {
    awk -F= '
        /^[[:space:]]*#/ { next }
        /^[[:space:]]*$/ { next }
        /^[A-Z_][A-Z0-9_]*=/ {
            split($0, parts, "=")
            if (parts[1] != "OPENAI_API_KEY") print parts[1]
        }
    ' "$REPO_ROOT/.env.example"
}

check_env_complete() {
    if [ ! -f "$REPO_ROOT/.env.example" ]; then
        fail "env vars" ".env.example not found at repo root"
        return 1
    fi
    local missing=() total=0 key value
    while IFS= read -r key; do
        total=$((total + 1))
        value=$(resolve_env_value "$key")
        if [ -z "$value" ]; then
            missing+=("$key")
        fi
    done < <(parse_required_env)

    if [ "${#missing[@]}" -eq 0 ]; then
        pass "env vars" "$total/$total required keys set"
        return 0
    fi

    fail "env vars" "${#missing[@]} of $total required key(s) missing"
    # If only a few are missing, list them inline; otherwise just
    # point at the canonical reference to keep the output readable.
    if [ "${#missing[@]}" -le 5 ]; then
        hint "Add to .env: ${missing[*]}"
    else
        hint "Run \`just init\` to create .env from .env.example, then re-run doctor."
        hint "Missing: ${missing[*]:0:5} … (+$((${#missing[@]} - 5)) more)"
    fi
    return 1
}

check_openai_key() {
    local value
    value=$(resolve_env_value OPENAI_API_KEY)
    if [ -z "$value" ]; then
        fail "OPENAI_API_KEY" "unset"
        hint "Get a key from https://platform.openai.com/api-keys and add it to .env."
        return 1
    fi
    if [ "$value" = "sk-..." ] || [ "$value" = "sk_test" ]; then
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

# Port checks. We resolve the backend port from the user's
# `BACKEND_URL` so the doctor follows whatever they've configured;
# the literal fallbacks (3000 / 5434) only fire when the variables
# are unset (e.g., a fresh checkout where doctor is run before
# `just init`) so the doctor itself stays usable in that broken state.
backend_url=$(resolve_env_value BACKEND_URL)
backend_url=${backend_url:-http://localhost:3000}
http_port=${backend_url##*:}
http_port=${http_port%%/*}
http_port=${http_port:-3000}

# The host port the postgres container binds on the host. Hardcoded in
# `docker-compose.yml` (5434) — the doctor matches that default.
postgres_port=5434

check_port_free "port $http_port" "$http_port" "Stop the conflicting process or update BACKEND_URL." || true
check_postgres_port "$postgres_port"                                                                 || true
check_port_free "port 5173" 5173 "Stop the conflicting process or move the web dev server."         || true

check_env_file    || true
check_env_complete || true
check_openai_key  || true

printf "\n"
if [ "$FAILED" -gt 0 ]; then
    printf "${RED}${BOLD}%d check(s) failed.${RESET}\n" "$FAILED"
    exit 1
fi
printf "${GREEN}${BOLD}All checks passed.${RESET}\n"
exit 0
