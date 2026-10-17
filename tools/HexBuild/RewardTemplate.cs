using SoulsFormats;
using System.Text.Json;
// Compact the old permutation rows, keeping all unrelated/native rows intact.
static class RewardTemplate{
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/reward-scaling/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Name(f)=="ItemLotParam_enemy.param");var table=parse("ItemLotParam_enemy",member.Bytes);
  var original=parse("ItemLotParam_enemy",native.Files.Single(f=>Name(f)=="ItemLotParam_enemy.param").Bytes);var nativeIds=original.Rows.Select(r=>r.ID).ToHashSet();
  var sourceRows=table.Rows.ToDictionary(r=>r.ID);var tableIds=RewardCatalog.HistoricalOwners.Select(RewardCatalog.Table).ToHashSet();
  // This reserved interval is the previous catalog's generated area. Only
  // category7 references to known hex item tables qualify for removal.
  var removed=table.Rows.Where(r=>r.ID>=95200000&&r.ID<95419260&&r.ID%10<=2&&
   !(RewardCatalog.TemplateLot<=r.ID&&r.ID<RewardCatalog.TemplateLot+3)&&
   Convert.ToInt32(r["lotItemCategory01"].Value)==7&&tableIds.Contains(Convert.ToInt32(r["lotItemId01"].Value))).Select(r=>r.ID).ToHashSet();
  Check(!removed.Overlaps(nativeIds),"Would remove an original game row");
  foreach(var template in RewardCatalog.Template(hexes))for(int i=0;i<3;i++){
   var row=sourceRows[template.Lot+i];Check(Convert.ToInt32(row["lotItemId01"].Value)==RewardCatalog.Table(template.Owners[i])&&Convert.ToInt32(row["lotItemCategory01"].Value)==7&&Convert.ToInt32(row["lotItem_Rarity"].Value)==3,"Unexpected template contents/classification");
  }
  var effectMember=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");
  var effects=parse("SpEffectParam",effectMember.Bytes);var effectSource=effects.Rows.Single(r=>r.ID==8000000);var effectsBefore=effects.Rows.ToDictionary(r=>r.ID);
  foreach(var name in new[]{"EquipParamGoods","AttachEffectParam","PermanentBuffParam"}){
   var existing=PARAM.Read(binder.Files.Single(f=>Name(f)==name+".param").Bytes).Rows.Select(r=>r.ID).ToHashSet();
   Check(hexes.All(h=>existing.Contains(h.Id)),"Ability wrappers incomplete; run the current full build first: "+name);
  }
  Check(hexes.All(h=>effectsBefore.ContainsKey(h.Id)),"Ability effects incomplete; run the current full build first");
  var legacy=Enumerable.Range(0,28).Select(RewardCatalog.LegacyHistory).ToHashSet();
  var removedHistory=effects.Rows.Where(r=>legacy.Contains(r.ID)).Select(r=>r.ID).ToHashSet();
  foreach(int id in removedHistory){var r=effectsBefore[id];Check(Convert.ToInt32(r["iconId"].Value)==-1&&Convert.ToInt32(r["saveCategory"].Value)==10,"Legacy history record is not neutral/hidden");}
  effects.Rows.RemoveAll(r=>removedHistory.Contains(r.ID));var addedEffects=new HashSet<int>();var expectedEffects=new Dictionary<int,PARAM.Row>();
  RewardCycle.Parameters((name,source,id)=>{
   Check(name=="SpEffectParam"&&source==8000000,"Unexpected packed history source");
   var row=new PARAM.Row(effectSource){ID=id};expectedEffects.Add(id,row);
   if(effectsBefore.ContainsKey(id))return row;
   effects.Rows.Add(row);addedEffects.Add(id);return row;
  });
  foreach(var pair in expectedEffects)if(effectsBefore.TryGetValue(pair.Key,out var existing))Check(CageAI.Same(pair.Value,existing),"Existing packed/reset effect differs");
  effects.Rows=effects.Rows.OrderBy(r=>r.ID).ToList();effectMember.Bytes=effects.Write();
  table.Rows.RemoveAll(r=>removed.Contains(r.ID));Check(table.Rows.Count<65536,"PARAM row capacity exceeded");member.Bytes=table.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Compression==binder.Compression&&actual.Version==binder.Version&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(f.ID!=member.ID&&f.ID!=effectMember.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var saved=parse("ItemLotParam_enemy",actual.Files.Single(f=>f.ID==member.ID).Bytes);var old=parse("ItemLotParam_enemy",before[member.ID]);
  Check(saved.Rows.Count==old.Rows.Count-removed.Count,"Unexpected row count");
  foreach(var row in saved.Rows)Check(CageAI.Same(row,sourceRows[row.ID]),"A retained row changed");
  Check(original.Rows.All(row=>saved.Rows.Any(r=>r.ID==row.ID)),"Native row missing");
  var savedEffects=parse("SpEffectParam",actual.Files.Single(f=>f.ID==effectMember.ID).Bytes);
  Check(savedEffects.Rows.Count==effectsBefore.Count-removedHistory.Count+addedEffects.Count,"Packed history row count changed");
  foreach(var row in savedEffects.Rows){if(!addedEffects.Contains(row.ID))Check(CageAI.Same(row,effectsBefore[row.ID]),"Unrelated effect changed");}
  Check(savedEffects.Rows.Where(r=>addedEffects.Contains(r.ID)).All(r=>Convert.ToInt32(r["saveCategory"].Value)==10&&Convert.ToInt32(r["iconId"].Value)==-1&&Convert.ToInt32(r["dontDeleteOnDead"].Value)==1&&Convert.ToInt32(r["isDisableNetSync"].Value)==1),"Packed records are not local persistent hidden effects");
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  // A larger virtual catalog exercises the same metadata writer and stable
  // history/table mapping without adding fake abilities to the actual package.
  var simulations=new List<object>();
  foreach(int count in new[]{hexes.Length,64,96,160,256}.Where(n=>n>=hexes.Length).Distinct()){
   var extra=Enumerable.Range(100,10000).Select(i=>95000000+i*100).Where(id=>!RewardCatalog.HistoricalOwners.Contains(id)).Take(count-hexes.Length).ToArray();
   var expanded=hexes.Concat(extra.Select(id=>hexes[0] with{Id=id})).ToArray();
   var registry=RewardCatalog.HistoricalOwners.Concat(extra).ToArray();var ids=RewardCatalog.PackedIds(registry.Length);
   Check(registry.Distinct().Count()==registry.Length&&ids.Distinct().Count()==ids.Length,"Expanded catalog IDs collide");
   var generated=new Dictionary<int,PARAM.Row>();
   RewardCycle.Parameters((name,source,id)=>{Check(name=="SpEffectParam"&&source==8000000,"Unexpected history template");var row=new PARAM.Row(effectSource){ID=id};generated.Add(id,row);return row;},ids);
   Check(generated.Count==ids.Length+1&&ids.All(generated.ContainsKey),"Expanded packed record generation incomplete");
   var events=new EMEVD();foreach(int id in new[]{1111,9100941}){var e=new EMEVD.Event(id);e.Instructions.Add(new(2004,21,new object[]{20000u,95002505}));events.Events.Add(e);}
   RewardCycle.Events(events,ids);
   foreach(var e in events.Events)Check(e.Instructions.Count==3&&e.Instructions.Any(i=>i.Bank==2004&&i.ID==8&&BitConverter.ToInt32(i.ArgData,4)==RewardCycle.Reset),"Expanded expedition reset notice incomplete");
   string path=Path.Combine(dir,$"catalog-{count}.rs");RewardCatalog.WriteExpanded(path,expanded,registry);Check(!File.ReadAllText(path).Contains("ORDERS"),"Permutation catalog regenerated");
   simulations.Add(new{abilities=count,rewardRows=3,packedParameterRows=ids.Length,maxActivePackedHistory=(registry.Length+7)/8,metadataBytes=new FileInfo(path).Length,savedRowsAndBothResetsGenerated=true});
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),inputRows=old.Rows.Count,outputRows=saved.Rows.Count,removedLegacyHistoryRows=removedHistory.Count,addedPackedHistoryRows=addedEffects.Count,activePackedHistoryLimit=(RewardCatalog.HistoricalOwners.Length+7)/8,removedPermutationRows=removed.Count,removedIdsSha256=Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(System.Text.Encoding.UTF8.GetBytes(string.Join(",",removed.Order())))),templateLot=RewardCatalog.TemplateLot,templateRows=3,hexCount=hexes.Length,simulations,sourceHashes=new[]{"tools/HexBuild/RewardCatalog.cs","tools/HexBuild/RewardTemplate.cs","tools/HexBuild/RewardCycle.cs","tools/HexBuild/Program.cs"}.ToDictionary(p=>p,p=>Hash(Path.Combine(root,p))),allRetainedRowsAndOtherTablesUnchanged=true,nativeAndRedEnvelopePreserved=true,eventsTextsAndUserSettingsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: reward permutations removed {removed.Count}; ItemLot rows {old.Rows.Count} -> {saved.Rows.Count}; one three-choice template, 256-ability metadata simulation passed");
 }
}
