"""Read original NR HavokScript prototypes; never decompile/recompile them.

Format/opcodes cross-checked against katalash/DSLuaDecompiler LuaFile.cs and
LanguageDecompilers/HksDecompiler.cs. Offsets are retained for surgical edits.
"""
import struct,sys,hashlib
from pathlib import Path
class Reader:
 def __init__(self,data):self.data=data;self.p=0xee;self.functions=[]
 def take(self,n):b=self.data[self.p:self.p+n];assert len(b)==n;self.p+=n;return b
 def u32(self):return struct.unpack('>I',self.take(4))[0]
 def string(self):
  n=struct.unpack('>Q',self.take(8))[0]
  if not n:return None
  b=self.take(n);assert b[-1]==0;return b[:-1].decode('utf-8')
 def function(self):
  f={'id':len(self.functions),'start':self.p};self.functions.append(f)
  f['header']=self.take(9);f['slots_offset']=self.p;f['slots']=self.u32();f['unknown']=self.u32()
  f['code_count_offset']=self.p;n=self.u32();self.take((-self.p)%4)
  f['code_offset']=self.p;f['code']=self.take(n*4);f['count']=n
  f['constants_count_offset']=self.p;nc=self.u32();f['constants_offset']=self.p;cs=[]
  for _ in range(nc):
   t=self.take(1)[0]
   if t==0:v=None
   elif t==1:v=bool(self.take(1)[0])
   elif t==3:v=struct.unpack('>f',self.take(4))[0]
   elif t==4:v=self.string()
   else:raise ValueError(f'Unknown constant {t}')
   cs.append(v)
  f['constants']=cs;f['constants_end']=self.p
  f['debug_flag']=self.u32();f['line_count_offset']=self.p;f['line_count']=self.u32()
  assert f['debug_flag']==1 and f['line_count']==n, 'Serialized debug line count must match instruction count'
  nl=self.u32();nu=self.u32();f['line_begin']=self.u32();f['line_end']=self.u32()
  f['path']=self.string();f['name']=self.string();f['lines_offset']=self.p;f['lines']=self.take(4*n)
  f['locals']=[]
  for _ in range(nl):
   name=self.string();a=self.p;start=self.u32();b=self.p;end=self.u32();f['locals'].append((name,a,start,b,end))
  for _ in range(nu):self.string()
  nk=self.u32();f['children']=[self.function() for _ in range(nk)];f['end']=self.p
  return f
def decode(w):
 return {'op':w>>25,'a':w&255,'b':(w>>17)&255,'c':(w>>8)&511,'bx':(w>>8)&0x1ffff}
def instructions(f):return [decode(w[0]) for w in struct.iter_unpack('>I',f['code'])]
if __name__=='__main__':
 data=Path(sys.argv[1] if len(sys.argv)>1 else 'assets/original/action/script/c0000.hks').read_bytes();r=Reader(data);r.function();assert data[r.p:]==bytes.fromhex('0000000100000000000000000000000000')
 print('functions',len(r.functions),'bytes',r.p,'sha256',hashlib.sha256(data).hexdigest())
 for f in r.functions:
  if 'ISENABLE_WALLJUMP' in f['constants']:
   print(f['id'],repr(f['name']),f['line_begin'],f['line_end'],f['slots'],'instructions',f['count'])
   if f['name']=='Update' or 'env' in f['constants'] and f['count']<150:
    print('CONSTANTS',list(enumerate(f['constants'])))
    for i,ins in enumerate(instructions(f)):
     if i<25 or ins['op'] in [6,27,91] and ins['bx']<len(f['constants']) and f['constants'][ins['bx']]=='ISENABLE_WALLJUMP':print(i,ins)
