using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class Snack {
 public const int Owner=95003600,First=96110000,Lot=95420000;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static int N(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  for(int digit=0;digit<4;digit++)for(int value=1;value<256;value++){
   var r=clone("SpEffectParam",8000000,First+digit*256+value);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);
  }
  for(int option=0;option<3;option++){
   var r=clone("ItemLotParam_enemy",6100002,Lot+option);Set(r,"lotItem_Rarity",3);
   // Native lot lottery selects one pool for this choice, then the native
   // table resolves its class/eligibility filters. No reward permutations.
   for(int i=1;i<=3;i++){
    bool enabled=option==2||i==1;
    Set(r,$"lotItemId{i:00}",enabled?new[]{4200100,4200200,4200300}[i-1]:0);
    Set(r,$"lotItemCategory{i:00}",enabled?7:0);Set(r,$"lotItemNum{i:00}",enabled?1:0);
    Set(r,$"lotItemBasePoint{i:00}",enabled?(option==2?new[]{800,100,100}[i-1]:1000):0);
   }
  }
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var generated=new Dictionary<(string,int),PARAM.Row>();
  var templates=new[]{"SpEffectParam","ItemLotParam_enemy"}.ToDictionary(n=>n,n=>Table(native,n));
  Mechanics((name,from,id)=>{var r=new PARAM.Row(templates[name].Rows.Single(r=>r.ID==from)){ID=id};generated.Add((name,id),r);return r;});
  var actual=templates.Keys.ToDictionary(n=>n,n=>Table(current,n));
  foreach(var(pair,row)in generated)Check(CageAI.Same(row,actual[pair.Item1].Rows.Single(r=>r.ID==pair.Item2)),"Extra Helping parameter differs from intended native template");
  var choices=actual["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=Lot&&r.ID<Lot+3).OrderBy(r=>r.ID).ToArray();
  Check(choices.Length==3,"Expected exactly three Extra Helping choices");
  for(int option=0;option<3;option++){
   var row=choices[option];Check(N(row,"lotItemBasePoint01")== (option==2?800:1000)&&N(row,"lotItemId01")==4200100,"Basic-pool weight mismatch");
   Check(N(row,"lotItemBasePoint02")== (option==2?100:0)&&N(row,"lotItemBasePoint03")== (option==2?100:0),"Mechanism/high weights must be 10% each in the last choice only");
   if(option==2)Check(N(row,"lotItemId02")==4200200&&N(row,"lotItemId03")==4200300,"Mechanism/high pool reference mismatch");
   Check(Enumerable.Range(1,8).Sum(i=>N(row,$"lotItemBasePoint{i:00}"))==1000,"Choice weights must sum to 1000");
  }
  var original=Table(native,"ItemTableParam");var tables=Table(current,"ItemTableParam");
  var pools=new[]{(4200100,15),(4200200,21),(4200300,20)};
  foreach(var(id,count)in pools){var a=original.Rows.Where(r=>r.ID==id).ToArray();var b=tables.Rows.Where(r=>r.ID==id).ToArray();Check(a.Length==count&&b.Length==count&&a.Zip(b).All(p=>CageAI.Same(p.First,p.Second)),"Native basic/mechanism/high pool changed");}
  var effects=Table(current,"SpEffectParam");
  for(int digit=0;digit<4;digit++)for(int value=1;value<256;value++){
   var r=effects.Rows.Single(r=>r.ID==First+digit*256+value);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("maxMpRate",1f),("stateInfo",0),("soul",0),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Check(Convert.ToDouble(r[f].Value)==Convert.ToDouble(v),$"Nonneutral credit marker {r.ID}/{f}");
  }
  Check(RewardCatalog.HistoryBit(Owner)==33&&RewardCatalog.HistoricalOwners.Length==34,"Stable history must append without reindexing");
  Check(RewardCatalog.PackedIds(34).All(id=>effects.Rows.Any(r=>r.ID==id)),"Missing packed history capacity");
  foreach(var n in new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods"})Check(Table(current,n).Rows.Count(r=>r.ID==Owner)==1,"Missing/duplicate Extra Helping owner metadata");
  Check(N(tables.Rows.Single(r=>r.ID==RewardCatalog.Table(Owner)),"itemId")==Owner,"Missing hex reward reference");
  return new{lot=Lot,choices=3,pools=pools.Select(p=>new{id=p.Item1,entries=p.Item2}),firstTwo=new[]{100,0,0},last=new[]{80,10,10},persistentRows=1020,activePersistentLimit=4,historyBit=33,historyBytes=5};
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  Validate(native,SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin")),parse);
  Console.WriteLine("PASS: installed Extra Helping native three-choice lotteries, 80/10/10 weights, original pools and persistent credit markers");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/snack/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Extra Helping ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
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
  Console.WriteLine("PASS: Extra Helping owner, native 3-choice lottery, persistent credit and bilingual text staged; all prior rows preserved");
 }
}
