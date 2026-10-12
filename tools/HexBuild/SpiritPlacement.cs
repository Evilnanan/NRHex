using SoulsFormats;
using System.Text.Json;

// Maintenance/placement evidence and the corpse-only cue's minimal param patch.
static class SpiritPlacement {
 public static void Probe(string root,Func<string,Memory<byte>,PARAM> parse){

 var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
 var rows=new Dictionary<string,object>();
 foreach(string table in new[]{"NpcParam","SpEffectParam"}){
 var p=parse(table,current.Files.Single(f=>Path.GetFileName(f.Name)==table+".param").Bytes);
 rows[table]=p.Rows.Where(r=>table=="NpcParam"?r.ID is 95300003 or 95300036 or 95300037:r.ID==95001802).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();
 }
 var output=Path.Combine(root,".work/spirit-placement-01744");Directory.CreateDirectory(output);
 File.WriteAllText(Path.Combine(output,"parameter-evidence.json"),JsonSerializer.Serialize(rows));

 }
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void StageVisibility(string root,Func<string,Memory<byte>,PARAM> parse){
  void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/spirit-cue-01755");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var effects=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var cue=effects.Rows.Single(r=>r.ID==95001802);
  Check(Convert.ToInt32(cue["vfxId"].Value)==95001803&&Convert.ToInt32(cue["dontDeleteOnDead"].Value)==1&&Convert.ToSingle(cue["effectEndurance"].Value)==1.5f,"Capture effect survival/link/duration changed");
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param");
  var rows=parse("SpEffectVfxParam",file.Bytes);var before=parse("SpEffectVfxParam",file.Bytes);
  var row=rows.Rows.Single(r=>r.ID==95001803);const string field="isVisibleDeadChr";
  int old=Convert.ToInt32(row[field].Value);Check(old==0||old==1,"Invalid corpse visibility flag");
  Check(Convert.ToInt32(row["midstSfxId"].Value)==461351&&Convert.ToInt32(row["midstDmyId"].Value)==220,"Capture mist resource/attachment changed");
  row[field].Value=Convert.ChangeType(1,row[field].Value.GetType());file.Bytes=rows.Write();
  string target=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(target);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var member in reopened.Files){
   var expected=binder.Files.Single(f=>f.ID==member.ID);
   Check(member.Name==expected.Name&&member.Flags==expected.Flags&&member.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder roundtrip mismatch");
   if(member.ID!=file.ID)Check(member.Bytes.Span.SequenceEqual(original[member.ID]),"Unrelated table changed");
  }
  var after=parse("SpEffectVfxParam",reopened.Files.Single(f=>f.ID==file.ID).Bytes).Rows.ToDictionary(r=>r.ID);
  Check(before.Rows.Count==after.Count,"Row count changed");
  foreach(var r in before.Rows)foreach(var cell in r.Cells){
   var actual=after[r.ID][cell.Def.InternalName].Value;
   if(r.ID==95001803&&cell.Def.InternalName==field)Check(Convert.ToInt32(actual)==1,"Visibility readback mismatch");
   else Check(JsonSerializer.Serialize(actual)==JsonSerializer.Serialize(cell.Value),$"Unrelated field changed: {r.ID}/{cell.Def.InternalName}");
  }
  File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(new{version="0.17.55",inputSha256=Hash(input),outputSha256=Hash(target),table="SpEffectVfxParam",rowId=95001803,field,before=old,after=1,effectId=95001802,sfxId=461351,dummyId=220,durationSeconds=1.5,unrelatedRowsAndFieldsPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: capture mist corpse visibility {old} -> 1; all other parameter fields preserved");
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
 string input=Path.Combine(root,"dist/mod/regulation.bin");
 var binder=SFUtil.DecryptNightreignRegulation(input);
 var entry=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
 var original=entry.Bytes.ToArray();var before=parse("SpEffectParam",original);
 var effects=parse("SpEffectParam",original);
 var cue=effects.Rows.Single(r=>r.ID==95001802);
 if(Convert.ToInt32(cue["vfxId"].Value)!=95001803||Convert.ToSingle(cue["effectEndurance"].Value)!=1.5f)
 throw new InvalidDataException("Unexpected capture cue");
 cue["dontDeleteOnDead"].Value=Convert.ChangeType(1,cue["dontDeleteOnDead"].Value.GetType());
 entry.Bytes=effects.Write();
 var output=Path.Combine(root,".work/spirit-placement-01744/staged");Directory.CreateDirectory(output);
 string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
 var read=SFUtil.DecryptNightreignRegulation(path);
 if(read.Version!=binder.Version||read.Files.Count!=binder.Files.Count||read.Compression!=binder.Compression)throw new InvalidDataException("Binder metadata changed");
 foreach(var f in binder.Files){var r=read.Files.Single(r=>r.ID==f.ID);if(r.Name!=f.Name||r.Flags!=f.Flags||!r.Bytes.Span.SequenceEqual(f.Bytes.Span))throw new InvalidDataException("Readback differs");}
 var final=parse("SpEffectParam",read.Files.Single(f=>f.ID==entry.ID).Bytes);
 if(final.Rows.Count!=before.Rows.Count)throw new InvalidDataException("Row count differs");
 int changes=0;
 foreach(var row in before.Rows){var now=final.Rows.Single(r=>r.ID==row.ID);foreach(var cell in row.Cells.Where(c=>c!=null)){
 if(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(now[cell.Def.InternalName].Value))continue;
 if(row.ID!=95001802||cell.Def.InternalName!="dontDeleteOnDead")throw new InvalidDataException("Unrelated field changed");changes++;
 }}
 if(changes!=1)throw new InvalidDataException("Expected exactly one changed field");
 File.WriteAllText(Path.Combine(output,"parameter-validation.json"),JsonSerializer.Serialize(new{version="0.17.44",inputSha256=Hash(input),outputSha256=Hash(path),changes,changedRow=95001802,changedField="dontDeleteOnDead",value=1,otherFieldsAndTablesUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("PASS: corpse cue survives death cleanup; one field changed; readback matched");
 }
}
