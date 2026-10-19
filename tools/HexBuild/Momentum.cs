using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class Momentum {
 public const int Owner=95003700,Pulse=Owner+1,Weak=Owner+2,Visual=Owner+3;
 public const float Bonus=1.3f,WeakAttack=.5f/Bonus;
 public const int AttackVisualSource=320500,WeakVisualSource=8800271;
 static readonly string[] AttackFields={"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"};
 static readonly string[] CutFields={"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void OwnerEffect(PARAM.Row r){
  Set(r,"stateInfo",2103);Set(r,"atkOccurrenceSpEffectId",Pulse);Set(r,"unknown_241a",0);
  foreach(string f in AttackFields)Set(r,f,Bonus);
  foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(r,f,1);
 }
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,PARAM native){
  foreach(int id in new[]{Pulse,Weak,Visual}){
   var r=clone("SpEffectParam",8000000,id);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",-1),("effectEndurance",id==Pulse?0f:2f),("dontDeleteOnDead",1),("isDisableNetSync",id==Pulse?1:0),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);
   if(id==Weak){foreach(string f in AttackFields)Set(r,f,WeakAttack);foreach(string f in CutFields)Set(r,f,1.5f);foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(r,f,1);}
   if(id!=Pulse){var source=native.Rows.Single(r=>r.ID==(id==Weak?WeakVisualSource:AttackVisualSource));foreach(string f in new[]{"vfxId","vfxId1"})Set(r,f,source[f].Value);}
  }
 }
 public static void Probe(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var sp=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  var ids=new[]{8000000,8600200,320500,8670000,8800271,8831221,7037701,312501,320800,8970050,8970051,8970052,8970053,8970054,8970055};
  Directory.CreateDirectory(Path.Combine(root,"work/momentum"));
  File.WriteAllText(Path.Combine(root,"work/momentum/native-effects.json"),JsonSerializer.Serialize(sp.Rows.Where(r=>ids.Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),new JsonSerializerOptions{WriteIndented=true}));
  sp.Rows=sp.Rows.Where(r=>ids.Contains(r.ID)).ToList();File.WriteAllBytes(Path.Combine(root,"work/momentum/native-effects.param"),sp.Write());
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  var original=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);var actual=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  foreach(int id in new[]{320500,8670000,AttackVisualSource,WeakVisualSource})Check(CageAI.Same(original.Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"Native effect source changed");
  Ambulance.VerifyRecovery(original,actual);
  var owner=actual.Rows.Single(r=>r.ID==Owner);var weak=actual.Rows.Single(r=>r.ID==Weak);
  Check(Convert.ToInt32(owner["stateInfo"].Value)==2103&&Convert.ToInt32(owner["atkOccurrenceSpEffectId"].Value)==Pulse,"Native hit notification missing");
  foreach(string f in AttackFields){Check(Convert.ToSingle(owner[f].Value)==Bonus&&Math.Abs(Convert.ToSingle(owner[f].Value)*Convert.ToSingle(weak[f].Value)-.5f)<.000001,"Incorrect native phase attack multiplier");}
  foreach(string f in CutFields)Check(Convert.ToSingle(weak[f].Value)==1.5f,"Incorrect vulnerability rate");
  foreach(int id in new[]{Weak,Visual}){
   var row=actual.Rows.Single(r=>r.ID==id);var source=original.Rows.Single(r=>r.ID==(id==Weak?WeakVisualSource:AttackVisualSource));
   foreach(string f in new[]{"vfxId","vfxId1"})Check(Convert.ToInt32(row[f].Value)==Convert.ToInt32(source[f].Value),"Native phase visual mismatch");
   Check(Convert.ToInt32(row["isDisableNetSync"].Value)==0&&Convert.ToSingle(row["effectEndurance"].Value)==2f&&Convert.ToInt32(row["saveCategory"].Value)==-1,"Phase lease metadata mismatch");
  }
  // The native full-HP bonus uses AttackRate, leaving base AttackPowerRate neutral.
  foreach(string f in AttackFields)Check(Convert.ToSingle(original.Rows.Single(r=>r.ID==320500)[f].Value)>1f,"Native full-HP attack reference changed");
  Check(Convert.ToInt32(owner["unknown_241a"].Value)==0,"All damage elements must qualify for native hit notification");
  foreach(string f in new[]{"physicsAttackPowerRate","magicAttackPowerRate","fireAttackPowerRate","thunderAttackPowerRate","darkAttackPowerRate"})Check(Convert.ToSingle(owner[f].Value)==1f&&Convert.ToSingle(weak[f].Value)==1f,"Base attack parameters must remain neutral");
  Check(RewardCatalog.HistoryBit(Owner)==34&&RewardCatalog.HistoricalOwners.Length>=35,"Stable history must append");
  return new{owner=Owner,positiveMultiplier=Bonus,negativeMultiplier=.5f,incomingMultiplier=1.5f,attackVisualSource=AttackVisualSource,weakVisualSource=WeakVisualSource,historyBit=34};
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));Validate(native,current,parse);
  var sp=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);sp.Rows=sp.Rows.Where(r=>new[]{Owner,Pulse,Weak,Visual,320500}.Contains(r.ID)).ToList();Directory.CreateDirectory(Path.Combine(root,"work/momentum"));File.WriteAllBytes(Path.Combine(root,"work/momentum/installed-effects.param"),sp.Write());
  Console.WriteLine("PASS: Momentum installed native +30%/-50% attack rates, 1.5 vulnerability and phase VFX");}
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/momentum/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Momentum ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);OwnerEffect(s);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Mechanics(Add,originals["SpEffectParam"]);
  Ambulance.RecoveryEffect(Add,originals["SpEffectParam"]);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM row capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var expected=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count,"Row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Existing parameter row changed");}
  var audit=Validate(native,final,parse);RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),hexCount=hexes.Length,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Count),rowCounts=tables.ToDictionary(p=>p.Key,p=>p.Value.Rows.Count),audit,existingRowsAndUnrelatedTablesUnchanged=true,eventsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Momentum owner, native attack rates, hit cycle and phase VFX and bilingual text staged; all prior rows preserved");
 }
}
