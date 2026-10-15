"""Independent readback of the six falling-attack notifications; never rebuild decompiled Lua."""
from pathlib import Path
import json,subprocess,re,os
root=Path(__file__).resolve().parents[1];work=root/'.work/slingshot-landing'
report=json.loads((work/'hks-validation.json').read_text())
decompiler=Path(os.environ.get('NRHEX_DSLUA',r'D:/Program Files/DSLuaDecompiler/DSLuaDecompiler.exe'))
for source,name in [('hks-before.hks','before.lua'),('c0000.hks','after.lua')]:
 subprocess.run([str(decompiler),str(work/source),'--included-function-ids',*[str(f['id']) for f in report['functions']],'-o',str(work/name)],check=True)
before=(work/'before.lua').read_text(encoding='utf-8-sig');after=(work/'after.lua').read_text(encoding='utf-8-sig')
clean,count=re.subn(r'    if env\(1116, 6999960\) == TRUE then\n        act\(2002, (6999963)\)\n    end\n','',after)
assert count==6 and clean==before,'Changes beyond the owner-gated notifications'
report['independentDecompilerReadbackPassed']=True
(work/'hks-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: independent decompiler finds only six owner-gated falling-attack notifications')
