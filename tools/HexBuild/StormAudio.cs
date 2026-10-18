using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class StormAudio {
 public static void Probe(string root,string smith,string game){
  var directory=Path.Combine(root,".work/storm-audio/native");Directory.CreateDirectory(directory);
  var assembly=Assembly.LoadFrom(Path.Combine(smith,"Andre.Formats.dll"));
  NativeLibrary.SetDllImportResolver(assembly,(name,_,_)=>name.Contains("bhd5_decrypt_rust")?NativeLibrary.Load(Path.Combine(smith,"bhd5_decrypt_rust.dll")):IntPtr.Zero);
  var keys=(Dictionary<string,string>)assembly.GetType("Andre.Formats.Util.ArchiveKeys")!.GetField("NightreignKeys",BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static)!.GetValue(null)!;
  var key=keys.Single(entry=>Path.GetFileNameWithoutExtension(entry.Key).Equals("sd",StringComparison.OrdinalIgnoreCase)).Value;
  var decrypt=assembly.GetType("Andre.Formats.Util.NativeRsa")!.GetMethods(BindingFlags.Public|BindingFlags.Static).Single(method=>method.Name=="Decrypt"&&method.GetParameters().Length==3);
  var source=File.ReadAllBytes(Path.Combine(game,"sd/sd.bhd"));
  var header=(byte[])decrypt.Invoke(null,new object[]{new Memory<byte>(source),key,2})!;
  var files=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(bucket=>bucket).ToDictionary(entry=>entry.FileNameHash);
  using var archive=File.OpenRead(Path.Combine(game,"sd/sd.bdt"));
  var banks=new Dictionary<string,string>();
  foreach(var name in new[]{"cs_m19.bnk","cs_main.bnk","cs_smain.bnk"}){
   ulong hash=0;foreach(var character in ("/"+name).ToLowerInvariant())hash=unchecked(hash*0x85+character);
   var data=files[hash].ReadFile(archive);File.WriteAllBytes(Path.Combine(directory,name),data);
   banks[name]=Convert.ToHexString(SHA256.HashData(data));
  }
  object[] Sounds(string path){
   var binder=BND4.Read(path);var tae=TAE.Read(binder.Files.Single(file=>file.Name.EndsWith("a860.tae")).Bytes);
   return tae.Animations.SelectMany(animation=>animation.Events.Where(entry=>entry.Type is >=128 and <=138).Select(entry=>(object)new{animation=animation.ID,entry.Type,entry.StartTime,entry.EndTime,hex=Convert.ToHexString(entry.GetParameterBytes(false))})).ToArray();
  }
  var original=Sounds(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
  var installed=Sounds(Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx"));
  bool unchanged=JsonSerializer.Serialize(original)==JsonSerializer.Serialize(installed);
  if(!unchanged)throw new InvalidDataException("Installed Storm Ruler sound events differ from native actions");
  File.WriteAllText(Path.Combine(directory,"inputs.json"),JsonSerializer.Serialize(new{sourceBhdSha256=Convert.ToHexString(SHA256.HashData(source)),banks,sounds=original,installedSoundsUnchanged=unchanged},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: selected original sound banks extracted; installed Storm Ruler sound events match native actions");
 }
}
