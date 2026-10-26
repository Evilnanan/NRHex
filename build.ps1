param([switch]$NoInstall, [switch]$DllOnly)
$ErrorActionPreference = 'Stop'
$buildDrive = [IO.Path]::GetPathRoot($PSScriptRoot)
$me3 = if ($env:NRHEX_ME3) { $env:NRHEX_ME3 } else { Join-Path $buildDrive 'Program Files/me3/bin/me3.exe' }
$game = if ($env:NRHEX_GAME) { $env:NRHEX_GAME } else { Join-Path $buildDrive 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game' }
$gameExe = Join-Path $game 'nightreign.exe'
$nativeExe = if ($env:NRHEX_NATIVE_EXE) { $env:NRHEX_NATIVE_EXE } else { $gameExe }
$cargoFeatures = if ($env:NRHEX_CARGO_FEATURES) { @('--features', $env:NRHEX_CARGO_FEATURES) } else { @() }
$cargoTarget = @()
$releaseFolder = Join-Path $PSScriptRoot 'target/release'
if ($env:NRHEX_CARGO_TARGET) {
    $cargoTarget = @('--target', $env:NRHEX_CARGO_TARGET)
    $releaseFolder = Join-Path $PSScriptRoot ('target/' + $env:NRHEX_CARGO_TARGET + '/release')
}
$compiledDll = Join-Path $releaseFolder 'NRHex.dll'
Push-Location $PSScriptRoot
try {
    if (-not $DllOnly) {
        New-Item -ItemType Directory -Path 'dist/reports' -Force | Out-Null
        foreach ($taskFolder in @('.work/arts', '.work/hks', '.work/actions', '.work/effects', '.work/combat', '.work/rewards', '.work/defs', '.work/backstep', '.work/behavior', '.work/icons')) {
            New-Item -ItemType Directory -Path $taskFolder -Force | Out-Null
        }
        & python -X utf8 tools/verify_workspace.py
        if ($LASTEXITCODE -ne 0) { throw 'Workspace inputs validation failed.' }
        dotnet run --project tools/ArtsProbe/ArtsProbe.csproj
        if ($LASTEXITCODE -ne 0) { throw 'Original skill data export failed.' }
        & python -X utf8 tools/build_arts_data.py
        if ($LASTEXITCODE -ne 0) { throw 'Skill pool build failed.' }
        dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot
        if ($LASTEXITCODE -ne 0) { throw 'Parameter/event/text build failed.' }
        python -X utf8 tools/verify_cage_groups.py
        if ($LASTEXITCODE -ne 0) { throw 'Cage group lifecycle/HUD/native reward verification failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --effect-sync-probe
        if ($LASTEXITCODE -ne 0) { throw 'Effect synchronization parameter readback failed.' }
        python -X utf8 tools/verify_effect_sync_native.py --full-build
        if ($LASTEXITCODE -ne 0) { throw 'Native effect synchronization verification failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --red-envelope-rewards-check
        if ($LASTEXITCODE -ne 0) { throw 'Red Envelope native rewards readback failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --essence-reaver-check
        if ($LASTEXITCODE -ne 0) { throw 'Essence Reaver parameter/lifecycle readback failed.' }
        python -X utf8 tools/verify_essence_reaver.py --installed
        if ($LASTEXITCODE -ne 0) { throw 'Essence Reaver native stacking/HUD/reset validation failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --cage-hp-check
        if ($LASTEXITCODE -ne 0) { throw 'Cage local correction parameter readback failed.' }
        & python -X utf8 tools/verify_cage_hp_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native cage multiplayer correction verification failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --contract-heart-probe
        if ($LASTEXITCODE -ne 0) { throw 'Contract and branch native parameter export failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --contract-heart-check
        if ($LASTEXITCODE -ne 0) { throw 'Contract and branch effect/event readback failed.' }
        python -X utf8 tools/verify_contract_heart_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native on-hit self-damage verification failed.' }
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
        & python -X utf8 tools/verify_team_rewards.py
        if ($LASTEXITCODE -ne 0) { throw 'Native team reward dispatch validation failed.' }
        & python -X utf8 tools/verify_reward_recovery.py
        if ($LASTEXITCODE -ne 0) { throw 'Native personal reward recovery validation failed.' }
        & python -X utf8 tools/verify_reward_reroll_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native choice dialog and reroll item-data validation failed.' }
        python -X utf8 tools/verify_reroll_sound.py
        if ($LASTEXITCODE -ne 0) { throw 'Native reroll confirmation sound validation failed.' }
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
        dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --sword-charge-damage-probe
        if ($LASTEXITCODE -ne 0) { throw 'Native sword charged damage fixture export failed.' }
        & python -X utf8 tools/verify_sword_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native sword category/magic binding validation failed.' }
        dotnet run --project tools/HexBuild/HexBuild.csproj -- $PSScriptRoot --sword-regain-probe
        if ($LASTEXITCODE -ne 0) { throw 'Native regain tier fixture export failed.' }
        & python -X utf8 tools/verify_sword_regain_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native sword regain validation failed.' }
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
        dotnet run --project tools/HexBuild -- $PSScriptRoot --storm-audio-probe
        if ($LASTEXITCODE -ne 0) { throw 'Storm Ruler native sound-bank probe failed.' }
        python -X utf8 tools/verify_storm_audio.py
        if ($LASTEXITCODE -ne 0) { throw 'Storm Ruler sound-bank validation failed.' }
        & python -X utf8 tools/verify_arts_behavior.py
        if ($LASTEXITCODE -ne 0) { throw 'Skill behavior source/table validation failed.' }
        & python -X utf8 tools/verify_arts_transfer.py
        if ($LASTEXITCODE -ne 0) { throw 'Skill drop/pickup instance binding validation failed.' }
        & python -X utf8 tools/verify_arts_constraints.py
        if ($LASTEXITCODE -ne 0) { throw 'Skill pool separation/projectile origin validation failed.' }
        & python -X utf8 tools/verify_arts_network_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native equipment overlay broadcast validation failed.' }
        & python -X utf8 tools/verify_noisy.py
        if ($LASTEXITCODE -ne 0) { throw 'Noisy Ghost native route/lava chain validation failed.' }
        if (Test-Path -LiteralPath '.work/cage-groups/manifest.json') {
            & python -X utf8 tools/build_spirit_boss_catalog.py --inplace
            if ($LASTEXITCODE -ne 0) { throw 'Native boss catalogue verification failed.' }
        } # Clean builds retain the checked-in boss catalogue guarded by the EXE/regulation versions.
        & python -X utf8 tools/build_new_hexes_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Balloon and Revenant action notifications failed.' }
        & python -X utf8 tools/build_death_dance_flask_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Death Dance native gradual Flask branch failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --death-dance-flask-check
        if ($LASTEXITCODE -ne 0) { throw 'Death Dance native Flask conversion state check failed.' }
        & python -X utf8 tools/build_chant_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Chant spell charge notifications failed.' }
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
        dotnet run --project tools/HexBuild -- $PSScriptRoot --heart-ready-fog-resources
        if ($LASTEXITCODE -ne 0) { throw 'Heartstealer ready mist resource validation failed.' }
        Copy-Item -LiteralPath 'work/heart-ready/fog-staged/sfxbnd_commoneffects_dlc01.ffxbnd.dcx' -Destination 'dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx' -Force
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
        dotnet run --project tools/HexBuild -- $PSScriptRoot --cat-wall-check
        if ($LASTEXITCODE -ne 0) { throw 'Cat wall-jump native data readback failed.' }
        python -X utf8 tools/verify_cat_wall_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Cat wall-jump behavior and native impulse validation failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --parry-actions
        if ($LASTEXITCODE -ne 0) { throw 'Parry native guard window generation failed.' }
        Copy-Item -LiteralPath '.work/parry/staged/c0000.anibnd.dcx' -Destination 'dist/mod/chr/c0000.anibnd.dcx' -Force
        Copy-Item -LiteralPath '.work/parry/staged/parry_data.rs' -Destination 'src/parry_data.rs' -Force
        dotnet run --project tools/HexBuild -- $PSScriptRoot --parry-paired-probe
        if ($LASTEXITCODE -ne 0) { throw 'Parry paired weapon parameter verification failed.' }
        python -X utf8 tools/verify_parry_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Parry native guard and paired weapon verification failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --parry-followup-actions
        if ($LASTEXITCODE -ne 0) { throw "Parry follow-up actions failed." }
        python -X utf8 tools/verify_parry_feedback_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Parry native spark instance verification failed.' }
        python -X utf8 tools/verify_parry_turn_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Parry native turn verification failed.' }
        Copy-Item -LiteralPath '.work/parry-fix/turn-native-validation.json' -Destination 'dist/reports/parry-turn-native-validation.json' -Force
        Copy-Item -LiteralPath '.work/parry-fix/spark-native-validation.json' -Destination 'dist/reports/parry-spark-native-validation.json' -Force
        Copy-Item -LiteralPath ".work/parry-fix/staged/c0000.anibnd.dcx" -Destination "dist/mod/chr/c0000.anibnd.dcx" -Force
        dotnet run --project tools/HexBuild -- $PSScriptRoot --night-blessing-actions
        if ($LASTEXITCODE -ne 0) { throw 'Night blessing asset generation failed.' }
        Copy-Item -LiteralPath '.work/night-blessing/staged/c0000.anibnd.dcx' -Destination 'dist/mod/chr/c0000.anibnd.dcx' -Force
        New-Item -ItemType Directory -Force 'dist/mod/script/talk' | Out-Null
        Copy-Item -LiteralPath '.work/night-blessing/staged/m00_00_00_00.talkesdbnd.dcx' -Destination 'dist/mod/script/talk/m00_00_00_00.talkesdbnd.dcx' -Force
        dotnet run --project tools/HexBuild -- $PSScriptRoot --ultimate-timing-actions
        if ($LASTEXITCODE -ne 0) { throw 'Ultimate formal-activation action generation failed.' }
        Copy-Item -LiteralPath 'work/ultimate-timing/staged/c0000.anibnd.dcx' -Destination 'dist/mod/chr/c0000.anibnd.dcx' -Force
        dotnet run --project tools/HexBuild -- $PSScriptRoot --regulation-compression
        if ($LASTEXITCODE -ne 0) { throw 'Regulation compression and decompressed content validation failed.' }
        python -X utf8 tools/verify_chant_hks.py
        if ($LASTEXITCODE -ne 0) { throw 'Final script is missing Chant charge notifications.' }
        python -X utf8 tools/verify_angel_chant_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Native Chant stamina addition failed.' }
    }
    if (-not $DllOnly) {
        python -X utf8 tools/build_ambulance_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Ambulance movement script patch failed.' }
    }
    if (-not $DllOnly) {
        python -X utf8 tools/build_wraith_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Wraith item action observers failed.' }
    }
    if (-not $DllOnly) {
        python -X utf8 tools/build_basics_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Back to Basics Ultimate gates failed.' }
    }
    python -X utf8 tools/build_basics_hks.py --check
    if ($LASTEXITCODE -ne 0) { throw 'Back to Basics Ultimate gates missing.' }
    python -X utf8 tools/build_wraith_hks.py --check
    if ($LASTEXITCODE -ne 0) { throw 'Wraith item action observers missing.' }
    if (-not $DllOnly) {
        dotnet run --project tools/HexBuild -- $PSScriptRoot --marking-poison-actions
        if ($LASTEXITCODE -ne 0) { throw 'Marking animation observer patch failed.' }
        python -X utf8 tools/build_marking_hks.py --inplace
        if ($LASTEXITCODE -ne 0) { throw 'Marking follow-up observer patch failed.' }
    }
    python -X utf8 tools/build_marking_hks.py --check
    if ($LASTEXITCODE -ne 0) { throw 'Marking follow-up observer missing.' }
    # Both modes verify the runtime; only Full regenerates game resources.
    dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-poise-check
    if ($LASTEXITCODE -ne 0) { throw 'Sword Endure protection readback failed.' }
    dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-range-catalog
    if ($LASTEXITCODE -ne 0) { throw 'Sword automatic parameter export failed.' }
    python -X utf8 tools/build_sword_auto.py
    if ($LASTEXITCODE -ne 0) { throw 'Sword automatic balance generation failed.' }
    cargo test --release --lib @cargoTarget @cargoFeatures
    if ($LASTEXITCODE -ne 0) { throw 'Runtime regression checks failed.' }
    foreach ($runtimeCheck in @('verify_sword_projectile.py', 'verify_glass_cannon_native.py', 'verify_death_dance.py', 'verify_erosion.py', 'verify_potential_fission.py', 'verify_reward_template.py', 'verify_ambulance.py', 'verify_echo_cast.py', 'verify_weapon_affinity.py', 'verify_snack.py', 'verify_marking.py', 'verify_wraith.py', 'verify_metal.py', 'verify_double_dip.py', 'verify_basics.py', 'verify_sniper.py', 'verify_momentum.py', 'verify_heartstealer.py', 'verify_cat_arena.py', 'verify_ultimate_awakening.py', 'verify_ultimate_unstoppable.py')) {
        python -X utf8 (Join-Path 'tools' $runtimeCheck)
        if ($LASTEXITCODE -ne 0) { throw "Runtime native verification failed: $runtimeCheck" }
    }
    cargo build --release --lib @cargoTarget @cargoFeatures
    if ($LASTEXITCODE -ne 0) { throw 'DLL build failed.' }
    cargo run --release --bin scancheck @cargoTarget @cargoFeatures -- $nativeExe
    if ($LASTEXITCODE -ne 0) { throw 'EXE signature check failed.' }
    if (-not $DllOnly) {
        python -X utf8 tools/verify_mighty_sword_native.py
        if ($LASTEXITCODE -ne 0) { throw 'Mighty/Sword native dispatch check failed.' }
        dotnet run --project tools/HexBuild -- $PSScriptRoot --sword-charged-probe
        if ($LASTEXITCODE -ne 0) { throw 'Sword native on-hit reference export failed.' }
        python -X utf8 tools/verify_sword_charged.py
        if ($LASTEXITCODE -ne 0) { throw 'Sword native weapon-hit rules check failed.' }
        python -X utf8 tools/verify_devils_gaze_immunity.py
        if ($LASTEXITCODE -ne 0) { throw 'Native bullet contact immunity validation failed.' }
        Copy-Item -LiteralPath '.work/devils-gaze-iframes/native-validation.json' -Destination 'dist/reports/devils-gaze-immunity-validation.json' -Force
    }
    if (-not $NoInstall) {
        python tools/native_layout.py (Join-Path $PSScriptRoot 'dist')
        if ($LASTEXITCODE -ne 0) { throw 'Native directory migration failed.' }
        New-Item -ItemType Directory -Path (Join-Path $PSScriptRoot 'dist/natives') -Force | Out-Null
        Copy-Item -LiteralPath $compiledDll -Destination (Join-Path $PSScriptRoot 'dist/natives/NRHex.dll') -Force
    }
    if (-not $DllOnly) {
        & $me3 profile show --file (Join-Path $PSScriptRoot 'dist/NRHex.me3')
        if ($LASTEXITCODE -ne 0) { throw 'ME3 profile check failed.' }
    }
    $recordArgs = @('--dll', $compiledDll)
    if ($DllOnly) { $recordArgs += '--dll-only' }
    if ($NoInstall) { $recordArgs += '--no-install' }
    & python -X utf8 tools/record_build.py @recordArgs
    if ($LASTEXITCODE -ne 0) { throw 'Current build record failed.' }
} finally { Pop-Location }
