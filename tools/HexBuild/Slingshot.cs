using SoulsFormats;
using System.Text.Json;
static class Slingshot {
 public const int Owner=95002300;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<int,int,PARAM.Row> clone){
  foreach(int id in new[]{6999960,6999961,6999962,6999963}){
   var row=clone(8000000,id);Set(row,"maxHpRate",1f);Set(row,"saveCategory",-1);Set(row,"spCategory",20);
   Set(row,"iconId",-1);Set(row,"spEffectTextId_1",-1);Set(row,"displayIconForPermamentEffect",0);Set(row,"effectEndurance",id==6999960?-1f:.001f);
  }
  for(int layer=1;layer<=3;layer++){
   var row=clone(321800,Owner+layer);
   Check(Convert.ToInt32(row["magicSubCategoryChange1"].Value)==102,"Native jump category changed");
   Set(row,"saveCategory",-1);Set(row,"iconId",-1);Set(row,"spEffectTextId_1",-1);Set(row,"effectEndurance",-1f);
   foreach(string f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(row,f,1f+.3f*layer);
  }
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string directory=Path.Combine(root,".work/slingshot/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Installed regulation version changed");
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"ID collision: {n} {id}");var row=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(row);added[n].Add(id);return row;}
  var h=hexes.Single(h=>h.Id==Owner);
  var effect=Add("SpEffectParam",8000000,Owner);Set(effect,"maxHpRate",1f);Set(effect,"spCategory",20);Set(effect,"iconId",h.SmallIcon);Set(effect,"spEffectTextId_1",Owner);Set(effect,"displayIconForPermamentEffect",0);
  var attach=Add("AttachEffectParam",95002200,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"attachTextId",Owner);Set(attach,"overrideBaseEffectId",Owner);Set(attach,"statusIconId",h.SmallIcon);
  var permanent=Add("PermanentBuffParam",95002200,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002200,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002200),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  Mechanics((source,id)=>Add("SpEffectParam",source,id));
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(tables["ItemLotParam_enemy"].Rows.Any(r=>r.ID==id))Check(Convert.ToInt32(tables["ItemLotParam_enemy"].Rows.Single(r=>r.ID==id)["lotItemId01"].Value)==target,"Old ball changed");
   else{var row=Add("ItemLotParam_enemy",95200000+slot,id);Set(row,"lotItemId01",target);}
  }
  RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  var destination=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(destination,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(destination);
  foreach(var f in final.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated member changed");}
  foreach(string n in names){var actual=parse(n,final.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);Check(actual.Rows.Count==baseline[n].Rows.Count+added[n].Count,"Row count mismatch");var old=baseline[n].Rows.OrderBy(r=>r.ID).ToArray();var retained=actual.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();Check(old.Length==retained.Length&&old.Zip(retained).All(p=>CageAI.Same(p.First,p.Second)),"Preexisting row changed");}
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(destination),abilityCount=hexes.Length,added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),originalRowsUnchanged=true,sprintWindowSeconds=1,maxStacks=3,horizontalBonusPerStack=.5,attackBonusPerStack=.3,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Slingshot sprint/jump parameters, stable rewards and bilingual texts staged/read back");
 }
}
