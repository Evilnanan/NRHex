using SoulsFormats;
using System.Text.Json;
static class Erosion {
 public const int Owner=95003100;
 public const int Carrier=Owner+1,First=Owner+10,ItemCarrier=Owner+2,ItemFirst=Owner+20;
 public const float AttackRate=.7f;
 public static readonly string[] AttackRates={"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"};
 static readonly int[] Sources={8110002,8110402,8110102,6800,8110502,8110202,6750};
 static readonly string[] Powers={"poizonAttackPower","diseaseAttackPower","bloodAttackPower","curseAttackPower","freezeAttackPower","sleepAttackPower","madnessAttackPower"};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string field,object value)=>r[field].Value=Convert.ChangeType(value,r[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Owners(PARAM.Row r){
  Set(r,"stateInfo",152);Set(r,"atkOccurrenceSpEffectId",Carrier);
  foreach(var field in AttackRates)Set(r,field,AttackRate);
  foreach(var f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(r,f,1);
 }
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  foreach(int id in new[]{Carrier,ItemCarrier}){
  var carrier=clone("SpEffectParam",8000000,id);
  foreach(var (f,v) in new (string,object)[]{("maxHpRate",1f),("effectEndurance",0f),("saveCategory",-1),("spCategory",-1),("isUseStatusAilmentAtkPowerCorrect",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(carrier,f,v);
  }
  foreach(var(first,power)in new[]{(First,30),(ItemFirst,10)})for(int i=0;i<7;i++){
   var r=clone("SpEffectParam",Sources[i],first+i);
   foreach(var f in Powers)Set(r,f,0);Set(r,Powers[i],power);Set(r,"isUseStatusAilmentAtkPowerCorrect",1);
  }
 }
 static PARAM.Row[] ItemRows(PARAM table){
  var rows=new List<PARAM.Row>{new(table.Rows.Single(r=>r.ID==Carrier)){ID=ItemCarrier}};
  for(int i=0;i<7;i++){
   var source=table.Rows.Single(r=>r.ID==First+i);
   Check(Convert.ToInt32(source[Powers[i]].Value)==30,"Weapon buildup changed");
   var row=new PARAM.Row(source){ID=ItemFirst+i};Set(row,Powers[i],10);rows.Add(row);
  }
  return rows.ToArray();
 }
 static object ValidateItems(BND4 native,BND4 binder,Func<string,Memory<byte>,PARAM> parse,string directory){
  var table=parse("SpEffectParam",binder.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  foreach(var row in ItemRows(table))Check(CageAI.Same(row,table.Rows.Single(r=>r.ID==row.ID)),"Item payload readback mismatch");
  var owner=table.Rows.Single(r=>r.ID==Owner);
  Check(AttackRates.All(f=>Convert.ToSingle(owner[f].Value)==AttackRate)&&Convert.ToInt32(owner["atkOccurrenceSpEffectId"].Value)==Carrier,"Owner changed");
  var source=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  foreach(int id in Sources)Check(CageAI.Same(source.Rows.Single(r=>r.ID==id),table.Rows.Single(r=>r.ID==id)),"Native ailment reference changed");
  var ids=new[]{Carrier,ItemCarrier}.Concat(Enumerable.Range(First,7)).Concat(Enumerable.Range(ItemFirst,7)).ToHashSet();
  table.Rows=table.Rows.Where(r=>ids.Contains(r.ID)).ToList();Directory.CreateDirectory(directory);File.WriteAllBytes(Path.Combine(directory,"installed-effects.param"),table.Write());
  return new{owner=Owner,weaponBuildup=30,itemBuildup=10,nativeStatusCorrectionRetained=true,nativeReferencesUnchanged=true};
 }
 public static void VerifyItems(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  ValidateItems(native,SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin")),parse,Path.Combine(root,"work/erosion-items"));
  Console.WriteLine("PASS: installed Erosion 30-point weapon / 10-point item payloads and native ailment references");
 }
 public static void StageItems(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,"work/erosion-items/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);var added=ItemRows(table);
  foreach(var row in added){Check(!table.Rows.Any(r=>r.ID==row.ID),"Item payload ID collision");table.Rows.Add(row);}
  Check(table.Rows.Count<65536,"SpEffectParam row capacity exceeded");table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=table.Write();
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var pair in binder.Files.Zip(actual.Files)){
   Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Regulation readback mismatch");
   if(pair.Second.ID!=file.ID)Check(pair.Second.Bytes.Span.SequenceEqual(before[pair.Second.ID]),"Unrelated table changed");
  }
  var old=parse("SpEffectParam",before[file.ID]);var now=parse("SpEffectParam",actual.Files.Single(f=>f.ID==file.ID).Bytes);var ids=added.Select(r=>r.ID).ToHashSet();
  Check(now.Rows.Count==old.Rows.Count+8&&old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!ids.Contains(r.ID)).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Existing parameter rows changed");
  var audit=ValidateItems(native,actual,parse,directory);
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),audit,addedRows=ids.Order().ToArray(),existingRowsAndOtherTablesUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Erosion item payloads and bilingual text staged; all existing parameters preserved");
 }
 public static void Tune(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var hex=hexes.Single(entry=>entry.Id==Owner);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/erosion-tuning/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Compression==RegulationCompression.Format,"Unexpected regulation compression");
  var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var fields=new Dictionary<string,Dictionary<string,object>>{
   ["SpEffectParam"]=AttackRates.ToDictionary(field=>field,field=>(object)AttackRate),
   ["AttachEffectParam"]=new(){["statusIconId"]=hex.SmallIcon},
   ["PermanentBuffParam"]=new(){["iconId"]=hex.LargeIcon},
   ["EquipParamGoods"]=new(){["iconId"]=hex.LargeIcon-1000}
  };
  fields["SpEffectParam"]["iconId"]=hex.SmallIcon;
  foreach(var (table,updates) in fields){
   var file=binder.Files.Single(file=>Name(file)==table+".param");var parameters=parse(table,file.Bytes);var row=parameters.Rows.Single(row=>row.ID==Owner);
   foreach(var (field,value) in updates){
    if(AttackRates.Contains(field))Check(Convert.ToSingle(row[field].Value) is .85f or .8f or AttackRate,$"Unexpected Erosion attack rate: {field}");
    else{
     int current=Convert.ToInt32(row[field].Value),previous=table=="PermanentBuffParam"?21206:table=="EquipParamGoods"?20206:22206;
     Check(current==previous||current==previous+1||current==Convert.ToInt32(value),$"Unexpected Erosion icon: {table}/{field}/{current}");
    }
    Set(row,field,value);
   }
   file.Bytes=parameters.Write();
  }
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  var changes=new List<object>();
  foreach(var pair in binder.Files.Zip(actual.Files)){
   var file=pair.Second;Check(pair.First.ID==file.ID&&pair.First.Name==file.Name&&pair.First.Flags==file.Flags&&pair.First.Bytes.Span.SequenceEqual(file.Bytes.Span),"Parameter readback mismatch");
   string table=Name(file).Replace(".param","");
   if(!fields.TryGetValue(table,out var updates)){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}
   var oldRows=parse(table,before[file.ID]).Rows;var newRows=parse(table,file.Bytes).Rows;Check(oldRows.Count==newRows.Count,"Row count changed");
   foreach(var rows in oldRows.Zip(newRows)){
    Check(rows.First.ID==rows.Second.ID&&rows.First.Name==rows.Second.Name,"Row identity changed");
    foreach(var cell in rows.First.Cells){
     string field=cell.Def.InternalName;object value=rows.Second[field].Value;
     if(rows.First.ID==Owner&&updates.TryGetValue(field,out var expected))Check(Equals(value,Convert.ChangeType(expected,value.GetType())),"Erosion tuning readback failed");
     if(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(value))continue;
     Check(rows.First.ID==Owner&&updates.ContainsKey(field),"Unrelated parameter field changed");
     changes.Add(new{table,id=Owner,field,before=cell.Value,after=value});
    }
   }
  }
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),owner=Owner,attackRate=AttackRate,damageReductionPercent=30,largeIcon=hex.LargeIcon,smallIcon=hex.SmallIcon,goodsIcon=hex.LargeIcon-1000,changes,unrelatedRowsAndFieldsPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Erosion 30% attack reduction and icons read back; all unrelated parameter fields preserved");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,"work/erosion/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Compression==RegulationCompression.Format,"Expected compatible DFLT regulation");
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  var sourceRows=names.ToDictionary(n=>n,n=>tables[n].Rows.ToLookup(r=>r.ID));
  var ids=names.ToDictionary(n=>n,n=>tables[n].Rows.Select(r=>r.ID).ToHashSet());
  PARAM.Row Add(string name,int source,int id){Check(ids[name].Add(id),$"ID collision {name}/{id}");var row=new PARAM.Row(sourceRows[name][source].Single()){ID=id};tables[name].Rows.Add(row);added[name].Add(id);return row;}
  var spNative=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  foreach(var(id,state)in Sources.Select((id,index)=>(id,new[]{2,5,6,116,260,436,437}[index]))){
   var r=spNative.Rows.Single(r=>r.ID==id);Check(Convert.ToInt32(r["stateInfo"].Value)==state,"Native state changed");Check(CageAI.Same(r,tables["SpEffectParam"].Rows.Single(r=>r.ID==id)),"Native reference modified in current package");
  }
  var h=hexes.Single(h=>h.Id==Owner);
  var owner=Add("SpEffectParam",8000000,Owner);Set(owner,"maxHpRate",1f);Set(owner,"spCategory",20);Set(owner,"iconId",h.SmallIcon);Set(owner,"spEffectTextId_1",Owner);Set(owner,"displayIconForPermamentEffect",0);Owners(owner);
  Mechanics(Add);
  var attach=Add("AttachEffectParam",95002700,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"attachTextId",Owner);Set(attach,"statusIconId",h.SmallIcon);Set(attach,"overrideBaseEffectId",Owner);
  var permanent=Add("PermanentBuffParam",95002700,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002700,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002700),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  int history=RewardCycle.Marker(hexes.Length-1);
  var marker=Add("SpEffectParam",RewardCycle.Marker(hexes.Length-2),history);
  var lots=tables["ItemLotParam_enemy"].Rows.ToDictionary(r=>r.ID);
  var inactive=RewardCatalog.InactiveRows(hexes);
  var removed=tables["ItemLotParam_enemy"].Rows.Where(r=>inactive.Contains(r.ID)).Select(r=>r.ID).ToHashSet();
  tables["ItemLotParam_enemy"].Rows.RemoveAll(r=>removed.Contains(r.ID));
  foreach(var entry in RewardCatalog.ActiveSerialized(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(lots.TryGetValue(id,out var old))Check(Convert.ToInt32(old["lotItemId01"].Value)==target,$"Existing reward triple moved at {id}: {old["lotItemId01"].Value} != {target}");
   else{var r=Add("ItemLotParam_enemy",95200000+slot,id);Set(r,"lotItemId01",target);}
  }
  foreach(var name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Name(f)==name+".param").Bytes=tables[name].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Compression==binder.Compression&&actual.Version==binder.Version&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var pair in binder.Files.Zip(actual.Files)){Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Parameter readback mismatch");if(!names.Any(n=>Name(pair.First)==n+".param"))Check(pair.Second.Bytes.Span.SequenceEqual(originals[pair.First.ID]),"Unrelated table changed");}
  foreach(var name in names){var file=actual.Files.Single(f=>Name(f)==name+".param");var before=parse(name,originals[file.ID]);var after=parse(name,file.Bytes);Check(after.Rows.Count==before.Rows.Count+added[name].Count-(name=="ItemLotParam_enemy"?removed.Count:0),"Row count changed");var previous=before.Rows.Where(r=>name!="ItemLotParam_enemy"||!removed.Contains(r.ID)).OrderBy(r=>r.ID).ToArray();var retained=after.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();Check(previous.Length==retained.Length&&previous.Zip(retained).All(p=>CageAI.Same(p.First,p.Second)),"Existing row or duplicate row changed");}
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");
  var ev=EMEVD.Read(eventInput);var evBefore=EMEVD.Read(eventInput);
  foreach(var id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);int at=e.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==RewardCycle.Marker(hexes.Length-2));Check(at>=0,"Guarded history reset anchor missing");
   e.Instructions.Insert(at+1,new(2004,21,new object[]{20000u,history}));foreach(var p in e.Parameters)if(p.InstructionIndex>at)p.InstructionIndex++;
  }
  ev.Write(eventOutput);var evAfter=EMEVD.Read(eventOutput);
  Check(evAfter.Format==evBefore.Format&&evAfter.Compression==evBefore.Compression&&evAfter.Events.Count==evBefore.Events.Count&&evAfter.StringData.SequenceEqual(evBefore.StringData)&&evAfter.LinkedFileOffsets.SequenceEqual(evBefore.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in evAfter.Events){Check(Cages.Same(e,ev.Events.Single(r=>r.ID==e.ID)),"Event readback mismatch");var copy=e;if(e.ID is 1111 or 9100941){int at=copy.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==history);Check(at>=0,"New history reset missing");copy.Instructions.RemoveAt(at);foreach(var p in copy.Parameters)if(p.InstructionIndex>at)p.InstructionIndex--;}Check(Cages.Same(copy,evBefore.Events.Single(r=>r.ID==e.ID)),"Unrelated event changed");}
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),added=added.ToDictionary(v=>v.Key,v=>v.Value.Order().ToArray()),removedInactiveRewardRows=removed.Order().ToArray(),hexCount=hexes.Length,history,effects=tables["SpEffectParam"].Rows.Where(r=>r.ID>=Owner&&r.ID<=First+6).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),existingActiveRowsAndUnrelatedRowsPreserved=true,nativeReferencesUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Erosion native attack-added ailments, stable rewards, guarded history cleanup and bilingual text staged and read back");
 }
}
