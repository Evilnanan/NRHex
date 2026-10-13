"""Independently decompile and remove only the accepted-request/input prefixes."""
from pathlib import Path
import json,subprocess,re,os
root=Path(__file__).resolve().parents[1];work=root/'.work/slingshot-requests'
report=json.loads((work/'hks-validation.json').read_text())
decompiler=Path(os.environ.get('NRHEX_DSLUA',str(root/'.work/tools/DSLuaDecompiler/DSLuaDecompiler/bin/Release/net9.0/DSLuaDecompiler.exe')))
for source,name in [('hks-before.hks','before.lua'),('c0000.hks','after.lua')]:
 subprocess.run([str(decompiler),str(work/source),'--included-function-ids','9','1255','-o',str(work/name)],check=True)
before=(work/'before.lua').read_text(encoding='utf-8-sig');after=(work/'after.lua').read_text(encoding='utf-8-sig')
request_prefix='''    if event == "W_SprintStart" and env(1116, 6999960) == TRUE then
        if 0 < env(1108, ACTION_ARM_L3) then
            act(2002, 6999965)
        else
            act(2002, 6999966)
        end
        act(2002, 6999967)
    end
'''
update_prefix='''    if env(1116, 6999960) == TRUE then
        if env(1108, ACTION_ARM_L3) > 0 then
            act(2002, 6999965)
        else
            act(2002, 6999966)
        end
    end
'''
assert after.count(request_prefix)==1 and after.count(update_prefix)==1
assert after.replace(request_prefix,'').replace(update_prefix,'')==before
report['independentDecompilerReadbackPassed']=True
(work/'hks-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: independent decompiler confirms only two notifications; original sprint/stamina/input calls unchanged')
