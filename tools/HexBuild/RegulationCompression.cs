using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// The enlarged ZSTD regulation passes SoulsFormats readback but fails in the
// game loader (0.17.136: SpEffectVfxParam reads unrelated AtkParam_Npc bytes).
// DFLT with identical decompressed binder contents passes the same startup.
static class RegulationCompression {
 public const DCX.Type Format=DCX.Type.DCX_DFLT_10000_44_9;
 public static void Normalize(string root) {
  string source=Path.Combine(root,"dist/mod/regulation.bin");
  string work=Path.Combine(root,".work/regulation-compression");Directory.CreateDirectory(work);
  var input=File.ReadAllBytes(source);var binder=SFUtil.DecryptNightreignRegulation(input);
  var before=binder.Write(DCX.Type.None);var previous=binder.Compression;
  string staged=Path.Combine(work,"regulation.bin");
  SFUtil.EncryptNightreignRegulation(staged,binder,Format);
  var actual=SFUtil.DecryptNightreignRegulation(staged);
  if(actual.Compression!=Format||!actual.Write(DCX.Type.None).AsSpan().SequenceEqual(before))
   throw new InvalidDataException("Regulation compression changed decompressed binder contents");
  foreach(var pair in binder.Files.Zip(actual.Files))
   if(pair.First.ID!=pair.Second.ID||pair.First.Name!=pair.Second.Name||pair.First.Flags!=pair.Second.Flags||!pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span))
    throw new InvalidDataException("Regulation member changed during recompression");
  // Avoid rewriting an already compatible file (encryption uses a fresh IV).
  if(previous!=Format){File.WriteAllBytes(Path.Combine(work,"before-regulation.bin"),input);File.Copy(staged,source,true);}
  var report=new{inputCompression=previous.ToString(),outputCompression=Format.ToString(),inputSha256=Convert.ToHexString(SHA256.HashData(input)),outputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(source))),decompressedSha256=Convert.ToHexString(SHA256.HashData(before)),decompressedBytes=before.Length,parameterFiles=binder.Files.Count,allDecompressedContentsPreserved=true,gameplayVerified=false};
  Directory.CreateDirectory(Path.Combine(root,"dist/reports"));
  File.WriteAllText(Path.Combine(root,"dist/reports/regulation-compression-validation.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
  var validationPath=Path.Combine(root,"dist/reports/validation.json");
  if(File.Exists(validationPath)){
   var validation=System.Text.Json.Nodes.JsonNode.Parse(File.ReadAllText(validationPath))!;
   validation["outputRegulationSha256"]=report.outputSha256;
   File.WriteAllText(validationPath,validation.ToJsonString(new JsonSerializerOptions{WriteIndented=true}));
  }
  Console.WriteLine($"PASS: {binder.Files.Count} parameter files and {before.Length} decompressed bytes preserved; regulation compression {Format}");
 }
}
