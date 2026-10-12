using SoulsFormats;
using System.Text.Json;

static class SpiritRework {
 static void Check(bool ok,string message){if(!ok)throw new InvalidDataException(message);}
 static void Set(PARAM.Row row,string field,object value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Configure(Func<string,int,int,PARAM.Row> add,PARAM.Row drain){
  Set(drain,"changeHpRate",100f/60f);Set(drain,"changeHpPoint",0);Set(drain,"motionInterval",1f);
  foreach(string field in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(drain,field,.25f);
  Set(drain,"vfxId",95001804);
  var capture=add("SpEffectParam",8000000,95001802);
  Set(capture,"maxHpRate",1f);Set(capture,"saveCategory",-1);Set(capture,"iconId",-1);Set(capture,"spEffectTextId_1",-1);Set(capture,"displayIconForPermamentEffect",0);
  Set(capture,"effectEndurance",1.5f);Set(capture,"spCategory",20);Set(capture,"vfxId",95001803);Set(capture,"effectTargetPcDeceased",1);Set(capture,"dontDeleteOnDead",1);
  // Native Necromancy's body mist: PlayerCommonParam +170 = 461351,
  // created at dummy 220 by 4CC3B0. Visual only, no death controller.
  var collectVfx=add("SpEffectVfxParam",8500501,95001803);
  Set(collectVfx,"initSfxId",-1);Set(collectVfx,"initDmyId",-1);
  Set(collectVfx,"midstSfxId",461351);Set(collectVfx,"midstDmyId",220);
  // The collection cue is attached after the defeated boss enters death state.
  Set(collectVfx,"isVisibleDeadChr",1);
  // Family c4340 a1830 uses 460801. Use the common body dummy (220)
  // instead of that family's model-specific 960 so boss models can show it.
  var spawnVfx=add("SpEffectVfxParam",8500501,95001804);
  Set(spawnVfx,"initSfxId",460801);Set(spawnVfx,"initDmyId",220);
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string output=Path.Combine(root,".work/spirit-rework-01743/staged");Directory.CreateDirectory(output);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","SpEffectVfxParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var added=new Dictionary<string,HashSet<int>>{{names[0],new()},{names[1],new()}};
  PARAM.Row Add(string table,int source,int id){var p=tables[table];Check(!p.Rows.Any(r=>r.ID==id),$"ID collision: {table} {id}");var row=new PARAM.Row(p.Rows.Single(r=>r.ID==source)){ID=id};p.Rows.Add(row);added[table].Add(id);return row;}
  var drain=tables["SpEffectParam"].Rows.Single(r=>r.ID==95001801);
  Check(Convert.ToSingle(drain["changeHpRate"].Value)==0f,"Expected pre-rework passive");Configure(Add,drain);
  foreach(string name in names){tables[name].Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));binder.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes=tables[name].Write();}
  string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var read=SFUtil.DecryptNightreignRegulation(path);Check(read.Version==binder.Version&&read.Files.Count==binder.Files.Count&&read.Compression==binder.Compression,"Binder metadata changed");
  foreach(var f in read.Files){var expected=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");}
  var fields=new List<object>();
  var allowed=new HashSet<string>{"changeHpRate","changeHpPoint","motionInterval","physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate","vfxId"};
  foreach(string name in names){
   var final=parse(name,read.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
   Check(final.Rows.Count==baseline[name].Rows.Count+added[name].Count,"Wrong row count");
   foreach(var r in baseline[name].Rows){var now=final.Rows.Single(v=>v.ID==r.ID);foreach(var c in r.Cells.Where(c=>c!=null)){
    if(JsonSerializer.Serialize(c.Value)==JsonSerializer.Serialize(now[c.Def.InternalName].Value))continue;
    Check(name=="SpEffectParam"&&r.ID==95001801&&allowed.Contains(c.Def.InternalName),"Unrelated parameter field changed");
    fields.Add(new{table=name,r.ID,field=c.Def.InternalName,before=c.Value,after=now[c.Def.InternalName].Value});
   }}
   foreach(int id in added[name])Check(final.Rows.Any(r=>r.ID==id),"Missing new row");
  }
  HexTexts.Stage(root,hexes,Path.Combine(output,"texts"),allRewards:true,onlyIds:[95001800]);
  File.WriteAllText(Path.Combine(output,"parameter-validation.json"),JsonSerializer.Serialize(new{version="0.17.43",inputSha256=Hash(input),outputSha256=Hash(path),fields,added=added.ToDictionary(p=>p.Key,p=>p.Value.ToArray()),native705200Unchanged=true,hpLossFractionPerSecond=1d/60d,damageMultiplier=.1,collectionSfx=461351,summonSfx=460801,unrelatedRowsAndTablesUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: spirit drain/damage/cues staged and re-read; original passive and other rows unchanged");
 }
 public static void Probe(string root,BND4 binder,Func<string,Memory<byte>,PARAM> parse){
  var all=new Dictionary<string,object>();
  foreach(string name in new[]{"SpEffectParam","SpEffectVfxParam","BuddyParam","NpcParam"}){
   var table=parse(name,binder.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
   IEnumerable<PARAM.Row> rows=table.Rows;
   if(name=="SpEffectParam")rows=rows.Where(r=>r.ID is >=270000 and <298000 or >=705000 and <=705299);
   if(name=="SpEffectVfxParam")rows=rows.Where(r=>r.ID is >=270000 and <298000 or >=503000 and <=504000 || r.Cells.Any(c=>c!=null&&c.Value is int v&&v>=460000&&v<462000));
   if(name=="BuddyParam")rows=rows.Where(r=>r.ID is >=27000000 and <=27200010);
   if(name=="NpcParam")rows=rows.Where(r=>r.ID is 137030000 or 143400000 or 149610000 or 25000040 or 30500110);
   all[name]=rows.Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();
  }
  var dir=Path.Combine(root,".work/spirit-rework-01743");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"native-parameters.json"),JsonSerializer.Serialize(all));
  var animations=new List<object>();
  var bundles=new List<object>();
  foreach(string file in Directory.GetFiles(Path.Combine(dir,"extracted/chr"),"*.anibnd.dcx")){
   var b=BND4.Read(file);
   bundles.Add(new{file=Path.GetFileName(file),files=b.Files.Select(f=>new{f.ID,f.Name})});
   foreach(var entry in b.Files.Where(f=>f.Name.EndsWith(".tae"))){
    var tae=TAE.Read(entry.Bytes);
    bundles.Add(new{tae=entry.Name,animationIds=tae.Animations.Select(a=>a.ID)});
    foreach(var a in tae.Animations.Where(a=>a.ID%1000000 is >=1820 and <=1835))
     animations.Add(new{file=Path.GetFileName(file),entry=entry.Name,a.ID,header=a.MiniHeader.ToString(),events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});
   }
  }
  File.WriteAllText(Path.Combine(dir,"native-animations.json"),JsonSerializer.Serialize(animations));
  File.WriteAllText(Path.Combine(dir,"native-bundles.json"),JsonSerializer.Serialize(bundles));
 }
}
