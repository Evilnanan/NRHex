using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class CageUsage {
 public const float IntroSeconds=4f;
 public const uint CounterOffset=5700;
 static EMEVD.Instruction I(int b,int id,params object[] a)=>new(b,id,a);
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static EMEVD.Instruction Copy(EMEVD.Instruction i)=>new(i.Bank,i.ID,i.ArgData.ToArray()){Layer=i.Layer};
 public static EMEVD.Event Open(EMEVD.Event native){
  Check(native.ID==90045000&&native.Instructions.Count==26&&native.Instructions[22].Bank==2003&&native.Instructions[22].ID==100,"Native gaol opener changed");
  var e=new EMEVD.Event(Cages.OpenEvent,EMEVD.Event.RestBehaviorType.Restart);
  void Param(int at,int target,int source,int size=4)=>e.Parameters.Add(new EMEVD.Parameter{InstructionIndex=at,TargetStartByte=target,SourceStartByte=source,ByteCount=size,UnkID=source/4+1});
  void Original(int index){int at=e.Instructions.Count;e.Instructions.Add(Copy(native.Instructions[index]));foreach(var p in native.Parameters.Where(p=>p.InstructionIndex==index))e.Parameters.Add(new EMEVD.Parameter{InstructionIndex=at,TargetStartByte=p.TargetStartByte,SourceStartByte=p.SourceStartByte,ByteCount=p.ByteCount,UnkID=p.UnkID});}
  void Busy(int source){int at=e.Instructions.Count;e.Instructions.Add(I(1003,2,(byte)1,(byte)1,(byte)0,0u));Param(at,4,source);}
  for(int i=0;i<10;i++)Original(i);
  Busy(20);Busy(24);
  int countCheck=e.Instructions.Count;e.Instructions.Add(I(3,12,(sbyte)-13,0u,(byte)3,(byte)4,5u));Param(countCheck,4,28);
  e.Instructions.AddRange([I(1000,101,(byte)0,(byte)1,(sbyte)-13),I(1004,0,(byte)1,20000u,Cages.Owner,(byte)1,(sbyte)0,1),I(1000,103,(byte)0),I(1000,103,(byte)8),I(1014,0)]);
  // Exhausted owners and nonowners use the original key check and settlement.
  for(int i=10;i<26;i++)Original(i);
  e.Instructions.Add(I(1014,8));
  e.Instructions.AddRange([I(1004,6,(byte)1,(byte)2,(byte)0),I(1,0,(sbyte)0,.1f),I(1004,6,(byte)1,(byte)3,(byte)0),I(1,0,(sbyte)0,.2f)]);
  Busy(20);Busy(24);
  // Category 5 belongs to this peer and map instance, unlike shared category 7.
  int increment=e.Instructions.Count;e.Instructions.Add(I(2003,127,0u,3u,5u));Param(increment,0,28);
  int request=e.Instructions.Count;e.Instructions.Add(I(2003,69,(byte)0,0u,(byte)1));Param(request,4,24);
  Original(22);Original(23);Original(24);
  e.Instructions.AddRange([I(1001,1,1),I(1000,4,(byte)1)]);return e;
 }
 public static void Intro(EMEVD.Event e){
  Check(e.Parameters.Count==0,"Expected nonparameterized cage birth");
  int at=e.Instructions.FindIndex(i=>i.Bank==2006&&i.ID==3);Check(at>=0,"Missing cage fog");
  var fog=e.Instructions[at];uint actor=BitConverter.ToUInt32(fog.ArgData,4);
  Check(BitConverter.ToInt32(fog.ArgData,8)==900&&BitConverter.ToInt32(fog.ArgData,12) is 690047 or 690046,"Unexpected old cage fog");
  Check(e.Instructions[at+1].Bank==1001&&e.Instructions[at+1].ID==0&&BitConverter.ToSingle(e.Instructions[at+1].ArgData)==4f,"Unexpected cage fog wait");
  e.Instructions[at]=I(2006,3,2,actor,900,690047);
 }
 public static void CheckCounter(EMEVD events,uint p){
  foreach(var e in events.Events)foreach(var i in e.Instructions){
   for(int n=0;n<i.ArgData.Length/4;n++){uint v=BitConverter.ToUInt32(i.ArgData,4*n);Check(v<p+CounterOffset||v>=p+CounterOffset+3,"Personal counter collides with an existing argument");}
   if(i.Bank==2003&&i.ID is 22 or 63&&i.ArgData.Length>=8){uint first=BitConverter.ToUInt32(i.ArgData),last=BitConverter.ToUInt32(i.ArgData,4);Check(last<p+CounterOffset||first>p+CounterOffset+2,"Counter falls in an existing batch clear range");}
  }
 }
 public static void Stage(string root){
  string output=Path.Combine(root,".work/cage-01794/staged/mod/event");Directory.CreateDirectory(output);
  var native=EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx"));var night=native.Events.Single(e=>e.ID==90065001);
  Check(BitConverter.ToInt32(night.Instructions[54].ArgData,12)==690050&&BitConverter.ToSingle(night.Instructions[55].ArgData,4)==2.5f&&BitConverter.ToInt32(night.Instructions[58].ArgData,12)==690047&&BitConverter.ToSingle(night.Instructions[59].ArgData,4)==4f,"Native night boss intro changed");
  var reports=new List<object>();
  foreach(string name in new[]{"common_func","m46_50_00_00","m46_60_00_00","m46_70_00_00","m46_80_00_00"}){
   string input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),target=Path.Combine(output,name+".emevd.dcx");var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changed=new HashSet<long>();
   if(name=="common_func"){
    var old=next.Events.Single(e=>e.ID==Cages.OpenEvent);Check(old.Parameters.Any(p=>p.SourceStartByte==24)&&!old.Parameters.Any(p=>p.SourceStartByte==28),"Expected previous cage opener");
    next.Events[next.Events.IndexOf(old)]=Open(native.Events.Single(e=>e.ID==90045000));changed.Add(old.ID);
   }else{
    uint p=(uint)(4600+int.Parse(name.Substring(4,2)))*10000;CheckCounter(before,p);
    var ctor=next.Events.Single(e=>e.ID==0);int bound=0;
    foreach(var i in ctor.Instructions.Where(i=>i.Bank==2000&&i.ID==6&&BitConverter.ToInt32(i.ArgData,4)==Cages.OpenEvent)){
     Check(i.ArgData.Length==36,"Unexpected cage initializer");var bytes=new byte[40];i.ArgData.CopyTo(bytes,0);BitConverter.GetBytes(p+CounterOffset).CopyTo(bytes,36);i.ArgData=bytes;bound++;
    }
    Check(bound>0,$"Expected cage openers in {name}, found {bound}");changed.Add(0);
    var births=next.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984).ToArray();Check(births.Length==84,"Expected 84 actor presentations");foreach(var e in births){Intro(e);changed.Add(e.ID);}
    var battles=next.Events.Where(e=>e.ID>=p+9710&&e.ID<p+9760).ToArray();Check(battles.Length==50,"Expected 50 encounters");
    foreach(var e in battles){var waits=e.Instructions.Where(i=>i.Bank==1001&&i.ID==0&&BitConverter.ToSingle(i.ArgData)==4f).ToArray();Check(waits.Length==1,"Expected one battle intro wait");waits[0].ArgData=I(1001,0,IntroSeconds).ArgData;changed.Add(e.ID);}
   }
   next.Write(target);var actual=EMEVD.Read(target);
   Check(actual.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&actual.Events.All(e=>Cages.Same(e,next.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
   Check(actual.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Cages.Same(e,before.Events.Single(v=>v.ID==e.ID))),"Unrelated event changed");
   Check(actual.Format==before.Format&&actual.Compression==before.Compression&&actual.StringData.SequenceEqual(before.StringData)&&actual.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Metadata changed");
   reports.Add(new{name,inputSha256=Hash(input),outputSha256=Hash(target),changed=changed.Order().ToArray(),events=actual.Events.Where(e=>changed.Contains(e.ID)).Select(Cages.Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/cage-01794/events.json"),JsonSerializer.Serialize(new{reports,nativeOpen=Cages.Dump(native.Events.Single(e=>e.ID==90045000)),nativeNightIntro=Cages.Dump(night),perPlayerPerGaolLimit=5,privateFlagCategory=5,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: personal gaol limit, native opening popup and night-boss intro staged; other events preserved");
 }
 public static void StageWithoutPrelude(string root){
  string output=Path.Combine(root,".work/cage-01795/staged/mod/event");Directory.CreateDirectory(output);
  var reports=new List<object>();
  foreach(string name in new[]{"common_func","m46_50_00_00","m46_60_00_00","m46_70_00_00","m46_80_00_00"}){
   string input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),target=Path.Combine(output,name+".emevd.dcx");var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changed=new HashSet<long>();
   if(name!="common_func"){
    uint p=(uint)(4600+int.Parse(name.Substring(4,2)))*10000;
    var births=next.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984).ToArray();Check(births.Length==84,"Expected 84 births");
    foreach(var e in births){
     int at=e.Instructions.FindIndex(i=>i.Bank==2006&&i.ID==3);Check(e.Parameters.Count==0&&at>=0,"Unexpected birth");
     Check(BitConverter.ToInt32(e.Instructions[at].ArgData,12)==690050&&e.Instructions[at+1].Bank==1001&&e.Instructions[at+1].ID==0&&BitConverter.ToSingle(e.Instructions[at+1].ArgData)==2.5f,"Expected prelude and delay");
     Check(e.Instructions[at+2].Bank==2006&&e.Instructions[at+2].ID==3&&BitConverter.ToInt32(e.Instructions[at+2].ArgData,12)==690047,"Expected main fog");
     e.Instructions.RemoveRange(at,2);changed.Add(e.ID);
    }
    var battles=next.Events.Where(e=>e.ID>=p+9710&&e.ID<p+9760).ToArray();Check(battles.Length==50,"Expected 50 battles");
    foreach(var e in battles){
     var wait=e.Instructions.Single(i=>i.Bank==1001&&i.ID==0&&BitConverter.ToSingle(i.ArgData)==6.5f);wait.ArgData=I(1001,0,IntroSeconds).ArgData;
     var sound=e.Instructions.Single(i=>i.Bank==2010&&i.ID==2&&BitConverter.ToInt32(i.ArgData,8)==999997300);
     e.Instructions.Remove(sound);changed.Add(e.ID);
    }
   }
   next.Write(target);var actual=EMEVD.Read(target);
   Check(actual.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&actual.Events.All(e=>Cages.Same(e,next.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
   Check(actual.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Cages.Same(e,before.Events.Single(v=>v.ID==e.ID))),"Unrelated event changed");
   Check(actual.Format==before.Format&&actual.Compression==before.Compression&&actual.StringData.SequenceEqual(before.StringData)&&actual.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Metadata changed");
   reports.Add(new{name,inputSha256=Hash(input),outputSha256=Hash(target),changed=changed.Order().ToArray(),events=actual.Events.Where(e=>changed.Contains(e.ID)||e.ID==0||e.ID==Cages.OpenEvent).Select(Cages.Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/cage-01795/events.json"),JsonSerializer.Serialize(new{reports,perPlayerPerGaolLimit=5,privateFlagCategory=5,preludeRemoved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: prelude removed; opener, personal counters and other events preserved");
 }
}
