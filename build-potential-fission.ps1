param([switch]$NoInstall)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    dotnet run --project tools/HexBuild -- $PSScriptRoot --potential-fission-stage
    if ($LASTEXITCODE -ne 0) { throw 'Fission staging failed.' }
    Copy-Item work/potential-fission/staged/fission_catalog.rs src/fission_catalog.rs -Force
    Copy-Item work/potential-fission/staged/hex_pool.rs src/hex_pool.rs -Force
    cargo test --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'Rust checks failed.' }
    python -X utf8 tools/verify_potential_fission.py
    if ($LASTEXITCODE -ne 0) { throw 'Native fission checks failed.' }
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    $game = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path (Split-Path $PSScriptRoot -Qualifier) 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
    cargo run --release --bin scancheck -- (Join-Path $game 'nightreign.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Runtime signatures failed.' }
    if (-not $NoInstall) {
        python -X utf8 tools/install_potential_fission.py
        if ($LASTEXITCODE -ne 0) { throw 'Fission installation failed.' }
    }
} finally { Pop-Location }
