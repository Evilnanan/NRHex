using SoulsFormats;
using System.Text.Json;
static class ParryFollowup {
 public const int Marker=95002405;
 public const short Right=24000,Left=24001,Sound=24002;
 // Follow the native guard-recoil clip, not a fixed animation-time cooldown.
 // GuardStart/GuardOn/GuardEnd are different clips and never get this marker.
 // Zero-endurance TAE effects stop when their owning clip exits or is cancelled.
 static bool Guard(TAE.Event e)=>e.Type==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==3;
 static float ReactionEnd(TAE.Animation a){
  var windows=a.Events.Where(Guard).OrderBy(e=>e.StartTime).ToArray();
  Check(windows.Length>0&&windows[0].StartTime==0,"Reaction lacks initial native guard: "+a.ID);
  float end=0;
  foreach(var w in windows){Check(w.StartTime<=end&&float.IsFinite(w.EndTime)&&w.EndTime>0,"Discontinuous native guard: "+a.ID);end=Math.Max(end,w.EndTime);}
  return end;
 }
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Support(Func<string,int,int,PARAM.Row> clone){
  var r=clone("SpEffectParam",8000000,Marker);
  void Set(string f,object v){r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());}
  Set("maxHpRate",1f);Set("effectEndurance",0f);Set("saveCategory",-1);Set("spCategory",20);
  Set("iconId",-1);Set("spEffectTextId_1",-1);Set("displayIconForPermamentEffect",0);
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/parry-fix/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var b=SFUtil.DecryptNightreignRegulation(input);
  var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var f=b.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var table=parse("SpEffectParam",f.Bytes);var old=parse("SpEffectParam",f.Bytes);
  Check(!table.Rows.Any(r=>r.ID==Marker),"Follow-up marker already installed");
  Support((n,src,id)=>{var r=new PARAM.Row(table.Rows.Single(r=>r.ID==src)){ID=id};table.Rows.Add(r);return r;});
  table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();f.Bytes=table.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var read=SFUtil.DecryptNightreignRegulation(output);
  foreach(var m in read.Files){Check(m.Bytes.Span.SequenceEqual(b.Files.Single(f=>f.ID==m.ID).Bytes.Span),"Parameter readback mismatch");if(m.ID!=f.ID)Check(m.Bytes.Span.SequenceEqual(before[m.ID]),"Other parameter table changed");}
  var result=parse("SpEffectParam",read.Files.Single(m=>m.ID==f.ID).Bytes);
  Check(result.Rows.Count==old.Rows.Count+1&&old.Rows.All(r=>CageAI.Same(r,result.Rows.Single(v=>v.ID==r.ID))),"Original effect changed");
  File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),added=Marker,originalRowsUnchanged=true},new JsonSerializerOptions{WriteIndented=true}));
  Actions(root,old);
 }
 static bool IsMarker(TAE.Event e)=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false))==Marker;
 static bool IsFeedback(TAE.Event e)=>e.Type is 96 or 129&&e.GetParameterBytes(false).Length>=18&&BitConverter.ToInt16(e.GetParameterBytes(false),16) is >=Right and <=Sound;
 static bool Generated(TAE.Event e)=>IsMarker(e)||IsFeedback(e);
 static bool Same(TAE.Event a,TAE.Event b){
  if(a.Type!=b.Type||a.StartTime!=b.StartTime||a.EndTime!=b.EndTime)return false;
  var x=a.GetParameterBytes(false);var y=b.GetParameterBytes(false);
  if(x.SequenceEqual(y))return true;
  // Only the serializer's trailing zero alignment on known variable records.
  if(a.Type is not (0 or 67 or 227)||Math.Abs(x.Length-y.Length)!=8)return false;
  int n=Math.Min(x.Length,y.Length);return x.Take(n).SequenceEqual(y.Take(n))&&x.Skip(n).Concat(y.Skip(n)).All(v=>v==0);
 }
 public static void Actions(string root,PARAM effects){
  Check(!effects.Rows.Any(r=>Convert.ToInt32(r["stateInfo"].Value) is >=Right and <=Sound),"Feedback state range collides with parameter effects");
  string dir=Path.Combine(root,".work/parry-fix/staged");Directory.CreateDirectory(dir);
  var native=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
  bool Counter(TAE.Event e)=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false)) is 173 or 174;
  var targets=new Dictionary<string,long[]>();
  var reactionEnds=new Dictionary<string,Dictionary<long,float>>();
  foreach(var f in native.Files.Where(f=>f.Name.EndsWith(".tae"))){
   var t=TAE.Read(f.Bytes);var ids=t.Animations.Where(a=>a.ID>=19200&&a.ID<=19495&&a.Events.Any(Guard)&&a.Events.Any(Counter)).Select(a=>a.ID).ToArray();
   if(ids.Length>0){targets[Path.GetFileName(f.Name)]=ids;reactionEnds[Path.GetFileName(f.Name)]=t.Animations.Where(a=>ids.Contains(a.ID)).ToDictionary(a=>a.ID,ReactionEnd);}
  }
  Check(targets.Values.Sum(a=>a.Length)==57&&targets.Values.SelectMany(a=>a).All(id=>id>=19200&&id<=19495),"Native guard-reaction catalogue changed");
  var executor=TAE.Read(native.Files.Single(f=>Path.GetFileName(f.Name)=="a00.tae").Bytes).Animations.Single(a=>a.ID==19480);
  var timing=executor.Events.Single(e=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false))==707000);
  bool Spark(TAE.Event e)=>e.Type==96&&BitConverter.ToInt32(e.GetParameterBytes(false))==462800;
  bool Audio(TAE.Event e)=>e.Type==129&&BitConverter.ToInt32(e.GetParameterBytes(false))==1&&BitConverter.ToInt32(e.GetParameterBytes(false),4)==809000;
  var spark=executor.Events.Single(Spark);var sound=executor.Events.Single(Audio);
  Check(spark.StartTime==0&&sound.StartTime==0&&BitConverter.ToInt16(sound.GetParameterBytes(false),16)==0,"Native feedback changed");
  var nativeSpark=spark.GetParameterBytes(false);
  Check(BitConverter.ToInt32(nativeSpark,8)==-1&&nativeSpark[14]==1,"Native spark instance policy changed");
  int addedFeedback=0;
  Check(timing.StartTime==0&&Math.Abs(timing.EndTime-5f/30f)<0.00001f,"Native Executor follow-up timing changed");
  string input=Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx");var b=BND4.Read(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  foreach(var f in b.Files.Where(f=>targets.ContainsKey(Path.GetFileName(f.Name)))){
   var t=TAE.Read(f.Bytes);
   foreach(long id in targets[Path.GetFileName(f.Name)]){
    var a=t.Animations.Single(a=>a.ID==id);a.Events.RemoveAll(Generated);
    var template=a.Events.First(Counter);var bytes=template.GetParameterBytes(false).ToArray();BitConverter.GetBytes(Marker).CopyTo(bytes,0);
    float end=reactionEnds[Path.GetFileName(f.Name)][id];
    Check(ReactionEnd(a)==end,"Installed native guard timing changed: "+id);
    a.Events.Add(new TAE.Event(0,end,67,template.Unk04,bytes,false){Group=template.Group});
    void Add(TAE.Event source,short condition,int? dummy){
     var raw=source.GetParameterBytes(false).ToArray();BitConverter.GetBytes(condition).CopyTo(raw,16);
     if(dummy.HasValue)BitConverter.GetBytes(dummy.Value).CopyTo(raw,4);
     // 4C1A80 inverts byte +0E into descriptor +0C. With slot=-1,
     // 4B3250 allocates a fresh instance on the TAE start edge only when
     // that descriptor flag is true. Executor's byte=1 reuses an old
     // instance by event address, swallowing sparks during rapid re-entry.
     // Keep duration, attachment and FFX intact; use native one-shot restart.
     if(source.Type==96)raw[14]=0;
     Check(source.Group!=null,"Native feedback group missing");
     var group=a.EventGroups.FirstOrDefault(g=>g.GroupType==source.Group!.GroupType);
     if(group==null){group=new TAE.EventGroup(source.Group!.GroupType){GroupData=source.Group!.GroupData};a.EventGroups.Add(group);}
     a.Events.Add(new TAE.Event(source.StartTime,source.EndTime,source.Type,source.Unk04,raw,false){Group=group});addedFeedback++;
    }
    // Executor's own reaction already contains these; preserve it once.
    if(!a.Events.Any(Spark)){Add(spark,Right,20);Add(spark,Left,21);}
    if(!a.Events.Any(Audio))Add(sound,Sound,null);
   }
   f.Bytes=t.Write();
  }
  string output=Path.Combine(dir,"c0000.anibnd.dcx");b.Write(output);var final=BND4.Read(output);
  foreach(var f in final.Files){
   Check(f.Bytes.Span.SequenceEqual(b.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Animation readback mismatch");
   if(!targets.ContainsKey(Path.GetFileName(f.Name))){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Other animation member changed");continue;}
   var old=TAE.Read(before[f.ID]);var now=TAE.Read(f.Bytes);Check(old.Animations.Count==now.Animations.Count,"Animation count changed");
   foreach(var pair in old.Animations.Zip(now.Animations)){
    Check(pair.First.ID==pair.Second.ID&&pair.First.AnimFileName==pair.Second.AnimFileName,"Animation identity changed");
    var x=pair.First.Events.Where(e=>!Generated(e)).ToArray();var y=pair.Second.Events.Where(e=>!Generated(e)).ToArray();
    Check(x.Length==y.Length&&x.Zip(y).All(v=>Same(v.First,v.Second)),"Original guard/attack event changed: "+pair.First.ID);
    Check(pair.Second.Events.Count(IsMarker)==(targets[Path.GetFileName(f.Name)].Contains(pair.Second.ID)?1:0),"Marker escaped guard reactions");
    foreach(var e in pair.Second.Events.Where(IsMarker))Check(e.StartTime==0&&e.EndTime==reactionEnds[Path.GetFileName(f.Name)][pair.Second.ID],"Wrong follow-up timing");
    bool target=targets[Path.GetFileName(f.Name)].Contains(pair.Second.ID);
    var feedback=pair.Second.Events.Where(IsFeedback).ToArray();
    Check(feedback.Length==(target?(x.Any(Spark)?0:2)+(x.Any(Audio)?0:1):0),"Wrong feedback count");
    foreach(var e in feedback){var raw=e.GetParameterBytes(false);short state=BitConverter.ToInt16(raw,16);
     Check(e.StartTime==0&&e.EndTime==(e.Type==96?spark.EndTime:sound.EndTime),"Feedback timing changed");
     Check(e.Type==96?Spark(e)&&state is Right or Left&&BitConverter.ToInt32(raw,4)==(state==Left?21:20):Audio(e)&&state==Sound,"Wrong feedback condition/hand");
     var expected=(e.Type==96?spark:sound).GetParameterBytes(false).ToArray();
     BitConverter.GetBytes(state).CopyTo(expected,16);
     if(e.Type==96){BitConverter.GetBytes(state==Left?21:20).CopyTo(expected,4);expected[14]=0;}
     Check(raw.SequenceEqual(expected),"Unexpected feedback parameter change");
    }
   }
  }
  // Check the whole archive, including manual-guard variants in TAE members
  // that have no recoil clips. A held guard must never supply REACTION.
  int manualClips=0;
  foreach(var f in final.Files.Where(f=>f.Name.EndsWith(".tae"))){
   foreach(var a in TAE.Read(f.Bytes).Animations){
    if(a.ID>=19000&&a.ID<19200){Check(!a.Events.Any(IsMarker),"Manual guard gained continuation: "+a.ID);manualClips++;}
    if(a.Events.Any(IsMarker))Check(targets.TryGetValue(Path.GetFileName(f.Name),out var ids)&&ids.Contains(a.ID),"Reaction marker escaped catalogue");
   }
  }
  Check(manualClips>0,"Native manual guard catalogue missing");
  File.WriteAllText(Path.Combine(dir,"actions.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),targets,reactionEnds,followupUntilNativeReactionExit=true,manualGuardClipsWithoutMarker=manualClips,nativeExecutorSeconds=timing.EndTime,markersWithinNativeGuard=true,originalEventsPreserved=true,addedFeedback,spark=462800,sound=809000,feedbackStates=new[]{Right,Left,Sound},sparkFreshInstanceOnStart=true,sparkTimingUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: 57 guard reactions; {addedFeedback} conditional native spark/sound events; original events preserved");
 }
}
