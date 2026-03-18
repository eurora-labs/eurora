#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$SCRIPT_DIR/data"

DB_URL="${REMOTE_DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/eurora}"

if [[ "$DB_URL" =~ ^postgresql://([^:]+):([^@]+)@([^:]+):([0-9]+)/(.+)$ ]]; then
    PGUSER="${BASH_REMATCH[1]}"
    PGPASSWORD="${BASH_REMATCH[2]}"
    PGHOST="${BASH_REMATCH[3]}"
    PGPORT="${BASH_REMATCH[4]}"
    PGDATABASE="${BASH_REMATCH[5]}"
else
    echo "Error: cannot parse REMOTE_DATABASE_URL" >&2
    exit 1
fi

export PGPASSWORD

if [[ "$PGHOST" != "localhost" && "$PGHOST" != "127.0.0.1" ]]; then
    echo "Error: seed script refuses to run against non-localhost database ($PGHOST)" >&2
    exit 1
fi

PSQL="psql -h $PGHOST -p $PGPORT -U $PGUSER -d $PGDATABASE -v ON_ERROR_STOP=1"

echo "Seeding database at $PGHOST:$PGPORT/$PGDATABASE ..."

$PSQL <<'SQL'
BEGIN;

-- Clear existing data in dependency order
TRUNCATE
    activity_assets,
    activity_threads,
    message_assets,
    token_usage,
    monthly_token_totals,
    messages,
    threads,
    activities,
    assets,
    login_tokens,
    refresh_tokens,
    oauth_credentials,
    oauth_state,
    password_credentials,
    plan_prices,
    users,
    plans
CASCADE;

COMMIT;
SQL

echo "  Loading plans..."
$PSQL -c "\COPY plans (id, name, description, created_at, updated_at, monthly_token_limit) FROM '$DATA_DIR/plans.csv' WITH (FORMAT csv, HEADER true);"

echo "  Loading users..."
$PSQL -c "\COPY users (id, username, email, display_name, email_verified, created_at, updated_at, stripe_customer_id, plan_id) FROM '$DATA_DIR/users.csv' WITH (FORMAT csv, HEADER true);"

echo "  Loading assets..."
$PSQL -c "\COPY assets (id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, created_at, updated_at, metadata) FROM '$DATA_DIR/assets.csv' WITH (FORMAT csv, HEADER true);"

echo "  Loading threads (without active_leaf_id)..."
$PSQL -c "\COPY threads (id, user_id, title, created_at, updated_at) FROM '$DATA_DIR/threads.csv' WITH (FORMAT csv, HEADER true);"

echo "  Loading messages ($(( $(wc -l < "$DATA_DIR/messages.csv") - 1 )) rows)..."
$PSQL -c "\COPY messages (id, thread_id, user_id, parent_message_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, hidden_from_ui, reasoning_blocks, created_at, updated_at) FROM '$DATA_DIR/messages.csv' WITH (FORMAT csv, HEADER true);"

echo "  Setting threads.active_leaf_id..."
$PSQL -f "$DATA_DIR/threads_active_leaf.sql"

echo "  Loading activities..."
$PSQL -c "\COPY activities (id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at) FROM '$DATA_DIR/activities.csv' WITH (FORMAT csv, HEADER true);"

echo "  Loading token_usage..."
$PSQL -c "\COPY token_usage (id, user_id, thread_id, message_id, input_tokens, output_tokens, reasoning_tokens, cache_creation_tokens, cache_read_tokens, created_at) FROM '$DATA_DIR/token_usage.csv' WITH (FORMAT csv, HEADER true);"

echo "  Loading monthly_token_totals..."
$PSQL -c "DELETE FROM monthly_token_totals;"
$PSQL -c "\COPY monthly_token_totals (user_id, year_month, total_tokens) FROM '$DATA_DIR/monthly_token_totals.csv' WITH (FORMAT csv, HEADER true);"

echo "Done. Verifying row counts..."
$PSQL <<'SQL'
SELECT 'plans' AS tbl, count(*) FROM plans
UNION ALL SELECT 'users', count(*) FROM users
UNION ALL SELECT 'threads', count(*) FROM threads
UNION ALL SELECT 'messages', count(*) FROM messages
UNION ALL SELECT 'assets', count(*) FROM assets
UNION ALL SELECT 'activities', count(*) FROM activities
UNION ALL SELECT 'token_usage', count(*) FROM token_usage
UNION ALL SELECT 'monthly_token_totals', count(*) FROM monthly_token_totals
ORDER BY 1;
SQL
