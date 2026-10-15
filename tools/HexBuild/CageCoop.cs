using SoulsFormats;
using System.Text.Json;

// Each peer runs the original player-count correction after its generator and
// enable/reset finish. Only cage copies use local correction effects: NPC HP
// packets carry current HP, while max HP is recomputed from local effects.
static class CageCoop {
 public const int Ready=95001603,CorrectionBase=95310000,EffectBase=95320000;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v){r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 static EMEVD.Instruction I(int b,int id,params object[] v)=>new(b,id,v);
 public static bool SameNativeNpc(PARAM.Row npc,PARAM.Row source){
  int original=Convert.ToInt32(source["multiPlayCorrectionParamId"].Value);
  if(Convert.ToInt32(npc["multiPlayCorrectionParamId"].Value)!=(original<0?original:original+CorrectionBase))return false;
  return npc.Cells.Where(c=>c!=null&&c.Def.InternalName is not "disableRespawn" and not "multiPlayCorrectionParamId").All(c=>JsonSerializer.Serialize(c.Value)==JsonSerializer.Serialize(source[c.Def.InternalName].Value));
 }
 public static void Parameters(Dictionary<string,PARAM> tables,Func<string,int,int,PARAM.Row> clone){
  var npcs=tables["NpcParam"].Rows.Where(r=>r.ID>=95300000&&r.ID<95300084).ToArray();Check(npcs.Length==84,"Expected 84 cage actors");
  var originalIds=npcs.Select(r=>Convert.ToInt32(r["multiPlayCorrectionParamId"].Value)).Where(id=>id>=0).Distinct().Order().ToArray();
  foreach(int source in originalIds){
   Check(source<10000,"Unexpected multiplayer correction source");var c=clone("MultiPlayCorrectionParam",source,CorrectionBase+source);
   foreach(string f in new[]{"client1SpEffectId","client2SpEffectId","client3SpEffectId"}){
    int id=Convert.ToInt32(c[f].Value);if(id<0)continue;Check(id<10000,"Unexpected native correction effect");
    int target=EffectBase+id;
    if(!tables["SpEffectParam"].Rows.Any(r=>r.ID==target)){var fx=clone("SpEffectParam",id,target);Check(Convert.ToInt32(fx["stateInfo"].Value)==282,"Correction must keep the native exclusive state");Set(fx,"isDisableNetSync",1);}
    Set(c,f,target);
   }
  }
  foreach(var npc in npcs){int id=Convert.ToInt32(npc["multiPlayCorrectionParamId"].Value);if(id>=0)Set(npc,"multiPlayCorrectionParamId",CorrectionBase+id);}
  var marker=clone("SpEffectParam",Cages.Mutation,Ready);Set(marker,"isDisableNetSync",1);Set(marker,"spCategory",20);
 }
 static void RemoveActivation(EMEVD.Event e,uint actor,int expected){
  Check(e.Parameters.Count==0,"Parameterized cage event");int count=0;
  for(int n=e.Instructions.Count-3;n>=0;n--){var a=e.Instructions[n];
   if(a.Bank!=1004||a.ID!=0||a.ArgData.Length!=20||a.ArgData[0]!=2||BitConverter.ToUInt32(a.ArgData,4)!=actor||BitConverter.ToInt32(a.ArgData,8)!=7999)continue;
   var b=e.Instructions[n+1];var c=e.Instructions[n+2];Check(b.Bank==2009&&b.ID==4&&BitConverter.ToUInt32(b.ArgData)==actor&&c.Bank==2004&&c.ID==8&&BitConverter.ToUInt32(c.ArgData)==actor&&BitConverter.ToInt32(c.ArgData,4)==7999,"Old scaling guard mismatch");e.Instructions.RemoveRange(n,3);count++;
  }
  Check(count==expected,"Unexpected scaling activation count");
 }
 public static void Birth(EMEVD.Event e,uint actor){
  RemoveActivation(e,actor,2);
  var reveals=e.Instructions.Select((i,n)=>(i,n)).Where(v=>v.i.Bank==2004&&v.i.ID==5&&BitConverter.ToUInt32(v.i.ArgData)==actor&&v.i.ArgData[4]==1).ToArray();Check(reveals.Length==1,"Expected common fresh/reconnect reveal");
  // Enabling a generated actor can reset its effects/sources. Activate AFTER
  // that operation, before collision/AI and the local shown flag/HUD.
  e.Instructions.InsertRange(reveals[0].n+1,[I(2009,4,actor),I(2004,8,actor,Ready)]);
 }
 public static void Battle(EMEVD.Event e,uint prefix,uint[] actors){
  foreach(uint actor in actors)RemoveActivation(e,actor,1);
  int at=e.Instructions.FindIndex(i=>i.Bank==1001&&i.ID==0&&BitConverter.ToSingle(i.ArgData)==CageUsage.IntroSeconds);Check(at>=0,"Battle intro wait missing");
  // The host cannot publish battle readiness before its own generated actors
  // have completed the same local correction/reveal as every replica.
  uint Shown(uint actor){uint index=actor-(prefix+9000);Check(index<84,"Unexpected cage actor");return prefix+(index<37?7600u:7750u)+index;}
  var conditions=actors.Select(actor=>I(3,0,(sbyte)4,(byte)1,(byte)0,Shown(actor))).Append(I(0,0,(sbyte)0,(byte)1,(sbyte)4));e.Instructions.InsertRange(at+1,conditions);
 }
 public static void Export(string root,Func<string,Memory<byte>,PARAM> parse,string? path=null){
  Directory.CreateDirectory(Path.Combine(root,".work/cage-hp-122"));
  var binder=SFUtil.DecryptNightreignRegulation(path??Path.Combine(root,"dist/mod/regulation.bin"));
  var tables=new[]{"NpcParam","SpEffectParam","MultiPlayCorrectionParam"}.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var ids=tables["NpcParam"].Rows.Where(r=>r.ID>=95300000&&r.ID<95300084).Select(r=>Convert.ToInt32(r["multiPlayCorrectionParamId"].Value)).Where(id=>id>=0).Distinct().Order().ToArray();Check(ids.Length>0&&ids.All(id=>id>=CorrectionBase),"Cage NPC has an unpatched correction reference");
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  foreach(int id in ids){var row=tables["MultiPlayCorrectionParam"].Rows.Single(r=>r.ID==id);var source=tables["MultiPlayCorrectionParam"].Rows.Single(r=>r.ID==id-CorrectionBase);var copy=new PARAM.Row(row){ID=source.ID};
   foreach(string f in new[]{"client1SpEffectId","client2SpEffectId","client3SpEffectId"}){
    int original=Convert.ToInt32(source[f].Value),current=Convert.ToInt32(row[f].Value);Check(current==(original<0?original:original+EffectBase),"Local effect reference mismatch");Set(copy,f,source[f].Value);
    if(original<0)continue;var fx=tables["SpEffectParam"].Rows.Single(r=>r.ID==current);var originalFx=tables["SpEffectParam"].Rows.Single(r=>r.ID==original);Check(Convert.ToInt32(fx["isDisableNetSync"].Value)==1,"Local correction must not be overwritten by replicated effects");var fxCopy=new PARAM.Row(fx){ID=original};Set(fxCopy,"isDisableNetSync",originalFx["isDisableNetSync"].Value);Check(CageAI.Same(fxCopy,originalFx),"Native multiplier/combat fields changed");
   }
   Check(CageAI.Same(copy,source),"Native multiplayer rule changed");
  }
  var effectIds=ids.SelectMany(id=>new[]{"client1SpEffectId","client2SpEffectId","client3SpEffectId"}.Select(f=>Convert.ToInt32(tables["MultiPlayCorrectionParam"].Rows.Single(r=>r.ID==id)[f].Value))).Where(id=>id>=0).Distinct().ToArray();
  File.WriteAllText(Path.Combine(root,".work/cage-hp-122/native-fixtures.json"),JsonSerializer.Serialize(new{corrections=ids.Select(id=>Dump(tables["MultiPlayCorrectionParam"].Rows.Single(r=>r.ID==id))),effects=effectIds.Select(id=>Dump(tables["SpEffectParam"].Rows.Single(r=>r.ID==id))),nativeFieldsAndMultipliersPreserved=true,localCorrectionEffects=true}));
  Console.WriteLine("PASS: native correction fields and local effect references read back");
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/cage-hp-122/staged/mod");Directory.CreateDirectory(Path.Combine(dir,"event"));
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"NpcParam","SpEffectParam","MultiPlayCorrectionParam"};var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));var originals=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Clone(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),"Parameter ID collision");var row=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(row);added[n].Add(id);return row;}
  Parameters(tables,Clone);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var read=SFUtil.DecryptNightreignRegulation(output);
  Check(read.Version==binder.Version&&read.Files.Count==binder.Files.Count&&read.Compression==binder.Compression,"Parameter binder metadata changed");
  foreach(var f in read.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");}
  foreach(string n in names){var actual=parse(n,read.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);Check(actual.Rows.Count==originals[n].Rows.Count+added[n].Count,"Unexpected parameter rows");foreach(var old in originals[n].Rows){var r=new PARAM.Row(actual.Rows.Single(v=>v.ID==old.ID));if(n=="NpcParam"&&old.ID>=95300000&&old.ID<95300084)Set(r,"multiPlayCorrectionParamId",old["multiPlayCorrectionParamId"].Value);Check(CageAI.Same(old,r),"Unrelated parameter field changed");}}
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block}_00_00",eventInput=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),eventOutput=Path.Combine(dir,"event",name+".emevd.dcx");uint p=(uint)(4600+block)*10000;
   var old=EMEVD.Read(eventInput);var ev=EMEVD.Read(eventInput);var changed=new HashSet<long>();
   foreach(var e in ev.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984)){uint actor=p+9000+(uint)(e.ID-p-9900);Birth(e,actor);changed.Add(e.ID);}
   foreach(var e in ev.Events.Where(e=>e.ID>=p+9710&&e.ID<p+9760)){uint[] actors=e.Instructions.Where(i=>i.Bank==2009&&i.ID==4).Select(i=>BitConverter.ToUInt32(i.ArgData)).ToArray();Battle(e,p,actors);changed.Add(e.ID);}
   Check(changed.Count==134,"Expected 84 births and 50 battles");ev.Write(eventOutput);var actual=EMEVD.Read(eventOutput);
   Check(actual.Events.Select(e=>e.ID).SequenceEqual(old.Events.Select(e=>e.ID))&&actual.Format==old.Format&&actual.Compression==old.Compression&&actual.StringData.SequenceEqual(old.StringData)&&actual.LinkedFileOffsets.SequenceEqual(old.LinkedFileOffsets),"Event metadata changed");
   Check(ev.Events.All(e=>Cages.Same(e,actual.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");Check(old.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Cages.Same(e,actual.Events.Single(v=>v.ID==e.ID))),"Unrelated event changed");
   reports.Add(new{name,prefix=p,inputSha256=Hash(eventInput),outputSha256=Hash(eventOutput),events=actual.Events.Where(e=>changed.Contains(e.ID)).Select(Cages.Dump)});
  }
  var corrections=tables["MultiPlayCorrectionParam"].Rows.Where(r=>added["MultiPlayCorrectionParam"].Contains(r.ID)).Select(r=>new{r.ID,source=r.ID-CorrectionBase,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)});
  var effects=tables["SpEffectParam"].Rows.Where(r=>added["SpEffectParam"].Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)});
  File.WriteAllText(Path.Combine(root,".work/cage-hp-122/validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),added=added.ToDictionary(v=>v.Key,v=>v.Value.Order().ToArray()),corrections,effects,reports,originalNPCCombatFieldsPreserved=true,originalCorrectionRowsPreserved=true,originalEffectsPreserved=true,unrelatedEventsAndMetadataPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Export(root,parse,output);
  Console.WriteLine("PASS: local native cage corrections and post-generation activation staged; original enemies, native multipliers, maps, rewards and other events preserved");
 }
}
