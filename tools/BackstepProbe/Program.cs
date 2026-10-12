using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Xml.Linq;
using SoulsFormats;
var root=@"D:\Projects\NRMod\NRHex";var smith=@"D:\Program Files\Smithbox";var game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
if(args.Contains("api")){foreach(var m in typeof(SFUtil).GetMethods().Where(m=>m.Name.Contains("Regulation")))Console.WriteLine(m);return;}
if(args.Contains("--tae-api")){foreach(var t in new[]{typeof(TAE),typeof(TAE.Event),typeof(TAE.Animation),typeof(TAE.EventGroup)}){Console.WriteLine(t);foreach(var m in t.GetConstructors())Console.WriteLine(m);foreach(var m in t.GetProperties())Console.WriteLine(m);foreach(var m in t.GetFields())Console.WriteLine(m);foreach(var m in t.GetMethods().Where(m=>m.Name.Contains("Parameter")||m.Name=="Write"))Console.WriteLine(m);}return;}
PARAM Parse(string g,string n,Memory<byte> bytes,ulong ver){var p=PARAM.Read(bytes);var x=XDocument.Load(Path.Combine(smith,$"Assets/PARAM/{g}/Defs/{(g=="ER"&&n=="SpEffectParam"?"SpEffect":n)}.xml"));foreach(var f in x.Root!.Element("Fields")!.Elements("Field").ToArray()){ulong first=(ulong?)f.Attribute("FirstVersion")??0,removed=(ulong?)f.Attribute("RemovedVersion")??ulong.MaxValue;if(first>ver||removed<=ver)f.Remove();else{f.Attribute("FirstVersion")?.Remove();f.Attribute("RemovedVersion")?.Remove();}}var path=Path.Combine(root,$".work/backstep/{g}-{n}-def.xml");x.Save(path);p.ApplyParamdef(PARAMDEF.XmlDeserialize(path));return p;}
object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
var nr=SFUtil.DecryptNightreignRegulation(Path.Combine(game,"regulation.bin"));
var sp=Parse("NR","SpEffectParam",nr.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes,ulong.Parse(nr.Version));
var ids=new[]{340700,340701,20380100,20382405,703550,703560,703565,7030800,7030801,7030802,350500,350501,350502,8640000,8640001,8640002,1460,1470,100002};
if(args.Contains("--group-evidence")){
 foreach(var path in new[]{"assets/original/chr/c0000.anibnd.dcx","dist/mod/chr/c0000.anibnd.dcx"}){
  var b=BND4.Read(Path.Combine(root,path));var t=TAE.Read(b.Files.Single(f=>Path.GetFileName(f.Name)=="a00.tae").Bytes);
  foreach(var a in t.Animations.Where(a=>new long[]{27000,27060,27100}.Contains(a.ID)))Console.WriteLine(JsonSerializer.Serialize(new{path,a.ID,groups=a.EventGroups.Select(g=>new{g.GroupType,g.GroupData}),events=a.Events.Where(e=>e.Type==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==8).Select(e=>new{e.Type,e.StartTime,e.EndTime,Group=e.Group==null?null:JsonSerializer.Serialize(e.Group,new JsonSerializerOptions{IncludeFields=true}),hex=Convert.ToHexString(e.GetParameterBytes(false))})},new JsonSerializerOptions{IncludeFields=true}));
 }return;
}
if(args.Contains("--backstep-tae")){
 var bundle=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));var records=new List<object>();
 foreach(var file in bundle.Files.Where(f=>f.Name.EndsWith(".tae"))){var tae=TAE.Read(file.Bytes);foreach(var a in tae.Animations.Where(a=>a.ID>=27000&&a.ID<27300))records.Add(new{file=Path.GetFileName(file.Name),a.ID,header=a.MiniHeader.ToString(),events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});}
 Directory.CreateDirectory(Path.Combine(root,".work/backstep"));File.WriteAllText(Path.Combine(root,".work/backstep/backstep-tae.json"),JsonSerializer.Serialize(records,new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("Backstep TAE exported: "+records.Count);return;
}
if(args.Contains("--attacks")){
 var attacks=Parse("NR","AtkParam",nr.Files.Single(f=>Path.GetFileName(f.Name)=="AtkParam_Pc.param").Bytes,ulong.Parse(nr.Version));
 var throws=Parse("NR","ThrowParam",nr.Files.Single(f=>Path.GetFileName(f.Name)=="ThrowParam.param").Bytes,ulong.Parse(nr.Version));
 File.WriteAllText(Path.Combine(root,"data/reference/actions/attack-throw.json"),JsonSerializer.Serialize(new{attacks=attacks.Rows.Select(Dump),throws=throws.Rows.Select(Dump)},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("Original player attacks and throws exported.");return;
}
if(args.Contains("--iframes")){
 var bundle=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));var events=new List<object>();
 foreach(var file in bundle.Files.Where(f=>f.Name.EndsWith(".tae"))){var tae=TAE.Read(file.Bytes);foreach(var a in tae.Animations){foreach(var e in a.Events.Where(e=>e.Type==0)){var bytes=e.GetParameterBytes(false);if(bytes.Length>=16&&new[]{8,67,68,94,143}.Contains(BitConverter.ToInt32(bytes)))events.Add(new{file=Path.GetFileName(file.Name),a.ID,e.StartTime,e.EndTime,flag=BitConverter.ToInt32(bytes),state=BitConverter.ToInt16(bytes,14),hex=Convert.ToHexString(bytes)});}}}
 File.WriteAllText(Path.Combine(root,".work/backstep/iframes-original.json"),JsonSerializer.Serialize(new{states=sp.Rows.Where(r=>new[]{412,473}.Contains(Convert.ToInt32(r["stateInfo"].Value))).Select(Dump),events},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("I-frame events exported: "+events.Count);return;
}
File.WriteAllText(Path.Combine(root,".work/backstep/nr-effects.json"),JsonSerializer.Serialize(sp.Rows.Where(r=>ids.Contains(r.ID)).Select(Dump),new JsonSerializerOptions{WriteIndented=true}));
if(args.Contains("--tae")){
 var source=Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx");var bundle=BND4.Read(source);var records=new List<object>();var wordHits=new List<object>();
 foreach(var file in bundle.Files.Where(f=>f.Name.EndsWith(".tae"))){var tae=TAE.Read(file.Bytes);
  foreach(var a in tae.Animations){
   var hit=a.Events.Where(e=>Enumerable.Range(0,e.GetParameterBytes(false).Length/4).Any(i=>new[]{473,2011,7030801,350501,8640001}.Contains(BitConverter.ToInt32(e.GetParameterBytes(false),i*4)))).ToArray();
   if(hit.Length>0)wordHits.Add(new{file=file.Name,a.ID,events=hit.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});
   if(a.ID>=30000&&a.ID<31000||a.ID>=60000&&a.ID<60500||a.ID>=20000&&a.ID<23000)records.Add(new{file=file.Name,a.ID,header=a.MiniHeader.ToString(),events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});
  }
 }
 File.WriteAllText(Path.Combine(root,"data/reference/actions/critical-tae.json"),JsonSerializer.Serialize(new{records,wordHits},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine($"TAE: {records.Count} dodge/throw clips, {wordHits.Count} state references");return;
}
var erPath=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING\Game\regulation.bin";
var er=SFUtil.DecryptERRegulation(erPath);var erSp=Parse("ER","SpEffectParam",er.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes,ulong.Parse(er.Version));
File.WriteAllText(Path.Combine(root,".work/backstep/er-effects.json"),JsonSerializer.Serialize(new{version=er.Version,rows=erSp.Rows.Where(r=>new[]{20380100,20382405,340700,340701}.Contains(r.ID)).Select(Dump)},new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine("Original NR/ER effect references exported.");
