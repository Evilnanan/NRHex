"""Exercise the inserted control flow, not only its encoded constants."""
import struct,unittest
from pathlib import Path
from build_onyx_hks import patch,QUERY,FP_QUERY
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
      calls=[];query=[];registers={i:('saved',i)for i in range(f['slots']-5)};saved=registers.copy()
      def env(*args):query.append(args);return int(finish and art==241)
      globals_={'env':env,'c_SwordArtsID':art,'c_SwordArtsHand':hand,'ExecEventAllBody':lambda e:calls.append(e)}
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
      self.assertEqual(query,[(QUERY,info['phase'],art,hand)])
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
    self.assertIn('LoopLoop',old['name']);self.assertEqual(a['op'],25);self.assertEqual(b['op'],25)
    self.assertEqual(a['a'],b['a']);self.assertEqual(old['constants'][a['bx']],344)
    self.assertEqual(new['constants'][b['bx']],FP_QUERY);replacements+=1
  self.assertEqual(replacements,3)
