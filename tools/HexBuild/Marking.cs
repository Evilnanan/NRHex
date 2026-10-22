using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class Marking {
 public const int Owner=95004200,Attack=600000110,Stamp=300000290,Impact=9;
 static string Name(BinderFile file)=>file.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static bool Removal(EMEVD.Instruction i,int effect)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==effect;
 public static void Events(EMEVD events){
  foreach(int id in new[]{1111,9100941}){
   var e=events.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0&&!e.Instructions.Any(i=>Removal(i,Owner)),"Marking reset already present or parameterized");
   int index=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(index>0&&e.Instructions.Take(index).Any(i=>i.Bank==3),"Missing guarded player-ready anchor");
   e.Instructions.Insert(index,new EMEVD.Instruction(2004,21,new object[]{20000u,Owner}));
  }
 }
 static void ValidateEvents(EMEVD events){foreach(int id in new[]{1111,9100941})Check(events.Events.Single(e=>e.ID==id).Instructions.Count(i=>Removal(i,Owner))==1,"Marking cleanup missing or duplicated");}
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string name)=>parse(name,b.Files.Single(f=>Name(f)==name+".param").Bytes);
  var art=Table(native,"SwordArtsParam").Rows.Single(r=>r.ID==106);Check(Convert.ToInt32(art["atkParamId"].Value)==Stamp,"Native Upward Cut attack reference changed");
  var original=Table(native,"AtkParam_Pc");var attacks=Table(current,"AtkParam_Pc");
  Check(Convert.ToInt32(original.Rows.Single(r=>r.ID==Stamp)["dmgLevel"].Value)==Impact&&Convert.ToInt32(original.Rows.Single(r=>r.ID==Attack)["dmgLevel"].Value)==1,"Native Marking/Upward Cut impact changed");
  foreach(int id in new[]{Stamp,300000295,Attack,600000120})Check(CageAI.Same(original.Rows.Single(r=>r.ID==id),attacks.Rows.Single(r=>r.ID==id)),"Native attack changed");
  var effects=Table(current,"SpEffectParam");var expected=new PARAM.Row(Table(native,"SpEffectParam").Rows.Single(r=>r.ID==8000000)){ID=Owner};
  foreach(var(field,value)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",22295),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(expected,field,value);
  Check(CageAI.Same(expected,effects.Rows.Single(r=>r.ID==Owner)),"Owner is not the neutral persistent template");
  Check(RewardCatalog.HistoryBit(Owner)==39&&RewardCatalog.HistoricalOwners[38]==95004100,"History slots must not be reused");
  var good=Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==Owner);
  Check(Convert.ToInt32(good["goodsType"].Value)==11&&Convert.ToInt32(good["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(good["rarity"].Value)==3&&Convert.ToInt32(good["iconId"].Value)==20295,"Reward good mismatch");
  var item=Table(current,"ItemTableParam").Rows.Single(r=>r.ID==RewardCatalog.Table(Owner));
  Check(Convert.ToInt32(item["itemId"].Value)==Owner&&Convert.ToInt32(item["itemCategory"].Value)==1&&Convert.ToInt32(item["chanceWeight"].Value)==1000,"Reward wiring mismatch");
  var attach=Table(current,"AttachEffectParam").Rows.Single(r=>r.ID==Owner);var permanent=Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==Owner);
  Check(Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(attach["statusIconId"].Value)==22295&&Convert.ToInt32(permanent["spEffectId"].Value)==Owner&&Convert.ToInt32(permanent["iconId"].Value)==21295,"Ability icon/effect mismatch");
  return new{owner=Owner,attack=Attack,reference=Stamp,impact=Impact,ironeyeOnly=true,historyBit=39,historyBytes=(RewardCatalog.HistoricalOwners.Length+7)/8,sharedAttacksUnchanged=true};
 }
 static void Fixture(string directory,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(directory);var rows=parse("AtkParam_Pc",current.Files.Single(f=>Name(f)=="AtkParam_Pc.param").Bytes);rows.Rows=rows.Rows.Where(r=>new[]{Stamp,300000295,Attack,600000120}.Contains(r.ID)).ToList();File.WriteAllBytes(Path.Combine(directory,"native-attacks.param"),rows.Write());
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));Validate(native,current,parse);Fixture(Path.Combine(root,"work/marking"),current,parse);
  string input=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");
  if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(input).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Raw common does not match installed package");input=raw;}
  ValidateEvents(EMEVD.Read(input));Console.WriteLine("PASS: installed Marking owner, native Upward Cut impact, original attacks, reward wiring and guarded resets");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/marking/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Marking ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM row capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var expected=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count,"Row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Existing parameter row changed");}
  var audit=Validate(native,final,parse);Fixture(dir,final,parse);RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);Events(ev);ev.Write(eventOutput);var actualEv=EMEVD.Read(eventOutput);ValidateEvents(actualEv);
  Check(actualEv.Format==oldEv.Format&&actualEv.Compression==oldEv.Compression&&actualEv.Events.Count==oldEv.Events.Count&&actualEv.StringData.SequenceEqual(oldEv.StringData)&&actualEv.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");
  foreach(var old in oldEv.Events){var actual=actualEv.Events.Single(e=>e.ID==old.ID);Check(Cages.Same(ev.Events.Single(e=>e.ID==old.ID),actual),"Event readback mismatch");if(old.ID is 1111 or 9100941)actual.Instructions.RemoveAll(i=>Removal(i,Owner));Check(Cages.Same(old,actual),"Unrelated event instructions/parameters changed");}
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Order().ToArray()),rowCounts=tables.ToDictionary(p=>p.Key,p=>p.Value.Rows.Count),audit,existingRowsAndUnrelatedTablesUnchanged=true,onlyTwoGuardedCleanupEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Marking neutral owner, native Upward Cut reference, append-only reward and two guarded resets staged");
 }
}
