# Block until the backend's /health endpoint responds, with a 120s ceiling
# to cover a slow first-time debug compile.
#
# Used by `just dev` to delay web / desktop startup until the backend has
# bound its port.
#
# Compatible with Windows PowerShell 5.1 and PowerShell 7+.

$ErrorActionPreference = "Stop"

$Url = if ($env:EURORA_HEALTH_URL) { $env:EURORA_HEALTH_URL } else { "http://localhost:3000/health" }
$TimeoutSecs = if ($env:EURORA_HEALTH_TIMEOUT_SECS) { [int]$env:EURORA_HEALTH_TIMEOUT_SECS } else { 120 }

# Suppress Invoke-WebRequest's progress bar — it floods CI logs and slows
# down 5.1 noticeably.
$ProgressPreference = "SilentlyContinue"

$deadline = (Get-Date).AddSeconds($TimeoutSecs)

while ($true) {
    try {
        $null = Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 2
        Write-Host "Backend is ready."
        exit 0
    } catch {
        if ((Get-Date) -gt $deadline) {
            [Console]::Error.WriteLine("Backend did not become ready within ${TimeoutSecs}s.")
            exit 1
        }
        Start-Sleep -Milliseconds 500
    }
}
