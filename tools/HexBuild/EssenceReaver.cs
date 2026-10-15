using SoulsFormats;
using System.Text.Json;

static class EssenceReaver {
 public const int Owner=95002900,Pulse=Owner+1,Layer=Owner+2,Reset=Owner+3,First=95006000;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 static void Neutral(PARAM.Row r){foreach(var (f,v) in new (string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);}
 static void Saved(PARAM.Row r){Neutral(r);Set(r,"saveCategory",10);Set(r,"dontDeleteOnDead",1);Set(r,"isDisableNetSync",1);}
 public static void Owners(PARAM.Row owner){Set(owner,"displayIconForPermamentEffect",1);Set(owner,"stateInfo",199);Set(owner,"applyIdOnKillSp",Pulse);Set(owner,"applyIdOnGetSoul",Pulse);}
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var pulse=clone("SpEffectParam",8000000,Pulse);Neutral(pulse);Set(pulse,"effectEndurance",0f);Set(pulse,"saveCategory",-1);Set(pulse,"isDisableNetSync",1);

  Saved(clone("SpEffectParam",8000000,Reset));
  for(int digit=0;digit<4;digit++)for(int value=1;value<256;value++)Saved(clone("SpEffectParam",8000000,First+digit*256+value));
 }
 public static void StageSingle(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/essence-single");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);Check(b.Compression==RegulationCompression.Format,"Expected compatible DFLT compression");
  var f=b.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var original=f.Bytes.ToArray();var sp=parse("SpEffectParam",original);
  var owner=sp.Rows.Single(r=>r.ID==Owner);Owners(owner);
  Check(sp.Rows.RemoveAll(r=>r.ID==Layer)==1,"Obsolete layer row missing");
  f.Bytes=sp.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var current=SFUtil.DecryptNightreignRegulation(output);Check(current.Compression==b.Compression&&current.Version==b.Version&&current.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var pair in b.Files.Zip(current.Files))Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Binder roundtrip changed");
  var before=parse("SpEffectParam",original);var after=parse("SpEffectParam",current.Files.Single(v=>v.ID==f.ID).Bytes);Check(before.Rows.Count==after.Rows.Count+1,"Row count changed");
  var changes=new List<object>();foreach(var old in before.Rows.Where(r=>r.ID!=Layer)){var row=after.Rows.Single(r=>r.ID==old.ID);if(old.ID!=Owner){Check(CageAI.Same(old,row),"Unrelated row changed");continue;}
   foreach(var cell in old.Cells.Where(c=>c!=null)){var name=cell.Def.InternalName;var actual=row[name].Value;if(Equals(cell.Value,actual))continue;
    var allowed=new[]{"displayIconForPermamentEffect"};
    Check(allowed.Contains(name),"Unexpected field changed");changes.Add(new{id=old.ID,field=name,before=cell.Value,after=actual});}}
  File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),compression=current.Compression.ToString(),removed=new[]{Layer},changes,effects=after.Rows.Where(r=>r.ID>=Owner&&r.ID<=Reset||r.ID>=First&&r.ID<First+1024||r.ID==RewardCycle.Marker(25)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),events=EMEVD.Read(Path.Combine(root,"dist/mod/event/common.emevd.dcx")).Events.Where(e=>e.ID is 1111 or 9100941).Select(Cages.Dump),unrelatedRowsAndFilesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: one owner icon; per-kill FP layer removed; all unrelated rows and files preserved");
 }
 public static void ExportChecks(string root,Func<string,Memory<byte>,PARAM> parse){
  var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var sp=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var ev=EMEVD.Read(Path.Combine(root,"dist/mod/event/common.emevd.dcx"));
  var dir=Path.Combine(root,".work/essence-reaver");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"installed.json"),JsonSerializer.Serialize(new{effects=sp.Rows.Where(r=>r.ID>=Owner&&r.ID<=Reset||r.ID>=First&&r.ID<First+1024||r.ID==RewardCycle.Marker(25)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),events=ev.Events.Where(e=>e.ID is 1111 or 9100941).Select(Cages.Dump)},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Current Essence Reaver parameter and guarded lifecycle checks exported");
 }
 public static void Events(EMEVD ev,bool includeHistory=false){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   int at=e.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==8&&BitConverter.ToInt32(i.ArgData,4)==RewardCycle.Reset);
   Check(at>=0&&!e.Instructions.Any(i=>i.Bank==2004&&i.ID==8&&BitConverter.ToInt32(i.ArgData,4)==Reset),"Missing guarded reset or duplicate Essence Reaver reset");
   var added=new List<EMEVD.Instruction>();
   if(includeHistory)added.Add(new(2004,21,new object[]{20000u,RewardCycle.Marker(25)}));
   added.Add(new(2004,21,new object[]{20000u,Reset}));added.Add(new(2004,8,new object[]{20000u,Reset}));
   e.Instructions.InsertRange(at,added);foreach(var p in e.Parameters)if(p.InstructionIndex>=at)p.InstructionIndex+=added.Count;
  }
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var directory=Path.Combine(root,".work/essence-reaver/staged");Directory.CreateDirectory(directory);
  var native=parse("SpEffectParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var onKill=native.Rows.Single(r=>r.ID==361100);var relic=native.Rows.Single(r=>r.ID==7500501);
  Check(Convert.ToInt32(onKill["stateInfo"].Value)==199&&Convert.ToInt32(onKill["applyIdOnKillSp"].Value)==361102&&Convert.ToInt32(onKill["applyIdOnGetSoul"].Value)==361101,"Native on-kill route changed");
  Check(Convert.ToInt32(relic["iconId"].Value)==20292&&Convert.ToSingle(relic["maxMpRate"].Value)==1.0075f,"Native Revenant maximum-FP reference changed");
  var icons=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"data/reference/ui/icon-ids.json")));
  var h=hexes.Single(h=>h.Id==Owner);Check(h.LargeIcon==21292&&h.SmallIcon==20292,"Maximum-FP icon catalog mismatch");
  Check(new[]{("item",h.LargeIcon),("status",h.SmallIcon)}.All(x=>icons.RootElement.EnumerateArray().Any(e=>e.GetProperty("kind").GetString()==x.Item1&&e.GetProperty("id").GetInt32()==x.Item2)),"Native icon missing");
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Regulation version mismatch");
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"ID collision {n}/{id}");var r=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var owner=Add("SpEffectParam",8000000,Owner);Neutral(owner);Set(owner,"iconId",h.SmallIcon);Set(owner,"spEffectTextId_1",Owner);Owners(owner);
  var attach=Add("AttachEffectParam",95002700,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"attachTextId",Owner);Set(attach,"statusIconId",h.SmallIcon);Set(attach,"overrideBaseEffectId",Owner);
  var permanent=Add("PermanentBuffParam",95002700,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002700,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002700),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  Mechanics(Add);Saved(Add("SpEffectParam",8000000,RewardCycle.Marker(25)));
  var lots=tables["ItemLotParam_enemy"].Rows.ToDictionary(r=>r.ID);
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(lots.TryGetValue(id,out var existing))Check(Convert.ToInt32(existing["lotItemId01"].Value)==target,"Existing reward triples moved");
   else{var r=Add("ItemLotParam_enemy",95200000+slot,id);Set(r,"lotItemId01",target);}
  }
  foreach(var n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Files.Count==binder.Files.Count&&final.Compression==binder.Compression,"Binder metadata changed");
  foreach(var f in final.Files){var expected=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  foreach(var n in names){var old=parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]);var actual=parse(n,final.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);Check(actual.Rows.Count==old.Rows.Count+added[n].Count,"Unexpected row count");foreach(var pair in old.Rows.OrderBy(r=>r.ID).Zip(actual.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)))Check(CageAI.Same(pair.First,pair.Second),"Existing parameter row changed");}
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(directory,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);Events(ev,true);ev.Write(eventOutput);var current=EMEVD.Read(eventOutput);
  Check(current.Format==oldEv.Format&&current.Compression==oldEv.Compression&&current.Events.Count==oldEv.Events.Count&&current.StringData.SequenceEqual(oldEv.StringData)&&current.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");
  Check(ev.Events.All(e=>Cages.Same(e,current.Events.Single(v=>v.ID==e.ID))),"Event serialization mismatch");
  var eventReport=current.Events.Where(e=>e.ID is 1111 or 9100941).Select(e=>JsonSerializer.SerializeToElement(Cages.Dump(e))).ToArray();
  foreach(var e in oldEv.Events){var n=current.Events.Single(v=>v.ID==e.ID);if(e.ID is 1111 or 9100941){int at=n.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==RewardCycle.Marker(25));Check(at>=0,"Reset block missing");n.Instructions.RemoveRange(at,3);foreach(var p in n.Parameters)if(p.InstructionIndex>=at+3)p.InstructionIndex-=3;}Check(Cages.Same(e,n),"Unrelated event changed");}
  RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexes,added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),effects=tables["SpEffectParam"].Rows.Where(r=>r.ID>=Owner&&r.ID<=Reset||r.ID>=First&&r.ID<First+1024||r.ID==RewardCycle.Marker(25)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),events=eventReport,previousEvents=oldEv.Events.Where(e=>e.ID is 1111 or 9100941).Select(Cages.Dump),allExistingRowsAndRewardTriplesPreserved=true,onlyTwoGuardedEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Essence Reaver, native FP layers, compact saved count, stable reward triples, guarded resets and bilingual texts staged and verified");
 }
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var sp=parse("SpEffectParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var direct=sp.Rows.Where(r=>Convert.ToInt32(r["iconId"].Value)==20492).Select(r=>r.ID).ToHashSet();
  var refs=sp.Rows.Where(r=>r.Cells.Any(c=>c!=null&&c.Def.InternalName.Contains("SpEffect",StringComparison.OrdinalIgnoreCase)&&direct.Contains(Convert.ToInt32(c.Value)))).Select(r=>r.ID).ToHashSet();
  var attach=parse("AttachEffectParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="AttachEffectParam.param").Bytes);
  var a=attach.Rows.Single(r=>r.ID==6500500);
  var ids=direct.Concat(refs).Concat(Enumerable.Range(7500500,61)).Concat(new[]{361100,361101,361102,8630100,8630101,8630102,8000000}).Concat(a.Cells.Where(c=>c!=null&&c.Def.InternalName.StartsWith("passiveSpEffectId")).Select(c=>Convert.ToInt32(c.Value))).Where(id=>id>0).ToHashSet();
  for(int i=0;i<4;i++)foreach(var r in sp.Rows.Where(r=>ids.Contains(r.ID)).ToArray())foreach(string field in new[]{"replaceSpEffectId","cycleOccurrenceSpEffectId","atkOccurrenceSpEffectId"})if(Convert.ToInt32(r[field].Value)>0)ids.Add(Convert.ToInt32(r[field].Value));
  var selected=sp.Rows.Where(r=>ids.Contains(r.ID)||r.ID==8000000||r.ID==8600100||r.ID==8600200).ToArray();
  var dir=Path.Combine(root,".work/essence-reaver");Directory.CreateDirectory(dir);
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var text=new List<object>();
  foreach(var lang in new[]{"zhocn","engus"})foreach(var name in new[]{"item_dlc01.msgbnd.dcx","menu_dlc01.msgbnd.dcx"}){
   var b=BND4.Read(Path.Combine(root,"assets/original/msg",lang,name));
   foreach(var f in b.Files.Where(f=>f.Name.EndsWith(".fmg")))foreach(var e in FMG.Read(f.Bytes).Entries.Where(e=>e.Text!=null&&(e.Text.Contains("专注值上限")||e.Text.Contains("maximum FP")||e.Text.Contains("打倒敌人时")||e.Text.Contains("defeating enemies",StringComparison.OrdinalIgnoreCase))))text.Add(new{lang,file=Path.GetFileName(f.Name),e.ID,e.Text});
  }
  File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(new{effects=selected.Select(Dump),attach=Dump(a),iconEffects=direct,referenceEffects=refs,text},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Native 20492 icon effects, incoming references and maximum-FP texts exported");
 }
}
