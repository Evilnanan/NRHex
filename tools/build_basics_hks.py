"""Seal both Ultimate input routes while retaining every native instruction."""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,decode
ROOT=Path(__file__).resolve().parents[1]
ULTIMATE=990045;READY=990046
def patch(source):
 r=Reader(source);r.function();edits=[];audit={}
 for fid in [105,107]:
  f=r.functions[fid];cs=f['constants'];assert ULTIMATE not in cs and READY not in cs
  words=[v[0]for v in struct.iter_unpack('>I',f['code'])];extra=[];changes=[]
  def constant(v):
   if v not in extra:extra.append(v)
   return len(cs)+extra.index(v)
  for i,w in enumerate(words):
   d=decode(w)
   if d['op']==91 and d['bx']<len(cs)and cs[d['bx']]=='ACTION_ARM_ULTRA':
    j=i-1;k=decode(words[j]);assert k['op']==25 and cs[k['bx']]==1106
    words[j]=(words[j]&~(0x1ffff<<8))|(constant(ULTIMATE)<<8);changes.append((j,1106,ULTIMATE))
   if d['op']==25 and cs[d['bx']]==370:
    words[i]=(w&~(0x1ffff<<8))|(constant(READY)<<8);changes.append((i,370,READY))
  assert sum(c[2]==ULTIMATE for c in changes)==1
  if fid==105:assert sum(c[2]==READY for c in changes)==2
  encoded=b''.join(b'\x03'+struct.pack('>f',v)for v in extra)
  while len(encoded)%4:encoded+=b'\0';extra.append(None)
  edits.extend([(f['code_offset'],len(f['code']),b''.join(struct.pack('>I',w)for w in words)),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,encoded)])
  audit[fid]={'name':f['name'],'changes':changes}
 target=source
 for p,n,b in sorted(edits,reverse=True):target=target[:p]+b+target[p+n:]
 a=Reader(target);a.function()
 for f,g in zip(r.functions,a.functions):
  assert f['count']==g['count'] and f['slots']==g['slots'] and f['lines']==g['lines']
  if f['id']in audit:
   before=[v[0]for v in struct.iter_unpack('>I',f['code'])];after=[v[0]for v in struct.iter_unpack('>I',g['code'])]
   changed={x[0]for x in audit[f['id']]['changes']};assert all(x==y for i,(x,y)in enumerate(zip(before,after))if i not in changed)
   assert g['constants'][:len(f['constants'])]==f['constants']
  else:assert g['code']==f['code'] and g['constants']==f['constants']
 inverse=[];shift=0
 for p,n,b in sorted(edits):inverse.append((p+shift,len(b),source[p:p+n]));shift+=len(b)-n
 restored=target
 for p,n,b in reversed(inverse):restored=restored[:p]+b+restored[p+n:]
 assert restored==source
 return target,audit
def verify(data):
 r=Reader(data);r.function()
 for fid in [105,107]:
  f=r.functions[fid];cs=f['constants'];ins=[decode(w[0])for w in struct.iter_unpack('>I',f['code'])]
  assert cs.count(float(ULTIMATE))==1
  calls=0
  for i,d in enumerate(ins):
   if d['op']==91 and d['bx']<len(cs)and cs[d['bx']]=='ACTION_ARM_ULTRA':
    before=ins[i-1];assert before['op']==25 and cs[before['bx']]==ULTIMATE;calls+=1
  assert calls==1
  ready=sum(d['op']==25 and cs[d['bx']]==READY for d in ins)
  assert ready==(2 if fid==105 else 0)
  assert not any(d['op']==25 and cs[d['bx']]==370 for d in ins)
 return True
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--inplace',action='store_true');a=p.parse_args();path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if a.check:verify(source);print('PASS: Back to Basics ground/air and two-button Ultimate gates');raise SystemExit
 try:verify(source);print('PASS: Back to Basics HKS gates already present');raise SystemExit
 except AssertionError:pass
 target,audit=patch(source);out=ROOT/'work/basics/hks';out.mkdir(parents=True,exist_ok=True);(out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),functions=audit,instructionCountsAndBranchesUnchanged=True,exactInversePassed=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: Back to Basics HKS gates; original branches, instructions and debug positions retained')
