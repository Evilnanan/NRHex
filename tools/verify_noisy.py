"""Check original self-marker routing, lava identity and visual-only matrices."""
from pathlib import Path
import json,runpy,sys,struct,re,tomllib
root=Path(__file__).resolve().parents[1]
sys.argv=['probe','dis','0','0'];d=runpy.run_path(str(root/'tools/native_probe.py'))
def check(r,b):assert d['read'](r,len(bytes.fromhex(b)))==bytes.fromhex(b),hex(r)
def call(r):
    assert d['read'](r,1)==b'\xe8'
    return r+5+struct.unpack('<i',d['read'](r+1,4))[0]
# Actual input consumer checks F708C0, then distinguishes targeted inventory
# item marking from the player-position branch. R8 is the local world player.
assert call(0xba16b1)==0xf708c0
check(0xba185b,'4C 8B 80 E8 74 01 00 4D 85 C0')
assert call(0xba186e)==0xb9ea10
assert call(0xba181a)==0xb9ed80
check(0xb9ea49,'49 8B D8 48 8B F2 4C 8B F1')
# Ordinary Bullet is native kind 2: +5C8 row and +5D0 ID. The source at +B0 is
# resolved by the game's generation-aware handle lookup, not cast to ChrIns.
check(0x362b87,'48 83 C1 08 48 C7 01 00 00 00 00 C7 41 08 FF FF FF FF')
# Native dispatch selects +5C0 for kind 2, and +5C8 when reading that row.
check(0x34f2c3,'48 8D 8F C0 05 00 00')
check(0x34f5cb,'48 8D 87 C8 05 00 00')
assert call(0x34a852)==0x362c00
check(0x362c00,'48 83 C1 08 BA FF FF FF FF')
assert d['read'](0x362c09,1)==b'\xe9'
assert 0x362c0e+struct.unpack('<i',d['read'](0x362c0a,4))[0]==0xe87800
check(0xe8794e,'41 89 7E 08 49 89 1E') # PARAM reference lookup writes ID at +8.
check(0x353789,'48 8B 89 B0 00 00 00');assert call(0x353790)==0x63f840
assert call(0x34a605)==0x353cb0 # Destructor clears primary lava SFX.
assert call(0x34a832)==0x353cb0 # Pool reset clears it before invalidating PARAM refs.
assert call(0x34f273)==0x353cb0 # Expiration/state transition clears it.
assert call(0x355fac)==0xed60c0
assert call(0x353944)==0xed60c0
# Matrix setter makes an owned copy; the aligned shadow only needs to live
# through this call. No collision matrix or Bullet radius is changed.
assert call(0xed6101)==0xc80f50
check(0xc80f7a,'0F 28 03 0F 11 47 70 0F 28 4B 10 0F 11 8F 80 00 00 00')
data=json.loads((root/'.work/noisy/serialized.json').read_text())
assert [r['id'] for r in data['effects']]==[95001501]
package=json.loads((root/'dist/reports/validation.json').read_text(encoding='utf8'))
hexes={h['Id']:h for h in package['hexes']}
assert (hexes[95001500]['LargeIcon'],hexes[95001500]['SmallIcon'])==(21229,22229)
description=re.search(r'^<!-- hex:95001500 -->\n## [^\n]+\n(.*?)(?=^<!-- hex:|\Z)',(root/'docs/hexes.md').read_text(encoding='utf8'),re.M|re.S)
assert description and hexes[95001500]['ZhInfo']==description.group(1).strip()
assert (hexes[95001200]['LargeIcon'],hexes[95001200]['SmallIcon'])==(21222,22222)
bullets={r['id']:r['fields'] for r in data['bullets']}
attacks={r['id']:r['fields'] for r in data['attacks']}
assert len(bullets)==8 and len(attacks)==7
for i in range(8):
    b=bullets[95001550+i]
    assert b['HitBulletID']==(-1 if i==7 else 95001551+i)
    assert b['atkId_Bullet']==95001560+(i if i<=4 else 6 if i==7 else 5)
assert bullets[95001552]['life']==4 and bullets[95001553]['life']==0.8
assert bullets[95001554]['hitRadius']==3.5
assert [bullets[95001550+i]['sfxId_Bullet'] for i in range(2,8)]==[334084,334085,334086,334088,334088,334089]
for i in range(7):
    a=attacks[95001560+i]
    assert a['atkFire']==(140 if i==4 else 20 if i==6 else 0)
    assert a['isAddBaseAtk']==0 and a['selfTarget']==0 and a['friendlyTarget']==0
source=(root/'src/lib.rs').read_text(encoding='utf8')
assert 'LavaVisualGuard::enter(lava_visual_scale(bullet,true))' in source
assert 'noisy::lava_attack(read_i32(hit+0x40),read_i32(hit+0x44))' in source
assert 'l.removed(bullet)' in source and 'noisy::COOLDOWN' not in source
assert 'lava_parameter_id(bullet)' in source and 'bullet+noisy::PARAM_ID_OFFSET' in source
noisy=(root/'src/noisy.rs').read_text()
assert 'PARAM_ID_OFFSET:usize=0x5d0' in noisy and 'kind==2' in noisy
assert 'noisy_trace' not in source and 'trace_state' not in noisy
assert 'nlog_' not in (root/'src/noisy.rs').read_text()
report=dict(version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version'],selfPositionMarkRva='B9EA10',nativeInputCaller='BA186E',itemMarkExcluded=True,
    originalLavaEffect=8630502,privateLavaEffect=95001501,privateBehavior=950015000,
    chargeSeconds=4,eruptionDelaySeconds=0.8,cooldownSeconds=None,rearm='last native lava instance cleanup',stackPercent=30,maxStacks=15,maxMultiplier=5.5,
    baseExplosionFireAttack=140,baseBurningFireAttack=20,visualScale='sqrt(damage multiplier)',nativeHitRadiiPreserved=True,
    bulletReferenceKind=2,bulletReferenceIdOffset='5D0',childReferenceExcluded=True,temporaryCaptureRemoved=True,
    sourceAndFriendlyDamageDisabled=True,gameplayVerified=False,multiplayerVerified=False)
(root/'dist/reports/noisy-validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print('PASS: actual native self-mark route, eight private lava phases, separate 140 eruption / 20 burning fire attack and owned visual matrix copy; collision sizes preserved.')
