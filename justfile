# Local development orchestration for Eurora.
#
# `just dev` is the one-command path: brings up Postgres, seeds the dev
# user/threads on first run, runs the backend, the web auth UI, and the
# desktop app — all wired to talk to each other on localhost.
#
# Individual recipes are exposed for when you need to iterate on one piece
# without restarting the rest.
#
#   just dev               full stack
#   just dev:backend       backend only (Postgres + cargo run)
#   just dev:web           web auth UI only
#   just dev:desktop       desktop app only
#   just dev:postgres      Postgres only (no seed, no backend)
#   just dev:reset         wipe the DB volume and re-seed
#   just logs              tail Postgres logs
#   just stop              tear down docker-compose containers (keeps volume)

set dotenv-load := true
set shell := ["bash", "-cu"]

default: dev

# Full stack: Postgres + backend + web + desktop, all in one terminal.
# Pieces share `just` itself as the supervisor; Ctrl-C stops them all.
dev:
    @command -v concurrently >/dev/null || (echo "Install pnpm deps first: pnpm install" && exit 1)
    just dev:postgres-up
    just dev:seed-if-empty
    concurrently --kill-others --names backend,web,desktop --prefix-colors cyan,green,yellow \
        "cargo run -p be-monolith" \
        "pnpm dev:web" \
        "pnpm dev:desktop"

dev\:backend:
    just dev:postgres-up
    just dev:seed-if-empty
    cargo run -p be-monolith

dev\:web:
    pnpm dev:web

dev\:desktop:
    pnpm dev:desktop

dev\:postgres:
    just dev:postgres-up

dev\:postgres-up:
    docker compose up -d postgres
    @echo "Waiting for Postgres to become healthy…"
    @until [ "$(docker compose ps -q postgres | xargs -r docker inspect -f '{{.State.Health.Status}}')" = "healthy" ]; do sleep 1; done
    @echo "Postgres is ready."

# Run the seed only if the users table is empty. Idempotent first-boot path.
dev\:seed-if-empty:
    @count=$(docker compose exec -T postgres psql -U postgres -d eurora -tAc "SELECT count(*) FROM users" 2>/dev/null || echo "missing"); \
    if [ "$count" = "missing" ] || [ "$count" = "0" ]; then \
        echo "Database is empty — running seed (creates dev@dev.com / password 'dev')"; \
        docker compose --profile seed up --no-deps --abort-on-container-exit seed; \
    else \
        echo "Database already populated ($count user(s)) — skipping seed."; \
    fi

# Force a re-seed: nuke the volume and start fresh.
dev\:reset:
    docker compose down -v
    just dev:postgres-up
    docker compose --profile seed up --no-deps --abort-on-container-exit seed

logs:
    docker compose logs -f postgres

stop:
    docker compose down
