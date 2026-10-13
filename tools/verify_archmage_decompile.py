"""Independent decompiler readback of Archmage's slot/hand entry prefixes."""
from pathlib import Path
import argparse,json,subprocess
ap=argparse.ArgumentParser();ap.add_argument('--decompiler',required=True);a=ap.parse_args()
root=Path(__file__).resolve().parents[1];work=root/'.work/archmage'
report=json.loads((work/'hks-validation.json').read_text())
cmd=['dotnet','--roll-forward','Major',str(Path(a.decompiler).resolve()),str(work/'c0000.hks'),'--included-function-ids']+[str(f['id']) for f in report['functions']]+['-o',str(work/'hks-patched.lua')]
p=subprocess.run(cmd,capture_output=True,text=True,encoding='utf-8');assert p.returncode==0,p.stdout+p.stderr
text=(work/'hks-patched.lua').read_text(encoding='utf-8-sig')
for record in report['functions']:
 body=text.split('function '+record['name']+'(',1)[1].split('\nfunction ',1)[0]
 assert 'env(1116, 6999936)' in body and 'GetVariable("IndexMagicSlot")' in body,record['name']
 token='6999937' if not record['used'] else ('6999938' if record['left'] else '6999940')
 assert token in body and 'MagicCommonFunction' in body,record['name']
 assert '-- Warning: Function ' not in body,record['name']
report['independentDecompilerVerified']=True
(work/'hks-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('PASS: independent decompiler confirms all 34 owner/slot/hand notifications and native bodies')
