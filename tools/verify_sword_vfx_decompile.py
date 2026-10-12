"""Compare independent decompiler outputs, allowing only the hand-aware pulse."""
from pathlib import Path
import argparse,hashlib,json,re,subprocess

ap=argparse.ArgumentParser();ap.add_argument('--decompiler',required=True);ap.add_argument('--before',required=True);args=ap.parse_args()
root=Path(__file__).resolve().parents[1];work=root/'.work/sword-vfx'
report=json.loads((work/'installation-validation.json').read_text());ids=[str(f['id']) for f in report['hks']['functions']]
decompiler=Path(args.decompiler).resolve();before=Path(args.before).resolve();after=root/'dist/mod/action/script/c0000.hks'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(before)==report['hks']['inputSha256'] and sha(after)==report['hks']['outputSha256']
command=['dotnet','--roll-forward','Major',str(decompiler)] if decompiler.suffix=='.dll' else [str(decompiler)]
for source,name in [(before,'before'),(after,'patched')]:
 subprocess.run(command+[str(source),'--included-function-ids',*ids,'-o',str(work/(name+'.lua'))],check=True)
old=(work/'before.lua').read_text(encoding='utf-8-sig');new=(work/'patched.lua').read_text(encoding='utf-8-sig')
pattern=r'        local (f\d+_local\d+) = act\n        local (f\d+_local\d+) = 2002\n        local (f\d+_local\d+) = 95000944\n        if c_Style == HAND_LEFT_BOTH then\n            \3 = 95000952\n        end\n        \1\(\2, \3\)\n'
new,right=re.subn(pattern,'        act(2002, 95000904)\n',new)
new,left=re.subn(r'act\(2002, 95000952\)','act(2002, 95000904)',new)
assert (right,left)==(12,11),(right,left)
# The new argument temporaries cause the decompiler to rename this existing
# local. Require the native declaration before normalizing its references.
renames=re.findall(r'local blend_type, (f\d+_local1) = GetHalfBlendInfo\(\)',new)
assert len(renames)==11
for name in renames:
 assert name not in old
 new=re.sub(r'\b'+name+r'\b','lower_state',new)
assert new==old,'Unexpected independently decompiled semantic change'
report['independentDecompiler']=dict(tool='https://github.com/katalash/DSLuaDecompiler',toolSha256=sha(decompiler),onlyHandAwarePulseChanged=True,rightCallbacks=right,leftCallbacks=left)
(work/'installation-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: independently decompiled all 23 callbacks; only hand-aware pulse changes after local-name normalization')
