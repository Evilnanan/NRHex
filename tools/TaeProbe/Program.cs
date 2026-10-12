using System.Reflection;
using System.Runtime.InteropServices;
using SoulsFormats;
var smith=@"D:\Program Files\Smithbox";
var game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>Assembly.LoadFrom(Path.Combine(smith,new AssemblyName(a.Name).Name+".dll"));
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
var source=@"D:\Projects\NRMod\NRHex\assets\original\chr\c0000.anibnd.dcx";
if(Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(source)))!="610F553520E5127DA7366FE6C619683ACAC11B7AEE6F6B2450B8186AF0B573E1")throw new InvalidDataException("Unexpected source animation bundle");
var b=BND4.Read(source);
var tae=TAE.Read(b.Files.Single(f=>f.ID==5000000).Bytes);
var records=tae.Animations.Where(a=>a.ID>=202400&&a.ID<=202430).Select(a=>new {a.ID,MiniHeader=a.MiniHeader.ToString(),Events=a.Events.Select(e=>new {e.Type,e.StartTime,e.EndTime,Hex=Convert.ToHexString(e.GetParameterBytes(false))})});
var text=System.Text.Json.JsonSerializer.Serialize(records,new System.Text.Json.JsonSerializerOptions{WriteIndented=true});
File.WriteAllText(@"D:\Projects\NRMod\NRHex\.work\actions\wall-tae.json",text);
foreach(var id in new long[]{202410,202411,202420,202430}){
 var a=tae.Animations.Single(a=>a.ID==id);
 var mask=a.Events.Single(e=>e.Type==320);
 var marker=a.Events.Single(e=>e.Type==66&&BitConverter.ToInt32(e.GetParameterBytes(false))==5031);
 if(mask.StartTime!=0f||mask.EndTime!=0.9f||mask.GetParameterBytes(false)[3]!=0||Math.Abs(marker.StartTime-11f/30f)>0.00001f||marker.EndTime!=100f)throw new InvalidDataException("Unexpected wall input/takeoff event");
}
Console.WriteLine("PASS: original animation bundle hash and four wall clips' 0.9s jump masks and 11/30s takeoff markers verified; no animation overrides generated.");
