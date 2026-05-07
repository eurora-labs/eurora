# Pre-flight check for `just dev`. Validates that the developer's machine
# has the tools and configuration we need before we try to bring the stack
# up. Exit code is the number of failed checks (capped at 1) so the script
# fits cleanly into CI gates and `just dev: doctor` dependencies.
#
# Side-effect-free by design: nothing is installed or written. Failures
# carry a one-line remediation hint pointing at the exact command to run.
#
# Companion to scripts/doctor.sh — keeps the same checks, output shape,
# and exit-code contract. Compatible with Windows PowerShell 5.1 and
# PowerShell 7+.
#
# Usage:
#   just doctor
#   pwsh ./scripts/doctor.ps1     # or: powershell -File ./scripts/doctor.ps1

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# The check glyphs (✓ ✗ ↳) and em-dashes elsewhere are Unicode; the legacy
# Windows console host defaults to a code page that mangles them. Force
# UTF-8 output so the script looks the same on every host.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$RepoRoot = Split-Path -Parent $PSScriptRoot

# ─── Color handling ────────────────────────────────────────────────────────
# ANSI escapes render in modern Windows Terminal, VS Code's terminal, and
# any pwsh 7+ host. Skip them on the legacy console host or when stdout is
# redirected so CI logs stay clean.
$useColor = $Host.UI.SupportsVirtualTerminal -and -not [Console]::IsOutputRedirected
if ($useColor) {
    $ESC    = [char]27
    $GREEN  = "$ESC[0;32m"
    $RED    = "$ESC[0;31m"
    $YELLOW = "$ESC[0;33m"
    $BOLD   = "$ESC[1m"
    $DIM    = "$ESC[2m"
    $RESET  = "$ESC[0m"
} else {
    $GREEN = $RED = $YELLOW = $BOLD = $DIM = $RESET = ""
}

# Width of the check name column. Tuned to fit the longest label without
# wrapping at 80 cols. Mirrors doctor.sh.
$Col = 18

$script:Failed = 0

# ─── Output helpers ────────────────────────────────────────────────────────

function Write-Pass {
    param([string]$Label, [string]$Detail = "")
    $padded = $Label.PadRight($Col)
    Write-Host ("  {0}{1}{2} {3} {4}{5}{6}" -f $GREEN, [char]0x2713, $RESET, $padded, $DIM, $Detail, $RESET)
}

function Write-Fail {
    param([string]$Label, [string]$Detail = "")
    $padded = $Label.PadRight($Col)
    Write-Host ("  {0}{1}{2} {3} {4}{5}{6}" -f $RED, [char]0x2717, $RESET, $padded, $RED, $Detail, $RESET)
    $script:Failed++
}

function Write-Hint {
    param([string]$Message)
    Write-Host ("    {0}{1} {2}{3}" -f $DIM, [char]0x21B3, $Message, $RESET)
}

# ─── Individual checks ─────────────────────────────────────────────────────

function Test-Command {
    param([string]$Label, [string]$Cmd, [string]$InstallHint)
    if (-not (Get-Command -Name $Cmd -ErrorAction SilentlyContinue)) {
        Write-Fail $Label "not installed"
        Write-Hint $InstallHint
        return $false
    }
    try {
        $version = (& $Cmd --version 2>&1 | Select-Object -First 1)
        # Native command exits don't auto-throw under $ErrorActionPreference
        # in PS 5.1 — turn a non-zero exit into a caught failure ourselves.
        if ($LASTEXITCODE -ne 0) {
            throw "exit $LASTEXITCODE"
        }
    } catch {
        Write-Fail $Label "not runnable"
        Write-Hint $InstallHint
        return $false
    }
    Write-Pass $Label $version
    return $true
}

function Test-DockerDaemon {
    & docker info *> $null
    if ($LASTEXITCODE -eq 0) {
        Write-Pass "docker daemon" "running"
        return $true
    }
    Write-Fail "docker daemon" "not reachable"
    Write-Hint "Start Docker Desktop."
    return $false
}

