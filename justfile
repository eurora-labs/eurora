# Local development orchestration for Eurora.
#
# `just dev` is the one-command path: brings up Postgres, seeds the dev
# user/threads on first run, runs the backend, the web auth UI, and the
# desktop app — all wired to talk to each other on localhost.
#
# Individual recipes are exposed for when you need to iterate on one piece
# without restarting the rest.
#
#   just init              first-run setup (.env, pnpm install)
#   just dev               full stack (backend hot-reloads via watchexec)
#   just ios               full stack with mobile on an iOS simulator (macOS only)
#   just ios-device        full stack with mobile on a physical iPhone (macOS only)
#   just dev-backend       backend only (Postgres + watchexec)
#   just dev-backend-once  backend only, no auto-restart (debugger / profiling)
#   just dev-web           web auth UI only
#   just dev-desktop       desktop app only
#   just dev-postgres      Postgres only (no seed, no backend)
#   just dev-migrate       apply schema migrations (idempotent)
#   just dev-reset         wipe the DB volume and re-seed
#   just doctor            validate .env / tooling before running
#   just logs              tail Postgres logs
#   just stop              tear down docker-compose containers (keeps volume)
#
# Cross-platform notes:
#
#   - Linux and macOS recipes run under `bash`. Windows recipes run under
#     Windows PowerShell (`powershell.exe`, ships with the OS) — no WSL,
#     no Git Bash required. Recipe bodies that contain non-trivial shell
#     logic are split via `[unix]` / `[windows]` attributes and delegate
#     to scripts/*.{sh,ps1}.
#
#   - Env handling: `set dotenv-load` reads `.env` at the workspace root and
#     exports every variable to the child processes spawned below. That
#     single file is the contract — `just` is the only thing that reads
#     `.env`. The Rust crates (binaries, tests, build scripts) only read
#     from process env via `std::env::var`, so production deploys (where
#     no `.env` exists) and CI (where vars come from `secrets.*`) walk
#     the same code path. Vite reads `.env` via `envDir`, since Vite is
#     not invoked through `just` directly.
#
#     `BACKEND_URL` and `WEB_URL` default to localhost via the `export`
#     declarations below, so they don't need to live in `.env`. Anything
#     already in the environment (CI, the `ios-device` recipe shell)
#     wins via `env_var_or_default`. Recipes can also override per-call
#     by re-exporting in their own shell.
#
#     To run a Rust binary or test outside `just`, export the variables
#     yourself first (e.g. `set -a; source .env; set +a; cargo run …`),
#     or use `direnv` (the repo ships an `.envrc`).

set dotenv-load
set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

export BACKEND_URL := env_var_or_default("BACKEND_URL", "http://localhost:3000")
export WEB_URL := env_var_or_default("WEB_URL", "http://localhost:5173")

default: dev

# ─── Full stack ────────────────────────────────────────────────────────────
#
# Postgres + backend + web + desktop, all in one terminal. Pieces share
# `just` itself as the supervisor; Ctrl-C stops them all.
#
# `_dev-backend-watch` runs the backend under `watchexec`, which terminates
# and re-runs `cargo run -p be-monolith` on every save under
# `crates/backend/` or `crates/common/`, plus the workspace manifests.
# `crates/app/` is intentionally excluded — those are clients of the
# backend, not dependencies of it, so a desktop or mobile edit shouldn't
# bounce the server. The watch is also extension-filtered to .rs / .toml
# so log writes and editor scratch files don't trigger restarts.
#
# Cost: a save in any backend or common crate restarts be-monolith even
# if the edit didn't touch its actual dep graph; benefit: you don't have
# to remember which crate triggers what. Use `dev-backend-once` for
# stable runs (debugger, profiling, watching a steady tracing tail).
#
# Each child of `concurrently` is itself a `just` recipe call. That keeps
# shell-quoting consistent across platforms (cmd.exe vs bash tokenize
# embedded quotes differently) and lets the per-platform shell handle the
# actual command.

# Postgres + backend + web + desktop, all in one terminal.
dev: _ensure-docker doctor dev-postgres-up dev-migrate dev-seed-if-empty
    pnpm exec concurrently --kill-others \
        --names backend,web,desktop --prefix-colors cyan,green,yellow \
        "just _dev-backend-watch" \
        "just _dev-web-after-backend" \
        "just _dev-desktop-after-backend"

_dev-backend-watch:
    watchexec --restart --exts rs,toml \
        --watch crates/backend --watch crates/common \
        --watch Cargo.toml --watch Cargo.lock \
        -- cargo run -p be-monolith

