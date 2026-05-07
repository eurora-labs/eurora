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

# Postgres user/db are hardcoded in docker-compose.yml for the dev
# stack — they're conventions, not user config. We use the same
# values here so the host-side psql probe lines up with what the
# container was provisioned with. If you change them in compose,
# change them here too.
PG_USER=postgres
PG_DB=eurora

psql_silent() {
    docker compose exec -T postgres psql -U "$PG_USER" -d "$PG_DB" -tAc "$1"
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
