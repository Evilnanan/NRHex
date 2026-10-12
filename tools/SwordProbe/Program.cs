using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Xml.Linq;
using System.Security.Cryptography;
using SoulsFormats;
var root=Path.GetFullPath(args[0]);const string smith=@"D:\Program Files\Smithbox";
const string game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>Assembly.LoadFrom(Path.Combine(smith,new AssemblyName(a.Name).Name+".dll"));
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
var source=Path.Combine(game,"regulation.bin");if(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(source)))!="876A3CA279A4561D0C69F81FE5E510C75C59AB9A8A201694F8CF4A42C91E0268")throw new Exception("Source mismatch");
var bnd=SFUtil.DecryptNightreignRegulation(source);var tables=new Dictionary<string,object>();
foreach(var name in new[]{"Magic","Bullet","AtkParam_Pc","SpEffectParam","BehaviorParam_PC","PlayerCommonParam"}){
 var defName=name.StartsWith("AtkParam")?"AtkParam":name.StartsWith("BehaviorParam")?"BehaviorParam":name;
 var xml=XDocument.Load(Path.Combine(smith,"Assets/PARAM/NR/Defs",defName+".xml"));
 foreach(var f in xml.Root!.Element("Fields")!.Elements("Field").ToArray()){
  ulong first=(ulong?)f.Attribute("FirstVersion")??0,removed=(ulong?)f.Attribute("RemovedVersion")??ulong.MaxValue;
  if(first>10350000||removed<=10350000)f.Remove();else{f.Attribute("FirstVersion")?.Remove();f.Attribute("RemovedVersion")?.Remove();}
 }
 var defPath=Path.Combine(root,".work/defs",defName+".xml");xml.Save(defPath);
 var p=PARAM.Read(bnd.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);p.ApplyParamdef(PARAMDEF.XmlDeserialize(defPath));
 IEnumerable<PARAM.Row> rows=p.Rows;
 if(name=="SpEffectParam")rows=rows.Where(r=>r.ID is >=8330900 and <=8330922 || r.ID is 1650 or 1651 or 7037001 or 706100 or 706110 or 706120);
 var data=rows.ToDictionary(r=>r.ID,r=>r.Cells.Where(c=>c!=null&&(name=="SpEffectParam"||name=="BehaviorParam_PC"||name=="PlayerCommonParam"||
  (name=="Magic"&&(c.Def.InternalName.StartsWith("ref")||c.Def.InternalName.StartsWith("subCategory")||c.Def.InternalName=="ezStateBehaviorType"))||
  (name=="Bullet"&&(c.Def.InternalName.Contains("bullet",StringComparison.OrdinalIgnoreCase)||new[]{"hitRadius","hitRadiusMax","hitRadiusTime","life","dist","initVellocity","maxVellocity","minVellocity","accelInRange","accelOutRange","shooterOffsetRange","behavId","behaviorId"}.Contains(c.Def.InternalName)))||
  (name=="AtkParam_Pc"&&(c.Def.InternalName.StartsWith("subCategory")||new[]{"atkPhys","atkMag","atkFire","atkThun","atkDark","atkSuperArmor","dmgLevel"}.Contains(c.Def.InternalName)))))
  .ToDictionary(c=>c.Def.InternalName,c=>c.Value));
 tables[name]=data;Console.WriteLine(name+": "+data.Count+" rows");
}
File.WriteAllText(Path.Combine(root,"data/reference/combat/sword-graph.json"),JsonSerializer.Serialize(tables));

