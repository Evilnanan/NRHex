"""Exercise the inserted control flow, not only its encoded constants."""
import struct,unittest,re
from pathlib import Path
from build_onyx_hks import patch,QUERY,FP_QUERY,START_END_QUERY
from hks_bytecode import Reader,decode
from mmv_hks_fragments import Fragment

class OnyxActionTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  source=Path('assets/original/action/script/c0000.hks').read_bytes()
  result,cls.changed=patch(source);cls.reader=Reader(result);cls.reader.function()
 def test_prefixes_preserve_registers_and_only_finish_the_channel_on_request(self):
  for index,info in self.changed.items():
   f=self.reader.functions[index];count=info['addedInstructions'];code=f['code'][:count*4]
   # The MMV adapter must accept precisely the same guarded fragment.
   self.assertIn('env',Fragment(code,f['constants']).lua())
   words=[decode(x[0])for x in struct.iter_unpack('>I',code)]
   for art in [241,1,13,323]:
    for hand in [0,1]:
     for finish in [False,True]:
      calls=[];query=[];registers={i:('saved',i)for i in range(f['slots']-7)};saved=registers.copy()
      def env(*args):query.append(args);return int(finish and art==241)
      globals_={'env':env,'c_SwordArtsID':art,'c_SwordArtsHand':hand,'GetVariable':lambda name:int(not finish),'ExecEventAllBody':lambda e:calls.append(e)}
      pc=0;returned=False
      while pc<len(words):
       w=words[pc];pc+=1;op,a,b,c,bx=[w[k]for k in ['op','a','b','c','bx']]
       if op==91:registers[a]=globals_[f['constants'][bx]]
       elif op==76:pass
       elif op==25:registers[a]=f['constants'][bx]
       elif op==2:
        value=registers[a](*(registers[i]for i in range(a+1,a+b)))
        if c==2:registers[a]=value
       elif op==4:
        if (registers[b]==registers[c])!=bool(a):pc+=1
       elif op==28:pc+=bx-0x10000+1
       elif op==9:returned=True;break
       else:self.fail(str(w))
      self.assertEqual(query,[(QUERY,info['phase'],art,hand,int(not finish))])
      self.assertEqual(returned,bool(info['phase']and finish and art==241))
      self.assertEqual(len(calls),int(returned))
      self.assertEqual({k:registers[k]for k in saved},saved)

 def test_free_fp_query_only_replaces_native_loop_queries(self):
  original=Reader(Path('assets/original/action/script/c0000.hks').read_bytes());original.function()
  replacements=0
  for old,new in zip(original.functions,self.reader.functions):
   added=self.changed.get(old['id'],{}).get('addedInstructions',0)
   before=[decode(x[0])for x in struct.iter_unpack('>I',old['code'])]
   after=[decode(x[0])for x in struct.iter_unpack('>I',new['code'][added*4:])]
   self.assertEqual(len(before),len(after))
   for a,b in zip(before,after):
    if a==b:continue
    self.assertEqual(a['op'],25);self.assertEqual(b['op'],25);self.assertEqual(a['a'],b['a'])
    if 'LoopLoop' in old['name']:
     self.assertEqual(old['constants'][a['bx']],344);self.assertEqual(new['constants'][b['bx']],FP_QUERY);replacements+=1
    else:
     self.assertIn('OneShot',old['name']);self.assertEqual(old['constants'][a['bx']],339);self.assertEqual(new['constants'][b['bx']],START_END_QUERY)
  self.assertEqual(replacements,3)

 def test_dispatch_query_ids_are_unique_and_match_hks(self):
  source=Path('src/ambulance.rs').read_text(encoding='utf8')
  names=set(re.findall(r'code\s*==\s*crate::(\w+)::(\w*QUERY)',source))
  values={}
  for module,name in names:
   text=Path(f'src/{module}.rs').read_text(encoding='utf8')
   match=re.search(rf'pub const {name}\s*:\s*i32\s*=\s*(\d+)',text)
   self.assertIsNotNone(match,(module,name))
   value=int(match[1]);self.assertNotIn(value,values,(module,name,values.get(value)))
   values[value]=(module,name)
  for name,value in [('QUERY',QUERY),('FP_QUERY',FP_QUERY),('START_END_QUERY',START_END_QUERY)]:
   self.assertEqual(values[value],('onyx_mastery',name))

 def test_migration_keeps_evasion_and_is_idempotent(self):
  from build_onyx_hks import early_transition
  current=Path('dist/mod/action/script/c0000.hks').read_bytes()
  reader=Reader(current);reader.function();legacy=bytearray(current)
  # Simulate the previous callback at its actual constant encoding; leave
  # all other 990055 callbacks, including evasion, untouched.
  count=0
  for f in reader.functions:
   if f['name'] not in ['SwordArtsOneShot_onUpdate','SwordArtsHalfOneShot_Upper_onUpdate']:continue
   chunk=current[f['constants_count_offset']+4:f['constants_end']]
   needle=b'\x03'+struct.pack('>f',START_END_QUERY)
   self.assertEqual(chunk.count(needle),1)
   off=f['constants_count_offset']+4+chunk.index(needle)
   legacy[off:off+5]=b'\x03'+struct.pack('>f',990055);count+=1
  self.assertEqual(count,2)
  result,changed=early_transition(bytes(legacy))
  self.assertEqual(len(changed),2)
  migrated=Reader(result);migrated.function()
  for old,new in zip(reader.functions,migrated.functions):
   if old['id'] in changed:
    for word in struct.iter_unpack('>I',new['code']):
     d=decode(word[0])
     if d['op']==25:self.assertNotEqual(new['constants'][d['bx']],990055)
   else:
    self.assertEqual(old['code'],new['code']);self.assertEqual(old['constants'],new['constants'])
  self.assertEqual(patch(result)[0],result)