_dev-web-after-backend: _wait-for-backend
    pnpm dev:web

_dev-desktop-after-backend: _wait-for-backend
    pnpm dev:desktop

# ─── Full stack (iOS) ──────────────────────────────────────────────────────
#
# Mirrors `just dev`, but the desktop app is replaced with the mobile app
# running on an iOS simulator. macOS-only — `tauri ios dev` requires Xcode.
#
# `--handle-input --default-input-target mobile` routes stdin to the
# mobile child so Tauri's interactive simulator picker actually receives
# your keystrokes. Without it, `concurrently` swallows stdin and the
# picker hangs.
#
# Unlike Android, iOS does not need a separate Vite process here: Tauri's
# `beforeDevCommand` in tauri.conf.json spawns Vite itself.

[macos]
ios: _ensure-docker doctor dev-postgres-up dev-migrate dev-seed-if-empty
    pnpm exec concurrently --kill-others --handle-input --default-input-target mobile \
        --names backend,web,mobile --prefix-colors cyan,green,magenta \
        "just _dev-backend-watch" \
        "just _dev-web-after-backend" \
        "just _dev-ios-after-backend"

[private]
[macos]
_dev-ios-after-backend: _wait-for-backend
    pnpm dev:ios

# ─── Full stack (iOS device) ───────────────────────────────────────────────
#
# Same shape as `just ios`, but uses a LAN-reachable host so a physical
# iPhone on the same Wi-Fi can reach this Mac. On a real device,
# `localhost` resolves to the iPhone itself, so every embedded reference
# to `localhost:3000` (backend) or `localhost:5173` (web auth) is broken.
#
# `scripts/lan-ip.sh` resolves the Mac's IP on its default-route
# interface and refuses to proceed if that's a CLAT46 synthesized
# address (RFC 7335) — typical when the upstream is iPhone Personal
# Hotspot on a 5G/IPv6-only carrier, where the IP exists only on the
# Mac. The recipe exports the resulting LAN host into the host-side
# processes spawned below:
#
#   - apps/mobile/vite.config.ts    reads TAURI_DEV_HOST for its bind
#   - apps/web/vite.config.ts       derives its bind from WEB_URL
#   - be-monolith                   reads BACKEND_URL / WEB_URL at runtime
#   - scripts/wait-for-backend.sh   polls the backend health URL
#
# The iOS cargo build that bakes the URLs into the binary runs inside
# xcodebuild's script phase, which doesn't reliably propagate parent-
# shell env vars. That path re-derives the host independently via
# `_ios-xcode-script` (called from `gen/apple/project.yml`), so the
# device build picks up the same LAN IP without depending on env
# survival across xcodebuild.
#
# `--host=$TAURI_DEV_HOST` is passed explicitly (not bare `--host`,
# which the simulator path uses) so we don't gamble on Tauri's
# auto-detection picking the same interface `lan-ip.sh` did.

[macos]
ios-device: _ensure-docker doctor dev-postgres-up dev-migrate dev-seed-if-empty
    #!/usr/bin/env bash
    set -euo pipefail
    host=$(./scripts/lan-ip.sh)
    echo "→ Using LAN host: $host"
    export TAURI_DEV_HOST="$host"
    export WEB_URL="http://$host:5173"
    export BACKEND_URL="http://$host:3000"
    export EURORA_HEALTH_URL="http://$host:3000/health"
    pnpm exec concurrently --kill-others --handle-input --default-input-target mobile \
        --names backend,web,mobile --prefix-colors cyan,green,magenta \
        "just _dev-backend-watch" \
        "just _dev-web-after-backend" \
        "just _dev-ios-device-after-backend"

[private]
[macos]
_dev-ios-device-after-backend: _wait-for-backend
    pnpm tauri ios dev --host="$TAURI_DEV_HOST" \
        --config crates/app/euro-mobile/tauri.conf.json \
        --features devtools

# Invoked by xcodebuild's preBuildScript in
# `crates/app/euro-mobile/gen/apple/project.yml`. Self-contained: re-
# derives the LAN host instead of relying on env vars surviving from
# the launching shell through tauri-cli, xcodebuild, and the script
# phase. Falls back to `localhost` when no LAN IP is available so
# offline simulator builds still succeed.

[private]
[macos]
[positional-arguments]
_ios-xcode-script *args:
    #!/usr/bin/env bash
    set -euo pipefail
    host=$("{{justfile_directory()}}/scripts/lan-ip.sh" 2>/dev/null || echo localhost)
    export BACKEND_URL="http://$host:3000"
    export WEB_URL="http://$host:5173"
    pnpm tauri ios xcode-script "$@"

