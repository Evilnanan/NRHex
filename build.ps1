param([switch]$NoInstall)
$ErrorActionPreference = 'Stop'
$buildDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
$me3 = Join-Path $buildDrive 'Program Files/me3/bin/me3.exe'
$gameExe = Join-Path $buildDrive 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game/nightreign.exe'
Push-Location $PSScriptRoot
try {
    foreach ($taskFolder in @('.work/arts', '.work/hks', '.work/actions', '.work/effects', '.work/combat', '.work/rewards', '.work/defs', '.work/backstep', '.work/behavior', '.work/icons')) {
        New-Item -ItemType Directory -Path $taskFolder -Force | Out-Null
    }
    & python -X utf8 tools/verify_workspace.py
    if ($LASTEXITCODE -ne 0) { throw 'Workspace inputs validation failed.' }
    dotnet run --project tools/ArtsProbe/ArtsProbe.csproj
    if ($LASTEXITCODE -ne 0) { throw 'Original skill data export failed.' }
    & python -X utf8 tools/build_arts_data.py
    if ($LASTEXITCODE -ne 0) { throw 'Skill pool build failed.' }
    & python -X utf8 tools/build_sword_ranges.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword range table build failed.' }
    dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot
    if ($LASTEXITCODE -ne 0) { throw 'Parameter/event/text build failed.' }
    & python -X utf8 tools/verify_cage_network_native.py --full-build
    if ($LASTEXITCODE -ne 0) { throw 'Monster Cage native network packet regression failed.' }
    & python -X utf8 tools/verify_cage_flag_storage.py
    if ($LASTEXITCODE -ne 0) { throw 'Monster Cage native flag storage validation failed.' }
    & python -X utf8 tools/verify_cage_mutation.py
    if ($LASTEXITCODE -ne 0) { throw 'Monster Cage map/event/reward validation failed.' }
    dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --cat-jump-probe
    if ($LASTEXITCODE -ne 0) { throw 'Original cat jump parameters failed.' }
    & python -X utf8 tools/verify_cat_jump_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Native cat jump height validation failed.' }
    & python -X utf8 tools/verify_spiritspring_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Native spiritspring height validation failed.' }
    dotnet run --project tools/FlaskProbe/FlaskProbe.csproj -- level-rewards
    if ($LASTEXITCODE -ne 0) { throw 'Level lifecycle event readback failed.' }
    & python -X utf8 tools/verify_level_rewards.py
    if ($LASTEXITCODE -ne 0) { throw 'Level reward binding/lifecycle validation failed.' }
    & python -X utf8 tools/verify_personal_rewards.py
    if ($LASTEXITCODE -ne 0) { throw 'Native personal reward isolation validation failed.' }
    & python -X utf8 tools/verify_reward_recovery.py
    if ($LASTEXITCODE -ne 0) { throw 'Native personal reward recovery validation failed.' }
    dotnet run --project tools/FlaskProbe/FlaskProbe.csproj -- built
    if ($LASTEXITCODE -ne 0) { throw 'Flask event readback failed.' }
    & python -X utf8 tools/verify_flask_events.py
    if ($LASTEXITCODE -ne 0) { throw 'Flask event control-flow validation failed.' }
    & python -X utf8 tools/build_walljump.py
    if ($LASTEXITCODE -ne 0) { throw 'Original-bytecode wall jump patch failed.' }
    & python -X utf8 tools/verify_walljump_decompile.py
    if ($LASTEXITCODE -ne 0) { throw 'Independent HKS decompiler readback failed.' }
    dotnet run --project tools/TaeProbe/TaeProbe.csproj
    if ($LASTEXITCODE -ne 0) { throw 'Original wall input/takeoff animation evidence failed.' }
    & python -X utf8 tools/verify_native_runtime.py
    if ($LASTEXITCODE -ne 0) { throw 'Native runtime binding validation failed.' }
    & python -X utf8 tools/verify_strength_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Native Strength dispatch validation failed.' }
    & python -X utf8 tools/verify_crystal_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Native Crystal Shell cleanup validation failed.' }
    dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --sword-category-probe --sword-test-probe --export-native-fixtures
    if ($LASTEXITCODE -ne 0) { throw 'Native sword AttackRate fixture export failed.' }
    & python -X utf8 tools/verify_sword_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Native sword category/magic binding validation failed.' }
    dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --escape-probe
    if ($LASTEXITCODE -ne 0) { throw 'Original low-HP effect export failed.' }
    & python -X utf8 tools/verify_escape_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Escape native speed/low-HP validation failed.' }
    & python -X utf8 tools/verify_quiet_runtime.py
    if ($LASTEXITCODE -ne 0) { throw 'Quiet gameplay runtime validation failed.' }
    & python -X utf8 tools/build_jiahao.py
    if ($LASTEXITCODE -ne 0) { throw 'Jiahao HKS action patch failed.' }
    dotnet run --project tools/JiahaoTae/JiahaoTae.csproj -- $PSScriptRoot
    if ($LASTEXITCODE -ne 0) { throw 'Jiahao native backstep animation patch failed.' }
    & python -X utf8 tools/verify_jiahao.py
    if ($LASTEXITCODE -ne 0) { throw 'Jiahao script readback failed.' }
    & python -X utf8 tools/build_arts_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Skill chain script patch failed.' }
    & python -X utf8 tools/build_arts_stance_fix.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Skill stance re-entry patch failed.' }
    & python -X utf8 tools/verify_arts.py
    if ($LASTEXITCODE -ne 0) { throw 'Skill native/script verification failed.' }
    & python -X utf8 tools/verify_arts_behavior.py
    if ($LASTEXITCODE -ne 0) { throw 'Skill behavior source/table validation failed.' }
    & python -X utf8 tools/verify_arts_transfer.py
    if ($LASTEXITCODE -ne 0) { throw 'Skill drop/pickup instance binding validation failed.' }
    & python -X utf8 tools/verify_arts_constraints.py
    if ($LASTEXITCODE -ne 0) { throw 'Skill pool separation/projectile origin validation failed.' }
    & python -X utf8 tools/verify_noisy.py
    if ($LASTEXITCODE -ne 0) { throw 'Noisy Ghost native route/lava chain validation failed.' }
    if (Test-Path -LiteralPath '.work/cage-groups/manifest.json') {
        & python -X utf8 tools/build_spirit_boss_catalog.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Native boss catalogue verification failed.' }
    } # Clean builds retain the checked-in boss catalogue guarded by the EXE/regulation versions.
    & python -X utf8 tools/build_new_hexes_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Balloon and Revenant action notifications failed.' }
    & python -X utf8 tools/build_sword_vfx_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Sword casting-hand visual patch failed.' }
    & python -X utf8 tools/build_sword_recovery_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Sword spell recovery patch failed.' }
    dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-fxr-stage
    if ($LASTEXITCODE -ne 0) { throw 'Sword private visual resource patch failed.' }
    Copy-Item -LiteralPath '.work/sword-vfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx' -Destination 'dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx' -Force
    dotnet run --project tools/HexBuild -- $PSScriptRoot --devils-gaze-resources
    if ($LASTEXITCODE -ne 0) { throw 'Native Libra resource extraction failed.' }
    dotnet run --project tools/HexBuild -- $PSScriptRoot --devils-gaze-resources-stage
    if ($LASTEXITCODE -ne 0) { throw 'Libra resource merge failed.' }
    Copy-Item -LiteralPath '.work/devils-gaze/staged/sfxbnd_commoneffects_dlc01.ffxbnd.dcx' -Destination 'dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx' -Force
    python -X utf8 tools/build_archmage_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Archmage spell-slot script validation failed.' }
    python -X utf8 tools/build_slingshot_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot action notification validation failed.' }
    python -X utf8 tools/verify_slingshot_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot independent decompiler verification failed.' }
    python -X utf8 tools/build_slingshot_cancel_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot cancellation HKS build failed' }
    python -X utf8 tools/verify_slingshot_cancel_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot cancellation HKS verification failed' }
    python -X utf8 tools/build_slingshot_landing_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot falling attack HKS build failed' }
    python -X utf8 tools/verify_slingshot_landing_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot falling attack HKS verification failed' }
    python -X utf8 tools/build_slingshot_requests_hks.py --inplace
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot request HKS build failed' }
    python -X utf8 tools/verify_slingshot_requests_hks.py
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot request HKS verification failed' }
    python -X utf8 tools/verify_archmage_secondary_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Archmage secondary FP native verification failed' }
    python -X utf8 tools/verify_slingshot_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Slingshot native impulse verification failed.' }
    dotnet run --project tools/HexBuild -- $PSScriptRoot --parry-actions
    if ($LASTEXITCODE -ne 0) { throw 'Parry native guard window generation failed.' }
    Copy-Item -LiteralPath '.work/parry/staged/c0000.anibnd.dcx' -Destination 'dist/mod/chr/c0000.anibnd.dcx' -Force
    Copy-Item -LiteralPath '.work/parry/staged/parry_data.rs' -Destination 'src/parry_data.rs' -Force
    dotnet run --project tools/HexBuild -- $PSScriptRoot --parry-followup-actions
    if ($LASTEXITCODE -ne 0) { throw "Parry follow-up actions failed." }
    python -X utf8 tools/verify_parry_feedback_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Parry native spark instance verification failed.' }
    python -X utf8 tools/verify_parry_turn_native.py
    if ($LASTEXITCODE -ne 0) { throw 'Parry native turn verification failed.' }
    Copy-Item -LiteralPath '.work/parry-fix/turn-native-validation.json' -Destination 'dist/parry-turn-native-validation.json' -Force
    Copy-Item -LiteralPath '.work/parry-fix/spark-native-validation.json' -Destination 'dist/parry-spark-native-validation.json' -Force
    Copy-Item -LiteralPath ".work/parry-fix/staged/c0000.anibnd.dcx" -Destination "dist/mod/chr/c0000.anibnd.dcx" -Force
    dotnet run --project tools/HexBuild -- $PSScriptRoot --night-blessing-actions
    if ($LASTEXITCODE -ne 0) { throw 'Night blessing asset generation failed.' }
    Copy-Item -LiteralPath '.work/night-blessing/staged/c0000.anibnd.dcx' -Destination 'dist/mod/chr/c0000.anibnd.dcx' -Force
    New-Item -ItemType Directory -Force 'dist/mod/script/talk' | Out-Null
    Copy-Item -LiteralPath '.work/night-blessing/staged/m00_00_00_00.talkesdbnd.dcx' -Destination 'dist/mod/script/talk/m00_00_00_00.talkesdbnd.dcx' -Force
    cargo build --release --lib
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    cargo run --release --bin scancheck -- $gameExe
    if ($LASTEXITCODE -ne 0) { throw 'EXE signature check failed.' }
    python -X utf8 tools/verify_devils_gaze_immunity.py
    if ($LASTEXITCODE -ne 0) { throw 'Native bullet contact immunity validation failed.' }
    Copy-Item -LiteralPath '.work/devils-gaze-iframes/native-validation.json' -Destination 'dist/devils-gaze-immunity-validation.json' -Force
    if (-not $NoInstall) {
        New-Item -ItemType Directory -Path "$PSScriptRoot\dist\mod\natives" -Force | Out-Null
        Copy-Item -LiteralPath "$PSScriptRoot\target\release\NRHex.dll" -Destination "$PSScriptRoot\dist\mod\natives\NRHex.dll" -Force
    }
    & $me3 profile show --file "$PSScriptRoot\dist\NRHex.me3"
    if ($LASTEXITCODE -ne 0) { throw 'ME3 profile check failed.' }
    & python -X utf8 tools/record_build.py
    if ($LASTEXITCODE -ne 0) { throw 'Current build record failed.' }
} finally { Pop-Location }
