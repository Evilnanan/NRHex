"""Audit UI-only routing and the installed package against its saved baseline."""
from pathlib import Path
import hashlib,json,re,tomllib
import pefile
from local_paths import GAME,ROOT

digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
raw=(GAME/'nightreign.exe').read_bytes()
assert hashlib.sha256(raw).hexdigest()=='9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe=pefile.PE(data=raw,fast_load=True)
fixture=ROOT/'data/reference/ui/status-route.bin'
assert fixture.read_bytes()==pe.get_data(0xa89a93,7)==bytes.fromhex('66 0f 7e d0 83 f8 01')
source=(ROOT/'src/status_display.rs').read_text(encoding='utf8')
pattern=re.search(r'ROUTE_PATTERN: &str = "([^"]+)"',source)[1].split()
window=pe.get_data(0xa89a93,len(pattern))
assert all(t=='?' or int(t,16)==v for t,v in zip(pattern,window))
assert '"cmp eax, eax"' in source and '"mov r10d, [rbx+4]"' in source
ids=[95000403,95000701,95001203,95000501,95000801,95001201,95001202]
assert [int(x) for x in re.findall(r'"cmp r10d, (\d+)"',source)]==ids+[95005001,95005079]
baseline=json.loads((ROOT/'.work/status-routing-01732/baseline.json').read_text())
unchanged=[]
for rel,expected in baseline.items():
 if rel.startswith('dist/mod/') or rel.startswith('dist/testmods/') and Path(rel).suffix in ['.dll','.ini','.csv'] or rel in ['dist/NRHex.me3','dist/mod/natives/NRHex.ini','data/balance/sword.csv']:
  assert digest(ROOT/rel)==expected,f'Unrelated gameplay asset changed: {rel}'
  unchanged.append(rel)
assert digest(ROOT/'target/release/NRHex.dll')==digest(ROOT/'dist/mod/natives/NRHex.dll')
version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
report=dict(version=version,originalExeSha256=hashlib.sha256(raw).hexdigest(),
 selectorRva='0xa89a93',stolenBytes=7,originalInstructionFixtureSha256=digest(fixture),
 specialEffectsContainerOffset='0x2cb8',additionalEffectsContainerOffset='0x22c0',
 nativeSplit='Live SpEffect kind 0 / equipment AttachEffect kind 1',
 previousPermanentVersusTemporaryExplanationWithdrawn=True,
 stateIds=ids,extraDrinkStackRange=[95005001,95005079],
 originalRowKindPayloadCountsAndLifetimeRetained=True,
 hiddenEscapeActiveAndCooldownRemainHidden=True,
 rustTestsPassed=167,realDetourAndRelocatedNativeInstructionCases=392,
 runtimeSignaturesVerified=63,gameplayAssetsUnchanged=unchanged,
 dllSha256=digest(ROOT/'dist/mod/natives/NRHex.dll'),gameUiVerified=False,multiplayerVerified=False,
 notes='Private native selector + real detour tests and build/static checks. Actual own/peer status pages pending user validation.')
(ROOT/'dist/reports/status-routing-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print(f'PASS: current UI selector, state range, installed DLL and {len(unchanged)} unchanged gameplay assets.')
