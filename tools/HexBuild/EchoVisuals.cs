using System.Text.Json;
using System.Security.Cryptography;
using SoulsFormats;
static class EchoVisuals {
 record Visual(int Magic,int Root,int Bullet,int Sfx,float Life,string Tae,long Animation);
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static string Name(BinderFile file)=>file.Name.Replace('\\','/').Split('/').Last();
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static Visual[] Read(string root){
  using var json=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"data/balance/echo-cast-visuals.json")));
  Check(json.RootElement.GetProperty("version").GetInt32()==10350000&&json.RootElement.GetProperty("template").GetInt32()==10503011,"Echo visual reference changed");
  var result=json.RootElement.GetProperty("visuals").EnumerateArray().Select(row=>new Visual(row.GetProperty("magic").GetInt32(),row.GetProperty("root").GetInt32(),row.GetProperty("bullet").GetInt32(),row.GetProperty("sfx").GetInt32(),row.GetProperty("life").GetSingle(),row.GetProperty("tae").GetString()!,row.GetProperty("animation").GetInt64())).ToArray();
  Check(result.Length==3&&result.Select(row=>row.Bullet).SequenceEqual(new[]{95003590,95003591,95003592}),"Echo visual IDs changed");
  var binder=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
  foreach(var visual in result){
   var animation=TAE.Read(binder.Files.Single(file=>file.Name.EndsWith(visual.Tae)).Bytes).Animations.Single(anim=>anim.ID==visual.Animation);
   Check(animation.Events.Any(evt=>evt.Type==96&&BitConverter.ToInt32(evt.GetParameterBytes(false))==visual.Sfx),"Echo visual absent from native release animation");
  }
  return result;
 }
 static void Configure(PARAM.Row row,Visual visual){
  foreach(var(field,value)in new(string,object)[]{("atkId_Bullet",-1),("sfxId_Bullet",visual.Sfx),("life",visual.Life),("numShoot",1),("hitRadius",0f),("sfxPostureType",0)})Set(row,field,value);
  foreach(var field in new[]{"spEffectId0","spEffectId1","spEffectId2","spEffectId3","spEffectId4"})Set(row,field,-1);
 }
 public static void Parameters(string root,Func<string,int,int,PARAM.Row> clone){foreach(var visual in Read(root))Configure(clone("Bullet",10503011,visual.Bullet),visual);}
 public static void Verify(string root,PARAM table){
  foreach(var visual in Read(root)){
   var expected=new PARAM.Row(table.Rows.Single(row=>row.ID==10503011)){ID=visual.Bullet};Configure(expected,visual);
   Check(CageAI.Same(expected,table.Rows.Single(row=>row.ID==visual.Bullet)),"Echo visual readback mismatch");
   Check(new[]{"initVellocity","minVellocity","maxVellocity","gravityInRange","gravityOutRange","FollowType","isSyncBulletCulcDumypolyPos","isUseSharedHitList"}.All(field=>Convert.ToSingle(expected[field].Value)==0),"Echo visual must be stationary and independent");
   Check(new[]{"HitBulletID","intervalCreateBulletId","sfxId_Hit","sfxId_Flick"}.All(field=>Convert.ToInt32(expected[field].Value)==-1),"Echo visual must not create damage or child chains");
  }
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  var input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var file=binder.Files.Single(file=>Name(file)=="Bullet.param");var table=parse("Bullet",file.Bytes);var old=parse("Bullet",file.Bytes);
  var ids=Read(root).Select(row=>row.Bullet).ToHashSet();
  PARAM.Row Clone(string _,int source,int id){Check(!table.Rows.Any(row=>row.ID==id),"Echo visual ID collision");var row=new PARAM.Row(table.Rows.Single(row=>row.ID==source)){ID=id};table.Rows.Add(row);return row;}
  Parameters(root,Clone);Verify(root,table);table.Rows=table.Rows.OrderBy(row=>row.ID).ToList();file.Bytes=table.Write();
  var directory=Path.Combine(root,"work/echo-release/staged");Directory.CreateDirectory(directory);var output=Path.Combine(directory,"regulation.bin");
  SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var result=SFUtil.DecryptNightreignRegulation(output);
  Check(result.Files.Count==binder.Files.Count&&result.Version==binder.Version&&result.Compression==binder.Compression,"Echo visual binder metadata changed");
  foreach(var entry in result.Files){var intended=binder.Files.Single(original=>original.ID==entry.ID);Check(entry.Name==intended.Name&&entry.Flags==intended.Flags&&entry.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Echo visual binder readback mismatch");if(entry.ID!=file.ID)Check(entry.Bytes.Span.SequenceEqual(before[entry.ID]),"Unrelated parameter changed");}
  var current=parse("Bullet",result.Files.Single(entry=>entry.ID==file.ID).Bytes);Verify(root,current);
  Check(current.Rows.Count==old.Rows.Count+ids.Count&&old.Rows.OrderBy(row=>row.ID).Zip(current.Rows.Where(row=>!ids.Contains(row.ID)).OrderBy(row=>row.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Native bullet row changed");
  string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),addedBulletIds=ids.Order().ToArray(),visualOnly=true,nativeRowsUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: three visual-only echo carriers staged; native rows and other parameters unchanged");
 }
}
