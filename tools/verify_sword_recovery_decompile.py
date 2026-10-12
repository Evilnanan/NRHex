"""Independent decompiler check of new recovery guards; native body is byte-exact."""
from hks_bytecode import Reader
from pathlib import Path
import argparse,json,re,subprocess
ap=argparse.ArgumentParser();ap.add_argument('--decompiler',required=True);args=ap.parse_args()
root=Path(__file__).resolve().parents[1];work=root/'.work/sword-vfx'
report=json.loads((work/'hks-validation.json').read_text());records=report['recovery']['functions']
assert len(records)==8,'Run against a newly staged recovery patch'
tool=Path(args.decompiler).resolve();output=work/'recovery-patched.lua'
subprocess.run(['dotnet','--roll-forward','Major',str(tool),str(work/'c0000.hks'),'--included-function-ids',*[str(f['id']) for f in records],'-o',str(output)],check=True)
text=output.read_text(encoding='utf-8-sig')
old=Reader((work/'recovery-before.hks').read_bytes());old.function()
new=Reader((work/'c0000.hks').read_bytes());new.function()
for f in records:
 before=old.functions[f['id']];after=new.functions[f['id']]
 assert after['code'][f['prefixInstructions']*4:]==before['code']
 body=text.split('function '+f['name']+'(',1)[1].split('\nfunction ',1)[0]
 assert 'if env(1116, 95000912) == TRUE and GetVariable("IndexMagicSlot") ~= 2 then' in body
 if 'MagicFireLeft' in f['name']:
  assert 'act(2002, 95000968)' in body
 else:
  assert re.search(r'local (f\d+_local\d+) = 95000960\n        if c_Style == HAND_LEFT_BOTH then\n            \1 = 95000968\n        end',body)
 assert 'MagicCommonFunction' in body and 'HalfBlendUpperCommonFunction' in body
report['recovery']['independentDecompilerGuardsVerified']=True
(work/'hks-validation.json').write_text(json.dumps(report,indent=2))
print('PASS: independent decompiler confirms all 8 ownership/skill/hand guards; native bodies byte-exact')
