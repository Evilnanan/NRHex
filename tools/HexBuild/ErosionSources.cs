using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;
static class ErosionSources {
 static int V(PARAM.Row r,string key)=>Convert.ToInt32(r[key].Value);
 static string Name(BinderFile f)=>Path.GetFileName(f.Name.Replace('\\','/'));
 static bool Same(PARAM.Row a,PARAM.Row b){
  var options=new JsonSerializerOptions{NumberHandling=System.Text.Json.Serialization.JsonNumberHandling.AllowNamedFloatingPointLiterals};
  return a.ID==b.ID&&a.Name==b.Name&&a.Cells.Count==b.Cells.Count&&a.Cells.Zip(b.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&JsonSerializer.Serialize(p.First.Value,options)==JsonSerializer.Serialize(p.Second.Value,options));
 }
 static void Check(bool b,string message){if(!b)throw new InvalidDataException(message);}
 public static void Catalog(string root,string input,string output,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(output);
  var b=SFUtil.DecryptNightreignRegulation(input);
  PARAM Table(string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var bullets=Table("Bullet").Rows.ToDictionary(r=>r.ID);
  var behaviors=Table("BehaviorParam_PC").Rows.ToDictionary(r=>r.ID);
  var effects=Table("SpEffectParam").Rows.ToDictionary(r=>r.ID);
  using var manifest=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"tools/erosion-sources.json")));
  var roots=manifest.RootElement.GetProperty("bullets").EnumerateObject().Select(p=>int.Parse(p.Name)).Where(bullets.ContainsKey).ToHashSet();
  var behaviorIds=manifest.RootElement.GetProperty("behaviors").EnumerateObject().Select(p=>int.Parse(p.Name)).ToHashSet();
  // Follow the actual target's attachment effects, including new MMV rows.
  var effectIds=Table("AttachEffectParam").Rows.SelectMany(r=>r.Cells.Where(c=>c.Def.InternalName.StartsWith("passiveSpEffectId_")).Select(c=>Convert.ToInt32(c.Value))).Where(id=>id>0).ToHashSet();
  var queue=new Queue<int>(effectIds);
  while(queue.TryDequeue(out var id))if(effects.TryGetValue(id,out var e)){
   if(V(e,"behaviorId")>0)behaviorIds.Add(V(e,"behaviorId"));
   foreach(var field in new[]{"replaceSpEffectId","cycleOccurrenceSpEffectId","atkOccurrenceSpEffectId"}){
    int child=V(e,field);if(child>0&&effectIds.Add(child))queue.Enqueue(child);
   }
  }
  foreach(int id in behaviorIds)if(behaviors.TryGetValue(id,out var behavior)&&V(behavior,"refType")==1&&bullets.ContainsKey(V(behavior,"refId")))roots.Add(V(behavior,"refId"));
  var ids=new HashSet<int>(roots);queue=new Queue<int>(roots);
  while(queue.TryDequeue(out var id))if(bullets.TryGetValue(id,out var row))foreach(var field in new[]{"HitBulletID","intervalCreateBulletId"}){
   int child=V(row,field);if(child>=0&&bullets.ContainsKey(child)&&ids.Add(child))queue.Enqueue(child);
  }
  // Record ambiguous reuse explicitly. Native goods and magic provenance takes
  // precedence at runtime; do not modify shared attack rows or their tags.
  var magicRoots=Table("Magic").Rows.SelectMany(r=>Enumerable.Range(1,10).Where(i=>r.Cells.Any(c=>c.Def.InternalName=="refCategory"+i)&&V(r,"refCategory"+i)==1).Select(i=>V(r,"refId"+i))).ToHashSet();
  HashSet<int> Closure(IEnumerable<int> seeds){
   var result=seeds.Where(bullets.ContainsKey).ToHashSet();var todo=new Queue<int>(result);
   while(todo.TryDequeue(out var id))foreach(var field in new[]{"HitBulletID","intervalCreateBulletId"}){
    int child=V(bullets[id],field);if(bullets.ContainsKey(child)&&result.Add(child))todo.Enqueue(child);
   }return result;
  }
  var shared=ids.Intersect(Closure(magicRoots)).Order().ToArray();
  var otherBehaviors=behaviors.Values.Where(r=>!behaviorIds.Contains(r.ID)&&V(r,"refType")==1).ToArray();
  var sharedOtherBehaviorBullets=ids.Intersect(Closure(otherBehaviors.Select(r=>V(r,"refId")))).Order().ToArray();
  foreach(int required in new[]{12421005,12421006,12301020,12301101,12301120,98802001})if(bullets.ContainsKey(required))Check(ids.Contains(required),"Missing passive projectile "+required);
  File.WriteAllText(Path.Combine(output,"erosion_sources.rs"),"// Generated from the target regulation; native item/spell provenance has priority.\npub const PASSIVE_BULLETS:&[i32]=&[\n"+string.Join("",ids.Order().Select(id=>$" {id},\n"))+"];\n");
  File.WriteAllText(Path.Combine(output,"erosion-sources-validation.json"),JsonSerializer.Serialize(new{inputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(input))),roots=roots.Order().ToArray(),passiveBullets=ids.Order().ToArray(),sharedMagicBullets=shared,sharedOtherBehaviorBullets,otherBehaviorReferences=otherBehaviors.Where(r=>ids.Contains(V(r,"refId"))).Select(r=>new{r.ID,bullet=V(r,"refId")}).ToArray(),behaviorIds=behaviorIds.Order().ToArray(),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"Erosion source catalog: {roots.Count} roots, {ids.Count} projectile rows, {shared.Length} shared spell bullets, {sharedOtherBehaviorBullets.Length} shared behavior bullets");
 }
 public static void TuneSpell(string input,string output,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(output);var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);
  string[] powers={"poizonAttackPower","diseaseAttackPower","bloodAttackPower","curseAttackPower","freezeAttackPower","sleepAttackPower","madnessAttackPower"};
  var changes=new List<object>();
  for(int i=0;i<7;i++){
   var row=table.Rows.Single(r=>r.ID==95003130+i);int old=V(row,powers[i]);Check(old==10,"Unexpected spell buildup "+row.ID);
   row[powers[i]].Value=Convert.ChangeType(Erosion.SpellPower,row[powers[i]].Value.GetType());
   changes.Add(new{id=row.ID,field=powers[i],before=old,after=Erosion.SpellPower});
  }
  file.Bytes=table.Write();string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(path);Check(actual.Files.Count==b.Files.Count&&actual.Version==b.Version&&actual.Compression==b.Compression,"Binder metadata changed");
  foreach(var pair in b.Files.Zip(actual.Files)){
   var f=pair.Second;Check(pair.First.ID==f.ID&&pair.First.Name==f.Name&&pair.First.Flags==f.Flags&&pair.First.Bytes.Span.SequenceEqual(f.Bytes.Span),"Roundtrip mismatch");
   if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");
  }
  var oldRows=parse("SpEffectParam",before[file.ID]).Rows;var now=parse("SpEffectParam",actual.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(oldRows.Count==now.Rows.Count,"Row count changed");
  foreach(var row in oldRows){
   if(row.ID>=95003130&&row.ID<=95003136)row[powers[row.ID-95003130]].Value=Convert.ChangeType(Erosion.SpellPower,row[powers[row.ID-95003130]].Value.GetType());
   Check(Same(row,now.Rows.Single(r=>r.ID==row.ID)),"Unrelated row or field changed "+row.ID);
  }
  VerifyTiers(path,parse);
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(input))),outputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),changes,onlySevenSpellBuildupFieldsChanged=true},new JsonSerializerOptions{WriteIndented=true}));
 }
 public static void VerifyTiers(string input,Func<string,Memory<byte>,PARAM> parse){
  var b=SFUtil.DecryptNightreignRegulation(input);var table=parse("SpEffectParam",b.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  string[] powers={"poizonAttackPower","diseaseAttackPower","bloodAttackPower","curseAttackPower","freezeAttackPower","sleepAttackPower","madnessAttackPower"};
  foreach(var (first,power)in new[]{(95003110,30),(95003120,10),(95003130,Erosion.SpellPower),(95003140,5)})for(int i=0;i<7;i++){
   var row=table.Rows.Single(r=>r.ID==first+i);
   for(int j=0;j<7;j++)Check(V(row,powers[j])==(i==j?power:0),"Incorrect tier/ailment power "+row.ID);
   Check(V(row,"isUseStatusAilmentAtkPowerCorrect")==1,"Native status correction disabled");
  }
  Console.WriteLine($"PASS: all 28 Erosion payloads read back: weapon30/spell{Erosion.SpellPower}/item10/passive5; native correction enabled");
 }
 public static void Stage(string root,string input,string output,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(output);var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);var old=table.Rows.ToDictionary(r=>r.ID);
  string[] powers={"poizonAttackPower","diseaseAttackPower","bloodAttackPower","curseAttackPower","freezeAttackPower","sleepAttackPower","madnessAttackPower"};
  var added=new List<int>();
  void Add(PARAM.Row r){Check(!old.ContainsKey(r.ID),"Erosion source ID already exists: "+r.ID);table.Rows.Add(r);added.Add(r.ID);}
  foreach(var (carrier,first,power)in new[]{(95003103,95003130,Erosion.SpellPower),(95003104,95003140,5)}){
   Add(new PARAM.Row(old[Erosion.Carrier]){ID=carrier});
   for(int i=0;i<7;i++){var r=new PARAM.Row(old[Erosion.First+i]){ID=first+i};r[powers[i]].Value=Convert.ChangeType(power,r[powers[i]].Value.GetType());Add(r);}
  }
  Check(table.Rows.Count<65536,"SpEffectParam capacity exceeded");table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=table.Write();
  var path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,b,b.Compression);var after=SFUtil.DecryptNightreignRegulation(path);
  foreach(var f in after.Files){Check(f.Bytes.Span.SequenceEqual(b.Files.Single(o=>o.ID==f.ID).Bytes.Span),"Roundtrip mismatch");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var actual=parse("SpEffectParam",after.Files.Single(f=>f.ID==file.ID).Bytes);Check(actual.Rows.Count==old.Count+16,"Unexpected row count");
  foreach(var row in actual.Rows)Check(Same(row,old.TryGetValue(row.ID,out var prior)?prior:table.Rows.Single(r=>r.ID==row.ID)),"Row changed: "+row.ID);
  File.WriteAllText(Path.Combine(output,"erosion-payloads-validation.json"),JsonSerializer.Serialize(new{inputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(input))),outputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),added,existingRowsAndOtherTablesUnchanged=true},new JsonSerializerOptions{WriteIndented=true}));
  Catalog(root,path,output,parse);
 }
}
