param([switch]$CaptureResearchBaseline)
$ErrorActionPreference = 'Stop'
$taskDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
$taskGame = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path $taskDrive 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
$taskPython = (Get-Command python -ErrorAction Stop).Source
$taskMe3 = if ($env:NRHEX_ME3) { $env:NRHEX_ME3 } else { Join-Path $taskDrive 'Program Files/me3/bin/me3.exe' }
function Check-Exit([string]$Stage) { if ($LASTEXITCODE -ne 0) { throw "$Stage failed." } }
Push-Location $PSScriptRoot
try {
    & $taskPython -X utf8 tools/verify_workspace.py
    Check-Exit 'Workspace inputs'
    if ($CaptureResearchBaseline) { & $taskPython -X utf8 tools/record_effect_sync.py --before --from-research }
    else { & $taskPython -X utf8 tools/record_effect_sync.py --before }
    Check-Exit 'Package baseline'
    cargo run --release --bin scancheck -- "$taskGame/nightreign.exe"
    Check-Exit 'Current native signatures'
    cargo test --lib --quiet
    Check-Exit 'Rust tests'
    dotnet run --project tools/HexBuild -- $PSScriptRoot --effect-sync-only
    Check-Exit 'Effect sync parameters'
    dotnet run --project tools/HexBuild -- $PSScriptRoot --effect-sync-probe
    Check-Exit 'Parameter readback'
    & $taskPython -X utf8 tools/verify_effect_sync_native.py
    Check-Exit 'Original native predicates and replica model'
    cargo build --release --lib
    Check-Exit 'DLL build'
    New-Item -ItemType Directory -Path "$PSScriptRoot/dist/mod/natives" -Force | Out-Null
    Copy-Item -LiteralPath "$PSScriptRoot/target/release/NRHex.dll" -Destination "$PSScriptRoot/dist/mod/natives/NRHex.dll" -Force
    & $taskMe3 profile show --file "$PSScriptRoot/dist/NRHex.me3"
    Check-Exit 'ME3 profile'
    & $taskPython -X utf8 tools/record_effect_sync.py
    Check-Exit 'Patch scope'
} finally { Pop-Location }