# ─── First-run setup ───────────────────────────────────────────────────────
# Copy .env.example to .env (if missing) and install JS deps. Idempotent.

[unix]
init:
    @./scripts/init.sh
    pnpm install

[windows]
init:
    @powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/init.ps1
    pnpm install

# ─── Pre-flight ────────────────────────────────────────────────────────────
# Verifies tools are present, ports are free, and `.env` has a real
# OPENAI_API_KEY before the stack tries to come up. Implementation lives in
# scripts/doctor.{sh,ps1} to keep this recipe small and the checks
# individually testable.

[unix]
doctor:
    @./scripts/doctor.sh

[windows]
doctor:
    @powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/doctor.ps1

# ─── Backend ───────────────────────────────────────────────────────────────

dev-backend: _ensure-docker doctor dev-postgres-up dev-migrate dev-seed-if-empty && _dev-backend-watch

# Backend only, single run, no auto-restart (debugger / profiling).
dev-backend-once: _ensure-docker doctor dev-postgres-up dev-migrate dev-seed-if-empty
    cargo run -p be-monolith

# Apply schema migrations against the running Postgres (idempotent).
dev-migrate:
    cargo run -p be-monolith -- --migrate-only

# ─── Web / desktop ─────────────────────────────────────────────────────────

dev-web:
    pnpm dev:web

dev-desktop:
    pnpm dev:desktop

# ─── Postgres ──────────────────────────────────────────────────────────────

dev-postgres: dev-postgres-up

dev-postgres-up:
    docker compose up -d --wait postgres
    @echo "Postgres is ready."

# Run the seed only if the users table is empty. Idempotent first-boot path.

[unix]
dev-seed-if-empty:
    @./scripts/seed-if-empty.sh

[windows]
dev-seed-if-empty:
    @powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/seed-if-empty.ps1

# Force a re-seed: nuke the volume and start fresh.
dev-reset:
    docker compose --profile seed down -v
    just dev-postgres-up
    just dev-migrate
    docker compose --profile seed up --no-deps --abort-on-container-exit seed

# ─── Tests ─────────────────────────────────────────────────────────────────
#
# `set dotenv-load` exports `.env` into these recipes, so workspace tests
# that read env vars (e.g. `OPENAI_API_KEY`, `BACKEND_URL`) pick them up
# without any per-test loader. CI provides the same variables via
# `env:`/`secrets.*` blocks, so the recipes are CI-portable too.
#
# Live integration tests (those that hit OpenAI / Ollama / live HTTP
# endpoints) live behind the `integration-tests` cargo feature; running
# the default `just test` does not exercise them.

# Workspace tests minus live-API integration tests.
test:
    cargo test --workspace

# Live-API integration tests (OpenAI, Ollama). Requires the relevant
# provider keys in `.env`; the `integration-tests` feature is what
# compiles the live test modules in.
test-integration:
    cargo test -p agent-chain --features integration-tests

# Variadic passthrough so scripts and contributors can run any cargo
# subcommand with `.env` already exported. Used by `scripts/clippy.sh`
# (via `CARGO="just cargo"`) and handy as `just cargo check -p foo`,
# `just cargo run -p be-monolith -- --migrate-only`, etc.
[positional-arguments]
cargo *args:
    cargo "$@"

# ─── Misc ──────────────────────────────────────────────────────────────────

logs:
    docker compose logs -f postgres

stop:
    docker compose down

# ─── Internal helpers ──────────────────────────────────────────────────────

# Start Docker Desktop if the daemon isn't already up. macOS-only side
# effect; on Linux the script no-ops and lets `doctor` surface the
# failure with a remediation hint (starting dockerd needs sudo). This
# lives outside `doctor` so the doctor itself stays side-effect-free.

[private]
[unix]
_ensure-docker:
    @./scripts/ensure-docker.sh

[private]
[windows]
_ensure-docker:
    @powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/ensure-docker.ps1

# Block until the backend's /health endpoint responds, with a 120s ceiling
# to cover a slow first-time debug compile. Without this, the Vite dev
# server tries to call /llm/info before the backend exists and the desktop
# app surfaces a connection error on boot.

[private]
[unix]
_wait-for-backend:
    @./scripts/wait-for-backend.sh

[private]
[windows]
_wait-for-backend:
    @powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/wait-for-backend.ps1
