#!/usr/bin/env bash
#
# Run the database seed only if the users table is empty. Idempotent
# first-boot path for `just dev`.
#
# Distinguishes three cases:
#   - schema absent  → bail with an actionable message (run `just dev-migrate`)
#   - schema present, users empty → run seed
#   - schema present, users present → skip
#
# `to_regclass('public.users')` is the schema-presence probe; it returns
# NULL without erroring if the table doesn't exist, which lets us tell
# "missing table" apart from a real psql failure.

set -euo pipefail

# Postgres credentials live in .env (loaded by the justfile via
# `set dotenv-load`). They're forwarded to the postgres container via
# docker-compose; we use the same values here so the host-side psql
# probe lines up with what the container was provisioned with.
: "${POSTGRES_USER:?POSTGRES_USER is required (run \`just init\` to create .env)}"
: "${POSTGRES_DB:?POSTGRES_DB is required (run \`just init\` to create .env)}"

psql_silent() {
    docker compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "$1"
}

schema=$(psql_silent "SELECT to_regclass('public.users')" | tr -d '[:space:]')
if [ -z "$schema" ]; then
    echo "Schema not migrated yet. Run 'just dev-migrate' (or 'just dev', which does it automatically)." >&2
    exit 1
fi

count=$(psql_silent "SELECT count(*) FROM users" | tr -d '[:space:]')
if [ "$count" = "0" ]; then
    echo "Database is empty — running seed (creates dev@dev.com / password 'dev')"
    docker compose --profile seed up --no-deps --abort-on-container-exit seed
else
    echo "Database already populated ($count user(s)) — skipping seed."
fi
