using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class BalloonBalance {
 public const float ThrustDamageRate=2.6f; // General x1.3, additional piercing damage taken x2.
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,".work/balloon-balance-01768/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),target=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var rows=parse("SpEffectParam",file.Bytes);var before=parse("SpEffectParam",file.Bytes);
  var row=rows.Rows.Single(r=>r.ID==95001700);const string field="thrustDamageCutRate";
  float old=Convert.ToSingle(row[field].Value);
  Check(old==1.3f||old==1.95f||old==ThrustDamageRate,"Unexpected Balloon piercing multiplier");
  Check(Convert.ToSingle(row["maxHpRate"].Value)==1.5f,"Unexpected Balloon HP multiplier");
  foreach(string f in new[]{"slashDamageCutRate","blowDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Check(Convert.ToSingle(row[f].Value)==1.3f,"Unexpected general damage multiplier");
  row[field].Value=ThrustDamageRate;file.Bytes=rows.Write();
  SFUtil.EncryptNightreignRegulation(target,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(target);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var member in reopened.Files){
   var expected=binder.Files.Single(f=>f.ID==member.ID);
   Check(member.Name==expected.Name&&member.Flags==expected.Flags&&member.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder roundtrip mismatch");
   if(member.ID!=file.ID)Check(member.Bytes.Span.SequenceEqual(original[member.ID]),"Unrelated table changed");
  }
  var after=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==file.ID).Bytes).Rows.ToDictionary(r=>r.ID);
  Check(before.Rows.Count==after.Count,"Row count changed");
  foreach(var r in before.Rows)foreach(var cell in r.Cells){
   var actual=after[r.ID][cell.Def.InternalName].Value;
   if(r.ID==95001700&&cell.Def.InternalName==field)Check(Convert.ToSingle(actual)==ThrustDamageRate,"Piercing multiplier readback mismatch");
   else Check(JsonSerializer.Serialize(actual)==JsonSerializer.Serialize(cell.Value),$"Unrelated field changed: {r.ID}/{cell.Def.InternalName}");
  }
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:[95001700]);
  File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(new{version="0.17.68",inputSha256=Hash(input),outputSha256=Hash(target),effectId=95001700,field,before=old,after=ThrustDamageRate,unrelatedRowsAndFieldsPreserved=true,hexes=hexes.Where(h=>h.Id==95001700),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Balloon piercing multiplier 1.3 -> 2.6; unrelated fields preserved; Chinese and English texts verified");
 }
}
