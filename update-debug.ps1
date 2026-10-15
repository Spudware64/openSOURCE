#Requires -Version 5.1
[CmdletBinding()]
param(
    [switch]$Run,
    [switch]$Trace,
    [string]$GamePath = 'C:\.programs\Steam\steamapps\common\Portal\portal',
    [string]$Map = 'maps/testchmb_a_00.bsp',
    [ValidateRange(1, 3600)][int]$TraceSeconds = 120
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($Trace -and -not $Run) { throw '-Trace requires -Run.' }
Push-Location -LiteralPath $PSScriptRoot
$oldTrace = $env:SOURCE_CONTROLLER_TRACE
$oldSeconds = $env:SOURCE_CONTROLLER_TRACE_SECONDS
try {
    foreach ($tool in @('git', 'cargo')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "Missing $tool. Install Git for Windows and Rust (rustup), then reopen PowerShell."
        }
    }
    & git pull --ff-only
    if ($LASTEXITCODE -ne 0) {
        throw 'git pull --ff-only failed. Check network/login with your Git credential manager, local edits, and branch divergence; no reset or credential changes were attempted.'
    }
    & cargo build
    if ($LASTEXITCODE -ne 0) { throw 'Debug build failed. Review Cargo errors above.' }
    if ($Run) {
        if (-not (Test-Path -LiteralPath (Join-Path $GamePath 'gameinfo.txt'))) {
            throw "No gameinfo.txt in '$GamePath'. Supply -GamePath pointing to the Portal portal directory."
        }
        # cargo metadata respects CARGO_TARGET_DIR and repository Cargo configuration.
        $metadataText = & cargo metadata --format-version 1 --no-deps
        if ($LASTEXITCODE -ne 0) { throw 'Could not locate Cargo target directory.' }
        $metadata = ($metadataText -join "`n") | ConvertFrom-Json
        $exe = Join-Path $metadata.target_directory 'debug\portal-bsp-viewer.exe'
        if (-not (Test-Path -LiteralPath $exe)) { throw "Debug executable not found: $exe" }
        $arguments = @('--game', $GamePath, '--map', $Map, '--play')
        if ($Trace) {
            $id = (Get-Date -Format 'yyyyMMdd-HHmmss-fff') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
            $directory = Join-Path $PSScriptRoot "logs\$id"
            New-Item -ItemType Directory -Path $directory -ErrorAction Stop | Out-Null
            $env:SOURCE_CONTROLLER_TRACE = Join-Path $directory 'controller.csv'
            $env:SOURCE_CONTROLLER_TRACE_SECONDS = [string]$TraceSeconds
            Write-Host "Diagnostics: $directory"
            # Native stderr is diagnostic output, not a PowerShell terminating error (PS 5.1).
            $ErrorActionPreference = 'Continue'
            & $exe @arguments 2>&1 | Tee-Object -FilePath (Join-Path $directory 'console.log')
            $code = $LASTEXITCODE
            $ErrorActionPreference = 'Stop'
        } else {
            & $exe @arguments
            $code = $LASTEXITCODE
        }
        if ($code -ne 0) { throw "Game exited with code $code. Review diagnostics above." }
    }
} finally {
    $env:SOURCE_CONTROLLER_TRACE = $oldTrace
    $env:SOURCE_CONTROLLER_TRACE_SECONDS = $oldSeconds
    Pop-Location
}
