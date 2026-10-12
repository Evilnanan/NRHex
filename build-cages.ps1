param([switch]$NoInstall)
$ErrorActionPreference = 'Stop'
$taskDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
$taskGame = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path $taskDrive 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
$taskPython = (Get-Command python -ErrorAction Stop).Source
$taskMe3 = if ($env:NRHEX_ME3) { $env:NRHEX_ME3 } else { Join-Path $taskDrive 'Program Files/me3/bin/me3.exe' }
Push-Location $PSScriptRoot
try {
    & $taskPython -X utf8 tools/verify_workspace.py
    if ($LASTEXITCODE -ne 0) { throw 'Workspace inputs validation failed.' }
    cargo run --release --bin scancheck -- "$taskGame/nightreign.exe"
    if ($LASTEXITCODE -ne 0) { throw 'Current EXE guard failed.' }
    & $taskPython -X utf8 tools/record_cage_patch.py --before
    if ($LASTEXITCODE -ne 0) { throw 'Cage patch baseline capture failed.' }
    dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --cage-only
    if ($LASTEXITCODE -ne 0) { throw 'Cage event build failed.' }
    & $taskPython -X utf8 tools/verify_cage_flag_storage.py
    if ($LASTEXITCODE -ne 0) { throw 'Native cage flag storage check failed.' }
    & $taskPython -X utf8 tools/verify_cage_groups.py
    if ($LASTEXITCODE -ne 0) { throw 'Cage group lifecycle/HUD/native reward verification failed.' }
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    cargo test --lib
    if ($LASTEXITCODE -ne 0) { throw 'Runtime regression checks failed.' }
    if (-not $NoInstall) {
        New-Item -ItemType Directory -Path "$PSScriptRoot/dist/mod/natives" -Force | Out-Null
        Copy-Item -LiteralPath "$PSScriptRoot/target/release/NRHex.dll" -Destination "$PSScriptRoot/dist/mod/natives/NRHex.dll" -Force
    }
    & $taskMe3 profile show --file "$PSScriptRoot/dist/NRHex.me3"
    if ($LASTEXITCODE -ne 0) { throw 'ME3 profile check failed.' }
    if ($NoInstall) { & $taskPython -X utf8 tools/record_cage_patch.py --no-install }
    else { & $taskPython -X utf8 tools/record_cage_patch.py --rust-tests 155 }
    if ($LASTEXITCODE -ne 0) { throw 'Cage patch scope check failed.' }
} finally { Pop-Location }