# Returns $true if `port` is in use, $false if it's free. Uses a short
# TcpClient probe so we don't depend on Test-NetConnection (slow, prints
# warnings on closed ports).
function Test-PortInUse {
    param([int]$Port)
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $iar = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
        $completed = $iar.AsyncWaitHandle.WaitOne(500, $false)
        if ($completed -and $client.Connected) {
            $client.EndConnect($iar)
            return $true
        }
        return $false
    } catch {
        return $false
    } finally {
        $client.Close()
    }
}

# True iff the host port is bound by our docker-compose Postgres container.
# `docker compose port` resolves the publish mapping directly (e.g.
# "0.0.0.0:5434"), which is more robust than scraping `docker ps`.
function Test-PortOwnedByEuroraPostgres {
    param([int]$Port)
    $binding = (& docker compose port postgres 5432 2>$null)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($binding)) {
        return $false
    }
    $boundPort = ($binding -split ":")[-1].Trim()
    return ($boundPort -eq "$Port")
}

function Test-PortFree {
    param([string]$Label, [int]$Port, [string]$HintMsg)
    if (Test-PortInUse $Port) {
        Write-Fail $Label "in use (port $Port)"
        Write-Hint $HintMsg
        return $false
    }
    Write-Pass $Label "free (port $Port)"
    return $true
}

function Test-PostgresPort {
    param([int]$Port)
    if (-not (Test-PortInUse $Port)) {
        Write-Pass "port $Port" "free"
        return $true
    }
    if (Test-PortOwnedByEuroraPostgres $Port) {
        Write-Pass "port $Port" "in use by Eurora postgres container"
        return $true
    }
    Write-Fail "port $Port" "in use by something else"
    Write-Hint "Stop the conflicting process or set EURORA_POSTGRES_PORT in .env."
    return $false
}

function Test-EnvFile {
    if (Test-Path -LiteralPath (Join-Path $RepoRoot ".env")) {
        Write-Pass ".env" "exists"
        return $true
    }
    Write-Fail ".env" "not found"
    Write-Hint "Run: just init"
    return $false
}

# Read `Key`'s value out of the local `.env` file. Returns $null if
# the file is absent or the key isn't present.
function Get-EnvValue {
    param([string]$Key)
    $envFile = Join-Path $RepoRoot ".env"
    if (-not (Test-Path -LiteralPath $envFile)) { return $null }
    foreach ($line in Get-Content -LiteralPath $envFile) {
        if ($line -match "^\s*$Key=(.*)$") {
            return $matches[1]
        }
    }
    return $null
}

# Resolve `Key`'s effective value. Process env wins (so values
# exported by `set dotenv-load` in the justfile take precedence); we
# fall back to grepping `.env` for the standalone invocation path.
# Empty string if neither defines it.
function Resolve-EnvValue {
    param([string]$Key)
    $value = [Environment]::GetEnvironmentVariable($Key)
    if (-not [string]::IsNullOrEmpty($value)) { return $value }
    $fileValue = Get-EnvValue $Key
    if ($null -eq $fileValue) { return "" }
    return $fileValue
}

# Names of every required env var, sourced from `.env.example`
# (every uncommented `KEY=VALUE` line). `.env.example` is the single
# source of truth — adding a required key means uncommenting it
# there, and doctor picks it up automatically.
#
# OPENAI_API_KEY is excluded because Test-OpenAiKey runs a more
# detailed check (placeholder detection) for it specifically.
function Get-RequiredEnvKeys {
    $envExample = Join-Path $RepoRoot ".env.example"
    if (-not (Test-Path -LiteralPath $envExample)) {
        return @()
    }
    $keys = New-Object System.Collections.Generic.List[string]
    foreach ($line in Get-Content -LiteralPath $envExample) {
        if ($line -match '^[A-Z_][A-Z0-9_]*=') {
            $key = ($line -split '=', 2)[0].Trim()
            if ($key -ne 'OPENAI_API_KEY') {
                [void]$keys.Add($key)
            }
        }
    }
    return $keys
}

