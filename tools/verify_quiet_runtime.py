"""Offline scope audit: no diagnostics in successful gameplay hot paths."""
from pathlib import Path
import json,re
root=Path(__file__).resolve().parents[1]
source=(root/'src/lib.rs').read_text(encoding='utf8')
# Unit fixtures may occur between runtime functions; the first cfg(test) is
# not the end of runtime code. Inspect the complete post-initialization tail.
hot=source[source.index('unsafe extern "system" fn on_rear_calculated'):]
assert 'nlog_info!' not in hot and 'nlog_debug!' not in hot and 'nlog_trace!' not in hot
assert not any(x in source for x in ['_DIAGNOSTICS','SWORD_OWNER_STATE','95000896','customNodes','WALL_EARLY_ACCEPTED','WALL_CANCEL_VISITS','GetCurrentThreadId','REWARD_PROBE','ReadProcessMemory','reward-probe-'])
assert 'cat_probe' not in source and 'sword_probe' not in source
assert not (root/'src/cat_probe.rs').exists() and not (root/'src/sword_probe.rs').exists()
logger=(root/'src/log.rs').read_text(encoding='utf8')
assert 'OutputDebugStringW' not in logger
assert 'f.probe()' in hot and 'r.note_refill_attempt(after.gauge>=99.9)' in hot
assert 'SWORD_LAST_CAST.store(GetTickCount64()' in hot and 'sword::CAST_PULSE' in hot
assert 'h.queue(source,caster_stats,amount)' in hot and 'shell.absorb(player,stats,s,eligible)' in hot
assert 'sword::heal_amount(read_i32(hit+0x280),spell_percentage(hit).unwrap_or(0.),ptr::read((hit+0x2c3) as *const u8)&2!=0)' in hot
hks=json.loads((root/'dist/hks-validation.json').read_text(encoding='utf8'))
assert hks['removedWallDiagnosticPulses']==[95000808,95000816,95000824,95000832]
assert hks['independentDecompilerReadbackPassed'] and hks['inversePatchReconstructsOriginal']
assert hks['earlyRetryOwnerGroundAcceptedPathsVerified'] and hks['ownerGroundRequestAcceptedPathsVerified']
script=(root/'tools/build_jiahao.py').read_text(encoding='utf8')
assert 'markers={269:6999701,272:6999701,275:6999701}' in script
assert 'BACK_END' not in (root/'src/jiahao.rs').read_text() and 'ATTACK_START' not in (root/'src/jiahao.rs').read_text()
arts=(root/'src/arts.rs').read_text(encoding='utf8')
assert not any(token in arts for token in ['nlog_', 'println!', 'std::fs', 'DIAGNOSTICS'])
assert 'arts_source_weapon(player,source,base)' in source and 's.tick()' in source
(root/'dist/quiet-runtime-validation.json').write_text(json.dumps({'version':'0.13.0','gameplayInfoLoggingAbsent':True,'diagnosticCountersAndOwnershipTraversalRemoved':True,'debuggerMirroringRemoved':True,'removedWallDiagnosticPulses':hks['removedWallDiagnosticPulses'],'functionalFlaskCapacityAndUltimateConfirmationAndCastingHeartbeatPresent':True,'wallOwnerGroundAcceptedBranchesVerified':True,'performanceMeasured':False},indent=2),encoding='utf8')
print('PASS: gameplay diagnostics removed; native function triggers and wall retry control flow preserved.')
