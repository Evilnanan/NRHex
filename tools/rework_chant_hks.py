from hks_bytecode import Reader, instructions, decode, struct

def upgrade(source):
 reader=Reader(source);reader.function();edits=[];changed=set()
 for function in reader.functions:
  if function['id'] not in {801,806,807,808,812,814}:continue
  constants=function['constants'];code=instructions(function)
  owner=next(index for index,word in enumerate(code) if word['op']==25 and constants[word['bx']]==6999920)
  start=owner-3+9;end=start+9
  if code[start]['op']==91 and constants[code[start]['bx']]=='IsChargeMagic':continue
  gate=code[start:end]
  assert [word['op'] for word in gate]==[91,76,25,25,2,91,76,4,28]
  assert constants[gate[0]['bx']]=='env' and constants[gate[2]['bx']]==1116 and constants[gate[3]['bx']]==100610
  assert gate[7]['a']==0 and constants[gate[5]['bx']]=='TRUE'
  assert code[end]['op']==91 and constants[code[end]['bx']]=='IsChargeMagic'
  def shifted(index):
   assert not start<index<end
   return index if index<=start else index-9
  words=[]
  for index,(original,) in enumerate(struct.iter_unpack('>I',function['code'])):
   if start<=index<end:continue
   word=code[index]
   if word['op']==28:
    target=index+word['bx']-0x10000+2
    offset=0x10000+shifted(target)-shifted(index)-2
    original=word['op']<<25|offset<<8|word['a']
    assert shifted(index)+decode(original)['bx']-0x10000+2==shifted(target)
   words.append(original)
  output=b''.join(struct.pack('>I',word) for word in words)
  lines=function['lines'][:start*4]+function['lines'][end*4:]
  edits.extend([(function['code_count_offset'],4,struct.pack('>I',len(words))),(function['code_offset'],len(function['code']),output),(function['line_count_offset'],4,struct.pack('>I',len(words))),(function['lines_offset'],len(function['lines']),lines)])
  for _,begin_offset,begin,end_offset,finish in function['locals']:
   edits.extend([(begin_offset,4,struct.pack('>I',shifted(begin))),(end_offset,4,struct.pack('>I',shifted(finish)))])
  changed.add(function['id'])
 assert len(changed) in {0,6}
 result=source
 for offset,length,value in sorted(edits,reverse=True):result=result[:offset]+value+result[offset+length:]
 after=Reader(result);after.function();assert source[reader.p:]==result[after.p:]
 for before,current in zip(reader.functions,after.functions):
  if before['id'] not in changed:assert before['code']==current['code'] and before['constants']==current['constants']
 return result
