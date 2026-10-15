param([switch]$NoInstall)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-regain-stage
    if ($LASTEXITCODE -ne 0) { throw 'Sword regain parameter staging failed.' }
    & ./build-texts.ps1 -NoInstall
    cargo test --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'Rust checks failed.' }
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    cargo run --release --bin scancheck
    if ($LASTEXITCODE -ne 0) { throw 'Runtime signatures failed.' }
    python -X utf8 tools/verify_sword_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword native validation failed.' }
    python -X utf8 tools/verify_sword_regain_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword native regain validation failed.' }
    if (-not $NoInstall) {
        python -X utf8 tools/install_sword_regain.py
        if ($LASTEXITCODE -ne 0) { throw 'Sword regain installation failed.' }
    }
} finally { Pop-Location }
