using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class UltimateRefreshVfx {
 const int Cooldown=95000402,Vfx=199852;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Apply(PARAM effects,PARAM visuals){
  var native=effects.Rows.Single(r=>r.ID==6999611);
  Check(Convert.ToSingle(native["ultimateArtGauge"].Value)==100f&&Convert.ToInt32(native["vfxId"].Value)==Vfx,"Native Favor of the Great Hollow refill changed");
  var visual=visuals.Rows.Single(r=>r.ID==Vfx);
  Check(Convert.ToInt32(visual["initSfxId"].Value)==803800&&Convert.ToInt32(visual["initDmyId"].Value)==220&&Convert.ToInt32(visual["midstSfxId"].Value)==-1&&Convert.ToInt32(visual["finishSfxId"].Value)==-1,"Native refill visual changed");
  // Runtime emits COOLDOWN only once, after a later Update confirms a full gauge.
  // Its initial-only VFX plays at that confirmation, with no cooldown-duration loop.
  var row=effects.Rows.Single(r=>r.ID==Cooldown);
  Check(Convert.ToSingle(row["effectEndurance"].Value)==60f&&Convert.ToSingle(row["ultimateArtGauge"].Value)==0f&&Convert.ToInt32(row["replaceSpEffectId"].Value)==-1&&Convert.ToInt32(row["cycleOccurrenceSpEffectId"].Value)==-1,"Unexpected Ultimate Refresh cooldown");
  Check(Convert.ToInt32(row["vfxId"].Value) is -1 or Vfx,"Unexpected Ultimate Refresh visual");
  row["vfxId"].Value=Vfx;
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,bool cooldownOnly=false){
  string field=cooldownOnly?"effectEndurance":"vfxId";
  int value=cooldownOnly?60:Vfx;
  string dir=Path.Combine(root,cooldownOnly?".work/ultimate-refresh-cooldown/staged":".work/ultimate-refresh-vfx/staged");Directory.CreateDirectory(dir);
  var common=BND4.Read(Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx"));
  foreach(string name in new[]{"f000803800.fxr","f000803800.ffxreslist"})
   Check(common.Files.Any(f=>Path.GetFileName(f.Name)==name),"Missing shared native refill resource: "+name);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var before=parse("SpEffectParam",file.Bytes);var effects=parse("SpEffectParam",file.Bytes);
  var visuals=parse("SpEffectVfxParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
  if(cooldownOnly){
   var cooldown=effects.Rows.Single(row=>row.ID==Cooldown);
   Check(Convert.ToSingle(cooldown[field].Value) is 45f or 60f,"Unexpected existing cooldown duration");
   cooldown[field].Value=60f;
   Check(Convert.ToInt32(cooldown["vfxId"].Value)==Vfx,"Unexpected cooldown visual");
  }
  Apply(effects,visuals);file.Bytes=effects.Write();
  SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(output);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var member in reopened.Files){
   var expected=binder.Files.Single(f=>f.ID==member.ID);
   Check(member.Name==expected.Name&&member.Flags==expected.Flags&&member.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder roundtrip mismatch");
   if(member.ID!=file.ID)Check(member.Bytes.Span.SequenceEqual(originals[member.ID]),"Unrelated table changed");
  }
  var after=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==file.ID).Bytes).Rows.ToDictionary(r=>r.ID);
  Check(before.Rows.Count==after.Count,"Effect count changed");
  foreach(var row in before.Rows)foreach(var cell in row.Cells){
   var actual=after[row.ID][cell.Def.InternalName].Value;
   if(row.ID==Cooldown&&cell.Def.InternalName==field)Check(Convert.ToSingle(actual)==value,"Cooldown field readback mismatch");
   else Check(JsonSerializer.Serialize(actual)==JsonSerializer.Serialize(cell.Value),$"Unrelated field changed: {row.ID}/{cell.Def.InternalName}");
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{
   inputSha256=Hash(input),outputSha256=Hash(output),effectId=Cooldown,nativeSourceEffect=6999611,vfxId=Vfx,sfxId=803800,dummyId=220,
   changedField=field,value,onlySelectedCooldownFieldChanged=true,initialOnly=true,existingCommonResourceVerified=true,gameplayVerified=false
  },new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: Ultimate Refresh only 95000402/{field} changed to {value}; all other parameters preserved");
 }
 public static void Probe(string root,BND4 binder,Func<string,Memory<byte>,PARAM> parse){
  var effects=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var visuals=parse("SpEffectVfxParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
  var chosen=effects.Rows.Where(r=>r.ID>=6999610&&r.ID<=6999619||r.ID==99823).ToArray();
  var ids=chosen.Select(r=>Convert.ToInt32(r["vfxId"].Value)).ToHashSet();
  string dir=Path.Combine(root,".work/ultimate-refresh-vfx");Directory.CreateDirectory(dir);
  object Row(PARAM.Row r)=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var events=new List<object>();
  foreach(string path in Directory.EnumerateFiles(Path.Combine(root,"assets/original/event"),"*.emevd.dcx")){
   foreach(var ev in EMEVD.Read(path).Events.Where(e=>e.ID!=0&&e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>BitConverter.ToInt32(i.ArgData,n*4) is >=6999610 and <=6999613))))
    events.Add(new{file=Path.GetFileName(path),ev.ID,instructions=ev.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})});
  }
  File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(new{effects=chosen.Select(Row),vfx=visuals.Rows.Where(r=>ids.Contains(r.ID)).Select(Row),events},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native Favor of the Great Hollow parameters, linked visuals and event references exported");
 }
}
