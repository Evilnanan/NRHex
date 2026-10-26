using SoulsFormats;
using System.Text.Json;

static class Archmage {
 const int Owner=95002100;
 static readonly int[] Tokens={6999936,6999937,6999938,6999939,6999940,6999941};
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void ExportTarget(string root,string overlay,Func<string,Memory<byte>,PARAM> parse){
  var path=Path.Combine(Path.GetFullPath(overlay),"mod/regulation.bin");var hash=Hash(path);
  var binder=SFUtil.DecryptNightreignRegulation(path);
  var tables=new Dictionary<string,object>();
  foreach(var name in new[]{"Magic","MagicTableParam"}){
   var fields=name=="Magic"?new[]{"ezStateBehaviorType","isEnchant","isShieldEnchant"}:new[]{"magicId","chanceWeight"};
   tables[name]=parse(name,binder.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()==name+".param").Bytes).Rows.Select(r=>new{r.ID,fields=fields.ToDictionary(k=>k,k=>r[k].Value)}).ToArray();
  }
  Check(Hash(path)==hash,"Archmage input changed during export");
  var output=Path.Combine(root,"work/compat/mmv/archmage-parameters.json");Directory.CreateDirectory(Path.GetDirectoryName(output)!);
  File.WriteAllText(output,JsonSerializer.Serialize(new{regulationPath=path,regulationSha256=hash,tables}));
  Console.WriteLine("Exported target Archmage parameters: "+output);
 }
 public static void Markers(Func<int,int,PARAM.Row> clone){
  foreach(int id in Tokens){
   var row=clone(8000000,id);Set(row,"maxHpRate",1f);Set(row,"saveCategory",-1);Set(row,"spCategory",20);
   Set(row,"iconId",-1);Set(row,"spEffectTextId_1",-1);Set(row,"displayIconForPermamentEffect",0);Set(row,"effectEndurance",id==Tokens[0]?-1f:.001f);
  }
 }
 public static void WriteData(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var spells=parse("Magic",original.Files.Single(f=>Path.GetFileName(f.Name)=="Magic.param").Bytes).Rows.ToDictionary(r=>r.ID);
  var lottery=parse("MagicTableParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="MagicTableParam.param").Bytes);
  var ids=lottery.Rows.Where(r=>Convert.ToInt32(r["chanceWeight"].Value)>0).Select(r=>Convert.ToInt32(r["magicId"].Value)).Where(id=>spells.ContainsKey(id)&&Convert.ToInt32(spells[id]["ezStateBehaviorType"].Value) is 0 or 1).Distinct().Order().ToArray();
  Check(ids.Length==160,"Native catalyst spell pool changed");
  // Original catalyst spells need type lookup even outside the random pool.
  var known=spells.Keys.Where(id=>Convert.ToInt32(spells[id]["ezStateBehaviorType"].Value) is 0 or 1).Order().ToArray();
  Check(known.Length==178,"Native spell classification changed");
  var random=ids.Where(id=>Convert.ToInt32(spells[id]["isEnchant"].Value)==0&&Convert.ToInt32(spells[id]["isShieldEnchant"].Value)==0).ToArray();
  Check(random.Length==151&&random.Count(id=>Convert.ToInt32(spells[id]["ezStateBehaviorType"].Value)==0)==64,"Native enchantment exclusions changed");
  File.WriteAllText(Path.Combine(root,"src/archmage_data.rs"),"// Generated from native MagicTableParam and Magic; classify all native spells independently of the random pool; NT flags are not current availability.\npub const SPELLS:&[(i32,u8)]=&[\n"+string.Join("\n",known.Select(id=>$" ({id},{spells[id]["ezStateBehaviorType"].Value}),"))+"\n];\n// Random choices exclude native weapon/shield enchantments.\npub const RANDOM_SPELLS:&[i32]=&["+string.Join(",",random)+"];\n");
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  WriteData(root,original,parse);
  string directory=Path.Combine(root,".work/archmage/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Installed regulation version changed");
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int source,int id){
   Check(!tables[name].Rows.Any(r=>r.ID==id),$"New ID collision: {name} {id}");
   var row=new PARAM.Row(tables[name].Rows.Single(r=>r.ID==source)){ID=id};tables[name].Rows.Add(row);added[name].Add(id);return row;
  }
  var h=hexes.Single(h=>h.Id==Owner);
  var effect=Add("SpEffectParam",8000000,Owner);Set(effect,"maxHpRate",1f);Set(effect,"spCategory",20);Set(effect,"iconId",h.SmallIcon);Set(effect,"spEffectTextId_1",Owner);Set(effect,"displayIconForPermamentEffect",0);
  Set(effect,"magicConsumptionRate",1f);Set(effect,"miracleConsumptionRate",1f);
  Check(Convert.ToSingle(effect["artsConsumptionRate"].Value)==1f,"Template must not make weapon skills free");
  var attach=Add("AttachEffectParam",95002000,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"passiveSpEffectId_2",-1);Set(attach,"attachTextId",Owner);Set(attach,"overrideBaseEffectId",Owner);Set(attach,"statusIconId",h.SmallIcon);
  var permanent=Add("PermanentBuffParam",95002000,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002000,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002000),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  Markers((source,id)=>Add("SpEffectParam",source,id));
  // The historical 19-ability serialization boundary is kept before adding
  // owner 21. Old balls retain the same lot IDs and all three choices.
  int oldEnd=95258140;
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot;int target=RewardCatalog.Table(entry.Owners[slot]);
   if(id<oldEnd){Check(Convert.ToInt32(tables["ItemLotParam_enemy"].Rows.Single(r=>r.ID==id)["lotItemId01"].Value)==target,"Old reward changed order or identity");}
   else{var row=Add("ItemLotParam_enemy",95200000+slot,id);Set(row,"lotItemId01",target);}
  }
  RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);
  foreach(string name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes=tables[name].Write();}
  string destination=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(destination,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(destination);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var member in reopened.Files){
   var intended=binder.Files.Single(f=>f.ID==member.ID);Check(member.Name==intended.Name&&member.Flags==intended.Flags&&member.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Regulation member readback mismatch");
   if(!names.Contains(Path.GetFileNameWithoutExtension(member.Name)))Check(member.Bytes.Span.SequenceEqual(before[member.ID]),"Unrelated parameter member changed");
  }
  foreach(string name in names){
   var actual=parse(name,reopened.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
   Check(actual.Rows.Count==baseline[name].Rows.Count+added[name].Count,"Row count mismatch");
   var oldRows=baseline[name].Rows.OrderBy(r=>r.ID).ToArray();var actualOld=actual.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();
   Check(actualOld.Length==oldRows.Length&&oldRows.Zip(actualOld).All(pair=>CageAI.Same(pair.First,pair.Second)),"Preexisting row or duplicate row changed");
   foreach(int id in added[name])Check(CageAI.Same(tables[name].Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"New row readback mismatch");
  }
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{sourceSha256=Hash(input),outputSha256=Hash(destination),abilityCount=hexes.Length,orderedChoices=hexes.Length*(hexes.Length-1)*(hexes.Length-2),added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),allPreexistingRowsUnchanged=true,allOtherTablesUnchanged=true,nativeMagicConsumptionRate=1,nativeMiracleConsumptionRate=1,artsConsumptionRate=1,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Archmage parameters, stable old reward lots and bilingual text staged/read back");
 }
 public static void StageSecondary(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/archmage-secondary/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var table=parse("SpEffectParam",member.Bytes);
  var owner=table.Rows.Single(r=>r.ID==Owner);var changes=new[]{"magicConsumptionRate","miracleConsumptionRate"};
  foreach(string field in changes){Check(Convert.ToSingle(owner[field].Value) is 0f or 1f,"Unexpected owner FP modifier");Set(owner,field,1f);}
  member.Bytes=table.Write();string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(output);Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in reopened.Files){var wanted=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Member mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var old=parse("SpEffectParam",before[member.ID]);var actual=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==member.ID).Bytes);Check(old.Rows.Count==actual.Rows.Count,"Row count changed");
  for(int i=0;i<old.Rows.Count;i++){
   var a=old.Rows[i];var b=actual.Rows[i];Check(a.ID==b.ID,"Row order changed");
   if(a.ID==Owner)foreach(string field in changes)Set(a,field,1f);
   Check(CageAI.Same(a,b),"Unintended effect field changed");
  }
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{sourceSha256=Hash(input),outputSha256=Hash(output),effectId=Owner,neutralOwnerFpRates=true,onlySecondarySlotRuntimeCostIsFree=true,otherParameterRowsAndFieldsUnchanged=true,otherTablesUnchanged=true,readbackPassed=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: only Archmage owner FP multipliers restored to 1; bilingual texts staged and read back");
 }
 public static void Probe(string root,BND4 binder,Func<string,Memory<byte>,PARAM> parse){
  var directory=Path.Combine(root,".work/archmage");Directory.CreateDirectory(directory);
  object Dump(PARAM.Row row)=>new{row.ID,fields=row.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var tables=new[]{"Magic","MagicTableParam","EquipParamWeapon","EquipParamCustomWeapon","SpEffectParam"}.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var data=tables.ToDictionary(k=>k.Key,k=>k.Key=="SpEffectParam"?k.Value.Rows.Where(row=>Convert.ToSingle(row["magicConsumptionRate"].Value)!=1f).Select(Dump).ToArray():k.Value.Rows.Select(Dump).ToArray());
  File.WriteAllText(Path.Combine(directory,"native-parameters.json"),JsonSerializer.Serialize(data));
  Console.WriteLine("Read-only spell, catalyst and spell FP multiplier data exported");
 }
}
