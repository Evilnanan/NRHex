"""Independently decompile the exact staged callback patch."""
import json,os,re,subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[1]
report = json.loads((root/'.work/fixes-01727/arts-stance-validation.json').read_text())
decompiler = os.environ.get('NRHEX_DSLUA')
if not decompiler:
    drive = Path(root.anchor)
    decompiler = next((str(p) for p in [drive/'Downloads/DSLuaDecompiler/DSLuaDecompiler.exe',
                                       drive/'Program Files/DSLuaDecompiler/DSLuaDecompiler.exe'] if p.exists()),None)
assert decompiler, 'Set NRHEX_DSLUA to the installed DSLuaDecompiler executable'
output = root/'.work/fixes-01727/arts-stance.lua'
subprocess.run([decompiler,str(root/'.work/fixes-01727/staged/mod/action/script/c0000.hks'),
                '--included-function-ids',*[str(f['id']) for f in report['functions']],'-o',str(output)],check=True,capture_output=True)
text = output.read_text(encoding='utf-8')
for f in report['functions']:
    body = text.split('function '+f['name']+'(',1)[1].split('\nfunction ',1)[0]
    assert 'Warning:' not in body and '6999804' in body
    prefix = body.split('act(2002, 6999801)',1)[0]
    # The decompiler sometimes retains reused scratch-register assignments.
    def call(name, arguments):
        if name+'('+arguments in prefix:return True
        return any(alias+'('+arguments in prefix for alias in re.findall(r'(\w+) = '+name+r'\n',prefix))
    assert call('act','2002, 6999805)')
    assert call('ExecEventHalfBlend',f['endEvent']+',')
    assert 'return' in body and 'act(2002, 6999801)' in body
    if f['id'] in [536,537,538,539]:
        for kind in [10,11,25,239]:assert f'~= {kind}' in body
report['independentDecompilePassed'] = True
(root/'.work/fixes-01727/arts-stance-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: independent decompile of all six stance callback patches.')
