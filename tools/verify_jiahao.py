"""Independent HKS decompile + original critical-animation evidence readback."""
from pathlib import Path
import subprocess,json,struct
root=Path(__file__).resolve().parents[1];report=json.loads((root/'dist/jiahao-hks-validation.json').read_text())
ids=[r['id'] for r in report['functions']];out=root/'.work/actions/jiahao-decompiled.lua'
subprocess.run([r'D:\Program Files\DSLuaDecompiler\DSLuaDecompiler.exe',str(root/'dist/mod/action/script/c0000.hks'),'--included-function-ids',*[str(i) for i in ids],'-o',str(out)],check=True,capture_output=True)
text=out.read_text(encoding='utf8')
for f in report['functions']:
 body=text.split('function '+f['name']+'(',1)[1].split('\nfunction ',1)[0]
 assert 'env(1116, 6999700)' in body,f['name']
 if f['id']==141:
  assert body.count('act(2002, 6999705)')==2
  assert 'request ~= ATTACK_REQUEST_BACKSTEP' in body or 'request == ATTACK_REQUEST_BACKSTEP' in body
  assert 'SetVariable("EnableTAE_BackStep", false)' in body
  assert body.count('SetVariable("EnableTAE_BackStep", true)')==2
  guarded=body.split('SetVariable("EnableTAE_BackStep", false)',1)[1].split('ExecEvent("W_DefaultBackStep")',1)[0]
  assert 'env(1116, 6999700)' in guarded and 'SetVariable("EnableTAE_BackStep", true)' in guarded
  assert 'ExecEvent("W_BackStepGuardOn_UpperLayer")' in body
 else:assert 'act(2002, 699970' in body
# Original unconditional Art-gauge critical marker belongs to locked critical clips.
evidence=json.loads((root/'data/reference/actions/critical-tae.json').read_text())
clips=[r for r in evidence['wordHits'] if any(e['Type']==67 and int.from_bytes(bytes.fromhex(e['hex'])[:4],'little')==7030801 for e in r['events'])]
assert len(clips)>=100 and all(31700<=r['ID']<=31799 for r in clips)
report.update(independentDecompilerReadbackPassed=True,originalCriticalMarker=7030801,originalCriticalClips=len(clips),criticalRequiresKill=False,criticalNativeEffectUnmodified=True,backstepIframesMotionSeconds=11/30,backstepIframesAt30Fps=11,iframeMechanism='Duchess JumpTable 8 copied onto 36 normal backstep clips; private owner state 25013; animation time controls lifetime',backstepHitCategory=129,frontalCriticalOnly=True,frontalCriticalGaugePercent=50,damageGaugePercent=-30)
tae=json.loads((root/'dist/jiahao-tae-validation.json').read_text())
assert report['guardBackstepOwnerTaeRestored'] and report['nonownerGuardBackstepTaeDisablePreserved'] and report['upperGuardLayerPreserved']
assert len(tae['changes'])==36 and tae['sourceDuchessClip']==27060 and abs(tae['motionSeconds']-11/30)<1e-7
assert tae['frames']==11 and tae['referenceFps']==30
assert tae['originalEventsAndAnimationHeadersUnchanged'] and tae['duchessAnimationsUnchanged'] and tae['unrelatedBinderMembersUnchanged']
assert tae['nativeGroupType']==0 and tae['allNewEventsHaveSerializedGroupMembership'] and tae['originalGroupAssociationsUnchanged']
assert all(e['flag']==8 and e['state']==25013 and e['start']==0 and abs(e['end']-11/30)<1e-7 for e in tae['changes'])
rows={r['id']:r['fields'] for r in json.loads((root/'.work/actions/jiahao-serialized.json').read_text())}
assert set(rows)=={95001300,6999700,6999701,6999705}
assert report['removedUnusedActionPulses']==[6999702,6999703,6999704]
assert set(ids)=={141,269,272,275}
from hks_bytecode import Reader
actual=Reader((root/'dist/mod/action/script/c0000.hks').read_bytes());actual.function()
assert not any(value in (6999702,6999703,6999704) for function in actual.functions for value in function['constants'])
assert 95001301 not in rows
assert rows[6999700]['stateInfo']==25013 and rows[6999700]['effectEndurance']==-1
assert rows[6999700]['maxHpRate']==1 and rows[6999700]['vfxId']==-1
assert rows[6999700]['spEffectTextId_1']==-1 and rows[6999700]['saveCategory']==-1
params=json.loads((root/'data/reference/actions/attack-throw.json').read_text())
back=[r for r in params['attacks'] if 129 in [r['fields']['subCategory'+str(i)] for i in range(1,6)]]
assert len(back)==173
names={r['ID']:r['Entries'][0] for r in json.loads(Path(r'D:/Program Files/Smithbox/Assets/PARAM/NR/Param Row Names/English/ThrowParam.json').read_text())['Entries']}
counts={'Backstab':0,'Riposte':0}
for row in params['throws']:
 name=names.get(row['ID'],'');f=row['fields'];front=f['atkAnimId'] in [31700,31750] or f['atkAnimId']==31720 and f['DiffAngMin']>0 and f['DiffAngMax']<0
 for kind in counts:
  if '['+kind+']' in name:counts[kind]+=1;assert front==(kind=='Riposte'),row['ID']
assert counts=={'Backstab':90,'Riposte':113}
# Same current-throw animation getter used by env(273), with typed ThrowParam offsets.
import pefile
pe=pefile.PE(r'D:/Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game/nightreign.exe')
for rva,value in [(0x3f8911,'488b06488b88b8010000488b8988000000e8697f0900'),(0x490890,'488b4910e9272a0000'),(0xea34d0,'488b014885c074048b4024c3'),(0xea3510,'488b014885c074048b4028c3')]:assert pe.get_data(rva,len(bytes.fromhex(value)))==bytes.fromhex(value),hex(rva)
source=(root/'src/lib.rs').read_text()
nullify=source.split('fn on_crystal_hit',1)[1].split('unsafe fn update_crystal',1)[0]
assert 'jiahao' not in nullify
assert 'read_i32(hit+8)==1' not in source
report['removedIncorrectHitKindGate']=True
assert 'jiahao::is_backstep_attack(&ptr::read_unaligned((hit+0x124)' in source
assert 'jiahao::IFRAME' not in source
assert '(jiahao::BACK_START..=jiahao::ROLL)' not in source
assert pe.get_data(0x508e20,4)==bytes.fromhex('440fb7d2') # full unsigned 16-bit state, no enum-array indexing
assert pe.get_data(0x508e82,4)==bytes.fromhex('66413bd2') # exact state equality after generic hash/list lookup
report.update(nativeBackstepTae=tae,nativeBackstepAttackRows=len(back),originalBackstabRowsExcluded=90,originalRiposteRowsAccepted=113,nativeThrowGetterRva='0x490890',oldJiahaoHitNullificationRemoved=True,timedIframeBuffRemoved=True)
(root/'dist/jiahao-hks-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print('PASS: independent action-script readback and original critical pulse in',len(clips),'locked critical animations.')
