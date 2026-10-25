param([string]$OutputDirectory = (Join-Path $PSScriptRoot 'releases'))
$ErrorActionPreference = 'Stop'
$taskOldParry = $env:NRHEX_PARRY_DATA
$taskOldSword = $env:NRHEX_SWORD_DATA
$taskOldEcho = $env:NRHEX_ECHO_DATA
Push-Location $PSScriptRoot
try {
    # Do not inherit MMV tables from another build.
    $env:NRHEX_PARRY_DATA = Join-Path $PSScriptRoot 'src/parry_data.rs'
    $env:NRHEX_SWORD_DATA = Join-Path $PSScriptRoot 'src/sword_map.rs'
    $env:NRHEX_ECHO_DATA = Join-Path $PSScriptRoot 'src/echo_cast_data.rs'
    & ./build-texts.ps1 -Check
    if ($LASTEXITCODE -ne 0) { throw 'Release texts are out of date.' }
    cargo build --release --lib --no-default-features --features cage-demand --target-dir "$PSScriptRoot/.work/no-hotkeys-target"
    if ($LASTEXITCODE -ne 0) { throw 'Release DLL build failed.' }
    python -X utf8 tools/package_release.py --output-directory $OutputDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed.' }
} finally {
    $env:NRHEX_PARRY_DATA = $taskOldParry
    $env:NRHEX_SWORD_DATA = $taskOldSword
    $env:NRHEX_ECHO_DATA = $taskOldEcho
    Pop-Location
}
