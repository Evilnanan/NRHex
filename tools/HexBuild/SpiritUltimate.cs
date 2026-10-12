using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class SpiritUltimate {
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string output=Path.Combine(root,".work/spirit-ultimate-01739");Directory.CreateDirectory(output);
  var input=Path.Combine(root,"dist/mod/regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var table=parse("SpEffectParam",member.Bytes);
  var old=parse("SpEffectParam",member.Bytes);
  var row=table.Rows.Single(r=>r.ID==95001801);
  Check(Math.Abs(Convert.ToSingle(row["changeHpRate"].Value)-100f/15f)<.0001f||Convert.ToSingle(row["changeHpRate"].Value)==0,"Unexpected stored spirit drain");
  Check(Convert.ToInt32(row["changeHpPoint"].Value)==0,"Unexpected flat spirit drain");
  row["changeHpRate"].Value=0f;
  member.Bytes=table.Write();
  var target=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,binder,binder.Compression);
  var read=SFUtil.DecryptNightreignRegulation(target);
  Check(read.Files.Count==binder.Files.Count&&read.Version==binder.Version&&read.Compression==binder.Compression,"Regulation metadata changed");
  foreach(var file in read.Files){
   var intended=binder.Files.Single(f=>f.ID==file.ID);
   Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Regulation readback failed");
   if(file.ID!=member.ID)Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated parameter table changed");
  }
  var final=parse("SpEffectParam",read.Files.Single(f=>f.ID==member.ID).Bytes);
  Check(final.Rows.Count==old.Rows.Count,"Effect row count changed");
  foreach(var r in final.Rows){var prior=old.Rows.Single(v=>v.ID==r.ID);foreach(var cell in r.Cells.Where(c=>c!=null)){
   if(r.ID==95001801&&cell.Def.InternalName=="changeHpRate")Check(Convert.ToSingle(cell.Value)==0,"Drain still present");
   else Check(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(prior[cell.Def.InternalName].Value),"Unrelated effect field changed");
  }}
  HexTexts.Stage(root,hexes,Path.Combine(output,"texts"),allRewards:true,onlyIds:[95001800]);
  File.WriteAllText(Path.Combine(output,"parameter-validation.json"),JsonSerializer.Serialize(new{version="0.17.39",inputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(input))),outputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(target))),changedEffect=95001801,changedField="changeHpRate",value=0,originalPassive705200Unchanged=true,otherRowsAndTablesUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: only private spirit HP drain removed; original passive preserved; current Chinese/English spirit texts staged");
 }
}
