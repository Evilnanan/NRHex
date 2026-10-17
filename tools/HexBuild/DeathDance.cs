using SoulsFormats;
using System.Text.Json;
static class DeathDance {
 public const int Owner=95003000;
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam"};
  var output=new Dictionary<string,object>();
  foreach(var name in names){var table=parse(name,original.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()==name+".param").Bytes);output[name]=table.Rows.Where(r=>r.ID is 8440200 or 7005600 or 7005601 or 7005602 or 501000 or 7010220 or 7010230 or 8000000 || name=="SpEffectParam"&&Convert.ToInt32(r["stateInfo"].Value)==449).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();}
  string dir=Path.Combine(root,"work/death-dance");Directory.CreateDirectory(dir);File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(output,new JsonSerializerOptions{WriteIndented=true}));
 }

 public const int Flask=Owner+1;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string field,object value)=>r[field].Value=Convert.ChangeType(value,r[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Owners(PARAM.Row r){Set(r,"stateInfo",449);Set(r,"regainRate",1f);}
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var r=clone("SpEffectParam",8440200,Flask);
  Set(r,"saveCategory",-1);Set(r,"iconId",-1);Set(r,"spEffectTextId_1",-1);Set(r,"displayIconForPermamentEffect",0);Set(r,"isDisableNetSync",1);
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,"work/death-dance/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Compression==RegulationCompression.Format,"Expected compatible DFLT regulation");
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int source,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"ID collision {name}/{id}");var row=new PARAM.Row(tables[name].Rows.Single(r=>r.ID==source)){ID=id};tables[name].Rows.Add(row);added[name].Add(id);return row;}
  var spNative=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  foreach(var(id,state)in new[]{(8440200,2015),(7005600,449),(7005601,449),(7005602,449)}){
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
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(lots.TryGetValue(id,out var old))Check(Convert.ToInt32(old["lotItemId01"].Value)==target,"Existing reward triples moved");
   else{var r=Add("ItemLotParam_enemy",95200000+slot,id);Set(r,"lotItemId01",target);}
  }
  foreach(var name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Name(f)==name+".param").Bytes=tables[name].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Compression==binder.Compression&&actual.Version==binder.Version&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var pair in binder.Files.Zip(actual.Files)){Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Parameter readback mismatch");if(!names.Any(n=>Name(pair.First)==n+".param"))Check(pair.Second.Bytes.Span.SequenceEqual(originals[pair.First.ID]),"Unrelated table changed");}
  foreach(var name in names){var file=actual.Files.Single(f=>Name(f)==name+".param");var before=parse(name,originals[file.ID]);var after=parse(name,file.Bytes);Check(after.Rows.Count==before.Rows.Count+added[name].Count,"Row count changed");var previous=before.Rows.OrderBy(r=>r.ID).ToArray();var retained=after.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();Check(previous.Length==retained.Length&&previous.Zip(retained).All(p=>CageAI.Same(p.First,p.Second)),"Existing row or duplicate row changed");}
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
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),added=added.ToDictionary(v=>v.Key,v=>v.Value.Order().ToArray()),hexCount=hexes.Length,history,effects=tables["SpEffectParam"].Rows.Where(r=>r.ID is Owner or Flask).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),existingRowsAndTriplesPreserved=true,nativeReferencesUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Death's Dance native flask/regain effects, stable rewards, guarded history cleanup and bilingual text staged and read back");
 }
}
