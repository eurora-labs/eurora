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

# Postgres credentials live in .env (loaded by the justfile via
# `set dotenv-load`). They're forwarded to the postgres container via
# docker-compose; we use the same values here so the host-side psql
# probe lines up with what the container was provisioned with.
foreach ($var in 'POSTGRES_USER', 'POSTGRES_DB') {
    $value = [System.Environment]::GetEnvironmentVariable($var)
    if ([string]::IsNullOrEmpty($value)) {
        [Console]::Error.WriteLine("$var is required (run 'just init' to create .env).")
        exit 1
    }
}

function Invoke-Psql {
    param([Parameter(Mandatory)][string]$Sql)
    $output = docker compose exec -T postgres psql -U $env:POSTGRES_USER -d $env:POSTGRES_DB -tAc $Sql
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
