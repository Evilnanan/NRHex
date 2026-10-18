# Exercise real build control flow with external tools replaced by fixtures.
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$work = Join-Path $root ('work/build-entrypoint-check/' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -Force | Out-Null
Microsoft.PowerShell.Management\Copy-Item (Join-Path $root 'build.ps1') (Join-Path $work 'build.ps1')
$global:NRHexBuildFixtureCalls = [System.Collections.Generic.List[object]]::new()
$global:NRHexBuildFixtureFailure = ''
function Assert-True($Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Record-Call([string]$Name, $Arguments) {
    $global:NRHexBuildFixtureCalls.Add([pscustomobject]@{ Name = $Name; Arguments = @($Arguments) })
    $global:LASTEXITCODE = if ($global:NRHexBuildFixtureFailure -and (($Arguments -join ' ') -like "*$global:NRHexBuildFixtureFailure*")) { 7 } else { 0 }
}
function cargo { Record-Call 'cargo' $args }
function python { Record-Call 'python' $args }
function dotnet { Record-Call 'dotnet' $args }
function nrhex-me3-fixture { Record-Call 'me3' $args }
function Copy-Item {
    param([string]$LiteralPath, [string]$Destination, [switch]$Force)
    Record-Call 'copy' @($LiteralPath, $Destination)
}
function Has-Call([string]$Name, [string]$Text) {
    @($global:NRHexBuildFixtureCalls | Where-Object { $_.Name -eq $Name -and (($_.Arguments -join ' ') -like "*$Text*") }).Count -gt 0
}
$names = @('NRHEX_GAME', 'NRHEX_ME3', 'NRHEX_CARGO_TARGET', 'NRHEX_NATIVE_EXE')
$before = @{}; foreach ($name in $names) { $before[$name] = [Environment]::GetEnvironmentVariable($name) }
$location = Get-Location
try {
    $env:NRHEX_GAME = Join-Path $work 'game'
    $env:NRHEX_ME3 = 'nrhex-me3-fixture'
    $env:NRHEX_CARGO_TARGET = 'x86_64-pc-windows-gnu'
    $env:NRHEX_NATIVE_EXE = 'Z:\fixture game\nightreign.exe'
    foreach ($mode in @('FullNoInstall', 'DllNoInstall', 'DllInstall', 'DllFailure')) {
        $global:NRHexBuildFixtureCalls.Clear()
        $global:NRHexBuildFixtureFailure = if ($mode -eq 'DllFailure') { 'verify_erosion.py' } else { '' }
        $options = @{ NoInstall = $mode -ne 'DllInstall'; DllOnly = $mode -ne 'FullNoInstall' }
        $failed = $false
        try { & (Join-Path $work 'build.ps1') @options } catch { $failed = $true; if ($mode -ne 'DllFailure') { throw } }
        Assert-True ((Get-Location).Path -eq $location.Path) 'Build did not restore its working directory'
        if ($mode -eq 'DllFailure') {
            Assert-True $failed 'Native verification failure was swallowed'
            Assert-True (-not (Has-Call 'cargo' 'build --release')) 'Build continued after verification failed'
            Assert-True (-not (Has-Call 'copy' 'NRHex.dll')) 'A failed build installed a DLL'
            Assert-True (-not (Has-Call 'python' 'record_build.py')) 'A failed build was recorded as successful'
        } else {
            Assert-True (Has-Call 'cargo' 'test --release --lib --target x86_64-pc-windows-gnu') 'Target was not forwarded to regression tests'
            foreach ($check in @('verify_death_dance.py', 'verify_erosion.py', 'verify_potential_fission.py', 'verify_reward_template.py', 'verify_ambulance.py')) {
                Assert-True (Has-Call 'python' $check) "Missing runtime verification: $check"
            }
            $scan = @($global:NRHexBuildFixtureCalls | Where-Object { $_.Name -eq 'cargo' -and $_.Arguments -contains 'scancheck' })
            Assert-True ($scan.Count -eq 1 -and $scan[0].Arguments[-1] -eq $env:NRHEX_NATIVE_EXE -and $scan[0].Arguments -contains 'x86_64-pc-windows-gnu') 'Signature scan did not receive the runner-visible EXE path'
            $record = @($global:NRHexBuildFixtureCalls | Where-Object { $_.Name -eq 'python' -and $_.Arguments -contains 'tools/record_build.py' })
            Assert-True ($record.Count -eq 1 -and (($record[0].Arguments -contains '--no-install') -eq $options.NoInstall)) 'Build record lost the installation mode'
            if ($mode -eq 'FullNoInstall') {
                Assert-True (Has-Call 'dotnet' '--effect-sync-probe') 'Missing current effect-sync fixture export'
                Assert-True (Has-Call 'python' 'verify_effect_sync_native.py --full-build') 'Full build still depends on a historical effect-sync patch'
                Assert-True (Has-Call 'python' 'verify_cage_groups.py') 'Missing group lifecycle verification'
                Assert-True (Has-Call 'dotnet' '--regulation-compression') 'Full build skipped resource generation'
                Assert-True (Has-Call 'me3' 'profile show') 'Full build skipped profile validation'
                Assert-True (-not (Has-Call 'copy' 'NRHex.dll')) 'NoInstall replaced the DLL'
            } else {
                Assert-True (-not (Has-Call 'dotnet' '')) 'DLL-only build invoked a resource generator'
                Assert-True (-not (Has-Call 'me3' '')) 'DLL-only build requires ME3'
                Assert-True (Has-Call 'python' '--dll-only') 'DLL build overwrote full-build records'
                Assert-True ((Has-Call 'copy' 'NRHex.dll') -eq ($mode -eq 'DllInstall')) 'DLL installation did not respect NoInstall'
                if ($mode -eq 'DllInstall') {
                    Assert-True (Has-Call 'copy' 'target*x86_64-pc-windows-gnu*release*NRHex.dll') 'Installed DLL came from the wrong target'
                }
            }
        }
        Write-Host "PASS: $mode"
    }
    $entrypoints = @(Get-ChildItem $root -Filter 'build*.ps1' | Sort-Object Name)
    $expected = @('build-no-hotkeys.ps1', 'build-texts.ps1', 'build.ps1')
    Assert-True ($entrypoints.Count -eq 3 -and @(Compare-Object $entrypoints.Name $expected).Count -eq 0) 'Unexpected build entrypoints remain'
    foreach ($entrypoint in $entrypoints) {
        $tokens = $null; $errors = $null
        [System.Management.Automation.Language.Parser]::ParseFile($entrypoint.FullName, [ref]$tokens, [ref]$errors) > $null
        Assert-True ($errors.Count -eq 0) "PowerShell syntax error: $($entrypoint.Name)"
    }
    Write-Host 'PASS: three entrypoints parsed; resource routing, target paths, failure propagation and installation gates verified'
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $before[$name]) }
}
