using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;

static class MmvArts {
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 public static void Export(string root,string overlay,Func<string,Memory<byte>,PARAM> parse){
  overlay=Path.GetFullPath(overlay);
  var paths=new[]{"mod/regulation.bin","mod/chr/c0000.anibnd.dcx","mod/msg/zhocn/item_dlc01.msgbnd.dcx"};
  string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Path.Combine(overlay,p))));
  var hashes=paths.ToDictionary(p=>p,Hash);var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(overlay,paths[0]));
  var tables=new Dictionary<string,object>();
  foreach(var name in new[]{"EquipParamWeapon","SwordArtsParam","BehaviorParam_PC"}){
   tables[name]=parse(name,binder.Files.Single(f=>Name(f)==name+".param").Bytes).Rows.Select(r=>new{r.ID,fields=r.Cells.Where(c=>c.Value is not byte[]).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();
  }
  var animations=new Dictionary<int,object>();
  foreach(var f in BND4.Read(Path.Combine(overlay,paths[1])).Files){
   var match=Regex.Match(Name(f),@"a(\d+)\.tae$");if(!match.Success)continue;
   int category=int.Parse(match.Groups[1].Value);if(category<600||category>=1000)continue;
   var tae=TAE.Read(f.Bytes);
   animations[category]=tae.Animations.Select(a=>new{a.ID,events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,data=Convert.ToHexString(e.GetParameterBytes(false))})}).ToArray();
  }
  var names=BND4.Read(Path.Combine(overlay,paths[2])).Files.Where(f=>Name(f).StartsWith("ArtsName")).SelectMany(f=>FMG.Read(f.Bytes).Entries).Where(e=>!string.IsNullOrWhiteSpace(e.Text)).GroupBy(e=>e.ID).ToDictionary(g=>g.Key,g=>g.First().Text);
  if(hashes.Any(p=>Hash(p.Key)!=p.Value))throw new InvalidDataException("Arts target changed during export");
  var output=Path.Combine(root,"work/compat/mmv/arts-parameters.json");Directory.CreateDirectory(Path.GetDirectoryName(output)!);
  File.WriteAllText(output,JsonSerializer.Serialize(new{overlay,hashes,tables,animations,names}));
  Console.WriteLine($"Exported MMV arts, {animations.Count} animation categories: {output}");
 }
}
