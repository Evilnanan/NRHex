using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class Snack {
 public const int Owner=95003600,First=96110000,Lot=95420000,PremiumLot=95420010;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static int N(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  foreach(int first in new[]{Lot,PremiumLot})for(int option=0;option<3;option++){
   var r=clone("ItemLotParam_enemy",6100002,first+option);Set(r,"lotItem_Rarity",-1);
   // Native lot lottery selects one pool for this choice, then the native
   // table resolves its class/eligibility filters. No reward permutations.
   for(int i=1;i<=3;i++){
    bool enabled=option==2||i==1;
    Set(r,$"lotItemId{i:00}",enabled?new[]{4200100,4200200,4200300}[i-1]:0);
    Set(r,$"lotItemCategory{i:00}",enabled?7:0);Set(r,$"lotItemNum{i:00}",enabled?1:0);
    Set(r,$"lotItemBasePoint{i:00}",enabled?(option==2?(first==PremiumLot?new[]{0,500,500}:new[]{900,50,50})[i-1]:1000):0);
   }
  }
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var generated=new Dictionary<(string,int),PARAM.Row>();
  var templates=new[]{"SpEffectParam","ItemLotParam_enemy"}.ToDictionary(n=>n,n=>Table(native,n));
  Mechanics((name,from,id)=>{var r=new PARAM.Row(templates[name].Rows.Single(r=>r.ID==from)){ID=id};generated.Add((name,id),r);return r;});
  var actual=templates.Keys.ToDictionary(n=>n,n=>Table(current,n));
  foreach(var(pair,row)in generated)Check(CageAI.Same(row,actual[pair.Item1].Rows.Single(r=>r.ID==pair.Item2)),"Extra Practice parameter differs from intended native template");
  foreach(int first in new[]{Lot,PremiumLot}){
  var choices=actual["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=first&&r.ID<first+3).OrderBy(r=>r.ID).ToArray();
  Check(choices.Length==3,"Expected exactly three Extra Practice choices");
  for(int option=0;option<3;option++){
   Check(N(choices[option],"lotItem_Rarity")==-1,"Reward ball must use the resolved item's native rarity");
   var row=choices[option];Check(N(row,"lotItemBasePoint01")== (option==2?(first==PremiumLot?0:900):1000)&&N(row,"lotItemId01")==4200100,"Basic-pool weight mismatch");
   Check(N(row,"lotItemBasePoint02")== (option==2?(first==PremiumLot?500:50):0)&&N(row,"lotItemBasePoint03")== (option==2?(first==PremiumLot?500:50):0),"Mechanism/high weights must match purchase quality in the last choice only");
   if(option==2)Check(N(row,"lotItemId02")==4200200&&N(row,"lotItemId03")==4200300,"Mechanism/high pool reference mismatch");
   Check(Enumerable.Range(1,8).Sum(i=>N(row,$"lotItemBasePoint{i:00}"))==1000,"Choice weights must sum to 1000");
  }
  }
  var original=Table(native,"ItemTableParam");var tables=Table(current,"ItemTableParam");
  var pools=new[]{(4200100,15),(4200200,21),(4200300,20)};
  foreach(var(id,count)in pools){var a=original.Rows.Where(r=>r.ID==id).ToArray();var b=tables.Rows.Where(r=>r.ID==id).ToArray();Check(a.Length==count&&b.Length==count&&a.Zip(b).All(p=>CageAI.Same(p.First,p.Second)),"Native basic/mechanism/high pool changed");}
  var effects=Table(current,"SpEffectParam");
  Check(!effects.Rows.Any(r=>r.ID>=First&&r.ID<First+1024),"Retired spending records must not remain in the parameter package");
  Check(RewardCatalog.HistoryBit(Owner)==33&&RewardCatalog.HistoricalOwners.Length>=34,"Stable history must append without reindexing");
  Check(RewardCatalog.PackedIds(34).All(id=>effects.Rows.Any(r=>r.ID==id)),"Missing packed history capacity");
  foreach(var n in new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods"})Check(Table(current,n).Rows.Count(r=>r.ID==Owner)==1,"Missing/duplicate Extra Practice owner metadata");
  Check(N(tables.Rows.Single(r=>r.ID==RewardCatalog.Table(Owner)),"itemId")==Owner,"Missing hex reward reference");
  return new{lot=Lot,premiumLot=PremiumLot,choices=3,pools=pools.Select(p=>new{id=p.Item1,entries=p.Item2}),firstTwo=new[]{100,0,0},last=new[]{90,5,5},premiumLast=new[]{0,50,50},persistentRows=0,pendingRewardCountInMemory=true,historyBit=33,historyBytes=5};
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,string? input=null){
  Validate(native,SFUtil.DecryptNightreignRegulation(input??Path.Combine(root,"dist/mod/regulation.bin")),parse);
  Console.WriteLine("PASS: installed Extra Practice native three-choice lotteries, 90/5/5 and 0/50/50 weights, original pools and no retired spending records");
 }
 public static void StageOdds(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,"work/extra-practice-odds/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var member=binder.Files.Single(file=>Name(file)=="ItemLotParam_enemy.param");var lots=parse("ItemLotParam_enemy",member.Bytes);
  var original=parse("ItemLotParam_enemy",native.Files.Single(file=>Name(file)=="ItemLotParam_enemy.param").Bytes);
  var expected=new Dictionary<int,PARAM.Row>();
  Mechanics((name,from,id)=>{var row=new PARAM.Row(original.Rows.Single(row=>row.ID==from)){ID=id};expected.Add(id,row);return row;});
  for(int option=0;option<3;option++){
   var row=lots.Rows.Single(row=>row.ID==Lot+option);var oldExpected=new PARAM.Row(expected[row.ID]);
   if(option==2){Set(oldExpected,"lotItemBasePoint01",800);Set(oldExpected,"lotItemBasePoint02",100);Set(oldExpected,"lotItemBasePoint03",100);}
   Check(CageAI.Same(row,oldExpected),"Unexpected pre-update Extra Practice lottery");
   Check(!lots.Rows.Any(row=>row.ID==PremiumLot+option),"Premium lottery ID collision");
   lots.Rows[lots.Rows.IndexOf(row)]=expected[row.ID];lots.Rows.Add(expected[PremiumLot+option]);
  }
  lots.Rows=lots.Rows.OrderBy(row=>row.ID).ToList();member.Bytes=lots.Write();
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var file in final.Files){
   var intended=binder.Files.Single(value=>value.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Regulation readback mismatch");
   if(file.ID!=member.ID){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated parameter table changed");continue;}
   var old=parse("ItemLotParam_enemy",before[file.ID]);var current=parse("ItemLotParam_enemy",file.Bytes);Check(current.Rows.Count==old.Rows.Count+3,"Unexpected lottery row count");
   foreach(var row in old.Rows.Where(row=>row.ID!=Lot+2))Check(CageAI.Same(row,current.Rows.Single(value=>value.ID==row.ID)),"Unrelated lottery changed");
  }
  var audit=Validate(native,final,parse);HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedLot=Lot+2,addedLots=Enumerable.Range(PremiumLot,3),unrelatedRowsAndTablesUnchanged=true,audit,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Extra Practice purchase-quality lotteries and targeted texts staged; dist untouched");
 }
 public static void StageRework(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/extra-practice/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");var effects=parse("SpEffectParam",member.Bytes);
  int removed=effects.Rows.RemoveAll(r=>r.ID>=First&&r.ID<First+1024);Check(removed is 0 or 1020,"Unexpected legacy credit records");member.Bytes=effects.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var expected=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");if(f.ID!=member.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse("SpEffectParam",before[f.ID]);var now=parse("SpEffectParam",f.Bytes);var retained=old.Rows.Where(r=>r.ID<First||r.ID>=First+1024).ToArray();Check(now.Rows.Count==retained.Length&&retained.Zip(now.Rows).All(p=>CageAI.Same(p.First,p.Second)),"Unrelated effect row or field changed");}
  var audit=Validate(native,final,parse);HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),removedCreditRows=removed,originalRewardPoolsAndAllRemainingRowsPreserved=true,audit,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Extra Practice retired credit records removed and bilingual descriptions staged; existing reward pools preserved");
 }
 public static void StageRarity(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,"work/snack/rarity-staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var member=binder.Files.Single(file=>Name(file)=="ItemLotParam_enemy.param");var lots=parse("ItemLotParam_enemy",member.Bytes);
  for(int option=0;option<3;option++)Set(lots.Rows.Single(row=>row.ID==Lot+option),"lotItem_Rarity",-1);
  member.Bytes=lots.Write();string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var file in final.Files){
   var expected=binder.Files.Single(candidate=>candidate.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");
   if(file.ID!=member.ID){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated parameter table changed");continue;}
   var original=parse("ItemLotParam_enemy",before[file.ID]);var current=parse("ItemLotParam_enemy",file.Bytes);Check(current.Rows.Count==original.Rows.Count,"Lot row count changed");
   foreach(var pair in original.Rows.Zip(current.Rows)){
    var restored=new PARAM.Row(pair.Second);if(restored.ID>=Lot&&restored.ID<Lot+3)Set(restored,"lotItem_Rarity",pair.First["lotItem_Rarity"].Value);
    Check(CageAI.Same(pair.First,restored),"Unrelated lot row or field changed");
   }
  }
  var audit=Validate(native,final,parse);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedLots=Enumerable.Range(Lot,3),changedField="lotItem_Rarity",value=-1,otherFieldsRowsAndTablesPreserved=true,audit,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: three Snack lots use resolved item rarity; weights, pools and unrelated resources preserved");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/snack/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Extra Practice ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
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
  var audit=Validate(native,final,parse);RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),hexCount=hexes.Length,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Count),rowCounts=tables.ToDictionary(p=>p.Key,p=>p.Value.Rows.Count),audit,existingRowsAndUnrelatedTablesUnchanged=true,eventsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Extra Practice owner, native 3-choice lottery and bilingual text staged; all prior rows preserved");
 }
}
