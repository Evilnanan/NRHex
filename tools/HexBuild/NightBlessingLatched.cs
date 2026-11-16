using SoulsFormats;
using System.Text.Json;

static partial class NightBlessing {

 static bool Same(PARAM.Row a,PARAM.Row b)=>a.ID==b.ID&&a.Cells.Count==b.Cells.Count&&a.Cells.Zip(b.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&(p.First.Value is float x&&p.Second.Value is float y?BitConverter.SingleToInt32Bits(x)==BitConverter.SingleToInt32Bits(y):System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(p.First.Value,p.Second.Value)));
 public static void Latched(string root,Func<string,Memory<byte>,PARAM> parse,string inputMod,string output){
  Directory.CreateDirectory(output);
  string input=Path.Combine(inputMod,"regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);
  var changed=Enumerable.Range(Owner+1,11).ToHashSet();
  Check(changed.All(id=>table.Rows.Count(r=>r.ID==id)==1),"Missing support effect");
  ValidateNative(binder,parse);
  Mechanics((name,source,id)=>{Check(name=="SpEffectParam","Unexpected table");var row=new PARAM.Row(table.Rows.Single(r=>r.ID==source)){ID=id};table.Rows.RemoveAll(r=>r.ID==id);table.Rows.Add(row);return row;});
  table.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=table.Write();
  string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(path);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in reopened.Files){var wanted=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback mismatch");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(originals[f.ID]),"Unrelated table changed");}
  var before=parse("SpEffectParam",originals[file.ID]);var after=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(after.Rows.Count==before.Rows.Count,"Unexpected row count");
  foreach(var row in before.Rows.Where(r=>!changed.Contains(r.ID)))Check(Same(row,after.Rows.Single(r=>r.ID==row.ID)),"Unrelated effect changed");
  foreach(var row in after.Rows.Where(r=>changed.Contains(r.ID))){
   Check(Convert.ToSingle(row["changeHpRate"].Value)==0&&Convert.ToInt32(row["changeHpPoint"].Value)==0&&Convert.ToInt32(row["replaceSpEffectId"].Value)==-1,"Legacy healing/backlash remains");
   foreach(string f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Check(Convert.ToSingle(row[f].Value)==1f,"Legacy attack boost remains");
  }
  File.WriteAllBytes(Path.Combine(output,"installed-effects.param"),reopened.Files.Single(f=>f.ID==file.ID).Bytes.ToArray());
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(output,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:inputMod);
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(path),changed=changed.Order(),unrelatedRowsAndTablesUnchanged=true,nativeRainUnchanged=true,legacyHealAttackBacklashNeutralized=true,effects=after.Rows.Where(r=>changed.Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: latched Night blessing support rows and bilingual texts read back; unrelated data preserved");
 }
}
