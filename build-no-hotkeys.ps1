param([string]$Destination = 'D:\Users\OwO\Documents\me3-profiles\NRHex')
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    # Separate output keeps the development DLL with hotkeys intact.
    cargo build --release --lib --no-default-features --features cage-demand --target-dir "$PSScriptRoot/.work/no-hotkeys-target"
    if ($LASTEXITCODE -ne 0) { throw 'No-hotkeys DLL build failed.' }
    python -X utf8 tools/package_no_hotkeys.py --destination $Destination
    if ($LASTEXITCODE -ne 0) { throw 'No-hotkeys packaging failed.' }
    $taskDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
    $taskMe3 = if ($env:NRHEX_ME3) { $env:NRHEX_ME3 } else { Join-Path $taskDrive 'Program Files/me3/bin/me3.exe' }
    & $taskMe3 profile show --file (Join-Path $Destination 'NRHex.me3')
    if ($LASTEXITCODE -ne 0) { throw 'Packaged ME3 profile check failed.' }
} finally { Pop-Location }
