"""Bind Onyx source hand and finish the bounded channel; retain native action guards."""
import argparse,hashlib,json,struct
from pathlib import Path
from hks_bytecode import Reader,decode
ROOT=Path(__file__).resolve().parents[1]
QUERY=990053
FP_QUERY=990054
START_END_QUERY=990056

def early_transition(source):
 """Replace only native animation-end queries in the bound startup updates."""
 r=Reader(source);r.function();edits=[];changed={}
 for f in r.functions:
  if QUERY not in f['constants'] or f['name'] not in ['SwordArtsOneShot_onUpdate','SwordArtsHalfOneShot_Upper_onUpdate']:continue
  cs=f['constants'];words=[w[0]for w in struct.iter_unpack('>I',f['code'])]
  sites=[i for i,w in enumerate(words)if decode(w)['op']==25 and cs[decode(w)['bx']] in (339,990055)]
  if not sites:continue
  k=len(cs);encoded=bytearray(b'\x03'+struct.pack('>f',START_END_QUERY));extra=1
  while len(encoded)%4:encoded.append(0);extra+=1
  for i in sites:words[i]=abx(25,decode(words[i])['a'],k)
  edits.extend([(f['code_offset'],len(f['code']),b''.join(struct.pack('>I',w)for w in words)),(f['constants_count_offset'],4,struct.pack('>I',k+extra)),(f['constants_end'],0,bytes(encoded))])
  changed[f['id']]=dict(name=f['name'],earlyTransitionQueries=sites)
 patched=source
 for off,size,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+size:]
 a=Reader(patched);a.function()
 for old,new in zip(r.functions,a.functions):
  code=bytearray(new['code'])
  for i in changed.get(old['id'],{}).get('earlyTransitionQueries',[]):code[i*4:i*4+4]=old['code'][i*4:i*4+4]
  assert code==old['code'] and new['constants'][:len(old['constants'])]==old['constants']
 return patched,changed

def abx(op,a,bx):return op<<25|bx<<8|a

def abc(op,a,b,c):return op<<25|b<<17|c<<8|a

def upgrade(source):
 """Upgrade only the verified legacy nine-instruction observer prefix."""
 r=Reader(source);r.function();edits=[];changed={}
 for f in r.functions:
  if QUERY not in f['constants']:continue
  words=[w[0]for w in struct.iter_unpack('>I',f['code'])];cs=f['constants'][:]
  offset=next(i-2 for i,w in enumerate(words)if decode(w)['op']==25 and cs[decode(w)['bx']]==QUERY)
  if offset:
   scratch=decode(words[offset])['a'];assert words[offset+12]==abc(2,scratch,6,2);continue
  scratch=decode(words[0])['a']
  assert decode(words[0])['op']==91 and cs[decode(words[0])['bx']]=='env'
  assert cs[decode(words[2])['bx']]==QUERY
  if words[12]==abc(2,scratch,6,2):continue
  assert words[8]==abc(2,scratch,5,2) and f['slots']==scratch+5
  extra=[];encoded=bytearray()
  def constant(value):
   if value in cs:return cs.index(value)
   i=len(cs)+len(extra);extra.extend([value,None,None]);b=value.encode()+b'\0'
   encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0');return i
  key=constant('GetVariable');name=constant('IsEnoughArtPointsL2')
  added=[abx(91,scratch+5,key),abx(76,20,key+1),abx(25,scratch+6,name),abc(2,scratch+5,2,2)]
  new=words[:8]+added+[abc(2,scratch,6,2)]+words[9:]
  assert new[:8]==words[:8] and new[13:]==words[9:]
  while len(encoded)%4:encoded.append(0);extra.append(None)
  code=b''.join(struct.pack('>I',w)for w in new)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',scratch+7)),(f['code_count_offset'],4,struct.pack('>I',len(new))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(new))),(f['lines_offset'],len(f['lines']),f['lines'][:32]+f['lines'][28:32]*4+f['lines'][32:])])
  for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+4 if start>=8 else start)),(b,4,struct.pack('>I',end+4 if end>=8 else end))])
  changed[f['id']]=dict(name=f['name'],addedInstructions=4,legacyUpgrade=True)
 patched=source
 for off,size,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+size:]
 inverse=[];shift=0
 for off,size,value in sorted(edits):inverse.append((off+shift,len(value),source[off:off+size]));shift+=len(value)-size
 restored=patched
 for off,size,value in reversed(inverse):restored=restored[:off]+value+restored[off+size:]
 assert restored==source
 patched,early=early_transition(patched);changed.update(early)
 verify(patched)
 return patched,changed

