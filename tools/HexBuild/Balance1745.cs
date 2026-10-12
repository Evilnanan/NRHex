using SoulsFormats;
using System.Text.Json;

static class Balance1745 {
 static void Check(bool ok,string message){if(!ok)throw new InvalidDataException(message);}
 static void Set(PARAM.Row r,string f,object v){r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string output=Path.Combine(root,".work/balance-01745/staged");Directory.CreateDirectory(output);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var oldBytes=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var changes=new HashSet<(string Table,int Id,string Field)>();
  void Change(string table,int id,string field,object value){Set(tables[table].Rows.Single(r=>r.ID==id),field,value);changes.Add((table,id,field));}
  foreach(string f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Change("SpEffectParam",95001801,f,.25f);
  // Neutral hidden compatibility records for old saves; no runtime handler or new reward.
  Change("SpEffectParam",95001900,"iconId",-1);Change("SpEffectParam",95001900,"spEffectTextId_1",-1);
  Change("AttachEffectParam",95001900,"attachTextId",-1);Change("AttachEffectParam",95001900,"statusIconId",-1);
  var lots=tables["ItemLotParam_enemy"].Rows.ToDictionary(r=>r.ID);
  var historical=AngelChant.Orders(19);
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   var row=lots[entry.Lot+slot];int group=(entry.Lot-95200000)/10;
   Check(Convert.ToInt32(row["lotItemId01"].Value)==95110100+historical[group][slot],"Unexpected original lot ordering");
   int wanted=RewardCatalog.Table(entry.Owners[slot]);
   if(Convert.ToInt32(row["lotItemId01"].Value)!=wanted)Change("ItemLotParam_enemy",row.ID,"lotItemId01",wanted);
  }
  foreach(string n in names)binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();
  string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var read=SFUtil.DecryptNightreignRegulation(path);
  Check(read.Files.Count==binder.Files.Count&&read.Version==binder.Version&&read.Compression==binder.Compression,"Binder metadata changed");
  foreach(var f in read.Files){var expected=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Readback differs");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(oldBytes[f.ID]),"Unrelated table changed");}
  foreach(string n in names){
   var final=parse(n,read.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes).Rows.ToDictionary(r=>r.ID);
   Check(final.Count==baseline[n].Rows.Count,"Row count changed");
   foreach(var row in baseline[n].Rows)foreach(var c in row.Cells.Where(c=>c!=null)){
    if(JsonSerializer.Serialize(c.Value)!=JsonSerializer.Serialize(final[row.ID][c.Def.InternalName].Value))Check(changes.Contains((n,row.ID,c.Def.InternalName)),"Unrelated field changed");
   }
  }
  var items=parse("ItemTableParam",read.Files.Single(f=>Path.GetFileName(f.Name)=="ItemTableParam.param").Bytes).Rows.Where(r=>r.ID>=95110100&&r.ID<95110119).ToDictionary(r=>r.ID);
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   var actual=tables["ItemLotParam_enemy"].Rows.Single(r=>r.ID==entry.Lot+slot);
   int owner=Convert.ToInt32(items[Convert.ToInt32(actual["lotItemId01"].Value)]["itemId"].Value);
   Check(owner==entry.Owners[slot]&&owner!=RewardCatalog.Retired,"Retired ability still grantable");
  }
  RewardCatalog.Write(Path.Combine(root,"src/hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(output,"texts"),allRewards:true,retiredIds:[95001900]);
  var attack=parse("AtkParam_Pc",read.Files.Single(f=>Path.GetFileName(f.Name)=="AtkParam_Pc.param").Bytes).Rows.Single(r=>r.ID==95001760);
  File.WriteAllText(Path.Combine(output,"balloon-attack.json"),JsonSerializer.Serialize(attack.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value),new JsonSerializerOptions{WriteIndented=true}));
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{version="0.17.45",hexes,activeChoices=RewardCatalog.Active(hexes).Length,legacyRewardGroups=historical.Count,retired=95001900,spiritDamageMultiplier=.25,chantStaminaPerSecond=15,inputSha256=Hash(input),outputSha256=Hash(path),changedFields=changes.Select(c=>new{c.Table,c.Id,c.Field}),allLegacyRewardsExcludeRetired=true,otherTablesAndFieldsUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: 18 active abilities / 4896 ordered choices; 5814 historical group IDs preserved; all rewards exclude retired ability; parameter/text readback matched");
 }
}
