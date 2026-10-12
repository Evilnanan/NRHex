param([switch]$Check, [switch]$NoInstall)
$ErrorActionPreference = 'Stop'
if ($Check -and $NoInstall) { throw 'Use either -Check or -NoInstall.' }
Push-Location $PSScriptRoot
try {
    $mode = if ($Check) { '--check-descriptions' } else { '--stage-translations' }
    & dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot $mode
    if ($LASTEXITCODE -ne 0) { throw 'Text validation/build failed; nothing installed.' }
    if ($Check -or $NoInstall) { return }

    $stage = Join-Path $PSScriptRoot '.work/texts'
    $report = Get-Content -LiteralPath (Join-Path $stage 'validation.json') -Raw | ConvertFrom-Json
    foreach ($inputText in @(
        @{ Path = $report.source; Hash = $report.sourceSha256 },
        @{ Path = $report.englishSource; Hash = $report.englishSourceSha256 },
        @{ Path = $report.statusSource; Hash = $report.statusSourceSha256 }
    )) {
        if ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $inputText.Path)).Hash -ne $inputText.Hash) {
            throw "Text source changed during staging: $($inputText.Path). Run again."
        }
    }
    $expected = @('zhocn/item_dlc01.msgbnd.dcx', 'zhocn/menu_dlc01.msgbnd.dcx', 'engus/item_dlc01.msgbnd.dcx', 'engus/menu_dlc01.msgbnd.dcx')
    $actual = @($report.reports | ForEach-Object { "$($_.language)/$($_.file)" })
    if ($actual.Count -ne 4 -or (Compare-Object $expected $actual)) { throw 'Unexpected text package list.' }
    $updates = foreach ($entry in $report.reports) {
        $relative = "$($entry.language)/$($entry.file)"
        $source = Join-Path $stage $relative
        $target = Join-Path $PSScriptRoot "dist/mod/msg/$relative"
        $backup = Join-Path $stage "before/$relative"
        if (-not $entry.unrelatedEntriesAndMembersUnchanged -or
            (Get-FileHash -LiteralPath $source).Hash -ne $entry.outputSha256 -or
            (Get-FileHash -LiteralPath $target).Hash -ne $entry.inputSha256) {
            throw "Text package changed or failed validation: $relative"
        }
        New-Item -ItemType Directory -Path (Split-Path $backup) -Force | Out-Null
        Copy-Item -LiteralPath $target -Destination $backup -Force
        @{ Source = $source; Target = $target; Backup = $backup; Changed = $entry.inputSha256 -ne $entry.outputSha256 }
    }
    try {
        foreach ($update in $updates) {
            if ($update.Changed) { Copy-Item -LiteralPath $update.Source -Destination $update.Target -Force }
        }
        & dotnet run --no-build --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --check-descriptions
        if ($LASTEXITCODE -ne 0) { throw 'Installed text readback failed.' }
    } catch {
        foreach ($update in $updates) {
            if ($update.Changed) { Copy-Item -LiteralPath $update.Backup -Destination $update.Target -Force }
        }
        throw
    }
    Write-Host 'Chinese and English texts updated in dist. Restart the game to load edited text.'
} finally { Pop-Location }
