using SoulsFormats;

static class TaeEventAppend {
 public static byte[] PrependEffect(byte[] source,IReadOnlyCollection<long> animations,int effect,float duration){
  if(source[4]!=0||source[7]!=0xff||BitConverter.ToInt32(source,8)!=0x1000d)
   throw new InvalidDataException("Unsupported TAE layout");
  var original=TAE.Read(source);
  using var stream=new MemoryStream();stream.Write(source);
  using var writer=new BinaryWriter(stream);
  var allowed=new bool[source.Length];
  long ReadLong(int at)=>BitConverter.ToInt64(source,at);
  int ReadInt(int at)=>BitConverter.ToInt32(source,at);
  void Long(long at,long value){stream.Position=at;writer.Write(value);if(at<source.Length)Array.Fill(allowed,true,(int)at,8);}
  void Int(long at,int value){stream.Position=at;writer.Write(value);if(at<source.Length)Array.Fill(allowed,true,(int)at,4);}
  long Append(byte[] bytes){stream.Position=stream.Length;while(stream.Position%16!=0)writer.Write((byte)0);long at=stream.Position;writer.Write(bytes);return at;}
  int headers=checked((int)ReadLong(0x58));
  int changed=0;
  for(int index=0;index<original.Animations.Count;index++){
   var animation=original.Animations[index];
   if(!animations.Contains(animation.ID))continue;
   int body=checked((int)ReadLong(headers+index*16+8));
   int events=checked((int)ReadLong(body)),groups=checked((int)ReadLong(body+8));
   int eventCount=ReadInt(body+32),groupCount=ReadInt(body+36);
   if(ReadLong(headers+index*16)!=animation.ID||eventCount!=animation.Events.Count||groupCount!=animation.EventGroups.Count)
    throw new InvalidDataException("TAE animation topology mismatch");
   int TimeBits(int eventIndex,int field)=>ReadInt(checked((int)ReadLong(events+eventIndex*24+field)));
   var times=Enumerable.Range(0,eventCount).SelectMany(eventIndex=>new[]{TimeBits(eventIndex,0),TimeBits(eventIndex,8)}).Concat(new[]{0,BitConverter.SingleToInt32Bits(duration)}).Distinct().OrderBy(BitConverter.Int32BitsToSingle).ToArray();
   long timeTable=Append(times.SelectMany(BitConverter.GetBytes).ToArray());
   long Time(int bits)=>timeTable+Array.IndexOf(times,bits)*4;
   var eventBytes=new byte[(eventCount+1)*24];
   source.AsSpan(events,eventCount*24).CopyTo(eventBytes.AsSpan(24));
   long eventTable=Append(eventBytes);
   Append(Array.Empty<byte>());
   long alignment=ReadLong(events+16)%16;
   stream.Position=stream.Length;writer.Write(new byte[checked((int)alignment)]);
   long data=stream.Length;writer.Write(new byte[32]);Int(data,302);Long(data+8,data+16);Int(data+16,effect);
   Long(eventTable,Time(0));Long(eventTable+8,Time(BitConverter.SingleToInt32Bits(duration)));Long(eventTable+16,data);
   for(int eventIndex=0;eventIndex<eventCount;eventIndex++){
    var entry=animation.Events[eventIndex];var payload=entry.GetParameterBytes(false);
    stream.Position=stream.Length;long relocatedData=stream.Position;
    writer.Write(entry.Type);writer.Write(entry.Unk04);writer.Write(relocatedData+16);writer.Write(payload);
    Long(eventTable+(eventIndex+1)*24,Time(TimeBits(eventIndex,0)));
    Long(eventTable+(eventIndex+1)*24+8,Time(TimeBits(eventIndex,8)));
    Long(eventTable+(eventIndex+1)*24+16,relocatedData);
   }
   var groupBytes=new byte[(groupCount+1)*32];source.AsSpan(groups,groupCount*32).CopyTo(groupBytes);
   long groupTable=Append(groupBytes);
   for(int groupIndex=0;groupIndex<groupCount;groupIndex++){
    int header=groups+groupIndex*32,amount=checked((int)ReadLong(header));
    if(amount==0)continue;
    int values=checked((int)ReadLong(header+8));var relocated=new byte[amount*4];
    for(int item=0;item<amount;item++){
     int oldHeader=ReadInt(values+item*4);
     if(oldHeader<events||oldHeader>=events+eventCount*24||(oldHeader-events)%24!=0)
      throw new InvalidDataException("TAE group references an invalid event");
     BitConverter.GetBytes(checked((int)(eventTable+24+oldHeader-events))).CopyTo(relocated,item*4);
    }
    Long(groupTable+groupIndex*32+8,Append(relocated));
   }
   long type=Append(new byte[16]);Long(type,302);
   long indices=Append(BitConverter.GetBytes(checked((int)eventTable)));
   long newGroup=groupTable+groupCount*32;
   Long(newGroup,1);Long(newGroup+8,indices);Long(newGroup+16,type);
   Long(body,eventTable);Long(body+8,groupTable);Long(body+16,timeTable);
   Int(body+32,eventCount+1);Int(body+36,groupCount+1);Int(body+40,times.Length);changed++;
  }
  if(changed!=animations.Count)throw new InvalidDataException("TAE target animation missing or duplicated");
  Append(Array.Empty<byte>());Int(12,checked((int)stream.Length));
  var result=stream.ToArray();
  for(int index=0;index<source.Length;index++)
   if(!allowed[index]&&source[index]!=result[index])throw new InvalidDataException("TAE original payload/layout changed");
  var parsed=TAE.Read(result);
  foreach(var pair in original.Animations.Zip(parsed.Animations)){
   bool selected=animations.Contains(pair.First.ID);var actual=pair.Second.Events.Skip(selected?1:0).ToArray();
   if(actual.Length!=pair.First.Events.Count)throw new InvalidDataException("TAE event count changed");
   foreach(var entries in pair.First.Events.Zip(actual)){
    var before=entries.First;var after=entries.Second;
    if(before.Type!=after.Type||before.Unk04!=after.Unk04||before.StartTime!=after.StartTime||before.EndTime!=after.EndTime||before.Group?.GroupType!=after.Group?.GroupType||!before.GetParameterBytes(false).SequenceEqual(after.GetParameterBytes(false)))
     throw new InvalidDataException($"TAE original event readback mismatch: animation={pair.First.ID}, event={pair.First.Events.IndexOf(before)}, type={before.Type}/{after.Type}, bytes={before.GetParameterBytes(false).Length}/{after.GetParameterBytes(false).Length}, group={before.Group?.GroupType}/{after.Group?.GroupType}");
   }
   if(selected){var added=pair.Second.Events[0];if(added.Type!=302||added.StartTime!=0||added.EndTime!=duration||BitConverter.ToInt32(added.GetParameterBytes(false))!=effect||added.Group?.GroupType!=302)throw new InvalidDataException("TAE added effect readback mismatch");}
  }
  return result;
 }
}
