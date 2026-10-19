using SoulsFormats;

static class TaeTiming {
 public static byte[] Rewrite(byte[] source,TAE desired,Func<TAE.Event,bool> selected){
  var original=TAE.Read(source);
  if(original.Animations.Count!=desired.Animations.Count)throw new InvalidDataException("TAE animation count changed");
  if(original.Animations.Zip(desired.Animations).Any(pair=>pair.First.ID!=pair.Second.ID||pair.First.Events.Count!=pair.Second.Events.Count))
   throw new InvalidDataException("Timing-only TAE update cannot add or reorder animations/events");
  if(source[4]!=0||source[7]!=0xff||BitConverter.ToInt32(source,8)!=0x1000d)throw new InvalidDataException("Unsupported TAE layout");
  using var stream=new MemoryStream();stream.Write(source);
  using var writer=new BinaryWriter(stream);
  long Offset(int at)=>BitConverter.ToInt64(source,at);
  void Long(long at,long value){stream.Position=at;writer.Write(value);}
  void Int(long at,int value){stream.Position=at;writer.Write(value);}
  var headers=checked((int)Offset(0x58));
  for(int index=0;index<original.Animations.Count;index++){
   var old=original.Animations[index];var next=desired.Animations[index];
   var body=checked((int)Offset(headers+index*16+8));var events=checked((int)Offset(body));
   if(Offset(headers+index*16)!=old.ID||BitConverter.ToInt32(source,body+32)!=old.Events.Count)throw new InvalidDataException("TAE raw animation mismatch");
   var times=new (float Start,float End)[old.Events.Count];bool changed=false;
   for(int eventIndex=0;eventIndex<old.Events.Count;eventIndex++){
    var before=old.Events[eventIndex];var after=next.Events[eventIndex];
    if(before.Type!=after.Type||before.Unk04!=after.Unk04||!before.GetParameterBytes(false).SequenceEqual(after.GetParameterBytes(false)))throw new InvalidDataException("Timing update changed an event payload");
    bool differs=before.StartTime!=after.StartTime||before.EndTime!=after.EndTime;
    if(differs&&(!selected(before)||!selected(after)))throw new InvalidDataException("Timing update changed an unrelated event");
    var start=BitConverter.ToSingle(source,checked((int)Offset(events+eventIndex*24)));
    var end=BitConverter.ToSingle(source,checked((int)Offset(events+eventIndex*24+8)));
    times[eventIndex]=differs?(after.StartTime,after.EndTime):(start,end);changed|=differs;
   }
   if(!changed)continue;
   stream.Position=stream.Length;while(stream.Position%16!=0)writer.Write((byte)0);
   var table=stream.Position;var offsets=new Dictionary<float,long>();
   foreach(var time in times.SelectMany(time=>new[]{time.Start,time.End}).Distinct().Order()){
    offsets.Add(time,stream.Position);writer.Write(time);
   }
   Long(body+16,table);Int(body+40,offsets.Count);
   for(int eventIndex=0;eventIndex<times.Length;eventIndex++){
    Long(events+eventIndex*24,offsets[times[eventIndex].Start]);Long(events+eventIndex*24+8,offsets[times[eventIndex].End]);
   }
  }
  if(stream.Length==source.Length)return source;
  stream.Position=stream.Length;while(stream.Position%16!=0)writer.Write((byte)0);
  Int(12,checked((int)stream.Length));
  var result=stream.ToArray();var parsed=TAE.Read(result);
  for(int index=0;index<parsed.Animations.Count;index++)foreach(var pair in parsed.Animations[index].Events.Zip(desired.Animations[index].Events))
   if(pair.First.StartTime!=pair.Second.StartTime||pair.First.EndTime!=pair.Second.EndTime)throw new InvalidDataException("Timing-only readback mismatch");
  return result;
 }
}
