using SoulsFormats;
using System.Text.Json;

static class ChantRework {
 public const int Visual=95002001;
 static void Check(bool valid,string message){if(!valid)throw new InvalidDataException(message);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,PARAM nativeEffects,PARAM nativeVisuals){
  var hunt=nativeEffects.Rows.Single(row=>row.ID==8970054);
  var visual=nativeVisuals.Rows.Single(row=>row.ID==73110);
  Check(Convert.ToInt32(hunt["vfxId"].Value)==73110&&Convert.ToInt32(visual["initSfxId"].Value)==674895&&Convert.ToInt32(visual["initDmyId"].Value)==220,"Beast's Hunt recovery visual changed");
  var effect=clone("SpEffectParam",8000000,Visual);
  foreach(var(field,value)in new(string,object)[]{("maxHpRate",1f),("effectEndurance",0.3f),("saveCategory",-1),("spCategory",20),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0),("vfxId",73110)})Set(effect,field,value);
  Check(Convert.ToInt32(effect["changeStaminaPoint"].Value)==0&&Convert.ToInt32(effect["staminaRecoverChangeSpeed"].Value)==0,"Visual must not restore stamina or grant the native Hunt buff");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string directory=Path.Combine(root,"work/chant-rework/staged");Directory.CreateDirectory(directory);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(directory,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);var original=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  var member=binder.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param");
  var effects=parse("SpEffectParam",member.Bytes);
  var baseline=parse("SpEffectParam",member.Bytes);
  var nativeEffects=parse("SpEffectParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param").Bytes);
  var nativeVisuals=parse("SpEffectVfxParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectVfxParam.param").Bytes);
  var previous=effects.Rows.SingleOrDefault(row=>row.ID==Visual);
  Mechanics((table,source,identity)=>{
   Check(table=="SpEffectParam"&&identity==Visual,"Unexpected parameter edit");
   if(previous!=null)effects.Rows.Remove(previous);
   var row=new PARAM.Row(nativeEffects.Rows.Single(row=>row.ID==source)){ID=identity};effects.Rows.Add(row);return row;
  },nativeEffects,nativeVisuals);
  effects.Rows=effects.Rows.OrderBy(row=>row.ID).ToList();member.Bytes=effects.Write();
  SFUtil.EncryptNightreignRegulation(output,binder,RegulationCompression.Format);
  var read=SFUtil.DecryptNightreignRegulation(output);
  Check(read.Version==binder.Version&&read.Files.Count==binder.Files.Count&&read.Compression==RegulationCompression.Format,"Regulation metadata mismatch");
  foreach(var file in read.Files){
   var expected=binder.Files.Single(candidate=>candidate.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Parameter readback mismatch");
   if(file.ID!=member.ID)Check(file.Bytes.Span.SequenceEqual(original[file.ID]),"Unrelated parameter table changed");
  }
  var final=parse("SpEffectParam",read.Files.Single(file=>file.ID==member.ID).Bytes);
  Check(final.Rows.Count==baseline.Rows.Count+(previous==null?1:0),"Unexpected row count");
  foreach(var row in baseline.Rows.Where(row=>row.ID!=Visual))Check(CageAI.Same(row,final.Rows.Single(candidate=>candidate.ID==row.ID)),"Unrelated effect changed");
  Check(CageAI.Same(effects.Rows.Single(row=>row.ID==Visual),final.Rows.Single(row=>row.ID==Visual)),"Visual readback mismatch");
  string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
  File.WriteAllText(Path.Combine(directory,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),effect=Visual,nativeVisual=73110,initialSfx=674895,stamina=30,visualOnly=true,unrelatedParametersUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{95002000});
  Console.WriteLine("PASS: isolated Beast's Hunt recovery visual and Chant text staged; unrelated parameters preserved");
 }
 public static void Probe(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var directory=Path.Combine(root,"work/chant-rework");Directory.CreateDirectory(directory);
  var effects=parse("SpEffectParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectParam.param").Bytes);
  var visuals=parse("SpEffectVfxParam",native.Files.Single(file=>Path.GetFileName(file.Name)=="SpEffectVfxParam.param").Bytes);
  var selected=effects.Rows.Where(row=>row.ID is >=8970050 and <=8970054 or 8000000 or 100610).ToArray();
  var visualIds=selected.SelectMany(row=>new[]{"vfxId","vfxId1","vfxId2","vfxId3","vfxId4","vfxId5","vfxId6","vfxId7"}.Where(field=>row.Cells.Any(cell=>cell.Def.InternalName==field)).Select(field=>Convert.ToInt32(row[field].Value))).ToHashSet();
  object Dump(PARAM.Row row)=>new{row.ID,fields=row.Cells.ToDictionary(cell=>cell.Def.InternalName,cell=>cell.Value)};
  var animations=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
  var records=new List<object>();
  foreach(var file in animations.Files.Where(file=>file.Name.EndsWith("a402.tae")||file.Name.EndsWith("a600.tae"))){
   var tae=TAE.Read(file.Bytes);
   foreach(var animation in tae.Animations.Where(animation=>animation.ID%100000 is >=45000 and <=45031))records.Add(new{file.Name,animation.ID,header=animation.MiniHeader.ToString(),events=animation.Events.Select(entry=>new{entry.Type,entry.StartTime,entry.EndTime,hex=Convert.ToHexString(entry.GetParameterBytes(false))})});
  }
  File.WriteAllText(Path.Combine(directory,"native.json"),JsonSerializer.Serialize(new{effects=selected.Select(Dump),visuals=visuals.Rows.Where(row=>visualIds.Contains(row.ID)).Select(Dump),animations=records},new JsonSerializerOptions{WriteIndented=true}));
 }
}
