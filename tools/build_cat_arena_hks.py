"""Upgrade only Cat wall-query call sites in an already composed HKS.

No instruction counts, branches, registers, lines or local ranges change.
Existing native effect queries in those same functions remain untouched.
"""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,instructions
ROOT=Path(__file__).resolve().parents[1];QUERY=990008;OWNER=95000800
TARGETS={1508:('JumpCommonFunction',2),1514:('ExecWallJump',1),1579:('WallJumpCancelEarly_onUpdate',1),1581:('WallJumpCancelLate_onUpdate',1)}
def patch(source):
 reader=Reader(source);reader.function();edits=[];reports=[];changed={}
 for fid,(name,count) in TARGETS.items():
  f=reader.functions[fid];assert f['name']==name
  words=instructions(f);sites=[]
  for i,x in enumerate(words[:-2]):
   y,z=words[i+1:i+3]
   if x['op']==25 and f['constants'][x['bx']]==1116 and y['op']==25 and f['constants'][y['bx']]==OWNER and z['op']==2:
    assert y['a']==x['a']+1 and z['a']==x['a']-1 and z['b']==3 and z['c']==2
    sites.append(i)
  assert len(sites)==count,(name,sites)
  assert QUERY not in f['constants']
  index=len(f['constants']);encoded=b'\x03'+struct.pack('>f',QUERY)+b'\0'*3
  # A float constant plus three nil constants adds exactly eight bytes,
  # preserving the next prototype's existing 4-byte alignment and caches.
  edits.extend([(f['constants_count_offset'],4,struct.pack('>I',index+4)),(f['constants_end'],0,encoded)])
  for i in sites:
   word=struct.unpack_from('>I',f['code'],4*i)[0];new=(word&~(0x1ffff<<8))|(index<<8)
   edits.append((f['code_offset']+4*i,4,struct.pack('>I',new)))
  changed[fid]=sites;reports.append(dict(id=fid,name=name,querySites=sites))
 patched=source
 for offset,length,value in sorted(edits,reverse=True):patched=patched[:offset]+value+patched[offset+length:]
 inverse=[];shift=0
 for offset,length,value in sorted(edits):inverse.append((offset+shift,len(value),source[offset:offset+length]));shift+=len(value)-length
 restored=patched
 for offset,length,value in sorted(inverse,reverse=True):restored=restored[:offset]+value+restored[offset+length:]
 assert restored==source
 after=Reader(patched);after.function();assert source[reader.p:]==patched[after.p:]
 for a,b in zip(reader.functions,after.functions):
  if a['id'] not in changed:
   if a['id']!=0:assert source[a['start']:a['end']]==patched[b['start']:b['end']]
   else:assert a['code']==b['code'] and a['constants']==b['constants']
   continue
  assert a['count']==b['count'] and a['lines']==b['lines'] and a['slots']==b['slots']
  assert b['constants']==a['constants']+[float(QUERY),None,None,None]
  for i,(x,y) in enumerate(zip(instructions(a),instructions(b))):
   if i in changed[a['id']]:assert x['op']==y['op']==25 and x['a']==y['a'] and b['constants'][y['bx']]==QUERY
   else:assert x==y
 return patched,reports

def check(source):
 reader=Reader(source);reader.function();total=0
 for fid,(name,count) in TARGETS.items():
  f=reader.functions[fid];assert f['name']==name;words=instructions(f);hits=[]
  for i,x in enumerate(words[:-2]):
   y,z=words[i+1:i+3]
   if x['op']==25 and f['constants'][x['bx']]in [QUERY,1116] and y['op']==25 and f['constants'][y['bx']]==OWNER and z['op']==2:
    assert f['constants'][x['bx']]==QUERY,'Ungated Cat wall query remains'
    assert y['a']==x['a']+1 and z['a']==x['a']-1 and (z['b'],z['c'])==(3,2);hits.append(i)
  assert len(hits)==count,(name,hits);total+=len(hits)
 assert sum(QUERY in f['constants']for f in reader.functions)==4
 return total
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--inplace',action='store_true');parser.add_argument('--check',action='store_true');args=parser.parse_args()
 path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if args.check:print('PASS:',check(source),'Cat wall queries use live arena eligibility');raise SystemExit
 patched,reports=patch(source);assert check(patched)==5
 work=ROOT/'work/cat-arena';work.mkdir(exist_ok=True,parents=True);(work/'before.hks').write_bytes(source);(work/'c0000.hks').write_bytes(patched)
 (work/'hks-validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(patched).hexdigest(),functions=reports,inverseRestoresInput=True,allOtherInstructionsAndFunctionsUnchanged=True,gameplayVerified=False),indent=2)+'\n')
 if args.inplace:path.write_bytes(patched)
 print('PASS: five Cat wall queries upgraded; exact inverse and unchanged branches/registers verified')
