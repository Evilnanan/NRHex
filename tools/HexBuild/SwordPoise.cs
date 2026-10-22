using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class SwordPoise {
 public const int Effect=95000916,Source=1650;
 static readonly string[] Fields={"dmgLv_None","dmgLv_S","dmgLv_M","dmgLv_L","dmgLv_BlowM","dmgLv_Push","dmgLv_Strike","dmgLv_BlowS","dmgLv_Min","dmgLv_Uppercut","dmgLv_BlowLL","dmgLv_Breath","defFlickPower","saReceiveDamageRate"};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Apply(PARAM.Row row,PARAM.Row source){foreach(string field in Fields)row[field].Value=source[field].Value;}
 public static void Validate(PARAM.Row row,PARAM.Row source){
  Check(Convert.ToInt32(source["spCategory"].Value)==1001&&Convert.ToInt32(source["categoryPriority"].Value)==0&&Convert.ToSingle(source["saReceiveDamageRate"].Value)==1f&&Convert.ToInt32(source["defFlickPower"].Value)==35,"Native Endure reference changed");
  Check(Fields.All(f=>Equals(row[f].Value,source[f].Value)),"Sword reactions/poise damage/rebound differ from native Endure");
  Check(Convert.ToInt32(row["spCategory"].Value)==1001&&Convert.ToInt32(row["categoryPriority"].Value)==0&&Convert.ToSingle(row["effectEndurance"].Value)==-1f&&Convert.ToInt32(row["saveCategory"].Value)==-1&&Convert.ToInt32(row["vfxId"].Value)==-1,"Sword casting lifetime/category/visual changed");
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var original=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var effects=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  Validate(effects.Rows.Single(r=>r.ID==Effect),original.Rows.Single(r=>r.ID==Source));
  Check(CageAI.Same(original.Rows.Single(r=>r.ID==Source),effects.Rows.Single(r=>r.ID==Source)),"Original Endure changed");
  Console.WriteLine("PASS: installed Sword casting protection matches native Endure; lifetime and original skill preserved");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/sword-endure/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==native.Version,"Regulation version changed");
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);
  var original=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);var source=original.Rows.Single(r=>r.ID==Source);
  var row=table.Rows.Single(r=>r.ID==Effect);var expected=new PARAM.Row(row);Apply(expected,source);Apply(row,source);Validate(row,source);
  file.Bytes=table.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var wanted=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Regulation readback mismatch");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var old=parse("SpEffectParam",before[file.ID]);var actual=parse("SpEffectParam",final.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(old.Rows.Count==actual.Rows.Count&&old.Rows.Zip(actual.Rows).All(p=>CageAI.Same(p.First.ID==Effect?expected:p.First,p.Second)),"Unrelated effect row or field changed");
  Validate(actual.Rows.Single(r=>r.ID==Effect),source);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),effect=Effect,source=Source,fields=Fields.ToDictionary(f=>f,f=>row[f].Value),onlySwordProtectionChanged=true,unrelatedRowsTablesAndFieldsPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Sword Endure protection staged; unrelated parameters and descriptions preserved");
 }
}
