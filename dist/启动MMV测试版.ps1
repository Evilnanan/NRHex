param([switch]$ValidateOnly)
$ErrorActionPreference = 'Stop'
$taskProfile = Join-Path $PSScriptRoot 'mmv/NRHex-MMV.me3'
if (-not (Test-Path -LiteralPath $taskProfile -PathType Leaf)) {
    throw '缺少 MMV 测试包，请先运行 build-mmv.ps1。'
}
$taskDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
$taskMe3 = if ($env:NRHEX_ME3) { $env:NRHEX_ME3 } else { Join-Path $taskDrive 'Program Files/me3/bin/me3.exe' }
$taskGame = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path $taskDrive 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
if ($ValidateOnly) {
    & $taskMe3 profile show --file $taskProfile
    if ($LASTEXITCODE -ne 0) { throw "ME3 profile check failed: $LASTEXITCODE" }
    return
}
if (Get-Process nightreign -ErrorAction SilentlyContinue) {
    throw '游戏仍在运行，请先退出当前版本。'
}
& $taskMe3 launch -g nightreign -p $taskProfile -e "$taskGame/nightreign.exe"
if ($LASTEXITCODE -ne 0) { throw "ME3 launch failed: $LASTEXITCODE" }
