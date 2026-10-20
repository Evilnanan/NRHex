using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class UltimateChallenge {
 public const int Owner=95004000,Buff=Owner+1;
 static string Name(BinderFile file)=>Path.GetFileName(file.Name.Replace('\\','/'));
 static void Check(bool condition,string why){if(!condition)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var buff=clone("SpEffectParam",8000000,Buff);
  foreach(var(field,value)in new(string,object)[]{("maxHpRate",1.5f),("targetPriority",9999f),("effectEndurance",15f),("spCategory",20),("saveCategory",-1),("dontDeleteOnDead",0),("bCurrHPIndependeMaxHP",1),("iconId",20429),("spEffectTextId_1",Buff),("displayIconForPermamentEffect",0)})Set(buff,field,value);
 }
 static bool Removal(EMEVD.Instruction instruction,int effect)=>instruction.Bank==2004&&instruction.ID==21&&instruction.ArgData.Length==8&&BitConverter.ToInt32(instruction.ArgData)==20000&&BitConverter.ToInt32(instruction.ArgData,4)==effect;
 public static void Events(EMEVD events){
  foreach(int identity in new[]{1111,9100941}){
   var entry=events.Events.Single(entry=>entry.ID==identity);
   Check(entry.Parameters.Count==0&&!entry.Instructions.Any(instruction=>Removal(instruction,Owner)||Removal(instruction,Buff)),"Challenge cleanup anchor changed");
   int index=identity==1111?9:entry.Instructions.FindLastIndex(instruction=>instruction.Bank==2004&&instruction.ID==21)+1;
   Check(index>0&&entry.Instructions.Take(index).Any(instruction=>instruction.Bank==3),"Missing guarded reset");
   entry.Instructions.InsertRange(index,new[]{Owner,Buff}.Select(effect=>new EMEVD.Instruction(2004,21,new object[]{20000u,effect})));
  }
 }
 static void ValidateEvents(EMEVD events){
  foreach(int identity in new[]{1111,9100941})foreach(int effect in new[]{Owner,Buff})
   Check(events.Events.Single(entry=>entry.ID==identity).Instructions.Count(instruction=>Removal(instruction,effect))==1,"Missing challenge cleanup");
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 binder,string name)=>parse(name,binder.Files.Single(file=>Name(file)==name+".param").Bytes);
  var original=Table(native,"SpEffectParam");var effects=Table(current,"SpEffectParam");PARAM.Row expected=null!;
  Mechanics((name,source,identity)=>expected=new PARAM.Row(original.Rows.Single(row=>row.ID==source)){ID=identity});
  var buff=effects.Rows.Single(row=>row.ID==Buff);
  Check(CageAI.Same(expected,buff),"Challenge buff differs from reviewed settings");
  var owner=effects.Rows.Single(row=>row.ID==Owner);
  Check(Convert.ToSingle(owner["maxHpRate"].Value)==1f&&Convert.ToSingle(owner["targetPriority"].Value)==0f&&Convert.ToInt32(owner["saveCategory"].Value)==10&&Convert.ToInt32(owner["iconId"].Value)==22297,"Owner must be neutral and persistent");
  Check(RewardCatalog.HistoryBit(Owner)==37,"History slot changed");
  var table=Table(current,"ItemTableParam").Rows.Single(row=>row.ID==RewardCatalog.Table(Owner));
  Check(Convert.ToInt32(table["itemId"].Value)==Owner&&Convert.ToInt32(table["itemCategory"].Value)==1&&Convert.ToInt32(table["chanceWeight"].Value)==1000,"Reward wiring changed");
  var good=Table(current,"EquipParamGoods").Rows.Single(row=>row.ID==Owner);
  Check(Convert.ToInt32(good["goodsType"].Value)==11&&Convert.ToInt32(good["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(good["rarity"].Value)==3&&Convert.ToInt32(good["iconId"].Value)==20297,"Reward good mismatch");
  var attach=Table(current,"AttachEffectParam").Rows.Single(row=>row.ID==Owner);
  var permanent=Table(current,"PermanentBuffParam").Rows.Single(row=>row.ID==Owner);
  Check(Convert.ToInt32(attach["statusIconId"].Value)==22297&&Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(permanent["spEffectId"].Value)==Owner&&Convert.ToInt32(permanent["iconId"].Value)==21297,"Ultimate icons or reward effects changed");
  return new{owner=Owner,buff=Buff,seconds=15,maxHpRate=1.5,targetPriority=9999,historyBit=37};
 }
 static void Fixture(string directory,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  var effects=parse("SpEffectParam",current.Files.Single(file=>Name(file)=="SpEffectParam.param").Bytes);
  effects.Rows=effects.Rows.Where(row=>new[]{Owner,Buff,8000000,7033302}.Contains(row.ID)).ToList();
  Directory.CreateDirectory(directory);File.WriteAllBytes(Path.Combine(directory,"installed-effects.param"),effects.Write());
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  Validate(native,current,parse);ValidateEvents(EMEVD.Read(Path.Combine(root,"dist/mod/event/common.emevd.dcx")));
  Fixture(Path.Combine(root,"work/ultimate-challenge"),current,parse);
  Console.WriteLine("PASS: installed Ultimate Challenge 15-second HP/target-priority buff, icons, reward and reset wiring");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,"work/ultimate-challenge/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(name=>name,name=>parse(name,binder.Files.Single(file=>Name(file)==name+".param").Bytes));
  var originals=names.ToDictionary(name=>name,name=>parse(name,native.Files.Single(file=>Name(file)==name+".param").Bytes));
  var added=names.ToDictionary(name=>name,name=>new HashSet<int>());
  PARAM.Row Add(string name,int source,int identity){
   Check(!tables[name].Rows.Any(row=>row.ID==identity),"Challenge ID collision");
   var row=new PARAM.Row(originals[name].Rows.First(row=>row.ID==source)){ID=identity};tables[name].Rows.Add(row);added[name].Add(identity);return row;
  }
  var owner=Add("SpEffectParam",8000000,Owner);
  foreach(var(field,value)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",22297),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(owner,field,value);
  var attach=Add("AttachEffectParam",8000000,Owner);
  foreach(var(field,value)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",22297),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(attach,field,value);
  var permanent=Add("PermanentBuffParam",8000000,Owner);
  foreach(var(field,value)in new[]{("spEffectId",Owner),("iconId",21297),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(permanent,field,value);
  var good=Add("EquipParamGoods",8000001,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",20297);Set(good,"rarity",3);
  var table=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(table,"disableParam_NT",0);Set(table,"itemId",Owner);Set(table,"chanceWeight",1000);
  Mechanics(Add);
  foreach(string name in names){Check(tables[name].Rows.Count<65536,"Parameter row capacity exceeded");tables[name].Rows=tables[name].Rows.OrderBy(row=>row.ID).ToList();binder.Files.Single(file=>Name(file)==name+".param").Bytes=tables[name].Write();}
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var file in final.Files){
   var intended=binder.Files.Single(candidate=>candidate.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Binder readback mismatch");
   var name=names.SingleOrDefault(name=>Name(file)==name+".param");
   if(name==null){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}
   var old=parse(name,before[file.ID]);var current=parse(name,file.Bytes);
   Check(current.Rows.Count==old.Rows.Count+added[name].Count&&old.Rows.OrderBy(row=>row.ID).Zip(current.Rows.Where(row=>!added[name].Contains(row.ID)).OrderBy(row=>row.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing parameter row changed");
  }
  var audit=Validate(native,final,parse);Fixture(directory,final,parse);RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(directory,"common.emevd.dcx");
  var events=EMEVD.Read(eventInput);var oldEvents=EMEVD.Read(eventInput);Events(events);events.Write(eventOutput);
  var actualEvents=EMEVD.Read(eventOutput);ValidateEvents(actualEvents);
  Check(actualEvents.Format==oldEvents.Format&&actualEvents.Compression==oldEvents.Compression&&actualEvents.Events.Count==oldEvents.Events.Count&&actualEvents.StringData.SequenceEqual(oldEvents.StringData)&&actualEvents.LinkedFileOffsets.SequenceEqual(oldEvents.LinkedFileOffsets),"Event metadata changed");
  foreach(var old in oldEvents.Events){
   var actual=actualEvents.Events.Single(entry=>entry.ID==old.ID);Check(Cages.Same(events.Events.Single(entry=>entry.ID==old.ID),actual),"Event readback mismatch");
   if(old.ID is 1111 or 9100941)actual.Instructions.RemoveAll(instruction=>Removal(instruction,Owner)||Removal(instruction,Buff));
   Check(Cages.Same(old,actual),"Unrelated event changed");
  }
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{Owner,95001300});
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,addedRows=added.ToDictionary(pair=>pair.Key,pair=>pair.Value.Order().ToArray()),audit,existingRowsAndUnrelatedTablesUnchanged=true,onlyTwoGuardedCleanupEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Ultimate Challenge staged; original parameters/events preserved; dist untouched");
 }
}
