using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Xml.Linq;
using System.Security.Cryptography;
using SoulsFormats;
var root=@"D:\Projects\NRMod\NRHex";var smith=@"D:\Program Files\Smithbox";var game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
Directory.CreateDirectory(Path.Combine(root,".work/skill-projectiles"));
var bnd=SFUtil.DecryptNightreignRegulation(Path.Combine(game,"regulation.bin"));
if(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Path.Combine(game,"regulation.bin"))))!="876A3CA279A4561D0C69F81FE5E510C75C59AB9A8A201694F8CF4A42C91E0268"||bnd.Version!="10350000")throw new InvalidDataException("Unexpected original regulation");
PARAM Parse(string n){var p=PARAM.Read(bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);var x=XDocument.Load(Path.Combine(smith,$"Assets/PARAM/NR/Defs/{n}.xml"));foreach(var f in x.Root!.Element("Fields")!.Elements("Field").ToArray()){ulong a=(ulong?)f.Attribute("FirstVersion")??0,z=(ulong?)f.Attribute("RemovedVersion")??ulong.MaxValue;if(a>ulong.Parse(bnd.Version)||z<=ulong.Parse(bnd.Version))f.Remove();else{f.Attribute("FirstVersion")?.Remove();f.Attribute("RemovedVersion")?.Remove();}}var path=Path.Combine(root,$".work/skill-projectiles/{n}.xml");x.Save(path);p.ApplyParamdef(PARAMDEF.XmlDeserialize(path));return p;}
object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
var anim=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));var records=new List<object>();var files=new List<string>();
foreach(var f in anim.Files.Where(f=>f.Name.EndsWith(".tae"))){
 files.Add(Path.GetFileName(f.Name));
 if(!new[]{"a617.tae","a700.tae","a701.tae","a702.tae","a703.tae","a705.tae","a708.tae","a769.tae","a801.tae","a802.tae"}.Contains(Path.GetFileName(f.Name)))continue;
 var tae=TAE.Read(f.Bytes);
 foreach(var a in tae.Animations)records.Add(new{file=Path.GetFileName(f.Name),a.ID,events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});
}
var bullets=Parse("Bullet");
File.WriteAllText(Path.Combine(root,".work/skill-projectiles/data.json"),JsonSerializer.Serialize(new{animations=records,taeFiles=files,bullets=bullets.Rows.Select(Dump)}));
Console.WriteLine($"Read {records.Count} bow/fire-shield/torch animation clips and {bullets.Rows.Count} original bullets.");
