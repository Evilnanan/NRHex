using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Xml.Linq;
using System.Security.Cryptography;
using SoulsFormats;
var root=Path.GetFullPath(args[0]);var output=Path.Combine(root,".work/icons");Directory.CreateDirectory(output);
const string smith=@"D:\Program Files\Smithbox";
const string game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
var options=new JsonSerializerOptions{WriteIndented=true};
var source=Path.Combine(game,"regulation.bin");
if(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(source)))!="876A3CA279A4561D0C69F81FE5E510C75C59AB9A8A201694F8CF4A42C91E0268")throw new Exception("Unsupported regulation");
var bnd=SFUtil.DecryptNightreignRegulation(source);
if(bnd.Version!="10350000")throw new Exception("Unsupported regulation version");
var names=new[]{"EquipParamGoods","PermanentBuffParam","AttachEffectParam","SpEffectParam","EquipParamAccessory","Magic","SwordArtsParam"};
var defs=new Dictionary<string,PARAMDEF>();
PARAM Parse(string name,Memory<byte> bytes){
 if(!defs.TryGetValue(name,out var def)){
  var xml=XDocument.Load(Path.Combine(smith,"Assets/PARAM/NR/Defs",name+".xml"));
  foreach(var f in xml.Root!.Element("Fields")!.Elements("Field").ToArray()){
   ulong first=(ulong?)f.Attribute("FirstVersion")??0,removed=(ulong?)f.Attribute("RemovedVersion")??ulong.MaxValue;
   if(first>10350000||removed<=10350000)f.Remove();else{f.Attribute("FirstVersion")?.Remove();f.Attribute("RemovedVersion")?.Remove();}
  }
  var path=Path.Combine(output,name+".xml");xml.Save(path);def=PARAMDEF.XmlDeserialize(path);defs[name]=def;
 }
 var p=PARAM.Read(bytes);p.ApplyParamdef(def);return p;
}
object Dump(PARAM p)=>p.Rows.Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null&&(c.Def.InternalName.Contains("icon",StringComparison.OrdinalIgnoreCase)||c.Def.InternalName.Contains("TextId")||c.Def.InternalName.StartsWith("passiveSpEffectId")||new[]{"spEffectId","vagrantItemLotId","goodsType","saveCategory","effectEndurance","displayIconForPermamentEffect","stateInfo"}.Contains(c.Def.InternalName))).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();
var original=names.ToDictionary(n=>n,n=>Dump(Parse(n,bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes)));
File.WriteAllText(Path.Combine(output,"native-params.json"),JsonSerializer.Serialize(original,options));
var mod=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
File.WriteAllText(Path.Combine(output,"current-hexes.json"),JsonSerializer.Serialize(names.Take(4).ToDictionary(n=>n,n=>DumpSubset(Parse(n,mod.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes))),options));
object DumpSubset(PARAM p){p.Rows.RemoveAll(r=>r.ID<95000100||r.ID>95001100);return Dump(p);}
var messages=BND4.Read(Path.Combine(root,"assets/original/msg/zhocn/item_dlc01.msgbnd.dcx"));
File.WriteAllText(Path.Combine(output,"native-texts.json"),JsonSerializer.Serialize(messages.Files.Where(f=>new[]{"AttachEffectName","AttachEffectInfo","PermanentBuffName","PermanentBuffInfo","PermanentBuffCaption","GoodsName","GoodsInfo","AccessoryName","MagicName","ArtsName"}.Any(n=>Path.GetFileName(f.Name).StartsWith(n))).ToDictionary(f=>Path.GetFileName(f.Name),f=>FMG.Read(f.Bytes).Entries.Where(e=>!string.IsNullOrEmpty(e.Text)).GroupBy(e=>e.ID).ToDictionary(g=>g.Key,g=>g.First().Text)),options));
foreach(var name in new[]{"01_common_h","21_common_dlc01_h"}){
 var binder=BND4.Read(Path.Combine(output,"extracted/menu",name+".sblytbnd.dcx"));
 var layouts=Path.Combine(output,"layouts",name);Directory.CreateDirectory(layouts);
 foreach(var file in binder.Files)File.WriteAllBytes(Path.Combine(layouts,Path.GetFileName(file.Name)),file.Bytes.ToArray());
 var tpf=TPF.Read(Path.Combine(output,"extracted/menu",name+".tpf.dcx"));
 var textures=Path.Combine(output,"textures",name);Directory.CreateDirectory(textures);
 foreach(var texture in tpf.Textures)File.WriteAllBytes(Path.Combine(textures,texture.Name+".dds"),texture.Bytes.ToArray());
 Console.WriteLine($"{name}: {binder.Files.Count} layouts, {tpf.Textures.Count} textures");
}
foreach(var name in new[]{"00_solo_h","20_solo_dlc01_h"}){
 var binder=BXF4.Read(Path.Combine(output,"extracted/menu",name+".tpfbhd"),Path.Combine(output,"extracted/menu",name+".tpfbdt"));
 File.WriteAllText(Path.Combine(output,name+"-files.json"),JsonSerializer.Serialize(binder.Files.Select(f=>new{f.ID,f.Name,size=f.Bytes.Length}),options));
 Console.WriteLine($"{name}: {binder.Files.Count} separate textures");
}
Console.WriteLine("PASS: read-only icon fields, original names/texts, layouts and DDS saved.");
