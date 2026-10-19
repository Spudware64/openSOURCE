# Run from PowerShell; no Python, guessed binary name, or renderer-only benchmark.
param(
    [string]$Repo = 'C:\.dev\portal-rust',
    [string]$Game = 'C:\.programs\Steam\steamapps\common\Portal\portal'
)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $Repo
if (!(Test-Path -LiteralPath (Join-Path $Game 'gameinfo.txt'))) { throw "Missing gameinfo.txt in $Game" }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$out = Join-Path $Repo "artifacts\performance-$stamp"
New-Item -ItemType Directory -Path $out -Force | Out-Null
$oldProfile = $env:SOURCE_FRAME_PROFILE
$oldDetail = $env:SOURCE_FRAME_DETAIL
try {
    $head = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'git rev-parse failed' }
    $status = & git status --short
    @("checkout=$head", "worktree=$status", "game=$Game", "WGPU_BACKEND=$env:WGPU_BACKEND") | Set-Content (Join-Path $out 'provenance.txt')
    try { Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion, AdapterRAM | Format-List | Out-File (Join-Path $out 'provenance.txt') -Append } catch { $_ | Out-File (Join-Path $out 'provenance.txt') -Append }
    $messages = @(& cargo build --release --message-format=json)
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed; not launching a stale executable' }
    $messages | Set-Content (Join-Path $out 'build.jsonl')
    $bins = @($messages | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.kind -contains 'bin' -and $_.executable } | ForEach-Object { $_.executable } | Select-Object -Unique)
    if ($bins.Count -ne 1) { throw "Expected one Cargo-reported application executable, found $($bins.Count)" }
    $exe = $bins[0]
    @("exe=$exe", "sha256=$((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash)") | Add-Content (Join-Path $out 'provenance.txt')
    $env:SOURCE_FRAME_DETAIL = '1'
    foreach ($map in @('escape_02', 'testchmb_a_06')) {
        $env:SOURCE_FRAME_PROFILE = Join-Path $out "$map.csv"
        Write-Host "`n${map}: stand still briefly without a linked pair, then repeat the reported slow interaction."
        Write-Host 'Look toward/away from the linked pair; approach, cross, stop, and cross back several times.'
        Write-Host 'If safe, repeat the same slow view after shrinking the window. Close the game normally to start the next map.'
        & $exe --game $Game --map $map --play 2>&1 | Tee-Object -FilePath (Join-Path $out "$map.log")
        if ($LASTEXITCODE -ne 0) { throw "Game exited with $LASTEXITCODE; partial capture retained in $out" }
        $detail = "$env:SOURCE_FRAME_PROFILE.frames.csv"
        if (!(Test-Path -LiteralPath $detail)) { throw "No detailed capture: $detail" }
        Import-Csv -LiteralPath $detail | Sort-Object { [double]$_.frame_work_ms } -Descending | Select-Object -First 12 frame,begin_gap_ms,frame_work_ms,simulation_ms,controller_nested_ms,physics_nested_ms,query_build_nested_ms,acquire_ms,submit_ms,present_ms,linked,crossings,width,height | Format-Table -AutoSize | Out-String | Set-Content (Join-Path $out "$map-worst.txt")
    }
} finally {
    $env:SOURCE_FRAME_PROFILE = $oldProfile
    $env:SOURCE_FRAME_DETAIL = $oldDetail
}
$zip = "$out.zip"
Compress-Archive -Path (Join-Path $out '*') -DestinationPath $zip
Write-Host "Send this capture: $zip"
