using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class Basics {
 public const int Owner=95004400;
 public static readonly int[] FamilyIdle={705061,705071,705081};
 static string Name(BinderFile file)=>file.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static bool Removal(EMEVD.Instruction i,int effect)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==effect;
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  float[] rates={0.7f,0.8f,0.5f};
  for(int i=0;i<3;i++){var r=clone("SpEffectParam",8000000,Owner+1+i);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("effectEndurance",2f),("saveCategory",-1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("characterSkillCooldownReduction",rates[i])})Set(r,f,v);}
  FamilyMechanics(clone);
 }
 public static void FamilyMechanics(Func<string,int,int,PARAM.Row> clone){
  for(int i=0;i<3;i++){var r=clone("SpEffectParam",FamilyIdle[i],Owner+4+i);Set(r,"motionInterval",Convert.ToSingle(r["motionInterval"].Value)*0.5f);}
 }
 public static void Events(EMEVD events){
  foreach(int id in new[]{1111,9100941}){
   var e=events.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0&&!e.Instructions.Any(i=>Removal(i,Owner)),"Basics reset already present or parameterized");
   int index=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;Check(index>0&&e.Instructions.Take(index).Any(i=>i.Bank==3),"Missing guarded player-ready anchor");
   e.Instructions.InsertRange(index,Enumerable.Range(0,4).Select(n=>new EMEVD.Instruction(2004,21,new object[]{20000u,Owner+n})));
  }
 }
 static void ValidateEvents(EMEVD events){foreach(int id in new[]{1111,9100941})foreach(int n in Enumerable.Range(0,4))Check(events.Events.Single(e=>e.ID==id).Instructions.Count(i=>Removal(i,Owner+n))==1,"Basics cleanup missing or duplicated");}
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string name)=>parse(name,b.Files.Single(f=>Name(f)==name+".param").Bytes);
  var effects=Table(current,"SpEffectParam");var original=Table(native,"SpEffectParam");
  for(int i=0;i<3;i++){var r=effects.Rows.Single(r=>r.ID==Owner+1+i);Check(Convert.ToSingle(r["characterSkillCooldownReduction"].Value)==new[]{0.7f,0.8f,0.5f}[i]&&Convert.ToSingle(r["effectEndurance"].Value)==2f&&Convert.ToInt32(r["saveCategory"].Value)==-1,"Native skill cooldown multiplier mismatch");}
  foreach(int id in new[]{7000800,7000801,7000802,7290000,7290001,705060,705070,705080})Check(CageAI.Same(original.Rows.Single(r=>r.ID==id),effects.Rows.Single(r=>r.ID==id)),"Native relic changed");
  for(int i=0;i<3;i++){
   var nativeRow=original.Rows.Single(r=>r.ID==FamilyIdle[i]);Check(CageAI.Same(nativeRow,effects.Rows.Single(r=>r.ID==FamilyIdle[i])),"Native family regeneration changed");
   var expected=new PARAM.Row(nativeRow){ID=Owner+4+i};Set(expected,"motionInterval",Convert.ToSingle(nativeRow["motionInterval"].Value)*0.5f);
   Check(CageAI.Same(expected,effects.Rows.Single(r=>r.ID==Owner+4+i)),"Private inactive effect differs beyond its healing interval");
   Check(Convert.ToSingle(nativeRow["changeHpRate"].Value)==-1f&&Convert.ToSingle(nativeRow["motionInterval"].Value)==new[]{0.6f,1.5f,2f}[i],"Native family healing baseline changed");
  }
  Check(Convert.ToInt32(effects.Rows.Single(r=>r.ID==Owner)["iconId"].Value)==22297,"Wrong ability status icon");
  Check(RewardCatalog.HistoryBit(Owner)==41&&RewardCatalog.HistoricalOwners[40]==95004300,"History slots must not be reused");
  var good=Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==Owner);var attach=Table(current,"AttachEffectParam").Rows.Single(r=>r.ID==Owner);var permanent=Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==Owner);
  Check(Convert.ToInt32(good["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(good["iconId"].Value)==20297&&Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(permanent["iconId"].Value)==21297,"Ultimate-series reward wiring mismatch");
  return new{owner=Owner,cooldownMultipliers=new[]{0.7f,0.8f,0.5f},familyDamageMultiplier=2f,familyReloadSpeed=2f,historyBit=41};
 }
 static void Fixture(string directory,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(directory);var rows=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);rows.Rows=rows.Rows.Where(r=>r.ID>=Owner&&r.ID<=Owner+6||new[]{7000800,7000801,7000802,705060,705070,705080}.Concat(FamilyIdle).Contains(r.ID)).ToList();File.WriteAllBytes(Path.Combine(directory,"installed-effects.param"),rows.Write());
 }
 public static void StageFamily(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/basics-family/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var member=b.Files.Single(f=>Name(f)=="SpEffectParam.param");var old=parse("SpEffectParam",member.Bytes);var rows=parse("SpEffectParam",member.Bytes);
  var originals=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  FamilyMechanics((name,source,id)=>{Check(name=="SpEffectParam"&&!rows.Rows.Any(r=>r.ID==id),"Family effect ID collision");var row=new PARAM.Row(originals.Rows.Single(r=>r.ID==source)){ID=id};rows.Rows.Add(row);return row;});
  rows.Rows=rows.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=rows.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in actual.Files){var intended=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter serialization mismatch");if(f.ID!=member.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var now=parse("SpEffectParam",f.Bytes);Check(now.Rows.Count==old.Rows.Count+3&&old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>r.ID<Owner+4||r.ID>Owner+6).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Old effect rows changed");}
  var audit=Validate(native,actual,parse);Fixture(dir,actual,parse);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),addedEffects=Enumerable.Range(4,3).Select(n=>Owner+n),onlyNewEffectsAdded=true,originalRowsAndUnrelatedTablesPreserved=true,audit,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: three private native family idle effects staged; only regeneration intervals halved");
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));Validate(native,current,parse);Fixture(Path.Combine(root,"work/basics"),current,parse);
  string input=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");
  if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(input).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Raw common does not match installed package");input=raw;}
  ValidateEvents(EMEVD.Read(input));Console.WriteLine("PASS: installed Back to Basics cooldowns, reward wiring and guarded resets");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/basics/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Basics ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Mechanics(Add);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM row capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var expected=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count,"Row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Existing parameter row changed");}
  var audit=Validate(native,final,parse);Fixture(dir,final,parse);RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);Events(ev);ev.Write(eventOutput);var actualEv=EMEVD.Read(eventOutput);ValidateEvents(actualEv);
  Check(actualEv.Format==oldEv.Format&&actualEv.Compression==oldEv.Compression&&actualEv.Events.Count==oldEv.Events.Count&&actualEv.StringData.SequenceEqual(oldEv.StringData)&&actualEv.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");
  foreach(var old in oldEv.Events){var actual=actualEv.Events.Single(e=>e.ID==old.ID);Check(Cages.Same(ev.Events.Single(e=>e.ID==old.ID),actual),"Event readback mismatch");if(old.ID is 1111 or 9100941)actual.Instructions.RemoveAll(i=>Enumerable.Range(0,4).Any(n=>Removal(i,Owner+n)));Check(Cages.Same(old,actual),"Unrelated event instructions/parameters changed");}
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Order().ToArray()),rowCounts=tables.ToDictionary(p=>p.Key,p=>p.Value.Rows.Count),audit,existingRowsAndUnrelatedTablesUnchanged=true,onlyTwoGuardedCleanupEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Back to Basics owner, native cooldown multipliers, append-only reward and guarded resets staged");
 }
}
