using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class HeartMist {
 public const int Owner=95002700,Cooldown=Owner+3,Pulse=Owner+6,Mist=Owner+7;
 static readonly int[] LegacyEvents={9100970,9100971,9100972};
 static readonly int[] Cleanup={Owner+1,Owner+2,Cooldown,Owner+4,Owner+5,Mist};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void OwnerEffect(PARAM.Row row,PARAM.Row native){
  var keep=new[]{"iconId","spEffectTextId_1","displayIconForPermamentEffect","saveCategory","spCategory"}.ToDictionary(f=>f,f=>row[f].Value);
  foreach(var c in native.Cells)row[c.Def.InternalName].Value=c.Value;
  foreach(var (f,v)in keep)Set(row,f,v);
  Set(row,"atkOccurrenceSpEffectId",Pulse);
 }
 public static void Support(Func<string,int,int,PARAM.Row> clone){
  // Retired saved IDs remain neutral and have no timers/replacement chains.
  foreach(int id in Enumerable.Range(Owner+1,6)){
   var r=clone("SpEffectParam",8000000,id);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0),("saveCategory",id==Cooldown?0:-1),("isDisableNetSync",1),("effectEndurance",id==Cooldown?30f:id==Pulse?0f:-1f),("dontDeleteOnDead",id==Cooldown?1:0)})Set(r,f,v);
   if(id==Pulse)Set(r,"effectTargetAttacker",1);
  }
  var mist=clone("SpEffectParam",8630302,Mist);
  // Do not reset the player's separate native on-attack accumulator.
  Set(mist,"replaceSpEffectId",-1);
 }
 static bool Init(EMEVD.Instruction i)=>i.Bank==2000&&i.ID==0&&i.ArgData.Length>=8&&LegacyEvents.Contains(BitConverter.ToInt32(i.ArgData,4));
 public static void Events(EMEVD ev){
  var ctor=ev.Events.Single(e=>e.ID==0);Check(ctor.Parameters.Count==0,"Common constructor became parameterized");
  ctor.Instructions.RemoveAll(Init);ev.Events.RemoveAll(e=>LegacyEvents.Contains((int)e.ID));
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   var missing=Cleanup.Where(effect=>!e.Instructions.Any(i=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData,4)==effect)).ToArray();
   int index=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(index>0,"Missing expedition reset anchor");
   e.Instructions.InsertRange(index,missing.Select(effect=>new EMEVD.Instruction(2004,21,new object[]{20000u,effect})));
   foreach(var p in e.Parameters)if(p.InstructionIndex>=index)p.InstructionIndex+=missing.Length;
  }
 }
 static void ValidateEvents(EMEVD ev){
  Check(!ev.Events.Any(e=>LegacyEvents.Contains((int)e.ID)),"Retired branch award/return event survives");
  Check(!ev.Events.Single(e=>e.ID==0).Instructions.Any(Init),"Retired return event is initialized");
  foreach(int id in new[]{1111,9100941})foreach(int effect in Cleanup)Check(ev.Events.Single(e=>e.ID==id).Instructions.Count(i=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData,4)==effect)==1,"Missing/duplicate expedition cleanup");
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var origin=Table(native,"SpEffectParam");var sp=Table(current,"SpEffectParam");var owner=sp.Rows.Single(r=>r.ID==Owner);
  var reference=origin.Rows.Single(r=>r.ID==8320400);var expected=new PARAM.Row(reference){ID=Owner};
  foreach(string f in new[]{"iconId","spEffectTextId_1","displayIconForPermamentEffect","saveCategory","spCategory"})Set(expected,f,owner[f].Value);
  Set(expected,"atkOccurrenceSpEffectId",Pulse);expected.Name=owner.Name;Check(CageAI.Same(owner,expected),"Native charged-hit filters changed");
  Check(Convert.ToInt32(owner["magicSubCategoryChange1"].Value)==100&&Convert.ToInt32(owner["stateInfo"].Value)==2103,"Charged hit selector missing");
  Check(Convert.ToInt32(sp.Rows.Single(r=>r.ID==Pulse)["effectTargetAttacker"].Value)==1,"Pulse cannot reach attacker");
  Check(Convert.ToSingle(sp.Rows.Single(r=>r.ID==Cooldown)["effectEndurance"].Value)==30f&&Convert.ToInt32(sp.Rows.Single(r=>r.ID==Cooldown)["replaceSpEffectId"].Value)==-1&&Convert.ToInt32(sp.Rows.Single(r=>r.ID==Cooldown)["dontDeleteOnDead"].Value)==1,"Cooldown is not neutral 30-second death-preserved effect");
  var mist=sp.Rows.Single(r=>r.ID==Mist);var mistExpected=new PARAM.Row(origin.Rows.Single(r=>r.ID==8630302)){ID=Mist,Name=mist.Name};Set(mistExpected,"replaceSpEffectId",-1);Check(CageAI.Same(mist,mistExpected),"Native charm emission metadata changed");
  foreach(string n in new[]{"Bullet","AtkParam_Pc"}){
   var a=Table(native,n);var b=Table(current,n);
   foreach(int id in new[]{12335000,12335001,10335000,10335001,13335000,13335010}.Where(id=>a.Rows.Any(r=>r.ID==id)))Check(CageAI.Same(a.Rows.Single(r=>r.ID==id),b.Rows.Single(r=>r.ID==id)),"Native charm or branch row changed");
  }
  foreach(int id in new[]{10335000,13335000,13335010})Check(Convert.ToInt32(Table(current,"Bullet").Rows.Single(r=>r.ID==id)["spEffectIDForShooter"].Value)==0,"Retired branch notification remains");
  Check(CageAI.Same(Table(native,"BehaviorParam_PC").Rows.Single(r=>r.ID==863030000),Table(current,"BehaviorParam_PC").Rows.Single(r=>r.ID==863030000)),"Native charm behavior changed");
  return new{owner=Owner,pulse=Pulse,cooldown=Cooldown,mist=Mist,chargedCategory=100,cooldownSeconds=30,nativeEmitter=8630302,nativeBehavior=863030000,nativeBullets=new[]{12335000,12335001},nativeCharmEffect=503350,nativeSourceParametersUnchanged=true};
 }
 static void Fixture(string root,BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  var directory=Path.Combine(root,"work/heartstealer-rework");Directory.CreateDirectory(directory);
  var sp=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  var orig=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  sp.Rows=sp.Rows.Where(r=>new[]{Owner,Cooldown,Pulse,Mist}.Contains(r.ID)).Concat(orig.Rows.Where(r=>new[]{8320400,8630302}.Contains(r.ID))).OrderBy(r=>r.ID).ToList();
  File.WriteAllBytes(Path.Combine(directory,"installed-effects.param"),sp.Write());
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));var proof=Validate(native,current,parse);
  var eventPath=Path.Combine(root,"dist/mod/event/common.emevd.dcx");
  var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");
  if(raw!=null){Check(Hash(eventPath).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Decompressed common event does not match installed container");eventPath=raw;}
  ValidateEvents(EMEVD.Read(eventPath));Fixture(root,native,current,parse);
  File.WriteAllText(Path.Combine(root,"work/heartstealer-rework/installed-check.json"),JsonSerializer.Serialize(proof,new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Heartstealer charged-hit filters, 30-second cooldown, original charm chain and retired branch events verified");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var directory=Path.Combine(root,"work/heartstealer-rework/staged");Directory.CreateDirectory(directory);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var current=SFUtil.DecryptNightreignRegulation(input);var before=current.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=current.Files.Single(f=>Name(f)=="SpEffectParam.param");var sp=parse("SpEffectParam",member.Bytes);var old=parse("SpEffectParam",member.Bytes);
  var nativeSp=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  OwnerEffect(sp.Rows.Single(r=>r.ID==Owner),nativeSp.Rows.Single(r=>r.ID==8320400));
  Support((n,source,id)=>{
   var baseline=nativeSp.Rows.Single(r=>r.ID==source);var row=new PARAM.Row(baseline){ID=id};
   var previous=sp.Rows.SingleOrDefault(r=>r.ID==id);if(previous!=null)sp.Rows.Remove(previous);else Check(id==Pulse||id==Mist,"Unexpected helper added");sp.Rows.Add(row);return row;
  });
  sp.Rows=sp.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=sp.Write();
  var bulletMember=current.Files.Single(f=>Name(f)=="Bullet.param");var bullets=parse("Bullet",bulletMember.Bytes);
  foreach(int id in new[]{10335000,13335000,13335010}){var row=bullets.Rows.Single(r=>r.ID==id);Check(Convert.ToInt32(row["spEffectIDForShooter"].Value)==Owner+2,"Unexpected branch source");Set(row,"spEffectIDForShooter",0);}bulletMember.Bytes=bullets.Write();
  var output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,current,current.Compression);var reopened=SFUtil.DecryptNightreignRegulation(output);var proof=Validate(native,reopened,parse);
  foreach(var f in reopened.Files){Check(f.Bytes.Span.SequenceEqual(current.Files.Single(g=>g.ID==f.ID).Bytes.Span),"Regulation serialization mismatch");if(f.ID!=member.ID&&f.ID!=bulletMember.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");}
  var read=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==member.ID).Bytes);Check(read.Rows.Count==old.Rows.Count+2,"Unexpected effect count");foreach(var row in old.Rows.Where(r=>r.ID<Owner||r.ID>Owner+7))Check(CageAI.Same(row,read.Rows.Single(r=>r.ID==row.ID)),"Unrelated effect changed");
  var previousBullets=parse("Bullet",before[bulletMember.ID]);var actualBullets=parse("Bullet",reopened.Files.Single(f=>f.ID==bulletMember.ID).Bytes);
  foreach(var row in previousBullets.Rows){var actual=new PARAM.Row(actualBullets.Rows.Single(r=>r.ID==row.ID));if(new[]{10335000,13335000,13335010}.Contains(row.ID))Set(actual,"spEffectIDForShooter",row["spEffectIDForShooter"].Value);Check(CageAI.Same(row,actual),"Unrelated bullet field changed");}
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(directory,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var eventBefore=EMEVD.Read(eventInput);
  Check(LegacyEvents.All(id=>ev.Events.Any(e=>e.ID==id)),"Expected old branch events missing");Events(ev);ev.Write(eventOutput);var actualEv=EMEVD.Read(eventOutput);ValidateEvents(actualEv);
  Check(JsonSerializer.Serialize(ev)==JsonSerializer.Serialize(actualEv),"Event round-trip mismatch");
  foreach(var e in eventBefore.Events.Where(e=>!LegacyEvents.Contains((int)e.ID))){var actual=actualEv.Events.Single(v=>v.ID==e.ID);if(e.ID is not (0 or 1111 or 9100941))Check(JsonSerializer.Serialize(e)==JsonSerializer.Serialize(actual),"Unrelated common event changed");else{var expected=new EMEVD.Event(e.ID,e.RestBehavior){Instructions=e.Instructions.ToList(),Parameters=e.Parameters};if(e.ID==0){expected.Instructions.RemoveAll(Init);Check(JsonSerializer.Serialize(expected.Instructions)==JsonSerializer.Serialize(actual.Instructions),"Constructor unrelated instruction changed");}else{var instructions=actual.Instructions.Where(i=>!(i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData,4)==Mist)).ToArray();Check(JsonSerializer.Serialize(e.Instructions)==JsonSerializer.Serialize(instructions),"Reset unrelated instruction changed");}}}
  Fixture(root,native,reopened,parse);HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),proof,removedEvents=LegacyEvents,otherRowsTablesAndEventsPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Heartstealer charm rework staged; native rows, unrelated params/events and bilingual text preserved");
 }
}
