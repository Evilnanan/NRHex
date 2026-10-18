using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
// Incremental resource staging for cloud validation; the full build registers
// exactly the same neutral owner through HexCatalog and RewardCatalog.
static class Ambulance {
 public const int Owner=95003400;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string field,object value)=>r[field].Value=Convert.ChangeType(value,r[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var input=Path.Combine(root,"dist/mod/regulation.bin");var dir=Path.Combine(root,"work/ambulance/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  PARAM.Row Add(string name,int source,int id){var table=tables[name];Check(!table.Rows.Any(r=>r.ID==id),"Ambulance row collision");var row=new PARAM.Row(originals[name].Rows.First(r=>r.ID==source)){ID=id};table.Rows.Add(row);return row;}
  var h=hexes.Single(h=>h.Id==Owner);
  var s=Add("SpEffectParam",8000000,Owner);Set(s,"maxHpRate",1f);Set(s,"spCategory",20);Set(s,"iconId",h.SmallIcon);Set(s,"spEffectTextId_1",Owner);Set(s,"displayIconForPermamentEffect",0);
  var a=Add("AttachEffectParam",8000000,Owner);
  foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);Set(p,"spEffectId",Owner);Set(p,"iconId",h.LargeIcon);Set(p,"displayNumericValue",2);Set(p,"displayPercentageSymbol",0);Set(p,"numericValue",0);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  foreach(var name in names){Check(tables[name].Rows.Count<65536,"PARAM row capacity exceeded");tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==name+".param").Bytes=tables[name].Write();}
  var output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in actual.Files){var intended=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Binder readback mismatch");
   var name=names.SingleOrDefault(n=>Name(f)==n+".param");if(name==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse(name,before[f.ID]);var current=parse(name,f.Bytes);Check(current.Rows.Count==old.Rows.Count+1,"Expected exactly one new row per table");
   var added=name=="ItemTableParam"?RewardCatalog.Table(Owner):Owner;
   Check(old.Rows.OrderBy(r=>r.ID).Zip(current.Rows.Where(r=>r.ID!=added).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"An existing row changed");
  }
  Check(RewardCatalog.HistoricalOwners.Length==32&&RewardCatalog.HistoryBit(Owner)==31,"History registry must append its next free bit");
  Check(RewardCatalog.PackedIds(32).All(id=>tables["SpEffectParam"].Rows.Any(r=>r.ID==id)),"Existing packed history capacity incomplete");
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  var effects=originals["SpEffectParam"].Rows.Where(r=>new[]{100300,100320,102022}.Contains(r.ID)).ToArray();
  Check(effects.All(r=>Convert.ToInt32(r["changeStaminaPoint"].Value)==1&&Convert.ToSingle(r["changeStaminaRate"].Value)==0f),"Native periodic locomotion debit changed");
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),owner=Owner,hexCount=hexes.Length,historyBit=31,newRows=names.ToDictionary(n=>n,n=>n=="ItemTableParam"?RewardCatalog.Table(Owner):Owner),nativeCosts=effects.Select(row=>new{row.ID,fields=row.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),existingRowsAndUnrelatedTablesUnchanged=true,rewardTemplateAndEventsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Ambulance neutral owner, reward metadata, packed history capacity and bilingual texts staged; existing rows preserved");
 }
}
