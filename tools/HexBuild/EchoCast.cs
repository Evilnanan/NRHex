using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class EchoCast {
 public const int Owner=95003500;
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 public static void OwnerEffect(PARAM.Row r){Set(r,"magicConsumptionRate",2.25f);Set(r,"miracleConsumptionRate",2.25f);}
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool p,string why){if(!p)throw new InvalidDataException(why);}
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var input=Path.Combine(root,"dist/mod/regulation.bin");var output=Path.Combine(root,"work/echo-casting/staged");Directory.CreateDirectory(output);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int source,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),"Echo row collision");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==source)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new (string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);OwnerEffect(s);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  var history=RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length).Where(id=>!tables["SpEffectParam"].Rows.Any(r=>r.ID==id)).ToArray();
  RewardCycle.Parameters((name,source,id)=>id==RewardCycle.Reset?tables[name].Rows.Single(r=>r.ID==id):Add(name,source,id),history);
  // Parameters also declares the reset effect, already present in current dist.
  foreach(var name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==name+".param").Bytes=tables[name].Write();}
  var target=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(target);
  Check(actual.Files.Count==b.Files.Count&&actual.Version==b.Version&&actual.Compression==b.Compression,"Binder metadata changed");
  foreach(var f in actual.Files){Check(f.Bytes.Span.SequenceEqual(b.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Binder roundtrip mismatch");var name=names.SingleOrDefault(n=>Name(f)==n+".param");if(name==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(name,before[f.ID]);var now=parse(name,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[name].Count,"Row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing row changed");}
  RewardCatalog.Write(Path.Combine(output,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(output,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(target),owner=Owner,spellFpRate=2.25,artsFpRate=Convert.ToSingle(s["artsConsumptionRate"].Value),addedRows=added,historyBytes=(RewardCatalog.HistoricalOwners.Length+7)/8,historyBit=RewardCatalog.HistoryBit(Owner),unrelatedRowsAndTablesUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
 }
}
