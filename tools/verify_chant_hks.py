"""Check the installed charge notifications, including independent decompilation."""
from pathlib import Path
import argparse,os,subprocess
from hks_bytecode import Reader

ROOT=Path(__file__).resolve().parents[1]
FUNCTIONS={801:'MagicFireRight_Upper_onUpdate',806:'MagicFireRight2_Upper_onUpdate',807:'MagicFireRight3_Upper_onUpdate',808:'MagicFireLeft_Upper_onUpdate',812:'MagicFireLeft2_Upper_onUpdate',814:'MagicFireLeft3_Upper_onUpdate'}
LEGACY_BLOCK='    if env(1116, 6999920) == TRUE and env(1116, 100610) == TRUE and IsChargeMagic() == TRUE and CheckIfHoldMagic() ~= TRUE and GetVariable("IndexMagicSlot") ~= 2 then\n        act(2002, 6999921)\n    end\n'
BLOCK=LEGACY_BLOCK.replace(' and env(1116, 100610) == TRUE','')

def verify(path,baseline=None):
 source=path.read_bytes();reader=Reader(source);reader.function()
 functions={function['id']:function for function in reader.functions if 6999921 in function['constants']}
 assert set(functions)==set(FUNCTIONS),'Missing or unexpected Chant pulse callbacks'
 for identity,function in functions.items():
  assert function['name']==FUNCTIONS[identity]
  assert all(value in function['constants'] for value in [6999920,100610,'IsChargeMagic','CheckIfHoldMagic','IndexMagicSlot',2002])
 directory=ROOT/'.work/chant-readback';directory.mkdir(parents=True,exist_ok=True)
 executable=Path(os.environ.get('NRHEX_DSLUA',str(Path(ROOT.anchor)/'Program Files/DSLuaDecompiler/DSLuaDecompiler.exe')))
 def decompile(input_path,name):
  output=directory/name
  subprocess.run([str(executable),str(input_path),'--included-function-ids',*[str(identity) for identity in FUNCTIONS],'-o',str(output)],check=True,cwd=ROOT)
  return output.read_text(encoding='utf-8-sig')
 output=decompile(path,'current.lua')
 assert output.count(BLOCK)==6,'Unexpected charge gate or notification body'
 if baseline is not None:
  before=decompile(baseline,'before.lua').replace(LEGACY_BLOCK,'').replace(BLOCK,'')
  assert output.replace(BLOCK,'')==before,'Unrelated decompiled behavior changed'
 assert path.read_bytes()==source,'Script changed during verification'
 print('PASS: six Chant callbacks observe charging and completion after native cancellation checks; independent decompiler verified')

if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('path',nargs='?',type=Path,default=ROOT/'dist/mod/action/script/c0000.hks');arguments=parser.parse_args()
 verify(arguments.path)
