$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $root 'dist/Stop-Nightreign.ps1')
$script:checks = 0
$created = [DateTime]'2026-10-17T18:20:46'

function Assert-Cleanup {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
    $script:checks++
}

function New-CleanupProcess {
    param([string]$Name, [int]$ProcessId, [int]$ParentId = 0, [DateTime]$Started = $created)
    [pscustomobject]@{ Name = $Name; ProcessId = $ProcessId; ParentProcessId = $ParentId; CreationDate = $Started; ExecutablePath = $null }
}

function Assert-TargetIds {
    param([object[]]$Processes, [object[]]$Records, [int[]]$Expected)
    $actual = @((Get-NightreignCleanupTargets -Processes $Processes -SteamRecords $Records) | ForEach-Object { $_.Process.ProcessId } | Sort-Object)
    Assert-Cleanup (($actual -join ',') -eq (($Expected | Sort-Object) -join ',')) "Unexpected target PIDs: $actual, expected $Expected"
}

$steam = New-CleanupProcess 'steam.exe' 100
$game = New-CleanupProcess 'nightreign.exe' 101 102
$launcher = New-CleanupProcess 'me3-launcher.exe' 102 103
$cli = New-CleanupProcess 'me3.exe' 103 100
$other = New-CleanupProcess 'me3-launcher.exe' 104 100
$protected = New-CleanupProcess 'start_protected_game.exe' 105
$anticheat = New-CleanupProcess 'EasyAntiCheat_EOS.exe' 106
$records = @([pscustomobject]@{ ProcessId = 102; Created = $created })
Assert-TargetIds @($game, $launcher, $cli, $steam, $other, $protected, $anticheat) @() @(101, 102, 103)
Assert-TargetIds @($launcher, $cli, $steam, $other) $records @(102, 103)
Assert-TargetIds @($launcher, $cli, $steam, $other) @() @()
Assert-TargetIds @($steam, $other, $protected, $anticheat) $records @()
$reused = New-CleanupProcess 'me3-launcher.exe' 102 100 ($created.AddMinutes(1))
Assert-TargetIds @($reused, $steam) $records @()
Assert-TargetIds @($game, $reused, $steam) @() @(101)
$child = New-CleanupProcess 'me3-launcher.exe' 107 102 ($created.AddSeconds(1))
Assert-TargetIds @($launcher, $child, $other) $records @(102, 107)
$oldChild = New-CleanupProcess 'me3-launcher.exe' 107 102 ($created.AddSeconds(-1))
Assert-TargetIds @($launcher, $oldChild) $records @(102)
Assert-Cleanup (Test-NightreignProcessIdentity $game $game) 'Same process identity rejected'
Assert-Cleanup (-not (Test-NightreignProcessIdentity $launcher $reused)) 'Reused PID accepted'
Assert-Cleanup (-not (Test-NightreignProcessIdentity $game $null)) 'Exited process accepted'

$directory = Join-Path $root '.work/stop-nightreign-check'
New-Item -ItemType Directory -Path $directory -Force | Out-Null
$script:fixtureLog = Join-Path $directory 'gameprocess_log.txt'
@'
[2026-10-17 18:20:46] AppID 2622380 adding PID 102 as a tracked process ""D:\Program Files\me3\bin\me3-launcher.exe""
[2026-10-17 18:20:46] AppID 1234567 adding PID 104 as a tracked process ""D:\Program Files\me3\bin\me3-launcher.exe""
[2026-10-17 18:20:46] AppID 2622380 adding PID 101 as a tracked process ""D:\Game\nightreign.exe""
[2026-10-17 18:20:46] AppID 2622380 no longer tracking PID 101, exit code 0
[invalid timestamp] AppID 2622380 adding PID 999 as a tracked process ""D:\me3-launcher.exe""
'@ | Set-Content -LiteralPath $script:fixtureLog -Encoding UTF8
$parsed = @(Get-NightreignSteamRecords $script:fixtureLog)
Assert-Cleanup ($parsed.Count -eq 1 -and $parsed[0].ProcessId -eq 102 -and $parsed[0].Created -eq $created) 'Steam AppID/name/timestamp filtering failed'
Assert-Cleanup (@(Get-NightreignSteamRecords (Join-Path $directory 'missing.log')).Count -eq 0) 'Missing log should be harmless'
Assert-TargetIds @($launcher, $other) $parsed @(102)

function Get-NightreignSteamLog { param($Processes) return $script:fixtureLog }
function Get-CimInstance {
    param($ClassName, [string]$Filter)
    if ($Filter) {
        $processId = [int]($Filter -replace '\D', '')
        if ($script:reuseDuringStop) {
            $script:remaining = @()
            return New-CleanupProcess 'nightreign.exe' $processId 0 ($created.AddHours(1))
        }
        return $script:remaining | Where-Object { $_.ProcessId -eq $processId }
    }
    return $script:remaining
}
function Stop-Process {
    param([int]$Id, [switch]$Force, $ErrorAction)
    $script:stopped += $Id
    $script:remaining = @($script:remaining | Where-Object { $_.ProcessId -ne $Id })
}
$script:remaining = @($game, $launcher, $cli, $other, $steam)
$script:stopped = @()
$script:reuseDuringStop = $false
$status = Invoke-NightreignCleanup -Preview
Assert-Cleanup ($status -eq 0 -and $script:stopped.Count -eq 0) 'Preview stopped a process'
$status = Invoke-NightreignCleanup -Timeout 2
Assert-Cleanup ($status -eq 0 -and ($script:stopped -join ',') -eq '101,102,103') 'Mock cleanup did not stop game before its launchers'
Assert-Cleanup (($script:remaining.ProcessId -join ',') -eq '104,100') 'Other game launcher or Steam was stopped'
$script:remaining = @($game)
$script:stopped = @()
$script:reuseDuringStop = $true
$status = Invoke-NightreignCleanup -Timeout 2
Assert-Cleanup ($status -eq 0 -and $script:stopped.Count -eq 0) 'PID reused immediately before stop was killed'
$status = Invoke-NightreignCleanup -Timeout 1
Assert-Cleanup ($status -eq 0 -and $script:stopped.Count -eq 0) 'Empty cleanup is not idempotent'
Write-Host "PASS: $script:checks cleanup checks; all stop calls mocked; no real processes terminated."
