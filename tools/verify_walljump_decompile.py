"""Independent decompiler readback; generated source is never compiled or loaded."""
from pathlib import Path
import subprocess,json
root=Path(__file__).resolve().parents[1]
exe=Path(r'D:\Program Files\DSLuaDecompiler\DSLuaDecompiler.exe')
old=root/'.work/hks/walljump-original.lua'
new=root/'.work/hks/walljump-patched.lua'
report=json.loads((root/'dist/hks-validation.json').read_text(encoding='utf-8'))
ids=sorted({1255,*[f['functionId'] for f in report['functions']]})
for source,dest in [(root/'assets/original/action/script/c0000.hks',old),(root/'dist/mod/action/script/c0000.hks',new)]:
 subprocess.run([str(exe),str(source),'--included-function-ids',*[str(i) for i in ids],'-o',str(dest)],check=True,cwd=root)
a=old.read_text(encoding='utf-8-sig');b=new.read_text(encoding='utf-8-sig')
prefix='    if env(1116, 95000800) == TRUE then\n        ISENABLE_WALLJUMP = TRUE\n    end\n'
assert b.count(prefix)==2, 'JumpCommonFunction and ExecWallJump'
for _ in range(2):
 b=b.replace(prefix,'',1)
prefix='    if env(1116, 95000800) == TRUE and env(201) ~= TRUE and ExecWallJump() == TRUE then\n        return\n    end\n'
assert b.count(prefix)==2, 'Early and late cancel retry'
for _ in range(2):
 b=b.replace(prefix,'',1)
early='''    if env(1116, 95000800) == TRUE and env(201) ~= TRUE and ExecWallJump() == TRUE then
        return TRUE
    end
'''
assert b.count(early)==1, 'Early retry must follow Act_Jump and precede the phase gate'
assert ('    if Act_Jump() == TRUE then\n        return TRUE\n    end\n'+early+'    if env(1116, 145)') in b
b=b.replace(early,'',1)
cast='''    if env(1116, 95000912) == TRUE and GetVariable("IndexMagicSlot") ~= 2 and env(1116, 95000904) ~= TRUE then
        act(2002, 95000904)
    end
'''
assert b.count(cast)==sum('castOwnerGuardPathsVerified' in f for f in report['functions'])
b=b.replace(cast,'')
assert b==a, 'Decompiler found a change outside the wall-request guards'
assert all(str(pulse) not in new.read_text(encoding='utf-8-sig') for pulse in [95000808,95000816,95000824,95000832]), 'Diagnostic pulses remain'
path=root/'dist/hks-validation.json';report=json.loads(path.read_text(encoding='utf-8'))
report['independentDecompilerReadbackPassed']=True
report['decompiledSourceDiff']='Only Cat wall guards plus Sword casting prefixes; no weapon-skill/cancel/recovery protection. All remaining decompiled source identical.'
path.write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print('PASS: independent decompiler finds exactly the owner guard; all remaining decompiled source identical.')
