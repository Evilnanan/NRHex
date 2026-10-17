param([switch]$NoInstall)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $work = Join-Path $PSScriptRoot 'work/combat-protection'
    New-Item -ItemType Directory -Force $work | Out-Null
    dotnet run --project tools/HexBuild -- $PSScriptRoot --combat-protection-texts
    if ($LASTEXITCODE -ne 0) { throw 'Combat text staging failed.' }
    cargo test --release --lib 2>&1 | Tee-Object (Join-Path $work 'rust-tests.log')
    if ($LASTEXITCODE -ne 0) { throw 'Rust checks failed.' }
    python -X utf8 tools/verify_death_dance.py
    if ($LASTEXITCODE -ne 0) { throw 'Native Death Dance execution failed.' }
    python -X utf8 tools/verify_erosion.py
    if ($LASTEXITCODE -ne 0) { throw 'Native Erosion execution failed.' }
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    $game = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path (Split-Path $PSScriptRoot -Qualifier) 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
    cargo run --release --bin scancheck -- (Join-Path $game 'nightreign.exe') 2>&1 | Tee-Object (Join-Path $work 'scancheck.log')
    if ($LASTEXITCODE -ne 0) { throw 'Runtime signatures failed.' }
    if (-not $NoInstall) {
        python -X utf8 tools/install_combat_protection.py
        if ($LASTEXITCODE -ne 0) { throw 'Combat fix installation failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --check-descriptions
        if ($LASTEXITCODE -ne 0) { throw 'Installed descriptions do not match.' }
    }
} finally { Pop-Location }
