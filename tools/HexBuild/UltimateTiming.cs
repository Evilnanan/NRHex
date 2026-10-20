using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class UltimateTiming {
 public const int Pulse=95000404;
 static readonly long[] Starts={599010,600000,600005,600100,600200,605000,610000,615000,620000,620005,630000,635000,640000,650000,660000,670000,680000,690000,695000};
 static string Name(BinderFile file)=>Path.GetFileName(file.Name.Replace('\\','/'));
 static void Check(bool condition,string message){if(!condition)throw new InvalidDataException(message);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static bool Effect(TAE.Event entry,int identity)=>entry.Type is 66 or 67 or 302&&entry.GetParameterBytes(false).Length>=4&&BitConverter.ToInt32(entry.GetParameterBytes(false))==identity;
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var row=clone("SpEffectParam",8000000,Pulse);
  foreach(var(field,value)in new(string,object)[]{("maxHpRate",1f),("effectEndurance",0.1f),("spCategory",20),("saveCategory",-1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(row,field,value);
 }
 static Dictionary<(string,long),TAE.Event> Anchors(BND4 native){
  var selected=new Dictionary<(string,long),TAE.Event>();
  foreach(var file in native.Files.Where(file=>Name(file).EndsWith(".tae"))){
   foreach(var animation in TAE.Read(file.Bytes).Animations.Where(animation=>animation.Events.Any(entry=>Effect(entry,6999615)))){
    var costs=animation.Events.Where(entry=>entry.Type==1000).ToArray();
    foreach(var cost in costs)Check(BitConverter.ToSingle(cost.GetParameterBytes(false)) is 70f or 100f||animation.ID==670010&&BitConverter.ToSingle(cost.GetParameterBytes(false))==0.3f,"Unexpected ultimate cost");
    if(!Starts.Contains(animation.ID)){Check(costs.Length==0||animation.ID==670010,"Unreviewed ultimate start");continue;}
    Check(costs.Length==1&&BitConverter.ToSingle(costs[0].GetParameterBytes(false)) is 70f or 100f,"Start cost changed");
    selected.Add((Name(file),animation.ID),costs[0]);
   }
  }
  Check(selected.Count==Starts.Length&&selected.Keys.Select(key=>key.Item2).Order().SequenceEqual(Starts.Order()),"Ultimate animation coverage changed");
  return selected;
 }
 static bool Same(TAE.Event left,TAE.Event right){
  if(left.Type!=right.Type||left.StartTime!=right.StartTime||left.EndTime!=right.EndTime||left.Unk04!=right.Unk04||left.Group?.GroupType!=right.Group?.GroupType)return false;
  var before=left.GetParameterBytes(false);var after=right.GetParameterBytes(false);int size=Math.Min(before.Length,after.Length);
  return Math.Abs(before.Length-after.Length)<=8&&before.Take(size).SequenceEqual(after.Take(size))&&before.Skip(size).Concat(after.Skip(size)).All(value=>value==0);
 }
 static void ValidateActions(BND4 original,BND4 patched,Dictionary<(string,long),TAE.Event> anchors){
  Check(original.Version==patched.Version&&original.Format==patched.Format&&original.Compression==patched.Compression&&original.Files.Count==patched.Files.Count,"Animation binder metadata changed");
  int count=0;
  foreach(var file in patched.Files){
   var before=original.Files.Single(candidate=>candidate.ID==file.ID);Check(file.Name==before.Name&&file.Flags==before.Flags,"Animation member metadata changed");
   if(!anchors.Keys.Any(key=>key.Item1==Name(file))){Check(file.Bytes.Span.SequenceEqual(before.Bytes.Span),"Unrelated animation member changed");continue;}
   var old=TAE.Read(before.Bytes);var current=TAE.Read(file.Bytes);
   Check(old.Animations.Count==current.Animations.Count,"Animation count changed");
   foreach(var animation in current.Animations){
    var previous=old.Animations.Single(candidate=>candidate.ID==animation.ID);
    var pulses=animation.Events.Where(entry=>Effect(entry,Pulse)).ToArray();
    if(anchors.TryGetValue((Name(file),animation.ID),out var cost)){
     Check(pulses.Length==1&&pulses[0].Type==302&&pulses[0].StartTime==cost.StartTime&&pulses[0].EndTime==cost.EndTime&&pulses[0].Group?.GroupType==302,"Formal-activation pulse mismatch");
     Check(animation.Events.FindIndex(entry=>entry==pulses[0])>animation.Events.FindIndex(entry=>entry.Type==1000),"Pulse must follow native consumption");
     Check(animation.EventGroups.Count==previous.EventGroups.Count+1,"Pulse group count mismatch");
     animation.EventGroups.Remove(pulses[0].Group!);animation.Events.Remove(pulses[0]);count++;
    }else Check(pulses.Length==0,"Pulse added to follow-up or drain animation");
    Check(animation.Events.Count==previous.Events.Count&&animation.Events.Zip(previous.Events).All(pair=>Same(pair.First,pair.Second)),"Existing event changed");
    for(int index=0;index<animation.Events.Count;index++)animation.Events[index].SetParameterBytes(false,previous.Events[index].GetParameterBytes(false));
   }
   Check(current.Write().SequenceEqual(old.Write()),"Existing TAE header, miniheader, groups or animation metadata changed");
  }
  Check(count==Starts.Length,"Formal-activation pulse coverage mismatch");
 }
 public static void Actions(string root,string? destination=null){
  string input=Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx"),directory=destination??Path.Combine(root,"work/ultimate-timing/staged");Directory.CreateDirectory(directory);
  var native=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));var anchors=Anchors(native);
  var binder=BND4.Read(input);var before=BND4.Read(input);
  var template=TAE.Read(native.Files.Single(file=>Name(file)=="a909.tae").Bytes).Animations.Single(animation=>animation.ID==690000).Events.Single(entry=>entry.Type==302);
  foreach(var file in binder.Files.Where(file=>anchors.Keys.Any(key=>key.Item1==Name(file)))){
   var tae=TAE.Read(file.Bytes);
   foreach(var animation in tae.Animations.Where(animation=>anchors.ContainsKey((Name(file),animation.ID)))){
    var anchor=anchors[(Name(file),animation.ID)];
    Check(animation.Events.Count(entry=>entry.Type==1000)==1&&Same(animation.Events.Single(entry=>entry.Type==1000),anchor),"Installed consumption event differs from native");
    Check(!animation.Events.Any(entry=>Effect(entry,Pulse)),"Timing patch already installed; verify rather than duplicate");
    var group=new TAE.EventGroup(302){GroupData=template.Group!.GroupData};animation.EventGroups.Add(group);
    var bytes=new byte[16];BitConverter.GetBytes(Pulse).CopyTo(bytes,0);
    animation.Events.Add(new TAE.Event(anchor.StartTime,anchor.EndTime,302,template.Unk04,bytes,false){Group=group});
   }
   file.Bytes=tae.Write();
  }
  string output=Path.Combine(directory,"c0000.anibnd.dcx");binder.Write(output);var final=BND4.Read(output);ValidateActions(before,final,anchors);
  File.WriteAllText(Path.Combine(directory,"actions-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),pulse=Pulse,entryOnlyEvent=302,starts=anchors.Select(pair=>new{file=pair.Key.Item1,animation=pair.Key.Item2,time=pair.Value.StartTime}),originalEventsUnchanged=true,drainAndFollowupsExcluded=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: 19 one-shot formal-activation markers; original events and follow-ups unchanged");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,"work/ultimate-timing/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var member=binder.Files.Single(file=>Name(file)=="SpEffectParam.param");var effects=parse("SpEffectParam",member.Bytes);
  var original=parse("SpEffectParam",native.Files.Single(file=>Name(file)=="SpEffectParam.param").Bytes);
  Mechanics((name,source,identity)=>{Check(!effects.Rows.Any(row=>row.ID==identity),"Pulse ID collision");var row=new PARAM.Row(original.Rows.Single(row=>row.ID==source)){ID=identity};effects.Rows.Add(row);return row;});
  effects.Rows=effects.Rows.OrderBy(row=>row.ID).ToList();member.Bytes=effects.Write();
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var file in final.Files){
   var intended=binder.Files.Single(candidate=>candidate.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Regulation readback mismatch");
   if(file.ID!=member.ID){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated parameter table changed");continue;}
   var old=parse("SpEffectParam",before[file.ID]);var current=parse("SpEffectParam",file.Bytes);
   Check(current.Rows.Count==old.Rows.Count+1&&old.Rows.All(row=>CageAI.Same(row,current.Rows.Single(candidate=>candidate.ID==row.ID))),"Existing effect changed");
  }
  File.WriteAllText(Path.Combine(directory,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),addedEffect=Pulse,existingRowsUnchanged=true},new JsonSerializerOptions{WriteIndented=true}));
  Actions(root,directory);Console.WriteLine("PASS: formal-activation resources staged; dist untouched");
 }
}
