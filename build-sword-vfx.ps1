$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-vfx-stage
    if ($LASTEXITCODE -ne 0) { throw 'Sword visual parameter staging failed.' }
    dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-fxr-stage
    if ($LASTEXITCODE -ne 0) { throw 'Sword Storm stance visual resource staging failed.' }
    python -X utf8 tools/build_sword_vfx_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword casting-hand script validation failed.' }
    python -X utf8 tools/build_sword_recovery_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword casting recovery script validation failed.' }
    cargo test --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'Rust checks failed.' }
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    python -X utf8 tools/install_sword_vfx.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword update installation failed.' }
} finally { Pop-Location }
