using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class RedEnvelope {
 const int Grant=95000102,Vfx=104392;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Apply(PARAM effects,PARAM visuals){
  // Rune of the Strong plays an initial SFX on its instantaneous rune grant.
  // Reuse that reference on our already once-only payout, with no extra reward.
  var native=effects.Rows.Single(r=>r.ID==8500102);
  Check(Convert.ToInt32(native["soul"].Value)==30000&&Convert.ToSingle(native["effectEndurance"].Value)==0f&&Convert.ToInt32(native["vfxId"].Value)==Vfx,"Native Rune of the Strong contract changed");
  var visual=visuals.Rows.Single(r=>r.ID==Vfx);
  Check(Convert.ToInt32(visual["initSfxId"].Value)==43&&Convert.ToInt32(visual["initDmyId"].Value)==220&&Convert.ToInt32(visual["midstSfxId"].Value)==-1&&Convert.ToInt32(visual["finishSfxId"].Value)==-1,"Native rune acquisition VFX changed");
  var row=effects.Rows.Single(r=>r.ID==Grant);
  Check(Convert.ToInt32(row["soul"].Value)==88888&&Convert.ToSingle(row["effectEndurance"].Value)==0f&&Convert.ToInt32(row["replaceSpEffectId"].Value)==-1&&Convert.ToInt32(row["cycleOccurrenceSpEffectId"].Value)==-1,"Unexpected Red Envelope payout");
  Check(Convert.ToInt32(row["vfxId"].Value) is -1 or Vfx,"Unexpected Red Envelope visual");
  row["vfxId"].Value=Vfx;
 }
 public static void Probe(string root,BND4 binder,Func<string,Memory<byte>,PARAM> parse){
  var effects=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var vfx=parse("SpEffectVfxParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
  var goods=parse("EquipParamGoods",binder.Files.Single(f=>Path.GetFileName(f.Name)=="EquipParamGoods.param").Bytes);
  var chosen=effects.Rows.Where(r=>Convert.ToInt32(r["soul"].Value)>0).ToArray();
  var vfxIds=chosen.Select(r=>Convert.ToInt32(r["vfxId"].Value)).ToHashSet();
  string dir=Path.Combine(root,".work/red-envelope");Directory.CreateDirectory(dir);
  object Row(PARAM.Row r)=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(new{
   effects=chosen.Select(Row),vfx=vfx.Rows.Where(r=>vfxIds.Contains(r.ID)||r.ID==104392).Select(Row),
   goods=goods.Rows.Where(r=>r.ID>=8500100&&r.ID<=8500102).Select(Row)
  },new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native rune grant effects, linked VFX and Rune of the Strong goods exported");
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/red-envelope/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var before=parse("SpEffectParam",file.Bytes);var effects=parse("SpEffectParam",file.Bytes);
  var visuals=parse("SpEffectVfxParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
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
   if(row.ID==Grant&&cell.Def.InternalName=="vfxId")Check(Convert.ToInt32(actual)==Vfx,"VFX readback mismatch");
   else Check(JsonSerializer.Serialize(actual)==JsonSerializer.Serialize(cell.Value),$"Unrelated field changed: {row.ID}/{cell.Def.InternalName}");
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{
   inputSha256=Hash(input),outputSha256=Hash(output),effectId=Grant,vfxId=Vfx,sfxId=43,dummyId=220,
   runeAmount=88888,onlyPayoutVfxChanged=true,gameplayVerified=false
  },new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Red Envelope payout references native Rune of the Strong initial VFX; only 95000102/vfxId changed; all other parameters preserved");
 }
}
