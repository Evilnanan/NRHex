using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class CatWall{
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  const string table="ChrPhysicsVelocityChangeParam";
  var native=parse(table,original.Files.Single(f=>Path.GetFileName(f.Name)==table+".param").Bytes);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var installed=SFUtil.DecryptNightreignRegulation(input);
  var current=parse(table,installed.Files.Single(f=>Path.GetFileName(f.Name)==table+".param").Bytes);
  foreach(int id in new[]{500,501,510}){
   var a=native.Rows.Single(r=>r.ID==id);var b=current.Rows.Single(r=>r.ID==id);
   if(!CageAI.Same(a,b))throw new InvalidDataException("Wall-jump velocity row already changed");
   if(Convert.ToSingle(a["horizontalVelocityChange"].Value)!=(id==510?-1.3f:4.8f)||Convert.ToSingle(a["verticalVelocityChange"].Value)!=(id==510?10f:9f)||Convert.ToInt32(a["fallControlParamId"].Value)!=100)throw new InvalidDataException("Native wall-jump impulse changed");
  }
  var dir=Path.Combine(root,".work/cat-wall-slope");Directory.CreateDirectory(dir);
  string source=Path.Combine(root,"assets/original/chr/c0000.behbnd.dcx");var behavior=BND4.Read(source);var graph=behavior.Files.Single(f=>Path.GetFileName(f.Name)=="c0000.hkx"&&f.Bytes.Span.IndexOf("WallJumpStart_F_F_CMSG"u8)>=0);var hkx=graph.Bytes.ToArray();
  File.WriteAllBytes(Path.Combine(dir,"native-behavior.hkx"),hkx);
  var bundle=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));var tae=TAE.Read(bundle.Files.Single(f=>Path.GetFileName(f.Name)=="a00.tae").Bytes);
  var clips=new[]{202410,202411,202420}.Select(id=>{var a=tae.Animations.Single(a=>a.ID==id);return new{id,impulses=a.Events.Where(e=>e.Type==530).Select(e=>new{row=BitConverter.ToInt32(e.GetParameterBytes(false)),start=e.StartTime,end=e.EndTime}).ToArray()};}).ToArray();
  if(!clips.SelectMany(c=>c.impulses).Select(v=>v.row).SequenceEqual(new[]{500,501,510}))throw new InvalidDataException("Native wall animation/impulse mapping changed");
  string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
  var report=new{
   sourceRegulationSha256=Hash(Path.Combine(Environment.CurrentDirectory,"regulation.bin")),
   installedRegulationSha256=Hash(input),behaviorMember=graph.Name,behaviorBundleSha256=Hash(source),behaviorSha256=Convert.ToHexString(SHA256.HashData(hkx)),
   rows=native.Rows.Where(r=>new[]{500,501,510}.Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),
   clips,installedParameterRowsUnchanged=true
  };
  File.WriteAllText(Path.Combine(dir,"reference.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: original/installed wall-jump velocity rows and native animation impulse mapping verified; no game assets changed");
 }
}
