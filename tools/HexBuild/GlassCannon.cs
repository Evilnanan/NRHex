using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class GlassCannon {
 public const float AttackRate=1.25f;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,".work/glass-cannon-01769/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),target=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var rows=parse("SpEffectParam",file.Bytes);var before=parse("SpEffectParam",file.Bytes);
  var row=rows.Rows.Single(r=>r.ID==95000300);
  string[] fields=["physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"];
  foreach(string f in fields){
   float old=Convert.ToSingle(row[f].Value);Check(old==1.4f||old==1.2f||old==AttackRate,"Unexpected Glass Cannon attack multiplier");
   row[f].Value=AttackRate;
  }
  foreach(string f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Check(Convert.ToSingle(row[f].Value)==10f,"Unexpected incoming damage multiplier");
  foreach(string f in new[]{"physicsAttackPowerRate","magicAttackPowerRate","fireAttackPowerRate","thunderAttackPowerRate","darkAttackPowerRate"})Check(Convert.ToSingle(row[f].Value)==1f,"Unexpected pre-defense attack modifier");
  file.Bytes=rows.Write();
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
   if(r.ID==95000300&&fields.Contains(cell.Def.InternalName))Check(Convert.ToSingle(actual)==AttackRate,"Attack multiplier readback mismatch");
   else Check(JsonSerializer.Serialize(actual)==JsonSerializer.Serialize(cell.Value),$"Unrelated field changed: {r.ID}/{cell.Def.InternalName}");
  }
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:[95000300]);
  File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(new{version="0.17.69",inputSha256=Hash(input),outputSha256=Hash(target),effectId=95000300,fields,attackRate=AttackRate,incomingDamageRate=10f,unrelatedRowsAndFieldsPreserved=true,hexes=hexes.Where(h=>h.Id==95000300),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Glass Cannon attack multiplier 1.25, incoming x10 retained; unrelated fields preserved; Chinese and English texts verified");
 }
}
