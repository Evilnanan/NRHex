using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class SwordRegain {
 public static void Apply(PARAM.Row marker){marker["stateInfo"].Value=(ushort)449;}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static void Check(bool condition,string message){if(!condition)throw new InvalidDataException(message);}
 public static void Export(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  string directory=Path.Combine(root,".work/sword-regain");Directory.CreateDirectory(directory);
  var effects=parse("SpEffectParam",original.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param").Bytes);
  var rows=new[]{7005600,7005601,7005602}.Select(id=>effects.Rows.Single(row=>row.ID==id)).ToArray();
  foreach(var (row,rate) in rows.Zip(new[]{1f,1.25f,1.35f})){
   Check(Convert.ToInt32(row["stateInfo"].Value)==449&&Convert.ToSingle(row["regainRate"].Value)==rate,"Native regain tier changed");
   effects.Rows=[row];File.WriteAllBytes(Path.Combine(directory,$"effect-{row.ID}.param"),effects.Write());
  }
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  Export(root,original,parse);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/sword-regain");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(member=>member.ID,member=>member.Bytes.ToArray());
  var member=binder.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param");
  var previous=parse("SpEffectParam",member.Bytes);var effects=parse("SpEffectParam",member.Bytes);
  var marker=effects.Rows.Single(row=>row.ID==95000912);
  Check(Convert.ToInt32(marker["stateInfo"].Value) is 0 or 449,"Unexpected Sword marker state");
  Check(Convert.ToSingle(marker["effectEndurance"].Value)==-1f&&Convert.ToSingle(marker["regainRate"].Value)==1f,"Unexpected Sword marker lifetime or regain multiplier");
  Apply(marker);member.Bytes=effects.Write();
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(output);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var actual in reopened.Files){
   var expected=binder.Files.Single(file=>file.ID==actual.ID);
   Check(actual.Name==expected.Name&&actual.Flags==expected.Flags&&actual.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regain asset readback failed");
   if(actual.ID!=member.ID)Check(actual.Bytes.Span.SequenceEqual(before[actual.ID]),"Unrelated parameter member changed");
  }
  var final=parse("SpEffectParam",reopened.Files.Single(file=>file.ID==member.ID).Bytes);
  Check(final.Rows.Count==previous.Rows.Count,"Effect row count changed");
  foreach(var pair in previous.Rows.Zip(final.Rows)){
   var restored=new PARAM.Row(pair.Second);
   if(restored.ID==95000912){Check(Convert.ToInt32(restored["stateInfo"].Value)==449,"Native regain state missing");restored["stateInfo"].Value=pair.First["stateInfo"].Value;}
   Check(CageAI.Same(pair.First,restored),"Unrelated effect row or field changed");
  }
  var native=parse("SpEffectParam",original.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param").Bytes);
  Check(Convert.ToInt32(native.Rows.Single(row=>row.ID==7005600)["stateInfo"].Value)==449,"Native regain reference changed");
  File.WriteAllText(Path.Combine(directory,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedEffect=95000912,changedFields=new[]{"stateInfo"},nativeState=449,onlyIntendedFieldChanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Sword marker enables native regain state 449; only its stateInfo changed, all other parameters preserved");
 }
}
