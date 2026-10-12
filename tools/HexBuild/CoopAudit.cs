using SoulsFormats;
using System.Text.Json;

static class CoopAudit {
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string output=Path.Combine(root,".work/coop-01749/staged/mod");Directory.CreateDirectory(Path.Combine(output,"event"));
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block:00}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),target=Path.Combine(output,"event",name+".emevd.dcx");uint p=(uint)(4600+block)*10000;
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changes=new List<object>();
   foreach(var e in next.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984)){
    changes.Add(new{e.ID,before=Cages.Dump(before.Events.Single(x=>x.ID==e.ID))});
    Cages.PeerReplicaLoading(e,p+9000+(uint)(e.ID-p-9900));
   }
   Check(changes.Count==84,"Missing cage births");next.Write(target);var read=EMEVD.Read(target);
   Check(read.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&read.Events.All(e=>Cages.Same(e,next.Events.Single(x=>x.ID==e.ID))),"Replica birth readback mismatch");
   Check(read.Events.Where(e=>e.ID<p+9900||e.ID>=p+9984).All(e=>Cages.Same(e,before.Events.Single(x=>x.ID==e.ID))),"Unrelated event changed");
   Check(read.StringData.SequenceEqual(before.StringData)&&read.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets)&&read.Format==before.Format&&read.Compression==before.Compression,"Event metadata changed");
   reports.Add(new{mapName=name,inputSha256=Hash(input),outputSha256=Hash(target),changes,events=read.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984).Select(Cages.Dump)});
  }
  string regulation=Path.Combine(root,"dist/mod/regulation.bin"),regOut=Path.Combine(output,"regulation.bin");
  var reg=SFUtil.DecryptNightreignRegulation(regulation);var original=reg.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=reg.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var effects=parse("SpEffectParam",file.Bytes);var beforeEffects=parse("SpEffectParam",file.Bytes);
  var ids=new[]{6999801,6999802,6999803,6999804,6999805};
  foreach(int id in ids){var row=effects.Rows.Single(r=>r.ID==id);row["isDisableNetSync"].Value=Convert.ChangeType(1,row["isDisableNetSync"].Value.GetType());}
  file.Bytes=effects.Write();SFUtil.EncryptNightreignRegulation(regOut,reg,reg.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(regOut);
  Check(reopened.Version==reg.Version&&reopened.Files.Count==reg.Files.Count&&reopened.Files.Zip(reg.Files).All(p=>p.First.ID==p.Second.ID&&p.First.Name==p.Second.Name&&p.First.Flags==p.Second.Flags&&p.First.Bytes.Span.SequenceEqual(p.Second.Bytes.Span)),"Regulation binder roundtrip mismatch");
  Check(reopened.Files.All(f=>f.ID==file.ID||f.Bytes.Span.SequenceEqual(original[f.ID])),"Unrelated parameter table changed");
  var actual=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(actual.Rows.Count==beforeEffects.Rows.Count&&actual.Rows.All(r=>CageAI.Same(r,effects.Rows.Single(x=>x.ID==r.ID))),"Effects did not roundtrip");
  foreach(var old in beforeEffects.Rows){var row=actual.Rows.Single(r=>r.ID==old.ID);foreach(var cell in old.Cells)Check(ids.Contains(old.ID)&&cell.Def.InternalName=="isDisableNetSync"||Equals(cell.Value,row[cell.Def.InternalName].Value),"Unexpected effect field changed");}
  File.WriteAllText(Path.Combine(root,".work/coop-01749/cages.json"),JsonSerializer.Serialize(reports,new JsonSerializerOptions{WriteIndented=true}));
  File.WriteAllText(Path.Combine(root,".work/coop-01749/parameters.json"),JsonSerializer.Serialize(new{inputSha256=Hash(regulation),outputSha256=Hash(regOut),localOnlyMarkers=ids,before=ids.Select(id=>new{id,isDisableNetSync=beforeEffects.Rows.Single(r=>r.ID==id)["isDisableNetSync"].Value}),onlyChangedField="isDisableNetSync",unrelatedRowsAndTablesPreserved=true},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: staged 336 replica birth events and five local-only arts control markers; unrelated data preserved");
 }
 public static void Probe(string root,Func<string,Memory<byte>,PARAM> parse,PARAM originalNpcs,string hash){
  string output=Path.Combine(root,".work/coop-01749");Directory.CreateDirectory(output);
  var opts=new JsonSerializerOptions{WriteIndented=true,IncludeFields=true};
  var threats=originalNpcs.Rows.Where(r=>r.Cells.Any(c=>System.Text.RegularExpressions.Regex.IsMatch(c.Def.InternalName,@"^spEffectID\d+$")&&Convert.ToInt32(c.Value) is >=7740 and <=7769 or >=7930 and <=7969))
   .Select(r=>r.ID).Order().ToArray();
  File.WriteAllText(Path.Combine(root,"data/reference/world/spirit-threat-npcs.json"),JsonSerializer.Serialize(new{sourceRegulationSha256=hash,ids=threats},opts));
  var reg=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  foreach(string table in new[]{"NpcParam","SpEffectParam"}){
   var rows=parse(table,reg.Files.Single(f=>Path.GetFileName(f.Name)==table+".param").Bytes).Rows;
   File.WriteAllText(Path.Combine(output,table+".json"),JsonSerializer.Serialize(rows.Where(r=>table=="NpcParam"?r.ID<95300000:r.ID>=7700&&r.ID<8000).Select(r=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),opts));
  }
  foreach(string name in new[]{"m46_50_00_00","m52_42_00_00"}){
   var original=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio",name+".msb.dcx"));
   var current=name.StartsWith("m46")?MSB_NR.Read(Path.Combine(root,"dist/mod/map/mapstudio",name+".msb.dcx")):original;
   File.WriteAllText(Path.Combine(output,name+"-parts.json"),JsonSerializer.Serialize(new{original=original.Parts.Enemies,patched=current.Parts.Enemies,generators=current.Events.Generators},opts));
  }
  var boss=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
  File.WriteAllText(Path.Combine(output,"source-parts.json"),JsonSerializer.Serialize(boss.Select(b=>new{b.npc,b.map,enemy=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio",b.map+".msb.dcx")).Parts.Enemies.Single(e=>e.EntityData.EntityID==b.entity)}),opts));
  Console.WriteLine("PASS: read-only native and cloned map actors exported");
 }
}
