param(
    [switch]$WhatIf,
    [ValidateRange(1, 60)][int]$TimeoutSeconds = 12
)

$ErrorActionPreference = 'Stop'

function Get-NightreignSteamLog {
    param([object[]]$Processes)
    $steam = $Processes | Where-Object { $_.Name -eq 'steam.exe' -and $_.ExecutablePath } | Select-Object -First 1
    if ($steam) { return Join-Path (Split-Path -Parent $steam.ExecutablePath) 'logs\gameprocess_log.txt' }
    $settings = Get-ItemProperty -LiteralPath 'HKCU:\Software\Valve\Steam' -ErrorAction SilentlyContinue
    if ($settings -and $settings.SteamPath) { return Join-Path $settings.SteamPath 'logs\gameprocess_log.txt' }
}

function Get-NightreignSteamRecords {
    param([string]$LogPath)
    if (-not $LogPath -or -not (Test-Path -LiteralPath $LogPath)) { return }
    $stream = $null
    $reader = $null
    try {
        $stream = [IO.File]::Open($LogPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete)
        [void]$stream.Seek([Math]::Max(0, $stream.Length - 4MB), [IO.SeekOrigin]::Begin)
        $reader = New-Object IO.StreamReader($stream)
        while (-not $reader.EndOfStream) {
            $line = $reader.ReadLine()
            if ($line -match '^\[(?<timestamp>[^\]]+)\] AppID 2622380 adding PID (?<process>\d+) as a tracked process .*[\\/]me3(?:-launcher)?\.exe') {
                $created = [DateTime]::MinValue
                if ([DateTime]::TryParseExact($Matches.timestamp, 'yyyy-MM-dd HH:mm:ss', [Globalization.CultureInfo]::InvariantCulture, [Globalization.DateTimeStyles]::None, [ref]$created)) {
                    [pscustomobject]@{ ProcessId = [int]$Matches.process; Created = $created }
                }
            }
        }
    } catch {
        Write-Warning "Cannot read Steam process log: $($_.Exception.Message)"
    } finally {
        if ($reader) { $reader.Dispose() } elseif ($stream) { $stream.Dispose() }
    }
}

function Get-NightreignCleanupTargets {
    param([object[]]$Processes, [object[]]$SteamRecords)
    $byId = @{}
    $targets = @{}
    foreach ($process in $Processes) { $byId[[int]$process.ProcessId] = $process }
    foreach ($process in $Processes) {
        $reason = $null
        if ($process.Name -eq 'nightreign.exe') { $reason = 'Nightreign game' }
        elseif ($process.Name -in @('me3.exe', 'me3-launcher.exe') -and $process.CreationDate) {
            foreach ($record in $SteamRecords) {
                if ([int]$record.ProcessId -eq [int]$process.ProcessId -and [Math]::Abs((([DateTime]$process.CreationDate) - $record.Created).TotalSeconds) -le 3) {
                    $reason = 'Steam-tracked Nightreign launcher'
                    break
                }
            }
        }
        if ($reason) { $targets[[int]$process.ProcessId] = [pscustomobject]@{ Process = $process; Reason = $reason } }
    }
    do {
        $added = $false
        foreach ($target in @($targets.Values)) {
            $child = $target.Process
            $parentId = [int]$child.ParentProcessId
            if (-not $targets.ContainsKey($parentId) -and $byId.ContainsKey($parentId)) {
                $parent = $byId[$parentId]
                if ($parent.Name -in @('me3.exe', 'me3-launcher.exe') -and $parent.CreationDate -and $child.CreationDate -and [DateTime]$parent.CreationDate -le [DateTime]$child.CreationDate) {
                    $targets[$parentId] = [pscustomobject]@{ Process = $parent; Reason = 'Nightreign launcher ancestor' }
                    $added = $true
                }
            }
        }
        foreach ($process in $Processes) {
            $processId = [int]$process.ProcessId
            $parentId = [int]$process.ParentProcessId
            if (-not $targets.ContainsKey($processId) -and $process.Name -in @('me3.exe', 'me3-launcher.exe') -and $targets.ContainsKey($parentId)) {
                $parent = $targets[$parentId].Process
                if ($parent.CreationDate -and $process.CreationDate -and [DateTime]$parent.CreationDate -le [DateTime]$process.CreationDate) {
                    $targets[$processId] = [pscustomobject]@{ Process = $process; Reason = 'Nightreign launcher descendant' }
                    $added = $true
                }
            }
        }
    } while ($added)
    $targets.Values | Sort-Object @{ Expression = { if ($_.Process.Name -eq 'nightreign.exe') { 0 } else { 1 } } }, @{ Expression = { $_.Process.ProcessId } }
}

function Test-NightreignProcessIdentity {
    param($Expected, $Current)
    return $null -ne $Current -and $Expected.CreationDate -and $Current.CreationDate -and
        [int]$Expected.ProcessId -eq [int]$Current.ProcessId -and $Expected.Name -eq $Current.Name -and
        [DateTime]$Expected.CreationDate -eq [DateTime]$Current.CreationDate
}

function Invoke-NightreignCleanup {
    param([switch]$Preview, [int]$Timeout = 12)
    Write-Host 'Nightreign force-stop: current expedition may end. Steam and other games are not stopped.'
    $deadline = (Get-Date).AddSeconds($Timeout)
    $stopped = @{}
    $failures = @{}
    do {
        $processes = @(Get-CimInstance Win32_Process)
        $logPath = Get-NightreignSteamLog -Processes $processes
        $records = @(Get-NightreignSteamRecords -LogPath $logPath)
        $targets = @(Get-NightreignCleanupTargets -Processes $processes -SteamRecords $records)
        if ($targets.Count -eq 0) {
            if ($stopped.Count -eq 0) { Write-Host 'No Nightreign game or confirmed ME3 leftovers found.' }
            else { Write-Host 'Done: game and confirmed launcher leftovers have exited.' }
            Write-Host 'If Steam still shows Stopping, exit and reopen Steam manually. This script never restarts Steam.'
            return 0
        }
        foreach ($target in $targets) {
            $process = $target.Process
            $processId = [int]$process.ProcessId
            $identity = '{0}/{1:o}' -f $processId, ([DateTime]$process.CreationDate)
            if ($Preview) {
                Write-Host "[Preview] $($process.Name) PID $processId - $($target.Reason)"
                continue
            }
            if ($failures.ContainsKey($identity)) { continue }
            $current = Get-CimInstance Win32_Process -Filter "ProcessId = $processId"
            if (-not (Test-NightreignProcessIdentity -Expected $process -Current $current)) { continue }
            try {
                Stop-Process -Id $processId -Force -ErrorAction Stop
                $stopped[$identity] = $true
                Write-Host "Stopped $($process.Name) PID $processId - $($target.Reason)"
            } catch {
                if (Get-Process -Id $processId -ErrorAction SilentlyContinue) {
                    $failures[$identity] = $_.Exception.Message
                    Write-Warning "Could not stop PID ${processId}: $($_.Exception.Message). Try Run as administrator."
                }
            }
        }
        if ($Preview) { return 0 }
        if ((Get-Date) -ge $deadline) { break }
        Start-Sleep -Milliseconds 400
    } while ($true)
    Write-Warning 'Some confirmed Nightreign processes remain. Check permissions or run this script as administrator.'
    return 1
}

if ($MyInvocation.InvocationName -ne '.') {
    try { exit (Invoke-NightreignCleanup -Preview:$WhatIf -Timeout $TimeoutSeconds) }
    catch { Write-Warning $_.Exception.Message; exit 1 }
}
