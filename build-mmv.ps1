param(
    [string]$MmvRoot = $(if ($env:NRHEX_MMV) { $env:NRHEX_MMV } else { 'D:\Users\OwO\Documents\me3-profiles\MMV' }),
    [string]$OriginalSfx = $env:NRHEX_ORIGINAL_SFX,
    [switch]$AuditOnly,
    [switch]$RecordBaseline,
    [switch]$StageOnly,
    [ValidateSet('dist','community','both')][string]$Variant = 'dist',
    [string]$CommunityDestination,
    [switch]$BuildOnly
)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $taskArgs = @('-X', 'utf8', 'tools/mmv_build.py', '--mmv', $MmvRoot)
    if ($OriginalSfx) { $taskArgs += @('--original-sfx', $OriginalSfx) }
    if ($AuditOnly) { $taskArgs += '--audit-only' }
    if ($RecordBaseline) { $taskArgs += '--record-baseline' }
    if ($StageOnly) { $taskArgs += '--stage-only' }
    $taskArgs += @('--variant', $Variant)
    if ($CommunityDestination) { $taskArgs += @('--community-destination', $CommunityDestination) }
    if ($BuildOnly) { $taskArgs += '--build-only' }
    & python @taskArgs
    if ($LASTEXITCODE -ne 0) { throw "MMV target stopped (exit $LASTEXITCODE). See work/compat/mmv/build-report.json." }
} finally { Pop-Location }
