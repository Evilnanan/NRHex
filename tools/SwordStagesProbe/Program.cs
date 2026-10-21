using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;
using SoulsFormats;

var root=Path.GetFullPath(args[0]);
var drive=Path.GetPathRoot(root)!;
var smith=Environment.GetEnvironmentVariable("NRHEX_SMITHBOX")??Path.Combine(drive,"Program Files/Smithbox");
var game=Environment.GetEnvironmentVariable("NRHEX_GAME")??Path.Combine(drive,"Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game");
AppDomain.CurrentDomain.AssemblyResolve+=(_,assembly)=>Assembly.LoadFrom(Path.Combine(smith,new AssemblyName(assembly.Name).Name+".dll"));
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(name,_,_)=>name.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,name.EndsWith(".dll")?name:name+".dll")):IntPtr.Zero);
var work=Path.Combine(root,"work/sword-projectile-range");Directory.CreateDirectory(work);
var xml=XDocument.Load(Path.Combine(smith,"Assets/PARAM/NR/Defs/Bullet.xml"));
foreach(var field in xml.Root!.Element("Fields")!.Elements("Field").ToArray()){
 ulong first=(ulong?)field.Attribute("FirstVersion")??0,removed=(ulong?)field.Attribute("RemovedVersion")??ulong.MaxValue;
 if(first>10350000||removed<=10350000)field.Remove();else{field.Attribute("FirstVersion")?.Remove();field.Attribute("RemovedVersion")?.Remove();}
}
var definition=Path.Combine(work,"Bullet.xml");xml.Save(definition);
var pairs=Regex.Matches(File.ReadAllText(Path.Combine(root,"src/sword_projectile_stages.rs")),@"\((\d+),(\d+)\)").Select(match=>(Parent:int.Parse(match.Groups[1].Value),Child:int.Parse(match.Groups[2].Value))).ToArray();
if(pairs.Length!=109||pairs.Distinct().Count()!=109)throw new Exception("Unexpected stage catalog");
var fields=new[]{"HitBulletID","intervalCreateBulletId","intervalCreateTimeMin","intervalCreateTimeMax","numShoot","FollowType","isSyncBulletCulcDumypolyPos","atkId_Bullet","sfxId_Bullet","life","initVellocity","maxVellocity","accelInRange","hitRadius","dmgHitRecordLifeTime","shootAngle","shootAngleInterval","shooterOffsetRange"};
var reports=new List<object>();
foreach(var path in new[]{Path.Combine(game,"regulation.bin"),Path.Combine(root,"dist/mod/regulation.bin")}){
 var binder=SFUtil.DecryptNightreignRegulation(path);
 if(binder.Version!="10350000")throw new Exception("Unexpected regulation version");
 var table=PARAM.Read(binder.Files.Single(file=>Path.GetFileName(file.Name)=="Bullet.param").Bytes);table.ApplyParamdef(PARAMDEF.XmlDeserialize(definition));
 var rows=table.Rows.ToDictionary(row=>row.ID);
 double Value(PARAM.Row row,string field)=>Convert.ToDouble(row.Cells.Single(cell=>cell.Def.InternalName==field).Value);
 foreach(var pair in pairs){
  var parent=rows[pair.Parent];var child=rows[pair.Child];
  if(pair.Parent is >=10682000 and <=10683025){
   if(Value(parent,"HitBulletID")!=pair.Child||Value(parent,"intervalCreateBulletId")!=-1||Value(child,"numShoot")!=1||Value(parent,"atkId_Bullet")!=Value(child,"atkId_Bullet")||Value(parent,"FollowType")!=0||Value(child,"FollowType")!=0||Value(parent,"isSyncBulletCulcDumypolyPos")!=0||Value(child,"isSyncBulletCulcDumypolyPos")!=0)throw new Exception($"Claw continuation mismatch: {pair}");
   continue;
  }
  if(pair.Parent>=10691000){
   if(Value(parent,"HitBulletID")!=pair.Child||Value(parent,"atkId_Bullet")!=0||Value(child,"atkId_Bullet")!=0||Value(parent,"sfxId_Bullet")!=-1||Value(child,"sfxId_Bullet")!=-1||Value(parent,"FollowType")!=0||Value(child,"FollowType")!=0)throw new Exception($"Lightning placement continuation mismatch: {pair}");
   continue;
  }
  if(pair.Parent is >=10500000 and <=10500052){
   if(Value(parent,"HitBulletID")!=pair.Child||Value(parent,"life")!=2||Value(parent,"initVellocity")!=2||Value(parent,"intervalCreateBulletId")!=-1||Value(child,"HitBulletID")!=-1||Value(child,"numShoot")!=1||Value(parent,"atkId_Bullet")!=Value(child,"atkId_Bullet")||Value(parent,"sfxId_Bullet")!=Value(child,"sfxId_Bullet")||Value(parent,"FollowType")!=0||Value(child,"FollowType")!=0)throw new Exception($"Rancor continuation mismatch: {pair}");
   continue;
  }
  if(pair.Parent is >=10407000 and <=10407006){
   bool collider=pair.Child==pair.Parent+1;
   if(Value(parent,collider?"intervalCreateBulletId":"HitBulletID")!=pair.Child||Value(parent,"atkId_Bullet")!=40070||Value(child,"atkId_Bullet")!=40070||Value(child,"numShoot")!=(collider?2:1)||Value(child,"sfxId_Bullet")!=-1||Value(parent,"initVellocity")!=15||Value(parent,"FollowType")!=0||Value(child,"FollowType")!=0||Value(parent,"dmgHitRecordLifeTime")!=99||Value(child,"dmgHitRecordLifeTime")!=99||Value(parent,"life")>0.66||Value(child,"life")>0.66)throw new Exception($"Arc evidence mismatch: {pair}");
   continue;
  }
  if(Value(parent,"HitBulletID")!=pair.Child||Value(parent,"intervalCreateBulletId")!=-1||Value(child,"numShoot")!=1||Value(parent,"atkId_Bullet")!=Value(child,"atkId_Bullet")||Value(parent,"sfxId_Bullet")!=Value(child,"sfxId_Bullet")||Value(parent,"life")<=0||Value(parent,"life")>0.41||Value(parent,"initVellocity")<=0||Value(child,"initVellocity")<=0||Value(parent,"FollowType")!=0||Value(child,"FollowType")!=0)throw new Exception($"Stage evidence mismatch: {pair}");
 }
 foreach(var id in new[]{10406000,10406001}){
  var row=rows[id];
  if(Value(row,"FollowType")!=0||Value(row,"isSyncBulletCulcDumypolyPos")!=1||Value(row,"initVellocity")!=30||Value(row,"maxVellocity")!=30||Value(row,"numShoot")!=1||Value(row,"HitBulletID")!=-1||Value(row,"intervalCreateBulletId")!=-1)throw new Exception($"Crystal Barrage flight evidence mismatch: {id}");
 }
 foreach(var id in new[]{10603000,10603050}){
  var row=rows[id];
  if(Value(row,"FollowType")!=0||Value(row,"isSyncBulletCulcDumypolyPos")!=1||Value(row,"initVellocity")!=(id==10603000?5:8)||Value(row,"numShoot")!=1||Value(row,"HitBulletID")!=-1||Value(row,"intervalCreateBulletId")!=-1)throw new Exception($"Whirl O Flame flight mismatch: {id}");
 }
 foreach(var first in new[]{10700000,10702000,10703000,10704000}){
  foreach(var offset in new[]{0,2,100,101}){
   var row=rows[first+offset];
   if(Value(row,"FollowType")!=(offset==0?4:0)||Value(row,"isSyncBulletCulcDumypolyPos")!=1||Value(row,"initVellocity")!=(offset==0?12:offset==2?6:22)||Value(row,"intervalCreateBulletId")!=-1)throw new Exception($"Dragon breath flight mismatch: {first+offset}");
  }
 }
 foreach(var id in new[]{10450000,10450001,10450002,10450050,10450051,10450052}){
  var row=rows[id];var charged=id>=10450050;
  if(Value(row,"HitBulletID")!=(charged?10450053:10450003)||Value(row,"initVellocity")!=(charged?33:25)||Value(row,"FollowType")!=0||Value(row,"isSyncBulletCulcDumypolyPos")!=0||Value(row,"intervalCreateBulletId")!=-1||Value(row,"numShoot")!=3)throw new Exception($"Shattering Crystal carrier mismatch: {id}");
 }
 foreach(var first in new[]{10670000,10670100}){
  if(Value(rows[first],"FollowType")!=0||Value(rows[first],"HitBulletID")!=first+1||Value(rows[first+1],"initVellocity")!=0||Value(rows[first+1],"life")!=1||Value(rows[first+1],"HitBulletID")!=first+2||Value(rows[first+2],"HitBulletID")!=first+3)throw new Exception($"Discus return chain mismatch: {first}");
 }
 if(Value(rows[10670100],"isInheritSfxToChild")!=1||Value(rows[10670101],"sfxId_Bullet")!=528114||Value(rows[10670101],"life")!=1)throw new Exception("Triple Rings visual handoff mismatch");
 foreach(var id in new[]{10705000,10705004}){
  var row=rows[id];
  if(Value(row,"FollowType")!=0||Value(row,"isSyncBulletCulcDumypolyPos")!=(id==10705000?1:0)||Value(row,"initVellocity")!=300||Value(row,"HitBulletID")!=10705001||Value(row,"atkId_Bullet")!=70500)throw new Exception($"Placidusax beam flight mismatch: {id}");
 }
 foreach(var id in new[]{10691010,10691060,10692009,10692109,10693001}){
  var row=rows[id];
  if(Value(row,"FollowType")!=0||Value(row,"isSyncBulletCulcDumypolyPos")!=0||Value(row,"atkId_Bullet")!=(id==10691060?1:0)||Value(row,"HitBulletID")!=id+1||Value(row,"sfxId_Bullet")!=-1||(id==10693001?Value(row,"shootAngleXZ")!=-89:Value(row,"gravityInRange")<=0))throw new Exception($"Lightning ground probe mismatch: {id}");
 }
 if(Value(rows[10693000],"initVellocity")!=200||Math.Abs(Value(rows[10693000],"life")-0.1)>0.0001||Value(rows[10693000],"HitBulletID")!=10693001||Value(rows[10693001],"HitBulletID")!=10693002)throw new Exception("Honed Bolt placement chain mismatch");
 var evidence=pairs.SelectMany(pair=>new[]{pair.Parent,pair.Child}).Concat(new[]{10406000,10406001}).Concat(Enumerable.Range(10407000,8)).Concat(rows.Keys.Where(id=>id is >=10450000 and <=10450062 or >=10500100 and <=10500162 or >=10421000 and <=10421080 or >=10670000 and <=10670105 or >=10682000 and <=10683099 or >=10603000 and <=10603099 or >=10691000 and <=10693099)).Distinct().Order().ToDictionary(id=>id,id=>rows[id].Cells.Where(cell=>id is >=10682000 and <=10693099 or >=10603000 and <=10603099||fields.Contains(cell.Def.InternalName)).ToDictionary(cell=>cell.Def.InternalName,cell=>cell.Value));
 foreach(var id in rows.Keys.Where(id=>id is >=10700000 and <=10705099 or >=10670000 and <=10670105))evidence[id]=rows[id].Cells.ToDictionary(cell=>cell.Def.InternalName,cell=>cell.Value);
 reports.Add(new{path,sha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),pairs=pairs.Length,rows=evidence});
}
File.WriteAllText(Path.Combine(work,"stage-parameters.json"),JsonSerializer.Serialize(new{reports,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine($"PASS: {pairs.Length} stage edges and special spell chains verified in original and installed regulation; gameplay untested.");
