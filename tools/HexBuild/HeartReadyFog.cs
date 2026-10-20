using SoulsFormats;
using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text.Json;

static class HeartReadyFog {
 public const int Sfx=950941;
 const string NativeHash="415A6BEC546CA4ED419AA6562E1927074D09E02A6D7567DD4B29BF68A220B517";
 const string PreviousHash="5C84B55C6317CC02F3AC9CD93A9A42B3C388C5956574C597CCDC6D7B218A2CDE";
 const string PatchedHash="1FA245071411543452058EC5108B9B9BD3BED6E65A128365D59F9BF1698ADCF4";
 static void Check(bool valid,string message){if(!valid)throw new InvalidDataException(message);}
 static string Name(BinderFile file)=>Path.GetFileName(file.Name.Replace('\\','/'));
 static string Hash(ReadOnlySpan<byte> bytes)=>Convert.ToHexString(SHA256.HashData(bytes));
 static byte[] Patch(ReadOnlySpan<byte> source){
  Check(Hash(source)==NativeHash,"Native charm FXR changed; recheck particle follow curve before patching");
  var result=source.ToArray();
  foreach(var(offset,before,after)in new(int,uint,uint)[]{(12,4090,Sfx),(20348,0,0x3f800000),(20368,0,0x3f800000),(20372,0x3fc90fda,0)}){
   Check(BinaryPrimitives.ReadUInt32LittleEndian(result.AsSpan(offset,4))==before,"Charm FXR follow-curve layout changed");
   BinaryPrimitives.WriteUInt32LittleEndian(result.AsSpan(offset,4),after);
  }
  foreach(var(offset,before)in new(int,float)[]{(19448,.1f),(19452,.1f),(19456,.1f),(19476,.140625f),(19488,.2109375f),(19500,.140625f)}){
   Check(BinaryPrimitives.ReadSingleLittleEndian(result.AsSpan(offset,4))==before,"Charm FXR spatial scale layout changed");
   BinaryPrimitives.WriteSingleLittleEndian(result.AsSpan(offset,4),before*.5f);
  }
  Check(Hash(result)==PatchedHash,"Unexpected private charm FXR output");
  return result;
 }
 public static void StageResources(string root,string smith,string game){
  string originalPath=Path.Combine(root,".work/storm-vfx-01757/extracted/sfx/sfxbnd_commoneffects.ffxbnd.dcx");
  if(!File.Exists(originalPath))StormVfx.Extract(root,smith,game);
  string installedBase=Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects.ffxbnd.dcx");
  var native=BND4.Read(File.Exists(installedBase)?installedBase:originalPath);
  var source=native.Files.Single(file=>Name(file)=="f000004090.fxr");
  var resources=native.Files.Single(file=>Name(file)=="f000004090.ffxreslist");
  Check(System.Text.Encoding.UTF8.GetString(resources.Bytes.Span).Replace("\r","").Trim()=="s21021_a.tif\ns34020_a.tif","Charm texture dependencies changed");
  foreach(string texture in new[]{"s21021_a.tpf","s34020_a.tpf"})Check(native.Files.Any(file=>Name(file)==texture),"Missing shared charm texture");
  var effect=Patch(source.Bytes.Span);
  string input=Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");
  var binder=BND4.Read(input);var baseline=binder.Files.Select(file=>new{file.ID,file.Name,file.Flags,Bytes=file.Bytes.ToArray()}).ToArray();var added=new List<string>();var updated=new List<int>();
  void Add(BinderFile template,int group,byte[] bytes){
   string name=template.Name.Replace("f000004090",$"f{Sfx:000000000}");
   var existing=binder.Files.SingleOrDefault(file=>Name(file)==Path.GetFileName(name.Replace('\\','/')));
   if(existing!=null){
    if(existing.Bytes.Span.SequenceEqual(bytes))return;
    Check(Name(existing)==$"f{Sfx:000000000}.fxr"&&Hash(existing.Bytes.Span)==PreviousHash,"Conflicting private charm FXR resource");
    existing.Bytes=bytes;updated.Add(existing.ID);return;
   }
   int identity=binder.Files.Where(file=>file.ID>=group&&file.ID<group+100000).Select(file=>file.ID).DefaultIfEmpty(group-1).Max()+1;
   Check(identity<group+100000,"FXR binder ID range exhausted");
   binder.Files.Add(new BinderFile(template.Flags,identity,name,bytes));added.Add(name);
  }
  Add(source,0,effect);Add(resources,400000,resources.Bytes.ToArray());
  binder.Files.Sort((left,right)=>left.ID.CompareTo(right.ID));
  string directory=Path.Combine(root,"work/heart-ready/fog-staged");Directory.CreateDirectory(directory);
  string output=Path.Combine(directory,Path.GetFileName(input));binder.Write(output);
  var final=BND4.Read(output);
  Check(final.Version==binder.Version&&final.Format==binder.Format&&final.Compression==binder.Compression&&final.BigEndian==binder.BigEndian&&final.Unicode==binder.Unicode&&final.Files.Count==binder.Files.Count,"FXR binder metadata changed");
  foreach(var expected in binder.Files){
   var actual=final.Files.Single(file=>file.ID==expected.ID);
   Check(actual.Name==expected.Name&&actual.Flags==expected.Flags&&actual.Bytes.Span.SequenceEqual(expected.Bytes.Span),"FXR member readback mismatch");
  }
  foreach(var previous in baseline){
   var actual=final.Files.Single(file=>file.ID==previous.ID);
   Check(actual.Name==previous.Name&&actual.Flags==previous.Flags,"Existing resource metadata changed");
   if(!updated.Contains(previous.ID))Check(actual.Bytes.Span.SequenceEqual(previous.Bytes),"Unrelated FXR resource changed");
  }
  File.WriteAllBytes(Path.Combine(directory,"f000004090.fxr"),source.Bytes.ToArray());
  File.WriteAllBytes(Path.Combine(directory,$"f{Sfx:000000000}.fxr"),effect);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(File.ReadAllBytes(input)),outputSha256=Hash(File.ReadAllBytes(output)),nativeFxrSha256=NativeHash,privateFxrSha256=PatchedHash,sfx=Sfx,spatialScale=.5,particleFollowFactor=1,followRotation=false,added,updated,unrelatedResourcesPreserved=true,nativeCharmUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: half-size private pink mist follows hand translation; native charm and unrelated resources preserved");
 }
}
