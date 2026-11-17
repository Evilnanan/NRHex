using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class RamSyncNames {
 static readonly string[] Names={"MARamSyncV1","MARamGeneration","MARamCycle","MARamSample","MARamRate","MARamPhase"};
 public static string Original(string root,string smith,string game){
  var assembly=Assembly.LoadFrom(Path.Combine(smith,"Andre.Formats.dll"));
  // A full build may already have registered the same Andre archive resolver.
  try{NativeLibrary.SetDllImportResolver(assembly,(n,a,p)=>n.Contains("bhd5_decrypt_rust")?NativeLibrary.Load(Path.Combine(smith,"bhd5_decrypt_rust.dll")):IntPtr.Zero);}catch(InvalidOperationException){}
  var keys=(Dictionary<string,string>)assembly.GetType("Andre.Formats.Util.ArchiveKeys")!.GetField("NightreignKeys",BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static)!.GetValue(null)!;
  var key=keys.Single(k=>Path.GetFileNameWithoutExtension(k.Key).Equals("data0",StringComparison.OrdinalIgnoreCase)).Value;
  var decrypt=assembly.GetType("Andre.Formats.Util.NativeRsa")!.GetMethods(BindingFlags.Public|BindingFlags.Static).Single(m=>m.Name=="Decrypt"&&m.GetParameters().Length==3);
  var encrypted=File.ReadAllBytes(Path.Combine(game,"data0.bhd"));
  var header=(byte[])decrypt.Invoke(null,new object[]{new Memory<byte>(encrypted),key,2})!;
  ulong hash=0;foreach(char c in "/action/variablenameid.txt")hash=unchecked(hash*0x85+c);
  var entry=BHD5.Read(header,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).Single(f=>f.FileNameHash==hash);
  using var stream=File.OpenRead(Path.Combine(game,"data0.bdt"));var data=entry.ReadFile(stream);
  var output=Path.Combine(root,"work/ram-animation-sync/original/action/variablenameid.txt");Directory.CreateDirectory(Path.GetDirectoryName(output)!);File.WriteAllBytes(output,data);return output;
 }
 public static void Stage(string source,string output){
  // Registry syntax and added names are ASCII; preserve non-ASCII comments
  // byte-for-byte (the stock file uses Shift-JIS, MMV can use another encoding).
  var raw=File.ReadAllBytes(source);var encoding=Encoding.Latin1;
  var text=encoding.GetString(raw);var body=text.TrimEnd('\0');var tail=text[body.Length..];
  var head=Regex.Match(body,@"(?m)^Num\s*=\s*(\d+)");if(!head.Success)throw new InvalidDataException("Missing native variable count");
  var rows=Regex.Matches(body,"(?m)^\\s*(\\d+)\\s*=\\s*\"([^\"]+)\"");
  var ids=rows.Select(m=>int.Parse(m.Groups[1].Value)).ToArray();var names=rows.Select(m=>m.Groups[2].Value).ToArray();
  int count=int.Parse(head.Groups[1].Value);
  if(count!=ids.Length||!ids.SequenceEqual(Enumerable.Range(1,count))||names.Distinct().Count()!=names.Length||Names.Any(names.Contains))throw new InvalidDataException("Variable registry is inconsistent or already patched");
  var ending=body.Contains("\r\n")?"\r\n":"\n";
  var numeric=head.Groups[1];var changed=body[..numeric.Index]+(count+Names.Length)+body[(numeric.Index+numeric.Length)..];
  if(!changed.EndsWith('\n'))changed+=ending;
  for(int i=0;i<Names.Length;i++)changed+=$"{count+i+1,-5}= \"{Names[i]}\"{ending}";
  changed+=tail;
  var bytes=encoding.GetBytes(changed);Directory.CreateDirectory(Path.GetDirectoryName(output)!);File.WriteAllBytes(output,bytes);
  if(!File.ReadAllBytes(output).AsSpan().SequenceEqual(bytes))throw new InvalidDataException("Variable registry readback mismatch");
  Console.WriteLine(JsonSerializer.Serialize(new{source,output,added=Names.Select((n,i)=>new{name=n,id=count+i+1}),sha256=Convert.ToHexString(SHA256.HashData(bytes)),originalEntriesPreserved=true}));
 }
}
