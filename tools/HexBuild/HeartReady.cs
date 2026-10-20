using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class HeartReady {
 public const int Ready=95002708;
 static void Check(bool valid,string message){if(!valid)throw new InvalidDataException(message);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static string Name(BinderFile file)=>Path.GetFileName(file.Name.Replace('\\','/'));
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,PARAM nativeVisuals){
  var charm=nativeVisuals.Rows.Single(row=>row.ID==5190);
  Check(Convert.ToInt32(charm["midstSfxId"].Value)==4090,"Native charm visual changed");
  var ready=clone("SpEffectParam",8000000,Ready);
  foreach(var(field,value)in new(string,object)[]{("maxHpRate",1f),("effectEndurance",2f),("saveCategory",-1),("spCategory",20),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0),("vfxId",Ready),("vfxId1",-1)})Set(ready,field,value);
  var visual=clone("SpEffectVfxParam",5190,Ready);
  Set(visual,"effectType",0);Set(visual,"midstDmyId",1);Set(visual,"midstSeId",-1);Set(visual,"midstSfxId",HeartReadyFog.Sfx);
  Set(visual,"isUseOffsetEnchantSfxSize",0);Set(visual,"traceSfxIdOffsetType",0);
  for(int index=0;index<8;index++){Set(visual,$"enchantStartDmyId_{index}",-1);Set(visual,$"enchantEndDmyId_{index}",-1);}
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string directory=Path.Combine(root,"work/heart-ready/right-hand-staged");Directory.CreateDirectory(directory);
  string modelPath=Path.Combine(root,"work/heart-ready/native-model/chr/c0000.chrbnd.dcx");
  var modelBinder=BND4.Read(modelPath);
  var model=FLVER2.Read(modelBinder.Files.Single(file=>Name(file)=="c0000.flver").Bytes);
  var hand=model.Dummies.Single(dummy=>dummy.ReferenceID==1);
  var nodeProperty=model.GetType().GetProperty("Nodes")??model.GetType().GetProperty("Bones");
  var nodes=JsonSerializer.SerializeToElement(nodeProperty!.GetValue(model));
  Check(hand.FollowAttachBone&&nodes[hand.AttachBoneIndex].GetProperty("Name").GetString()=="R_Hand","Native right-hand dummy changed");
  string overridePath=Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects.ffxbnd.dcx");
  var common=BND4.Read(File.Exists(overridePath)?overridePath:Path.Combine(root,".work/storm-vfx-01757/extracted/sfx/sfxbnd_commoneffects.ffxbnd.dcx"));
  Check(common.Files.Any(file=>Name(file)=="f000004090.fxr"),"Native charm FXR missing from common effects");
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(directory,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);var original=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var tables=new[]{"SpEffectParam","SpEffectVfxParam"}.ToDictionary(name=>name,name=>parse(name,binder.Files.Single(file=>Name(file)==name+".param").Bytes));
  var sources=tables.Keys.ToDictionary(name=>name,name=>parse(name,native.Files.Single(file=>Name(file)==name+".param").Bytes));
  var added=new Dictionary<string,List<int>>{{"SpEffectParam",new()},{"SpEffectVfxParam",new()}};
  Mechanics((name,source,identity)=>{
   var previous=tables[name].Rows.SingleOrDefault(row=>row.ID==identity);
   if(previous==null)added[name].Add(identity);else tables[name].Rows.Remove(previous);
   var row=new PARAM.Row(sources[name].Rows.Single(row=>row.ID==source)){ID=identity,Name=previous?.Name};tables[name].Rows.Add(row);return row;
  },sources["SpEffectVfxParam"]);
  foreach(var(name,table)in tables){table.Rows=table.Rows.OrderBy(row=>row.ID).ToList();binder.Files.Single(file=>Name(file)==name+".param").Bytes=table.Write();}
  SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var file in final.Files){
   var expected=binder.Files.Single(candidate=>candidate.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Resource readback mismatch");
   string tableName=Path.GetFileNameWithoutExtension(Name(file));
   if(!tables.ContainsKey(tableName)){Check(file.Bytes.Span.SequenceEqual(original[file.ID]),"Unrelated table changed");continue;}
   var before=parse(tableName,original[file.ID]);var after=parse(tableName,file.Bytes);
   Check(after.Rows.Count==before.Rows.Count+added[tableName].Count,"Unexpected parameter row count");
   foreach(var row in before.Rows){
    var current=after.Rows.Single(candidate=>candidate.ID==row.ID);
    if(row.ID!=Ready){Check(CageAI.Same(row,current),"Unrelated parameter row changed");continue;}
    var restored=new PARAM.Row(current);
    var allowed=tableName=="SpEffectParam"?new[]{"vfxId1"}:new[]{"effectType","midstDmyId","midstSfxId"}.Concat(Enumerable.Range(0,8).Select(index=>$"enchantStartDmyId_{index}")).ToArray();
    foreach(string field in allowed)Set(restored,field,row[field].Value);
    Check(CageAI.Same(row,restored),"Unrelated readiness field changed");
   }
  }
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),ready=Ready,visuals=new[]{Ready},nativeCharmSfx=4090,privateCharmSfx=HeartReadyFog.Sfx,leaseSeconds=2,rightHandDummy=1,attachBone="R_Hand",modelSha256=Hash(modelPath),onlyReadinessVisualChanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: readiness visual bound to actor R_Hand dummy 1; weapon/left bindings removed; unrelated parameters unchanged");
 }
 public static void Probe(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var effects=parse("SpEffectParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param").Bytes);
  var visuals=parse("SpEffectVfxParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectVfxParam.param").Bytes);
  var bullets=parse("Bullet",native.Files.Single(file=>Path.GetFileName(file.Name)=="Bullet.param").Bytes);
  object Dump(PARAM.Row row)=>new{row.ID,fields=row.Cells.ToDictionary(cell=>cell.Def.InternalName,cell=>cell.Value)};
  string directory=Path.Combine(root,"work/heart-ready");Directory.CreateDirectory(directory);
  File.WriteAllText(Path.Combine(directory,"native-probe.json"),JsonSerializer.Serialize(new{effects=effects.Rows.Where(row=>row.ID is 503350 or 708680 or 8630300 or 8630301 or 8630302||row.ID>=3000&&row.ID<3200).Select(Dump),visuals=visuals.Rows.Select(Dump),bullets=bullets.Rows.Where(row=>row.ID is 10335000 or 10335001 or 12335000 or 12335001).Select(Dump)}));
  Console.WriteLine("Native charm, weapon coatings and visual fields exported; no installed files changed");
 }
}
