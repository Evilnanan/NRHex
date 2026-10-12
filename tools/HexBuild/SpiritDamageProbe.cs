using SoulsFormats;
using System.Text.Json;

// Read the installed parameter package; diagnostics never change game inputs.
static class SpiritDamageProbe {
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,".work/chant-spirit-01746");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="NpcParam.param");
  var npc=parse("NpcParam",file.Bytes);var before=parse("NpcParam",file.Bytes);
  void Check(bool ok,string message){if(!ok)throw new InvalidDataException(message);}
  string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
  int[] families=[137030000,143400000,149610000];
  Check(families.All(id=>Convert.ToInt32(npc.Rows.Single(r=>r.ID==id)["isCalculatePvPDamage"].Value)==1),"Native family damage classification changed");
  var changed=new List<int>();
  var spirits=npc.Rows.Where(r=>r.ID>=95400000&&r.ID<95500000).ToArray();
  Check(spirits.Length>3000,"Private NPC catalog incomplete");
  foreach(var row in spirits){
   var cell=row["isCalculatePvPDamage"];if(Convert.ToInt32(cell.Value)==1)continue;
   cell.Value=Convert.ChangeType(1,cell.Value.GetType());changed.Add(row.ID);
  }
  file.Bytes=npc.Write();string path=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var output=SFUtil.DecryptNightreignRegulation(path);Check(output.Files.Count==binder.Files.Count&&output.Version==binder.Version&&output.Compression==binder.Compression,"Binder metadata changed");
  foreach(var member in output.Files){var expected=binder.Files.Single(f=>f.ID==member.ID);Check(member.Name==expected.Name&&member.Flags==expected.Flags&&member.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder readback differs");}
  var final=parse("NpcParam",output.Files.Single(f=>f.ID==file.ID).Bytes).Rows.ToDictionary(r=>r.ID);
  Check(final.Count==before.Rows.Count,"NPC row count changed");
  foreach(var row in before.Rows)foreach(var cell in row.Cells.Where(c=>c!=null)){
   var actual=final[row.ID][cell.Def.InternalName].Value;
   if(changed.Contains(row.ID)&&cell.Def.InternalName=="isCalculatePvPDamage")Check(Convert.ToInt32(actual)==1,"Classification was not written");
   else Check(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(actual),"Unrelated NPC field changed");
  }
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:[95002000]);
  File.WriteAllText(Path.Combine(dir,"parameter-validation.json"),JsonSerializer.Serialize(new{version="0.17.46",inputSha256=Hash(input),outputSha256=Hash(path),nativeFamilyIds=families,privateNpcRows=spirits.Length,changedNpcIds=changed,field="isCalculatePvPDamage",value=1,originalNpcRowsUnchanged=true,otherTablesUnchanged=true,spiritDamageMultiplier=.25,chantStaminaPerSecond=30,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {changed.Count} private NPC damage classifications aligned with native families; all original NPCs and other fields preserved");
 }
 public static void Run(string root,Func<string,Memory<byte>,PARAM> parse){
  string source=Path.Combine(root,"dist/mod/regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(source);
  var output=new Dictionary<string,object>();
  foreach(string name in new[]{"NpcParam","AtkParam_Npc","SpEffectParam"}){
   var table=parse(name,binder.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
   bool Wanted(string f)=>name=="NpcParam" ? f.Contains("spEffect")||f.Contains("SpEffect")||f.Contains("atk",StringComparison.OrdinalIgnoreCase)||f.Contains("def",StringComparison.OrdinalIgnoreCase)||f.Contains("hp",StringComparison.OrdinalIgnoreCase)||f.Contains("Correct")||f.Contains("Necromancy") : name=="AtkParam_Npc" ? f.Contains("atk",StringComparison.OrdinalIgnoreCase)||f.Contains("correct",StringComparison.OrdinalIgnoreCase)||f.Contains("spEffect")||f.Contains("damage",StringComparison.OrdinalIgnoreCase) : f.Contains("Attack",StringComparison.OrdinalIgnoreCase)||f.Contains("Damage",StringComparison.OrdinalIgnoreCase)||f.Contains("Correct")||f.Contains("replace")||f.Contains("cycle")||f.Contains("changeHp")||f.Contains("Target")||f.Contains("ParamChange");
   output[name]=table.Rows.Select(r=>new{r.ID,r.Name,fields=r.Cells.Where(c=>c!=null&&Wanted(c.Def.InternalName)).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();
  }
  string dir=Path.Combine(root,".work/chant-spirit-01746");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"damage-parameters.json"),JsonSerializer.Serialize(new{inputSha256=Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(source))),tables=output}));
  Console.WriteLine("PASS: installed NPC, attack and effect damage fields exported read-only");
 }
}
