using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Read-only evidence for content coverage, separate from resource merge checks.
static class MmvCoverage {
 public static void Export(string root,string mmv,string originalPath,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
  string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
  var inputs=new[]{Path.Combine(mmv,"mod/regulation.bin"),Path.Combine(root,"dist/mmv/mod/regulation.bin")};
  var hashes=inputs.ToDictionary(p=>p,Hash);
  hashes[originalPath]=Hash(originalPath);
  var binders=new Dictionary<string,BND4>{{"original",original},{"mmv",SFUtil.DecryptNightreignRegulation(inputs[0])},{"installed",SFUtil.DecryptNightreignRegulation(inputs[1])}};
  var names=new[]{"EquipParamWeapon","SwordArtsParam","NpcParam","BuddyParam","MagicTableParam","EquipParamGoods","ShopLineupParam"};
  var tables=new Dictionary<string,object>();
  foreach(var name in names){
   tables[name]=binders.ToDictionary(pair=>pair.Key,pair=>parse(name,pair.Value.Files.Single(f=>Name(f)==name+".param").Bytes).Rows.Select(r=>new{r.ID,r.Name,fields=r.Cells.Where(c=>c.Value is not byte[]).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray());
  }
  var textPath=Path.Combine(mmv,"mod/msg/zhocn/item_dlc01.msgbnd.dcx");hashes[textPath]=Hash(textPath);
  var text=BND4.Read(textPath).Files.Where(f=>Name(f).Contains("Name")).ToDictionary(Name,f=>FMG.Read(f.Bytes).Entries.Where(e=>!string.IsNullOrWhiteSpace(e.Text)).Select(e=>new{e.ID,e.Text}).ToArray());
  if(hashes.Any(p=>Hash(p.Key)!=p.Value))throw new InvalidDataException("Audit inputs changed");
  var output=Path.Combine(root,"work/compat/mmv/content-audit/parameters.json");Directory.CreateDirectory(Path.GetDirectoryName(output)!);
  File.WriteAllText(output,JsonSerializer.Serialize(new{hashes,tables,text}));
  Console.WriteLine("Read-only MMV content coverage inputs exported: "+output);
 }
}
