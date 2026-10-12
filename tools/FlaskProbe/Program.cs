using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;
var root=@"D:\Projects\NRMod\NRHex";
var smith=@"D:\Program Files\Smithbox";
var game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>Assembly.LoadFrom(Path.Combine(smith,new AssemblyName(a.Name).Name+".dll"));
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
if(args.Contains("deployment")){
 var deployRecords=new List<object>();
 foreach(var path in Directory.EnumerateFiles(Path.Combine(root,"assets/original/event"),"*.emevd.dcx")){
  var ev=EMEVD.Read(path);
  foreach(var e in ev.Events.Where(e=>e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>BitConverter.ToInt32(i.ArgData,n*4)==99202))))deployRecords.Add(new{file=Path.GetFileName(path),e.ID,e.RestBehavior,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})});
 }
 File.WriteAllText(Path.Combine(root,"data/reference/rewards/deployment-events.json"),JsonSerializer.Serialize(deployRecords,new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("Native bird deployment events: "+deployRecords.Count);return;
}
if(args.Contains("level-rewards")){
 var rewardRecords=new List<object>();
 foreach(var name in new[]{"common","m10_00_00_00"}){
  var path=Path.Combine(Path.Combine(root,"assets/original/event"),name+".emevd.dcx");
  var e=EMEVD.Read(path);
  foreach(var ev in e.Events)foreach(var i in ev.Instructions.Where(i=>(i.Bank==3&&i.ID==8)||(i.Bank==2004&&i.ID==21)))rewardRecords.Add(new{file=name,ev.ID,i.Bank,instruction=i.ID,hex=Convert.ToHexString(i.ArgData)});
 }
 var rewardBuilt=EMEVD.Read(Path.Combine(root,"dist/mod/event/common.emevd.dcx"));
 var events=rewardBuilt.Events.Where(e=>e.ID==9100940||e.ID==9100941||e.ID==1111).Select(e=>new{e.ID,e.RestBehavior,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})});
 File.WriteAllText(Path.Combine(root,".work/rewards/level-reward-events.json"),JsonSerializer.Serialize(new{nativeExamples=rewardRecords,events},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("Read back level lifecycle events and native map/clear examples");return;
}
if(args.Contains("flows")){
 var ev=EMEVD.Read(Path.Combine(root,"assets/original/event/common.emevd.dcx"));
 foreach(var e in ev.Events.Where(e=>e.ID!=0&&((e.ID>=1140&&e.ID<=1149)||e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(j=>BitConverter.ToInt32(i.ArgData,j*4)==9040||BitConverter.ToInt32(i.ArgData,j*4)==9041))))){
  Console.WriteLine("event="+e.ID);foreach(var i in e.Instructions)Console.WriteLine(i.Bank+"["+i.ID+"] "+Convert.ToHexString(i.ArgData));
 }
 foreach(var i in ev.Events.Single(e=>e.ID==0).Instructions.Where(i=>i.Bank==2000&&i.ArgData.Length>=8&&BitConverter.ToInt32(i.ArgData,4)>=1140&&BitConverter.ToInt32(i.ArgData,4)<=1149))Console.WriteLine("init "+Convert.ToHexString(i.ArgData));
 return;
}
if(args.Contains("church")){
 foreach(var path in Directory.EnumerateFiles(Path.Combine(root,"assets/original/event"),"*.emevd.dcx").Where(p=>Path.GetFileName(p).StartsWith("m41")||Path.GetFileName(p).StartsWith("m51_00"))){
  var ev=EMEVD.Read(path);Console.WriteLine(Path.GetFileName(path));
  foreach(var e in ev.Events){Console.WriteLine("event="+e.ID);foreach(var i in e.Instructions)Console.WriteLine(i.Bank+"["+i.ID+"] "+Convert.ToHexString(i.ArgData));}
 }
 foreach(var e in EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx")).Events.Where(e=>new long[]{90015009,90015208}.Contains(e.ID))){
  Console.WriteLine("common_func event="+e.ID);foreach(var i in e.Instructions)Console.WriteLine(i.Bank+"["+i.ID+"] "+Convert.ToHexString(i.ArgData));
 }
 return;
}
if(args.Contains("callers")){
 foreach(var path in Directory.EnumerateFiles(Path.Combine(root,"assets/original/event"),"*.emevd.dcx")){
  var ev=EMEVD.Read(path);
  foreach(var e in ev.Events)for(int n=0;n<e.Instructions.Count;n++){
   var i=e.Instructions[n];
   if((i.Bank==2004&&i.ID==86)||(i.Bank==2000&&i.ArgData.Length>=8&&(BitConverter.ToInt32(i.ArgData,4)==90005919||(BitConverter.ToInt32(i.ArgData,4)==90015208&&BitConverter.ToInt32(i.ArgData,i.ArgData.Length-4)==9040))))
    Console.WriteLine(Path.GetFileName(path)+" event="+e.ID+" index="+n+" opcode="+i.Bank+"["+i.ID+"] "+Convert.ToHexString(i.ArgData));
  }
 }
 return;
}
if(args.Length>0&&args[0]=="tae"){
 var bundle=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
 var hits=new List<object>();
 foreach(var file in bundle.Files.Where(f=>f.Name.EndsWith(".tae"))){
  var tae=TAE.Read(file.Bytes);
  foreach(var a in tae.Animations){
   if(!a.Events.Any(e=>e.Type==66&&new[]{7010299,501000}.Contains(BitConverter.ToInt32(e.GetParameterBytes(false)))))continue;
   hits.Add(new{file=file.Name,a.ID,events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})});
  }
 }
 File.WriteAllText(Path.Combine(root,".work/actions/flask-tae.json"),JsonSerializer.Serialize(hits,new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Flask TAE clips: "+hits.Count);return;
}
var records=new List<object>();
bool built=args.Contains("built");
foreach(var name in new[]{"common","common_func","m41_00_00_00","m41_01_00_00","m41_30_00_00","m51_00_00_00"}){
 var path=Path.Combine(root,built?"dist/mod/event":"assets/original/event",name+".emevd.dcx");
 if(built&&name=="common_func"&&!File.Exists(path))path=Path.Combine(root,"assets/original/event",name+".emevd.dcx");
 var original=Path.Combine(Path.Combine(root,"assets/original/event"),name+".emevd.dcx");
 if(!built&&File.Exists(original)&&!SHA256.HashData(File.ReadAllBytes(path)).SequenceEqual(SHA256.HashData(File.ReadAllBytes(original))))throw new Exception("Prior source mismatch");
 var ev=EMEVD.Read(path);
 foreach(var e in ev.Events.Where(e=>name.StartsWith("m")||e.Instructions.Any(i=>i.Bank==2004&&new[]{86,87,95}.Contains(i.ID))||(e.ID>=9100910&&e.ID<=9100930)||e.ID==90015208||e.ID==9100901)){
  records.Add(new{file=name,sha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),e.ID,e.RestBehavior,
   instructions=e.Instructions.Select((i,n)=>new{index=n,i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData),values=Enumerable.Range(0,i.ArgData.Length/4).Select(o=>BitConverter.ToInt32(i.ArgData,o*4))}),e.Parameters});
  Console.WriteLine(name+" "+e.ID+" "+e.Instructions.Count+" "+string.Join(",",e.Instructions.Select((i,n)=>(i,n)).Where(p=>p.i.Bank==2004&&new[]{86,87,95}.Contains(p.i.ID)).Select(p=>p.n+":"+p.i.ID)));
 }
}
File.WriteAllText(Path.Combine(root,built?".work/rewards/flask-built-events.json":"data/reference/rewards/flask-events.json"),JsonSerializer.Serialize(records,new JsonSerializerOptions{WriteIndented=true}));
