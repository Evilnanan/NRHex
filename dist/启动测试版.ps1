$ErrorActionPreference = 'Stop'
$taskPending = Join-Path $PSScriptRoot 'pending-update'
$taskManifest = Join-Path $taskPending 'manifest.json'
if (Test-Path -LiteralPath $taskManifest) {
    if (Get-Process nightreign -ErrorAction SilentlyContinue) {
        throw '请先退出当前游戏，再启动新版。不会自动关闭游戏。'
    }
    $taskUpdate = Get-Content -LiteralPath $taskManifest -Raw -Encoding UTF8 | ConvertFrom-Json
    $taskInstalled = Join-Path $PSScriptRoot 'natives/NRHex.dll'
    $taskCurrentHash = (Get-FileHash -LiteralPath $taskInstalled -Algorithm SHA256).Hash
    if ($taskCurrentHash -notin @($taskUpdate.previousDllSha256, $taskUpdate.dllSha256)) {
        throw '当前 DLL 与待更新版本的基线不符，请重新构建后更新。'
    }
    foreach ($taskFile in $taskUpdate.files) {
        $taskFrom = [IO.Path]::GetFullPath((Join-Path $taskPending $taskFile.path))
        $taskTo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $taskFile.path))
        if (-not $taskFrom.StartsWith($taskPending + '\', [StringComparison]::OrdinalIgnoreCase) -or
            -not $taskTo.StartsWith($PSScriptRoot + '\', [StringComparison]::OrdinalIgnoreCase) -or
            (Get-FileHash -LiteralPath $taskFrom -Algorithm SHA256).Hash -ne $taskFile.sha256) {
            throw '待更新文件校验失败，未启动游戏。'
        }
    }
    foreach ($taskFile in $taskUpdate.files) {
        $taskFrom = Join-Path $taskPending $taskFile.path
        $taskTo = Join-Path $PSScriptRoot $taskFile.path
        $taskParent = Split-Path -Parent $taskTo
        if (-not (Test-Path -LiteralPath $taskParent)) {
            New-Item -ItemType Directory -Path $taskParent -Force | Out-Null
        }
        Copy-Item -LiteralPath $taskFrom -Destination $taskTo -Force
        if ((Get-FileHash -LiteralPath $taskTo -Algorithm SHA256).Hash -ne $taskFile.sha256) {
            throw '更新文件回读校验失败，未启动游戏。'
        }
    }
    Remove-Item -LiteralPath $taskManifest
}
$taskDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
$taskMe3 = if ($env:NRHEX_ME3) { $env:NRHEX_ME3 } else { Join-Path $taskDrive 'Program Files/me3/bin/me3.exe' }
$taskGame = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path $taskDrive 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
& $taskMe3 launch -g nightreign -p "$PSScriptRoot\NRHex.me3" -e "$taskGame/nightreign.exe"
if ($LASTEXITCODE -ne 0) { throw "ME3 launch failed: $LASTEXITCODE" }
