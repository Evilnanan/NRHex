using SoulsFormats;
using System.Text.Json;

static partial class NightBlessing{
 public static void Rate(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  ValidateNative(native,parse);
  string directory=Path.Combine(root,"work/night-blessing-rate/staged");Directory.CreateDirectory(directory);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(directory,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  Check(binder.Version==native.Version,"Installed regulation version changed");
  var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var member=binder.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param");
  var old=parse("SpEffectParam",member.Bytes);var effects=parse("SpEffectParam",member.Bytes);
  var generated=new Dictionary<int,PARAM.Row>();
  Mechanics((name,source,identity)=>{
   Check(name=="SpEffectParam","Unexpected table");
   var row=new PARAM.Row(effects.Rows.Single(candidate=>candidate.ID==source)){ID=identity};
   generated.Add(identity,row);return row;
  });
  foreach(int identity in new[]{Owner+1,Owner+2}){
   var row=effects.Rows.Single(candidate=>candidate.ID==identity);
   Check(Convert.ToSingle(row["motionInterval"].Value)==1f&&Convert.ToSingle(row["effectEndurance"].Value)==15f,"Healing timer changed");
   Check(Convert.ToSingle(row["changeHpRate"].Value) is -4f or -6f,"Unexpected previous healing rate");
   int previous=Convert.ToInt32(row["changeHpPoint"].Value);
   Check(previous==(identity==Owner+1?-30:-60)||previous==(identity==Owner+1?-45:-90),"Unexpected previous fixed healing");
   foreach(string field in new[]{"changeHpRate","changeHpPoint"})Set(row,field,generated[identity][field].Value);
  }
  member.Bytes=effects.Write();SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var file in final.Files){
   var expected=binder.Files.Single(candidate=>candidate.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Readback mismatch");
   if(file.ID!=member.ID)Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated parameter table changed");
  }
  var actual=parse("SpEffectParam",final.Files.Single(file=>file.ID==member.ID).Bytes);
  var originalEffects=parse("SpEffectParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param").Bytes);
  Check(actual.Rows.Count==old.Rows.Count,"Unexpected row count");
  foreach(var row in old.Rows){
   var current=actual.Rows.Single(candidate=>candidate.ID==row.ID);
   var restored=new PARAM.Row(current);
   if(row.ID==Owner+1||row.ID==Owner+2){
    var rain=originalEffects.Rows.Single(candidate=>candidate.ID==(row.ID==Owner+1?99810:99811));
    Check(Convert.ToSingle(current["changeHpRate"].Value)==-3f*Convert.ToSingle(rain["changeHpRate"].Value),"Healing percentage is not three times native rain");
    Check(Convert.ToInt32(current["changeHpPoint"].Value)==-3*Convert.ToInt32(rain["changeHpPoint"].Value),"Fixed healing is not three times native rain");
    foreach(string field in new[]{"changeHpRate","changeHpPoint"})Set(restored,field,row[field].Value);
   }
   Check(CageAI.Same(row,restored),"Unrelated effect field changed");
  }
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{
   inputSha256=Hash(input),outputSha256=Hash(output),multiplier=3,
   changedEffects=new[]{Owner+1,Owner+2},changedFields=new[]{"changeHpRate","changeHpPoint"},
   recoveryPerSecond=new[]{new{maxHpPercent=6,flat=45},new{maxHpPercent=6,flat=90}},
   onlyHealingRateChanged=true,gameplayVerified=false
  },new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: both healing tiers equal 3x native base rain; only four fields changed; all other parameters preserved");
 }
}
