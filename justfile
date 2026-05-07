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
#   just dev               full stack (backend hot-reloads via cargo-watch)
#   just dev-backend       backend only (Postgres + cargo-watch)
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
#   - Env handling: `set dotenv-load := true` reads `.env` at the workspace
#     root and exports every variable to the child processes spawned below.
#     That single file is the contract — Vite reads it via `envDir`, the
#     Rust binaries inherit it from the shell, and the mobile build bakes
#     the relevant keys at compile time.

set dotenv-load
set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

default: dev

# ─── Full stack ────────────────────────────────────────────────────────────
#
# Postgres + backend + web + desktop, all in one terminal. Pieces share
# `just` itself as the supervisor; Ctrl-C stops them all.
#
# `cargo watch -x 'run -p be-monolith'` recompiles + restarts the backend
# on every save in any workspace crate. That covers cross-cutting changes
# (e.g. tweaking llm-core or be-thread-service) without you having to
# remember which crate triggers what. Cost: a save in an unrelated crate
# also restarts the backend; benefit: you never have to think about it.
# Use `dev-backend-once` for cases where you want a stable run (debugger,
# profiling, watching a steady tracing tail).
#
# Each child of `concurrently` is itself a `just` recipe call. That keeps
# shell-quoting consistent across platforms (cmd.exe vs bash tokenize
# embedded quotes differently) and lets the per-platform shell handle the
# actual command.

# Postgres + backend + web + desktop, all in one terminal.
dev: doctor dev-postgres-up dev-migrate dev-seed-if-empty
    pnpm exec concurrently --kill-others \
        --names backend,web,desktop --prefix-colors cyan,green,yellow \
        "just _dev-backend-watch" \
        "just _dev-web-after-backend" \
        "just _dev-desktop-after-backend"

_dev-backend-watch:
    cargo run -p be-monolith
    # cargo watch -x 'run -p be-monolith'

_dev-web-after-backend: _wait-for-backend
    pnpm dev:web

_dev-desktop-after-backend: _wait-for-backend
    pnpm dev:desktop

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

dev-backend: doctor dev-postgres-up dev-migrate dev-seed-if-empty
    cargo run -p be-monolith
    # cargo watch -x 'run -p be-monolith'

# Backend only, single run, no auto-restart (debugger / profiling).
dev-backend-once: doctor dev-postgres-up dev-migrate dev-seed-if-empty
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
    docker compose down -v
    just dev-postgres-up
    just dev-migrate
    docker compose --profile seed up --no-deps --abort-on-container-exit seed

# ─── Misc ──────────────────────────────────────────────────────────────────

logs:
    docker compose logs -f postgres

stop:
    docker compose down

# ─── Internal helpers ──────────────────────────────────────────────────────

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
