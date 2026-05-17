# Run the database seed only if the users table is empty. Idempotent
# first-boot path for `just dev`.
#
# Distinguishes three cases:
#   - schema absent  → bail with an actionable message
#   - schema present, users empty → run seed
#   - schema present, users present → skip
#
# Compatible with Windows PowerShell 5.1 and PowerShell 7+.

$ErrorActionPreference = "Stop"

# Postgres user/db are hardcoded in docker-compose.yml for the dev
# stack — they're conventions, not user config. We use the same
# values here so the host-side psql probe lines up with what the
# container was provisioned with. If you change them in compose,
# change them here too.
$PG_USER = 'postgres'
$PG_DB = 'eurora'

function Invoke-Psql {
    param([Parameter(Mandatory)][string]$Sql)
    $output = docker compose exec -T postgres psql -U $PG_USER -d $PG_DB -tAc $Sql
    if ($LASTEXITCODE -ne 0) {
        throw "psql failed (exit $LASTEXITCODE): $Sql"
    }
    return ($output -join "`n").Trim()
}

$schema = Invoke-Psql "SELECT to_regclass('public.users')"
if ([string]::IsNullOrWhiteSpace($schema)) {
    [Console]::Error.WriteLine("Schema not migrated yet. Run 'just dev-migrate' (or 'just dev', which does it automatically).")
    exit 1
}

$count = Invoke-Psql "SELECT count(*) FROM users"
if ($count -eq "0") {
    Write-Host "Database is empty — running seed (creates dev@dev.com / password 'dev')"
    docker compose --profile seed up --no-deps --abort-on-container-exit seed
    exit $LASTEXITCODE
}

Write-Host "Database already populated ($count user(s)) — skipping seed."
