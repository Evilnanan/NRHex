using SoulsFormats;
using System.Text.Json;

static partial class ContractHeart {
 public const int Contract=95002600, Heart=95002700;
 public const int SelfDamage=Contract+1, Receipt=Heart+1, Used=Heart+2, Cooldown=Heart+3, Return=Heart+4, Second=Heart+5;
 public const int InitialLot=95127000, ReturnLot=95127010;
 static readonly int[] BranchBullets={10335000,13335000,13335010};
 static readonly int[] EventIds={9100970,9100971,9100972};
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v){r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());}
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  // Native on-hit effects select the attacker. The DLL consumes this marker
  // and executes one native HP cost plus the separate hit-effect notification.
  var pulse=clone("SpEffectParam",8600105,SelfDamage);
  Set(pulse,"changeHpPoint",0);Set(pulse,"changeHpRate",0f);Set(pulse,"bCurrHPIndependeMaxHP",0);Set(pulse,"isDisableNetSync",1);
  foreach(int id in new[]{Receipt,Used,Cooldown,Return,Second}){
   var r=clone("SpEffectParam",8000000,id);
   Set(r,"maxHpRate",1f);Set(r,"spCategory",20);Set(r,"iconId",-1);Set(r,"spEffectTextId_1",-1);Set(r,"displayIconForPermamentEffect",0);
   Set(r,"saveCategory",id==Used?-1:id==Receipt?10:0);Set(r,"effectEndurance",id==Cooldown?60f:-1f);
   if(id is Cooldown or Return or Second)Set(r,"dontDeleteOnDead",1);
   if(id is Used or Second)Set(r,"isDisableNetSync",1);
   if(id==Cooldown)Set(r,"replaceSpEffectId",Return);
  }
 }
 public static void Owners(PARAM.Row contract){Set(contract,"stateInfo",2103);Set(contract,"atkOccurrenceSpEffectId",SelfDamage);foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(contract,f,0);}
 public static void Branches(PARAM bullets){
  foreach(int id in BranchBullets){var r=bullets.Rows.Single(r=>r.ID==id);Check(Convert.ToInt32(r["spEffectIDForShooter"].Value)==0,"Native branch shooter effect changed");Set(r,"spEffectIDForShooter",Used);}
 }
 public static void Lots(PARAM original,Func<string,int,int,PARAM.Row> clone){
  var template=original.Rows.First(r=>Convert.ToInt32(r["lotItemCategory01"].Value)==1);
  foreach(int id in new[]{InitialLot,ReturnLot}){
   var r=clone("ItemLotParam_map",template.ID,id);
   for(int i=1;i<=8;i++){Set(r,$"lotItemId{i:00}",-1);Set(r,$"lotItemCategory{i:00}",0);Set(r,$"lotItemBasePoint{i:00}",0);Set(r,$"lotItemNum{i:00}",0);}
   Set(r,"lotItemId01",3350);Set(r,"lotItemCategory01",1);Set(r,"lotItemBasePoint01",1000);Set(r,"lotItemNum01",id==InitialLot?2:1);Set(r,"getItemFlagId",0);Set(r,"cumulateNumFlagId",0);Set(r,"cumulateNumMax",0);
  }
 }
 static EMEVD.Instruction I(int bank,int id,params object[] values)=>new(bank,id,values);
 static EMEVD.Instruction Has(int group,int effect,bool on)=>I(4,5,(sbyte)group,20000u,effect,(byte)(on?1:0),(sbyte)0,1f);
 static EMEVD.Instruction Wait()=>I(0,0,(sbyte)0,(byte)1,(sbyte)1);
 static EMEVD.Instruction Give(int lot)=>I(2003,4,lot);
 static EMEVD.Event UseEvent(){
  var e=new EMEVD.Event(EventIds[1],EMEVD.Event.RestBehaviorType.Restart);
  // Keep a notification arriving at settlement until ReturnEvent clears the
  // previous request; it then starts the next interval instead of being lost.
  e.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Used,true),Wait(),I(1004,2,(byte)1,20000u,Return,(byte)1,(sbyte)0,1),I(2004,21,20000u,Used),I(1004,2,(byte)1,20000u,Heart,(byte)0,(sbyte)0,1),I(1004,0,(byte)2,20000u,Cooldown,(byte)1,(sbyte)0,1),I(2004,8,20000u,Cooldown),I(1000,4,(byte)1),I(1004,2,(byte)1,20000u,Second,(byte)1,(sbyte)0,1),I(2004,8,20000u,Second),I(1000,4,(byte)1)]);
  return e;
 }
 static EMEVD.Event ReturnEvent(){
  var e=new EMEVD.Event(EventIds[2],EMEVD.Event.RestBehaviorType.Restart);
  e.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Heart,true),Has(1,Return,true),Wait(),I(2004,21,20000u,Return),Give(ReturnLot),I(1004,2,(byte)1,20000u,Second,(byte)0,(sbyte)0,1),I(2004,21,20000u,Second),Give(ReturnLot),I(1000,4,(byte)1)]);
  return e;
 }
 public static void Events(EMEVD ev){
  var parent=ev.Events.Single(e=>e.ID==0);Check(parent.Parameters.Count==0,"Common constructor is parameterized");
  foreach(int id in EventIds){Check(!ev.Events.Any(e=>e.ID==id),"Heartstealer event collision");parent.Instructions.Insert(0,I(2000,0,0,id,0));}
  var initial=new EMEVD.Event(EventIds[0],EMEVD.Event.RestBehaviorType.Restart);
  initial.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Heart,true),Has(1,Receipt,false),Wait(),I(2004,8,20000u,Receipt),Give(InitialLot),Has(0,Heart,false),I(1000,4,(byte)1)]);
  var used=UseEvent();var returned=ReturnEvent();
  ev.Events.AddRange([initial,used,returned]);
  // Per-expedition receipts and pending returns follow existing deployment
  // guards, not pointer changes or revival. Hub reset also removes stale state.
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);int index=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(index>0,"Missing lifecycle reset anchor");e.Instructions.InsertRange(index,new[]{Receipt,Used,Cooldown,Return,Second}.Select(v=>I(2004,21,20000u,v)));
   foreach(var p in e.Parameters)if(p.InstructionIndex>=index)p.InstructionIndex+=5;
  }
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var directory=Path.Combine(root,".work/contract-heart/staged");Directory.CreateDirectory(directory);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Regulation version mismatch");
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy","ItemLotParam_map","Bullet"};
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"ID collision: {n} {id}");var r=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  foreach(var h in hexes.Where(h=>h.Id is Contract or Heart)){
   var s=Add("SpEffectParam",8000000,h.Id);Set(s,"maxHpRate",1f);Set(s,"spCategory",20);Set(s,"iconId",h.SmallIcon);Set(s,"spEffectTextId_1",h.Id);Set(s,"displayIconForPermamentEffect",0);if(h.Id==Contract)Owners(s);
   var a=Add("AttachEffectParam",95002500,h.Id);Set(a,"passiveSpEffectId_1",h.Id);Set(a,"attachTextId",h.Id);Set(a,"overrideBaseEffectId",h.Id);Set(a,"statusIconId",h.SmallIcon);
   var p=Add("PermanentBuffParam",95002500,h.Id);Set(p,"spEffectId",h.Id);Set(p,"iconId",h.LargeIcon);
   var g=Add("EquipParamGoods",95002500,h.Id);Set(g,"vagrantItemLotId",h.Id);Set(g,"iconId",h.LargeIcon-1000);
   var t=Add("ItemTableParam",RewardCatalog.Table(95002500),RewardCatalog.Table(h.Id));Set(t,"itemId",h.Id);
  }
  Mechanics(Add);Branches(tables["Bullet"]);Lots(tables["ItemLotParam_map"],Add);
  var oldLots=tables["ItemLotParam_enemy"].Rows.ToDictionary(r=>r.ID);
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(oldLots.TryGetValue(id,out var existing))Check(Convert.ToInt32(existing["lotItemId01"].Value)==target,"Existing reward triple moved");
   else{var r=Add("ItemLotParam_enemy",95200000+slot,id);Set(r,"lotItemId01",target);}
  }
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  string destination=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(destination,binder,binder.Compression);var final=SFUtil.DecryptNightreignRegulation(destination);
  foreach(var f in final.Files){Check(f.Name==binder.Files.Single(v=>v.ID==f.ID).Name&&f.Bytes.Span.SequenceEqual(binder.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Regulation readback failed");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  foreach(string n in names){var actual=parse(n,final.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);Check(actual.Rows.Count==baseline[n].Rows.Count+added[n].Count,"Unexpected row count");foreach(var pair in baseline[n].Rows.OrderBy(r=>r.ID).Zip(actual.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID))){var old=pair.First;var r=pair.Second;if(n=="Bullet"&&BranchBullets.Contains(r.ID)){var copy=new PARAM.Row(r);Set(copy,"spEffectIDForShooter",old["spEffectIDForShooter"].Value);Check(CageAI.Same(old,copy),"Branch modification escaped shooter notification");}else Check(CageAI.Same(old,r),"Existing row changed");}}
  File.WriteAllText(Path.Combine(directory,"mechanics.json"),JsonSerializer.Serialize(new{effects=tables["SpEffectParam"].Rows.Where(r=>r.ID>=Contract&&r.ID<=Second).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),lots=tables["ItemLotParam_map"].Rows.Where(r=>r.ID is InitialLot or ReturnLot).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})},new JsonSerializerOptions{WriteIndented=true}));
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(directory,"common.emevd.dcx");var events=EMEVD.Read(eventInput);var eventBefore=EMEVD.Read(eventInput);Events(events);events.Write(eventOutput);var readEvents=EMEVD.Read(eventOutput);
  Check(events.Events.Zip(readEvents.Events).All(p=>JsonSerializer.Serialize(p.First)==JsonSerializer.Serialize(p.Second)),"Event instruction/argument/parameter readback failed");
  File.WriteAllText(Path.Combine(directory,"events.json"),JsonSerializer.Serialize(readEvents.Events.Where(e=>EventIds.Contains((int)e.ID)).Select(e=>new{e.ID,e.RestBehavior,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})}),new JsonSerializerOptions{WriteIndented=true}));
  Check(readEvents.Format==eventBefore.Format&&readEvents.Compression==eventBefore.Compression&&readEvents.StringData.SequenceEqual(eventBefore.StringData)&&readEvents.LinkedFileOffsets.SequenceEqual(eventBefore.LinkedFileOffsets)&&readEvents.Events.Count==eventBefore.Events.Count+3,"Event metadata changed");
  foreach(var old in eventBefore.Events){var current=readEvents.Events.Single(e=>e.ID==old.ID);if(old.ID==0)current.Instructions.RemoveAll(i=>i.Bank==2000&&i.ID==0&&EventIds.Contains(BitConverter.ToInt32(i.ArgData,4)));if(old.ID is 1111 or 9100941)current.Instructions.RemoveAll(i=>i.Bank==2004&&i.ID==21&&new[]{Receipt,Used,Cooldown,Return,Second}.Contains(BitConverter.ToInt32(i.ArgData,4)));Check(JsonSerializer.Serialize(old)==JsonSerializer.Serialize(current),"Existing event behavior changed");}
  RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(destination),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexes,added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),branchBullets=BranchBullets,originalRowsAndRewardTriplesPreserved=true,originalEventsPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Ominous Pact and Heartstealer staged; native parameter/event/reward/text readback passed");
 }
 public static void ExportChecks(string root,Func<string,Memory<byte>,PARAM> parse,string? regulation=null,string? eventFile=null){
  var dir=Path.Combine(root,".work/contract-heart/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(regulation??Path.Combine(root,"dist/mod/regulation.bin"));
  var tables=new[]{"SpEffectParam","ItemLotParam_map","Bullet"}.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  foreach(int id in BranchBullets)Check(Convert.ToInt32(tables["Bullet"].Rows.Single(r=>r.ID==id)["spEffectIDForShooter"].Value)==Used,"Branch use notification missing");
  File.WriteAllText(Path.Combine(dir,"mechanics.json"),JsonSerializer.Serialize(new{effects=tables["SpEffectParam"].Rows.Where(r=>r.ID>=Contract&&r.ID<=Second).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),lots=tables["ItemLotParam_map"].Rows.Where(r=>r.ID is InitialLot or ReturnLot).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})},new JsonSerializerOptions{WriteIndented=true}));
  var ev=EMEVD.Read(eventFile??Path.Combine(root,"dist/mod/event/common.emevd.dcx"));
  var ctor=ev.Events.Single(e=>e.ID==0);
  Check(EventIds.All(id=>ctor.Instructions.Count(i=>i.Bank==2000&&i.ID==0&&BitConverter.ToInt32(i.ArgData,4)==id)==1),"Local events not initialized exactly once");
  foreach(int id in new[]{1111,9100941})Check(new[]{Receipt,Used,Cooldown,Return,Second}.All(effect=>ev.Events.Single(e=>e.ID==id).Instructions.Count(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==effect)==1),"Lifecycle cleanup missing");
  File.WriteAllText(Path.Combine(dir,"events.json"),JsonSerializer.Serialize(ev.Events.Where(e=>EventIds.Contains((int)e.ID)).Select(e=>new{e.ID,e.RestBehavior,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})}),new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: branch notifications, local event initialization and lifecycle cleanup verified; check fixtures exported");
 }
 public static void StageDamageMarker(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/contract-damage/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var f=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var sp=parse("SpEffectParam",f.Bytes);var old=parse("SpEffectParam",f.Bytes);
  var pulse=sp.Rows.Single(r=>r.ID==SelfDamage);Check(Convert.ToInt32(pulse["effectTargetAttacker"].Value)==1&&Convert.ToInt32(pulse["changeHpPoint"].Value)==0,"Contract pulse route changed");
  Set(pulse,"changeHpRate",0f);Set(pulse,"isDisableNetSync",1);f.Bytes=sp.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var result=SFUtil.DecryptNightreignRegulation(output);
  Check(result.Version==binder.Version&&result.Files.Count==binder.Files.Count&&result.Compression==binder.Compression,"Binder metadata changed");
  foreach(var member in result.Files){var intended=binder.Files.Single(v=>v.ID==member.ID);Check(member.Name==intended.Name&&member.Flags==intended.Flags&&member.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(member.ID!=f.ID)Check(member.Bytes.Span.SequenceEqual(before[member.ID]),"Unrelated table changed");}
  var actual=parse("SpEffectParam",result.Files.Single(v=>v.ID==f.ID).Bytes);Check(old.Rows.Count==actual.Rows.Count,"Unexpected effect row count");
  foreach(var p in old.Rows.Zip(actual.Rows)){var r=new PARAM.Row(p.Second);if(r.ID==SelfDamage)foreach(string field in new[]{"changeHpRate","isDisableNetSync"})Set(r,field,p.First[field].Value);Check(CageAI.Same(p.First,r),"Unrelated effect field changed");}
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedEffect=SelfDamage,changedFields=new[]{"changeHpRate","isDisableNetSync"},nativeDamageCriteria=12,otherFieldsRowsAndTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  ExportChecks(root,parse,output);
  Console.WriteLine("PASS: Contract request is now a neutral local marker; delayed HP deduction removed; all other fields/rows/tables preserved");
 }
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var tables=new[]{"SpEffectParam","EquipParamGoods","Bullet","AtkParam_Pc","ItemLotParam_enemy","ItemLotParam_map"}.ToDictionary(n=>n,n=>parse(n,original.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var report=new Dictionary<string,object>();
  int[] effects={6820400,6820401,6820500,6820501,8000000,1753,1760,1763,1765,1604004,8600100,8600105,8600200,8600201,8600202,8600205,8600206,8600207,7122000,7122001,7170100,7170101,8610700,8610701,8610702,511086};
  report["SpEffectParam"]=tables["SpEffectParam"].Rows.Where(r=>effects.Contains(r.ID)).Select(Dump).ToArray();
  report["EquipParamGoods"]=tables["EquipParamGoods"].Rows.Where(r=>r.ID==3350).Select(Dump).ToArray();
  report["Bullet"]=tables["Bullet"].Rows.Where(r=>r.ID is 10335000 or 10335001).Select(Dump).ToArray();
  report["AtkParam_Pc"]=tables["AtkParam_Pc"].Rows.Where(r=>r.ID is 7 or 11101701).Select(Dump).ToArray();
  report["ItemLotParam_map"]=tables["ItemLotParam_map"].Rows.Take(12).Select(Dump).ToArray();
  report["BranchLots"]=tables["ItemLotParam_enemy"].Rows.Where(r=>Enumerable.Range(1,8).Any(i=>Convert.ToInt32(r[$"lotItemId{i:00}"].Value)==3350)).Select(Dump).ToArray();
  var dir=Path.Combine(root,".work/contract-heart");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"native-parameters.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Read-only native contract/self-damage and Bewitching Branch parameters exported");
 }
}
