using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class OnyxBehavior {
 public static void Stage(string root,string source,string output,string work){
  Directory.CreateDirectory(work);Directory.CreateDirectory(Path.GetDirectoryName(output)!);
  var b=BND4.Read(source);var original=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var graph=b.Files.Single(f=>f.ID==7000500&&f.Name.EndsWith("c0000.hkx"));
  var input=Path.Combine(work,"before.hkx");var patched=Path.Combine(work,"after.hkx");
  File.WriteAllBytes(input,graph.Bytes.ToArray());
  var start=new ProcessStartInfo("python"){UseShellExecute=false,CreateNoWindow=true};
  foreach(var arg in new[]{Path.Combine(root,"tools/build_onyx_behavior.py"),input,patched})start.ArgumentList.Add(arg);
  using(var process=Process.Start(start)!){process.WaitForExit();if(process.ExitCode!=0)throw new InvalidDataException("Onyx behavior graph patch failed");}
  graph.Bytes=File.ReadAllBytes(patched);b.Write(output);var actual=BND4.Read(output);
  if(actual.Files.Count!=b.Files.Count||actual.Version!=b.Version||actual.Compression!=b.Compression)throw new InvalidDataException("Behavior binder metadata changed");
  foreach(var (old,now) in b.Files.Zip(actual.Files)){
   if(old.ID!=now.ID||old.Name!=now.Name||old.Flags!=now.Flags||!old.Bytes.Span.SequenceEqual(now.Bytes.Span))throw new InvalidDataException("Behavior binder roundtrip failed");
   if(now.ID!=graph.ID&&!now.Bytes.Span.SequenceEqual(original[now.ID]))throw new InvalidDataException("Unrelated behavior member changed");
  }
  string motionSource=Path.Combine(Path.GetDirectoryName(source)!,"c0000_a9x.anibnd.dcx");
  if(!File.Exists(motionSource))motionSource=Path.Combine(root,"assets/original/chr/c0000_a9x.anibnd.dcx");
  string motionOutput=Path.Combine(Path.GetDirectoryName(output)!,"c0000_a9x.anibnd.dcx");
  var motionStart=new ProcessStartInfo("dotnet"){UseShellExecute=false,CreateNoWindow=true};
  foreach(var arg in new[]{"run","--project",Path.Combine(root,"tools/OnyxMotion"),"--",root,motionSource,motionOutput})motionStart.ArgumentList.Add(arg);
  using(var process=Process.Start(motionStart)!){process.WaitForExit();if(process.ExitCode!=0)throw new InvalidDataException("Onyx planted motion build failed");}
  // Keep build reports outside the deployed mod resource tree.
  File.Move(motionOutput+".validation.json",Path.Combine(work,"motion-validation.json"),true);
  File.WriteAllText(Path.Combine(work,"validation.json"),JsonSerializer.Serialize(new{file="chr/c0000.behbnd.dcx",sha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(output))).ToLowerInvariant(),sourceSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(source))).ToLowerInvariant(),onlyBehaviorGraphMemberChanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
 }
}