def patch(source):
 r=Reader(source);r.function();edits=[];changed={}
 if any(QUERY in f['constants']for f in r.functions):return upgrade(source)
 selected={f['id']:int('LoopLoop'in f['name'])for f in r.functions if f['name']and f['name'].startswith('SwordArts')and f['name'].endswith('_onUpdate')and ('OneShot'in f['name']or 'LoopLoop'in f['name'])}
 for fid,phase in selected.items():
  f=r.functions[fid];assert QUERY not in f['constants'],'Onyx observer already installed'
  cs=f['constants'][:];extra=[];encoded=bytearray()
  def constant(value):
   if value in cs:return cs.index(value)
   if value in extra:return len(cs)+extra.index(value)
   index=len(cs)+len(extra)
   if isinstance(value,str):extra.extend([value,None,None]);b=value.encode()+b'\0';encoded.extend(b'\x04'+struct.pack('>Q',len(b))+b+b'\0\0')
   else:extra.append(float(value));encoded.extend(b'\x03'+struct.pack('>f',value))
   return index
  scratch=f['slots']
  def glob(reg,key):
   k=constant(key);return [abx(91,reg,k),abx(76,20,k+1)]
  # Native SetSwordArtsPointInfo records 0 for the paid startup, 1 for
  # insufficient FP/requirements. Unlike a short TAE effect it survives until
  # IsAnimEnd and is overwritten by the next startup (including empty FP).
  prefix=glob(scratch,'env')+[abx(25,scratch+1,constant(QUERY)),abx(25,scratch+2,constant(phase))]+glob(scratch+3,'c_SwordArtsID')+glob(scratch+4,'c_SwordArtsHand')
  prefix+=glob(scratch+5,'GetVariable')+[abx(25,scratch+6,constant('IsEnoughArtPointsL2')),abc(2,scratch+5,2,2),abc(2,scratch,6,2)]
  if phase:
   prefix+=[abx(25,scratch+1,constant(1)),abc(4,0,scratch,scratch+1),0];jump=len(prefix)-1
   # Finish all-body using the matching existing art end event. Onyx uses the
   # ordinary right-hand path; shield variants retain their own end states.
   event='W_SwordArts'+('Both'if'Both'in f['name']else'Left'if'Left'in f['name']else'')+'LoopEnd'
   prefix+=glob(scratch,'ExecEventAllBody')+[abx(25,scratch+1,constant(event)),abc(2,scratch,2,1),abc(9,0,1,0)]
   prefix[jump]=abx(28,0,0x10000+len(prefix)-jump-2)
  old=[x[0]for x in struct.iter_unpack('>I',f['code'])];body=old[:];fp=[]
  if phase:
   for i,w in enumerate(old):
    d=decode(w)
    if d['op']==25 and cs[d['bx']]==344:
     body[i]=abx(25,d['a'],constant(FP_QUERY));fp.append(i)
  while len(encoded)%4:encoded.extend(b'\0');extra.append(None)
  new=prefix+body
  # Entry insertion preserves every relative branch and skip pair unchanged.
  code=b''.join(struct.pack('>I',x)for x in new);n=len(prefix)
  edits.extend([(f['slots_offset'],4,struct.pack('>I',scratch+7)),(f['code_count_offset'],4,struct.pack('>I',len(new))),(f['code_offset'],len(f['code']),code),(f['constants_count_offset'],4,struct.pack('>I',len(cs)+len(extra))),(f['constants_end'],0,bytes(encoded)),(f['line_count_offset'],4,struct.pack('>I',len(new))),(f['lines_offset'],len(f['lines']),f['lines'][:4]*n+f['lines'])])
  for _,a,start,b,end in f['locals']:edits.extend([(a,4,struct.pack('>I',start+n if start else 0)),(b,4,struct.pack('>I',end+n))])
  changed[fid]=dict(name=f['name'],phase=phase,originalInstructions=len(old),addedInstructions=n,fpQueries=fp)
 patched=source
 for off,size,value in sorted(edits,reverse=True):patched=patched[:off]+value+patched[off+size:]
 actual=Reader(patched);actual.function();assert len(actual.functions)==len(r.functions)
 for f,a in zip(r.functions,actual.functions):
  if f['id']in changed:
   info=changed[f['id']];tail=bytearray(a['code'][info['addedInstructions']*4:])
   for i in info['fpQueries']:tail[i*4:(i+1)*4]=f['code'][i*4:(i+1)*4]
   assert tail==f['code']and a['constants'][:len(f['constants'])]==f['constants']
  else:assert source[f['start']:f['end']]==patched[a['start']:a['end']]if not f['children']else a['code']==f['code']
 inverse=[];shift=0
 for off,size,value in sorted(edits):inverse.append((off+shift,len(value),source[off:off+size]));shift+=len(value)-size
 restored=patched
 for off,size,value in reversed(inverse):restored=restored[:off]+value+restored[off+size:]
 assert restored==source
 patched,early=early_transition(patched)
 for fid,info in early.items():changed[fid].update(info)
 return patched,changed

def verify(data):
 r=Reader(data);r.function();assert all(QUERY in r.functions[i]['constants']for i in [553,554,566,567]);assert any(FP_QUERY in f['constants']for f in r.functions)
 for f in r.functions:
  if QUERY in f['constants']:
   words=[w[0]for w in struct.iter_unpack('>I',f['code'])];offset=next(i-2 for i,w in enumerate(words)if decode(w)['op']==25 and f['constants'][decode(w)['bx']]==QUERY);scratch=decode(words[offset])['a']
   assert words[offset+12]==abc(2,scratch,6,2) and 'IsEnoughArtPointsL2' in f['constants']
 assert all(START_END_QUERY in f['constants']for f in r.functions if f['name']in ['SwordArtsOneShot_onUpdate','SwordArtsHalfOneShot_Upper_onUpdate'])
 return True
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--inplace',action='store_true');p.add_argument('--check',action='store_true');a=p.parse_args();path=ROOT/'dist/mod/action/script/c0000.hks';source=path.read_bytes()
 if a.check:verify(source);print('PASS: Onyx action callbacks present');raise SystemExit
 try:verify(source);print('PASS: Onyx callbacks already present');raise SystemExit
 except AssertionError:pass
 target,changed=patch(source);out=ROOT/'.work/onyx-mastery/hks';out.mkdir(parents=True,exist_ok=True);(out/'before.hks').write_bytes(source);(out/'c0000.hks').write_bytes(target)
 (out/'validation.json').write_text(json.dumps(dict(inputSha256=hashlib.sha256(source).hexdigest(),outputSha256=hashlib.sha256(target).hexdigest(),functions=changed,originalControlFlowRetained=True,exactInversePassed=True),indent=2)+'\n')
 if a.inplace:path.write_bytes(target)
 print('PASS: Onyx action callbacks; original control flow retained and exact inverse checked')
