# Ensure the Docker daemon is reachable before `just dev` runs the
# doctor. Doctor itself is side-effect-free by contract — this script
# is the place where we're allowed to *act*.
#
# Companion to scripts/ensure-docker.sh.

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) { exit 0 }
& docker info *> $null
if ($LASTEXITCODE -eq 0) { exit 0 }

# Docker Desktop installs to a known path; fall back to PATH lookup.
$candidates = @(
    "$Env:ProgramFiles\Docker\Docker\Docker Desktop.exe",
    "$Env:LOCALAPPDATA\Programs\Docker\Docker\Docker Desktop.exe"
)
$exe = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $exe) {
    $cmd = Get-Command "Docker Desktop.exe" -ErrorAction SilentlyContinue
    if ($cmd) { $exe = $cmd.Source }
}
if (-not $exe) { exit 0 }

Write-Host "Docker daemon not running — starting Docker Desktop…"
Start-Process -FilePath $exe | Out-Null

if ($Env:EURORA_DOCKER_TIMEOUT_SECS) {
    $deadline = [int]$Env:EURORA_DOCKER_TIMEOUT_SECS
} else {
    $deadline = 90
}
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($true) {
    & docker info *> $null
    if ($LASTEXITCODE -eq 0) { break }
    if ($sw.Elapsed.TotalSeconds -ge $deadline) {
        Write-Error "Docker did not become ready within ${deadline}s."
        exit 1
    }
    Start-Sleep -Seconds 1
}
Write-Host "Docker is ready."
exit 0
