# Local development orchestration for Eurora.
#
# `just dev` is the one-command path: brings up Postgres, seeds the dev
# user/threads on first run, runs the backend, the web auth UI, and the
# desktop app — all wired to talk to each other on localhost.
#
# Individual recipes are exposed for when you need to iterate on one piece
# without restarting the rest.
#
#   just bootstrap         first-run setup (.env, pnpm install)
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
# Env handling: `set dotenv-load := true` reads `.env` at the workspace
# root and exports every variable to the child processes spawned below.
# That single file is the contract — Vite reads it via `envDir`, the
# Rust binaries inherit it from the shell, and the mobile build bakes
# the relevant keys at compile time.

set dotenv-load := true
set shell := ["bash", "-cu"]

default: dev

# Full stack: Postgres + backend + web + desktop, all in one terminal.
# Pieces share `just` itself as the supervisor; Ctrl-C stops them all.
#
# `cargo watch -x 'run -p be-monolith'` recompiles + restarts the backend
# on every save in any workspace crate. That covers cross-cutting changes
# (e.g. tweaking llm-core or be-thread-service) without you having to
# remember which crate triggers what. Cost: a save in an unrelated crate
# also restarts the backend; benefit: you never have to think about it.
# Use `dev-backend-once` for cases where you want a stable run (debugger,
# profiling, watching a steady tracing tail).
dev: doctor
    just dev-postgres-up
    just dev-migrate
    just dev-seed-if-empty
    pnpm exec concurrently --kill-others --names backend,web,desktop --prefix-colors cyan,green,yellow \
        "cargo watch -x 'run -p be-monolith'" \
        "just _wait-for-backend && pnpm dev:web" \
        "just _wait-for-backend && pnpm dev:desktop"

# Block until the backend's /health endpoint responds, with a 120s ceiling
# to cover a slow first-time debug compile. Used by `dev` to delay web /
# desktop startup until the backend has bound its port — without this, the
# Vite dev server tries to call /llm/info before the backend exists and
# the desktop app surfaces a connection error on boot.
_wait-for-backend:
    @timeout 120 bash -c 'until curl -fsS http://localhost:3000/health >/dev/null 2>&1; do sleep 0.5; done' \
        && echo "Backend is ready." \
        || (echo "Backend did not become ready within 120s." >&2; exit 1)

# First-run setup: copy .env.example to .env and install JS deps.
# Idempotent — safe to run again any time.
bootstrap:
    @if [ ! -f .env ]; then \
        cp .env.example .env; \
        echo ".env created from .env.example — open it and set OPENAI_API_KEY."; \
    else \
        echo ".env already exists — leaving it alone."; \
    fi
    pnpm install

# Pre-flight checks for `just dev`. Verifies tools are present, ports are
# free, and `.env` has a real OPENAI_API_KEY before the stack tries to come
# up. Implementation lives in scripts/doctor.sh to keep this recipe small
# and the checks individually testable.
doctor:
    @./scripts/doctor.sh

dev-backend: doctor
    just dev-postgres-up
    just dev-migrate
    just dev-seed-if-empty
    cargo watch -x 'run -p be-monolith'

# Single stable backend run, no auto-restart. Use this when attaching a
# debugger or watching a steady log tail; otherwise prefer `dev-backend`.
dev-backend-once: doctor
    just dev-postgres-up
    just dev-migrate
    just dev-seed-if-empty
    cargo run -p be-monolith

# Apply schema migrations against the running Postgres. Reuses the same
# `sqlx::migrate!` pass the backend runs on every startup, so a fresh
# `just dev` can ensure the schema exists before seed runs.
dev-migrate:
    cargo run -p be-monolith -- --migrate-only

dev-web:
    pnpm dev:web

dev-desktop:
    pnpm dev:desktop

dev-postgres:
    just dev-postgres-up

dev-postgres-up:
    docker compose up -d --wait postgres
    @echo "Postgres is ready."

# Run the seed only if the users table is empty. Idempotent first-boot path.
#
# Distinguishes three cases:
#   - schema absent  → bail with an actionable message (run `just dev-migrate`)
#   - schema present, users empty → run seed
#   - schema present, users present → skip
#
# `to_regclass('public.users')` is the schema-presence probe; it returns
# NULL without erroring if the table doesn't exist, which lets us tell
# "missing table" apart from a real psql failure.
dev-seed-if-empty:
    @schema=$(docker compose exec -T postgres psql -U postgres -d eurora -tAc "SELECT to_regclass('public.users')"); \
    schema=$(echo "$schema" | tr -d '[:space:]'); \
    if [ -z "$schema" ]; then \
        echo "Schema not migrated yet. Run 'just dev-migrate' (or 'just dev', which does it automatically)." >&2; \
        exit 1; \
    fi; \
    count=$(docker compose exec -T postgres psql -U postgres -d eurora -tAc "SELECT count(*) FROM users"); \
    count=$(echo "$count" | tr -d '[:space:]'); \
    if [ "$count" = "0" ]; then \
        echo "Database is empty — running seed (creates dev@dev.com / password 'dev')"; \
        docker compose --profile seed up --no-deps --abort-on-container-exit seed; \
    else \
        echo "Database already populated ($count user(s)) — skipping seed."; \
    fi

# Force a re-seed: nuke the volume and start fresh.
dev-reset:
    docker compose down -v
    just dev-postgres-up
    just dev-migrate
    docker compose --profile seed up --no-deps --abort-on-container-exit seed

logs:
    docker compose logs -f postgres

stop:
    docker compose down
