using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class MarkingPoison {
 public const int Carrier=95004201,MarkHit=95004202,Burst=95004203,Begin=95004204,Source=7500202;
 static readonly long[] Starts={520000,520003,525000};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 public static void OwnerEffect(PARAM.Row row){Set(row,"stateInfo",152);Set(row,"atkOccurrenceSpEffectId",Carrier);}
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  foreach(int id in new[]{Carrier,MarkHit,Begin}){
   var row=clone("SpEffectParam",8000000,id);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("effectEndurance",0f),("saveCategory",-1),("spCategory",20),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0),("isDisableNetSync",1)})Set(row,f,v);
  }
  var burst=clone("SpEffectParam",Source,Burst);Set(burst,"invocationConditionsStateChange1",0);
 }
 static BND4 ActionsInput(string root){
  string path=Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx");var raw=Environment.GetEnvironmentVariable("NRHEX_MARKING_ACTION_RAW");
  if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(path).Equals(Environment.GetEnvironmentVariable("NRHEX_MARKING_ACTION_SHA256"),StringComparison.OrdinalIgnoreCase),"Raw animation fixture does not match current package");return BND4.Read(raw);}return BND4.Read(path);
 }
 static bool Pulse(TAE.Event e)=>e.Type==302&&e.GetParameterBytes(false).Length>=4&&BitConverter.ToInt32(e.GetParameterBytes(false))==Begin;
 static void CheckActions(BND4 bundle){
  var tae=TAE.Read(bundle.Files.Single(f=>Name(f)=="a902.tae").Bytes);
  foreach(long id in Starts){var a=tae.Animations.Single(a=>a.ID==id);Check(a.Events.Count(Pulse)==1&&a.Events.Single(Pulse).StartTime==0&&a.Events.Single(Pulse).EndTime==1f/30f,"Marking begin pulse mismatch");Check(a.Events.Count(e=>e.Type==1001)==1,"Marking skill start changed");}
  Check(bundle.Files.Where(f=>Name(f).EndsWith(".tae")).Sum(f=>TAE.Read(f.Bytes).Animations.Sum(a=>a.Events.Count(Pulse)))==3,"Unexpected Marking pulse outside skill starts");
 }
 static object StageActions(string root,string output){
  var before=ActionsInput(root);var b=ActionsInput(root);var file=b.Files.Single(f=>Name(f)=="a902.tae");var tae=TAE.Read(file.Bytes);
  Check(!tae.Animations.SelectMany(a=>a.Events).Any(Pulse),"Marking begin pulse already installed");
  foreach(long id in Starts){
   var a=tae.Animations.Single(a=>a.ID==id);Check(a.Events.Count(e=>e.Type==1001)==1&&a.Events.Any(e=>e.Type==1),"Native Marking start/collision missing");
  }
  file.Bytes=TaeEventAppend.PrependEffect(file.Bytes.ToArray(),Starts,Begin,1f/30f);b.Write(output);var final=BND4.Read(output);CheckActions(final);
  Check(final.Version==before.Version&&final.Format==before.Format&&final.Compression==before.Compression&&final.Files.Count==before.Files.Count,"Animation binder metadata changed");
  foreach(var member in final.Files){
   var old=before.Files.Single(f=>f.ID==member.ID);Check(member.Name==old.Name&&member.Flags==old.Flags,"Animation member metadata changed");
   if(member.ID!=file.ID){Check(member.Bytes.Span.SequenceEqual(old.Bytes.Span),"Unrelated animation member changed");continue;}
   var restored=TAE.Read(member.Bytes);foreach(var a in restored.Animations.Where(a=>Starts.Contains(a.ID))){var pulse=a.Events.Single(Pulse);a.Events.Remove(pulse);a.EventGroups.Remove(pulse.Group!);}
   var previous=TAE.Read(old.Bytes);Check(restored.Animations.Count==previous.Animations.Count,"Animation count changed");
   foreach(var a in restored.Animations){var prior=previous.Animations.Single(p=>p.ID==a.ID);Check(a.Events.Count==prior.Events.Count,"Event count changed");
    foreach(var pair in a.Events.Zip(prior.Events)){var x=pair.First;var y=pair.Second;var xb=x.GetParameterBytes(false);var yb=y.GetParameterBytes(false);int n=Math.Min(xb.Length,yb.Length);
     Check(x.Type==y.Type&&x.StartTime==y.StartTime&&x.EndTime==y.EndTime&&x.Unk04==y.Unk04&&x.Group?.GroupType==y.Group?.GroupType&&Math.Abs(xb.Length-yb.Length)<=8&&xb.Take(n).SequenceEqual(yb.Take(n))&&xb.Skip(n).Concat(yb.Skip(n)).All(v=>v==0),"Existing animation event changed");
     x.SetParameterBytes(false,yb);
    }
   }
   Check(restored.Write().AsSpan().SequenceEqual(previous.Write()),"Existing animation metadata changed");
  }
  return new{starts=Starts,effect=Begin,onlyBeginPulsesAdded=true,originalEventsAndAnimationsPreserved=true,originalBinaryLayoutPreserved=true,outputSha256=Hash(output)};
 }
 public static void Actions(string root){
  try{CheckActions(ActionsInput(root));Console.WriteLine("PASS: Marking animation begin observers already installed");return;}catch(InvalidDataException){}
  string dir=Path.Combine(root,"work/marking-poison/actions");Directory.CreateDirectory(dir);string output=Path.Combine(dir,"c0000.anibnd.dcx");StageActions(root,output);File.Copy(output,Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx"),true);
  Console.WriteLine("PASS: Marking animation begin observers installed; native motion retained");
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse,string dir){
  var original=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);var effects=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  Check(Convert.ToInt32(original.Rows.Single(r=>r.ID==Source)["invocationConditionsStateChange1"].Value)==2,"Native poison condition changed");
  PARAM.Row Expected(string table,int source,int id)=>new(original.Rows.Single(r=>r.ID==source)){ID=id};
  var expected=new Dictionary<int,PARAM.Row>();PARAM.Row Clone(string table,int source,int id){var row=Expected(table,source,id);expected.Add(id,row);return row;}Mechanics(Clone);
  foreach(var(id,row)in expected)Check(CageAI.Same(row,effects.Rows.Single(r=>r.ID==id)),"Private Marking effect mismatch");
  foreach(int id in new[]{6902,6903,7500200,7500201,Source,8320700})Check(CageAI.Same(original.Rows.Single(r=>r.ID==id),effects.Rows.Single(r=>r.ID==id)),"Native Marking/poison reference changed");
  var owner=effects.Rows.Single(r=>r.ID==Marking.Owner);Check(Convert.ToInt32(owner["stateInfo"].Value)==152&&Convert.ToInt32(owner["atkOccurrenceSpEffectId"].Value)==Carrier,"Marking carrier missing");
  var sourceVfx=parse("SpEffectVfxParam",native.Files.Single(f=>Name(f)=="SpEffectVfxParam.param").Bytes);var currentVfx=parse("SpEffectVfxParam",current.Files.Single(f=>Name(f)=="SpEffectVfxParam.param").Bytes);Check(CageAI.Same(sourceVfx.Rows.Single(r=>r.ID==8500202),currentVfx.Rows.Single(r=>r.ID==8500202)),"Native poison burst visual changed");
  Directory.CreateDirectory(dir);effects.Rows=effects.Rows.Where(r=>expected.ContainsKey(r.ID)||new[]{Marking.Owner,Source,8320700}.Contains(r.ID)).ToList();File.WriteAllBytes(Path.Combine(dir,"installed-effects.param"),effects.Write());
  return new{source=Source,burst=Burst,requiresPoison=false,nativeDamageAndVisualPreserved=true,damageRate=effects.Rows.Single(r=>r.ID==Burst)["changeHpRate"].Value,damagePoints=effects.Rows.Single(r=>r.ID==Burst)["changeHpPoint"].Value,vfx=8500202};
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){Validate(native,SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin")),parse,Path.Combine(root,"work/marking-poison"));CheckActions(ActionsInput(root));Console.WriteLine("PASS: Marking native poison burst clone, backstep-attack reference and three guarded skill starts");}
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,"work/marking-poison/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var file=b.Files.Single(f=>Name(f)=="SpEffectParam.param");var rows=parse("SpEffectParam",file.Bytes);var old=parse("SpEffectParam",file.Bytes);var source=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  var owner=rows.Rows.Single(r=>r.ID==Marking.Owner);Check(Convert.ToInt32(owner["stateInfo"].Value)==0&&Convert.ToInt32(owner["atkOccurrenceSpEffectId"].Value)==-1,"Unexpected Marking owner");OwnerEffect(owner);
  var added=new HashSet<int>();PARAM.Row Clone(string table,int from,int id){Check(!rows.Rows.Any(r=>r.ID==id),"Marking effect ID collision");var row=new PARAM.Row(source.Rows.Single(r=>r.ID==from)){ID=id};rows.Rows.Add(row);added.Add(id);return row;}Mechanics(Clone);
  Check(rows.Rows.Count<65536,"Effect row capacity exceeded");rows.Rows=rows.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=rows.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var member in final.Files){var wanted=b.Files.Single(f=>f.ID==member.ID);Check(member.Name==wanted.Name&&member.Flags==wanted.Flags&&member.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Regulation readback mismatch");if(member.ID!=file.ID)Check(member.Bytes.Span.SequenceEqual(before[member.ID]),"Unrelated table changed");}
  var actual=parse("SpEffectParam",final.Files.Single(f=>f.ID==file.ID).Bytes);Check(actual.Rows.Count==old.Rows.Count+added.Count,"Effect row count changed");foreach(var previous in old.Rows){var expected=new PARAM.Row(previous);if(previous.ID==Marking.Owner)OwnerEffect(expected);Check(CageAI.Same(expected,actual.Rows.Single(r=>r.ID==previous.ID)),"Unrelated effect row/field changed");}
  var audit=Validate(native,final,parse,dir);var actions=StageActions(root,Path.Combine(dir,"c0000.anibnd.dcx"));HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Marking.Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),addedRows=added.Order().ToArray(),audit,actions,otherRowsFieldsTablesAndAnimationsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("PASS: Marking poison burst parameters, skill starts and bilingual text staged");
 }
}
