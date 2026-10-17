param([switch]$NoInstall,[switch]$ProtectionOnly)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    if (-not $ProtectionOnly) {
        dotnet run --project tools/HexBuild -- $PSScriptRoot --death-dance-stage
        if ($LASTEXITCODE -ne 0) { throw 'Death Dance resource staging failed.' }
        Copy-Item work/death-dance/staged/hex_pool.rs src/hex_pool.rs -Force
    }
    cargo test --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'Rust checks failed.' }
    python -X utf8 tools/verify_death_dance.py
    if ($LASTEXITCODE -ne 0) { throw 'Native Death Dance execution failed.' }
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    $game = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path (Split-Path $PSScriptRoot -Qualifier) 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
    cargo run --release --bin scancheck -- (Join-Path $game 'nightreign.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Runtime signatures failed.' }
    if (-not $NoInstall) {
        if ($ProtectionOnly) { python -X utf8 tools/install_death_dance.py --dll-only }
        else { python -X utf8 tools/install_death_dance.py }
        if ($LASTEXITCODE -ne 0) { throw 'Death Dance installation failed.' }
    }
} finally { Pop-Location }
