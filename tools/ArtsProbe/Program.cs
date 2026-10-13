using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Xml.Linq;
using System.Security.Cryptography;
using SoulsFormats;
var root=Path.GetFullPath(args.FirstOrDefault(a=>!a.StartsWith("--"))??Directory.GetCurrentDirectory());
var drive=Path.GetPathRoot(root)!;
var smith=Environment.GetEnvironmentVariable("NRHEX_SMITHBOX")??Path.Combine(drive,"Program Files/Smithbox");
var game=Environment.GetEnvironmentVariable("NRHEX_GAME")??Path.Combine(drive,"Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game");
Directory.CreateDirectory(Path.Combine(root,".work/arts"));
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
var bnd=SFUtil.DecryptNightreignRegulation(Path.Combine(game,"regulation.bin"));
if(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Path.Combine(game,"regulation.bin"))))!="876A3CA279A4561D0C69F81FE5E510C75C59AB9A8A201694F8CF4A42C91E0268"||bnd.Version!="10350000")throw new InvalidDataException("Unsupported original regulation");
var skillTexts=new Dictionary<string,Dictionary<int,string>>();var textSources=new List<object>();
foreach(var lang in new[]{"zhocn","engus"}){
 var input=Path.Combine(root,"assets/original/msg",lang,"item_dlc01.msgbnd.dcx");var textBinder=BND4.Read(input);var entries=new Dictionary<int,string>();
 var members=textBinder.Files.Where(f=>Path.GetFileName(f.Name).StartsWith("ArtsName")&&Path.GetFileName(f.Name).EndsWith(".fmg")).OrderBy(f=>Path.GetFileName(f.Name)=="ArtsName.fmg"?0:1).ToArray();
 if(members.Length!=2)throw new InvalidDataException("Unexpected ArtsName members");
 foreach(var member in members)foreach(var e in FMG.Read(member.Bytes).Entries)if(!string.IsNullOrWhiteSpace(e.Text))entries[e.ID]=e.Text;
 skillTexts[lang]=entries;textSources.Add(new{lang,sha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(input))),members=members.Select(f=>Path.GetFileName(f.Name)).ToArray()});
}
File.WriteAllText(Path.Combine(root,".work/arts/arts-names.json"),JsonSerializer.Serialize(new{sources=textSources,names=skillTexts},new JsonSerializerOptions{WriteIndented=true}));
if(args.Contains("--names")){Console.WriteLine("Actual Chinese/English ArtsName FMGs exported.");return;}
PARAM Parse(string n){var p=PARAM.Read(bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);var x=XDocument.Load(Path.Combine(smith,$"Assets/PARAM/NR/Defs/{(n.StartsWith("ItemLot")?"ItemLotParam":n.StartsWith("BehaviorParam_")?"BehaviorParam":n)}.xml"));foreach(var f in x.Root!.Element("Fields")!.Elements("Field").ToArray()){ulong a=(ulong?)f.Attribute("FirstVersion")??0,z=(ulong?)f.Attribute("RemovedVersion")??ulong.MaxValue;if(a>ulong.Parse(bnd.Version)||z<=ulong.Parse(bnd.Version))f.Remove();else{f.Attribute("FirstVersion")?.Remove();f.Attribute("RemovedVersion")?.Remove();}}var path=Path.Combine(root,$".work/arts/{n}.xml");x.Save(path);p.ApplyParamdef(PARAMDEF.XmlDeserialize(path));return p;}
object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
var tables=new[]{"SwordArtsParam","SwordArtsTableParam","EquipParamWeapon","EquipParamCustomWeapon","SpEffectParam","BehaviorParam_PC"}.ToDictionary(n=>n,Parse);
var report=new{version=bnd.Version,arts=tables["SwordArtsParam"].Rows.Select(Dump),artTables=tables["SwordArtsTableParam"].Rows.Select(Dump),weapons=tables["EquipParamWeapon"].Rows.Select(Dump),custom=tables["EquipParamCustomWeapon"].Rows.Select(Dump),effects=tables["SpEffectParam"].Rows.Where(r=>Convert.ToSingle(r["artsConsumptionRate"].Value)!=1).Select(Dump),behaviors=tables["BehaviorParam_PC"].Rows.Select(Dump)};
File.WriteAllText(Path.Combine(root,".work/arts/parameters.json"),JsonSerializer.Serialize(report));
Console.WriteLine("Original skills, weapon skill associations, random tables and FP effects exported.");
