using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class EscapeVfx {
 public const int Visual=95001204;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Apply(PARAM effects,PARAM visuals,PARAM bullets){
  var native=bullets.Rows.Single(r=>r.ID==30903000);
  Check(Convert.ToInt32(native["spEffectId0"].Value)==703540&&Convert.ToInt32(native["sfxId_Bullet"].Value)==460820,"Native Restage visual chain changed");
  Check(Convert.ToInt32(effects.Rows.Single(r=>r.ID==703540)["stateInfo"].Value)==601,"Native Restage state changed");
  var cue=new PARAM.Row(visuals.Rows.Single(r=>r.ID==104392)){ID=Visual};
  // Initial-only emitter: reuse the Restage visual, never its attack bullet/state.
  cue["initSfxId"].Value=460820;cue["initDmyId"].Value=(short)220;
  Check(Convert.ToInt32(cue["midstSfxId"].Value)==-1&&Convert.ToInt32(cue["finishSfxId"].Value)==-1,"Visual template is not initial-only");
  var existing=visuals.Rows.SingleOrDefault(r=>r.ID==Visual);
  if(existing!=null)Check(CageAI.Same(existing,cue),"Escape VFX ID collision");
  else {visuals.Rows.Add(cue);visuals.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));}
  var effect=effects.Rows.Single(r=>r.ID==Visual);
  Check(Convert.ToInt32(effect["vfxId"].Value) is 5280 or Visual&&Convert.ToSingle(effect["effectEndurance"].Value) is 10f or 0.2f,"Unexpected Escape visual baseline");
  Check(Convert.ToInt32(effect["stateInfo"].Value)==0&&Convert.ToInt32(effect["replaceSpEffectId"].Value)==-1&&Convert.ToInt32(effect["cycleOccurrenceSpEffectId"].Value)==-1,"Escape cue must have no gameplay state or chain");
  effect["vfxId"].Value=Visual;effect["effectEndurance"].Value=0.2f;
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/escape-duchess-vfx/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","SpEffectVfxParam"};
  var files=names.ToDictionary(n=>n,n=>binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param"));
  var before=files.ToDictionary(v=>v.Key,v=>parse(v.Key,v.Value.Bytes));
  var tables=files.ToDictionary(v=>v.Key,v=>parse(v.Key,v.Value.Bytes));
  Apply(tables["SpEffectParam"],tables["SpEffectVfxParam"],parse("Bullet",binder.Files.Single(f=>Path.GetFileName(f.Name)=="Bullet.param").Bytes));
  foreach(var (name,file) in files)file.Bytes=tables[name].Write();
  SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(output);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var file in reopened.Files){
   var expected=binder.Files.Single(f=>f.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder roundtrip mismatch");
   if(!files.Values.Any(f=>f.ID==file.ID))Check(file.Bytes.Span.SequenceEqual(original[file.ID]),"Unrelated table changed");
  }
  foreach(string name in names){
   var after=parse(name,reopened.Files.Single(f=>f.ID==files[name].ID).Bytes);
   var byId=after.Rows.ToDictionary(r=>r.ID);
   Check(after.Rows.Count==tables[name].Rows.Count,"Unexpected row count");
   foreach(var row in tables[name].Rows)Check(CageAI.Same(row,byId[row.ID]),"Parameter readback mismatch");
   foreach(var row in before[name].Rows)foreach(var cell in row.Cells){
    if(name=="SpEffectParam"&&row.ID==Visual&&cell.Def.InternalName is "vfxId" or "effectEndurance")continue;
    Check(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(byId[row.ID][cell.Def.InternalName].Value),$"Unrelated field changed: {name}/{row.ID}/{cell.Def.InternalName}");
   }
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),effectId=Visual,vfxId=Visual,nativeBullet=30903000,sfxId=460820,dummyId=220,initialOnly=true,visualDuration=0.2f,activeDuration=10f,cooldownDuration=60f,unrelatedParametersPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Escape Plan uses initial-only Restage VFX; two visual-effect fields and one private VFX row; all gameplay parameters preserved");
 }
 public static void Probe(string root,BND4 binder,Func<string,Memory<byte>,PARAM> parse){
  var effects=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var visuals=parse("SpEffectVfxParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
  var chosen=effects.Rows.Where(r=>r.ID>=703540&&r.ID<703550).ToArray();
  var ids=chosen.Select(r=>Convert.ToInt32(r["vfxId"].Value)).ToHashSet();
  string dir=Path.Combine(root,".work/escape-duchess-vfx");Directory.CreateDirectory(dir);
  object Row(PARAM.Row r)=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var animations=new List<object>();
  foreach(var file in BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx")).Files.Where(f=>f.Name.EndsWith(".tae")))
   foreach(var a in TAE.Read(file.Bytes).Animations.Where(a=>a.ID%1000000==9101||a.Events.Any(e=>Enumerable.Range(0,e.GetParameterBytes(false).Length/4).Any(n=>BitConverter.ToInt32(e.GetParameterBytes(false),n*4) is >=703540 and <703550))))
    animations.Add(new{file=file.Name,a.ID,events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});
  var weapons=parse("EquipParamWeapon",binder.Files.Single(f=>Path.GetFileName(f.Name)=="EquipParamWeapon.param").Bytes);
  var behaviors=parse("BehaviorParam_PC",binder.Files.Single(f=>Path.GetFileName(f.Name)=="BehaviorParam_PC.param").Bytes).Rows.Where(r=>r.ID%10000 is 6150 or 6185||Convert.ToInt32(r["behaviorJudgeId"].Value) is 6150 or 6185).ToArray();
  var bulletTable=parse("Bullet",binder.Files.Single(f=>Path.GetFileName(f.Name)=="Bullet.param").Bytes);
  var bullets=behaviors.Where(r=>Convert.ToInt32(r["refType"].Value)==1).Select(r=>Convert.ToInt32(r["refId"].Value)).ToHashSet();
  foreach(var b in bulletTable.Rows.Where(r=>r.ID is >=7030000 and <7040000||r.Cells.Any(c=>c.Def.InternalName.Contains("spEffect",StringComparison.OrdinalIgnoreCase)&&Convert.ToInt32(c.Value) is >=703540 and <703550)))bullets.Add(b.ID);
  for(int pass=0;pass<8;pass++)foreach(var b in bulletTable.Rows.Where(r=>bullets.Contains(r.ID)).ToArray())foreach(string f in new[]{"HitBulletID"})if(Convert.ToInt32(b[f].Value)>=0)bullets.Add(Convert.ToInt32(b[f].Value));
  File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(new{effects=chosen.Select(Row),vfx=visuals.Rows.Where(r=>ids.Contains(r.ID)).Select(Row),animations,weapons=weapons.Rows.Where(r=>Convert.ToInt32(r["equipModelId"].Value)==5031).Select(Row),behaviors=behaviors.Select(Row),bullets=bulletTable.Rows.Where(r=>bullets.Contains(r.ID)).Select(Row)},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native Duchess Restage effects and animations exported");
 }
}
