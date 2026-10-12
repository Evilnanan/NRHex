using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class AbilityBuffs {
 public static void Probe(string root,Func<string,Memory<byte>,PARAM> parse){
  string path=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(root,".work/ability-buffs-01748");
  Directory.CreateDirectory(output);
  var bnd=SFUtil.DecryptNightreignRegulation(path);
  var ids=new[]{95001760,12064105,95001564,95001566,12048214,12048216};
  var attacks=parse("AtkParam_Pc",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="AtkParam_Pc.param").Bytes);
  File.WriteAllText(Path.Combine(output,"attacks.json"),JsonSerializer.Serialize(new{sourceSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),rows=attacks.Rows.Where(r=>ids.Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)})},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: read-only current Balloon/Noisy attack parameters exported");
 }
}
