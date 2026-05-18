# First-run setup: copy .env.example to .env if .env doesn't exist yet.
# Idempotent — safe to re-run.
#
# Compatible with Windows PowerShell 5.1 and PowerShell 7+.

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$envFile = Join-Path $repoRoot ".env"
$envExample = Join-Path $repoRoot ".env.example"

if (-not (Test-Path -LiteralPath $envFile)) {
    Copy-Item -LiteralPath $envExample -Destination $envFile
    Write-Host ".env created from .env.example — open it and set OPENAI_API_KEY."
} else {
    Write-Host ".env already exists — leaving it alone."
}