function Test-EnvComplete {
    if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot ".env.example"))) {
        Write-Fail "env vars" ".env.example not found at repo root"
        return $false
    }
    $required = @(Get-RequiredEnvKeys)
    $missing = New-Object System.Collections.Generic.List[string]
    foreach ($key in $required) {
        $value = Resolve-EnvValue $key
        if ([string]::IsNullOrEmpty($value)) {
            [void]$missing.Add($key)
        }
    }
    if ($missing.Count -eq 0) {
        Write-Pass "env vars" ("{0}/{0} required keys set" -f $required.Count)
        return $true
    }
    Write-Fail "env vars" ("{0} of {1} required key(s) missing" -f $missing.Count, $required.Count)
    if ($missing.Count -le 5) {
        Write-Hint ("Add to .env: " + ($missing -join ' '))
    } else {
        Write-Hint "Run ``just init`` to create .env from .env.example, then re-run doctor."
        $shown = ($missing | Select-Object -First 5) -join ' '
        $rest = $missing.Count - 5
        Write-Hint ("Missing: $shown … (+$rest more)")
    }
    return $false
}

function Test-OpenAiKey {
    $value = Resolve-EnvValue "OPENAI_API_KEY"
    if ([string]::IsNullOrEmpty($value)) {
        Write-Fail "OPENAI_API_KEY" "unset"
        Write-Hint "Get a key from https://platform.openai.com/api-keys and add it to .env."
        return $false
    }
    if ($value -eq "sk-..." -or $value -eq "sk_test") {
        Write-Fail "OPENAI_API_KEY" "still set to a placeholder"
        Write-Hint "Replace the placeholder in .env with a real key from https://platform.openai.com/api-keys."
        return $false
    }
    Write-Pass "OPENAI_API_KEY" "set"
    return $true
}

# ─── Main ──────────────────────────────────────────────────────────────────

Write-Host ("{0}Eurora dev environment doctor{1}" -f $BOLD, $RESET)
Write-Host ("{0}─────────────────────────────{1}" -f $DIM, $RESET)

$dockerOk = Test-Command "docker" "docker" "Install Docker Desktop: https://docs.docker.com/get-docker/"
if ($dockerOk) {
    Test-DockerDaemon | Out-Null
    & docker compose version *> $null
    if ($LASTEXITCODE -eq 0) {
        $version = (& docker compose version --short 2>$null)
        if ([string]::IsNullOrWhiteSpace($version)) { $version = "v2" }
        Write-Pass "docker compose" $version
    } else {
        Write-Fail "docker compose" "v2 not found"
        Write-Hint "Update Docker; v1 'docker-compose' is unsupported."
    }
}

Test-Command "cargo"       "cargo"       "Install Rust via https://rustup.rs"      | Out-Null
Test-Command "cargo-watch" "cargo-watch" "Install with: cargo install cargo-watch" | Out-Null
Test-Command "pnpm"        "pnpm"        "Install with: corepack enable"           | Out-Null
Test-Command "just"        "just"        "Install with: cargo install just"        | Out-Null

# Port checks. We resolve the port from the user's env so the doctor
# follows whatever HTTP_ADDR / EURORA_POSTGRES_PORT they've configured;
# the literal fallbacks (3000 / 5433) only fire when the variables are
# unset (e.g., a fresh checkout where doctor runs before `just init`)
# so the doctor itself stays usable in that broken state.
$httpAddr = Resolve-EnvValue "HTTP_ADDR"
if ([string]::IsNullOrEmpty($httpAddr)) {
    $httpPort = "3000"
} else {
    $httpPort = ($httpAddr -split ':')[-1]
    if ([string]::IsNullOrEmpty($httpPort)) { $httpPort = "3000" }
}

$postgresPort = Resolve-EnvValue "EURORA_POSTGRES_PORT"
if ([string]::IsNullOrEmpty($postgresPort)) {
    $postgresPort = "5433"
}

Test-PortFree     "port $httpPort" ([int]$httpPort)  "Stop the conflicting process or set HTTP_ADDR."           | Out-Null
Test-PostgresPort ([int]$postgresPort)                                                                          | Out-Null
Test-PortFree     "port 5173"     5173               "Stop the conflicting process or move the web dev server." | Out-Null

Test-EnvFile     | Out-Null
Test-EnvComplete | Out-Null
Test-OpenAiKey   | Out-Null

Write-Host ""
if ($script:Failed -gt 0) {
    Write-Host ("{0}{1}{2} check(s) failed.{3}" -f $RED, $BOLD, $script:Failed, $RESET)
    exit 1
}
Write-Host ("{0}{1}All checks passed.{2}" -f $GREEN, $BOLD, $RESET)
exit 0
