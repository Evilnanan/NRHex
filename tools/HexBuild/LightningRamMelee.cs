using SoulsFormats;
using System.Text.Json;
static partial class LightningRam {
 static readonly string[] MeleeNames=["atkType","subCategory1","subCategory2","subCategory3","subCategory4","subCategory5","isDisableBothHandsAtkBonus"];
 static void MeleeFields(PARAM.Row row){
  foreach(var(name,value)in new(string,int)[]{("atkType",1),("subCategory1",112),("subCategory2",0),("subCategory3",0),("subCategory4",128),("subCategory5",130),("isDisableBothHandsAtkBonus",1)})Set(row,name,value);
 }
 static bool MeleeSame(PARAM.Row a,PARAM.Row b)=>a.ID==b.ID&&a.Cells.Count==b.Cells.Count&&a.Cells.Zip(b.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&(p.First.Value is float x&&p.Second.Value is float y?BitConverter.SingleToInt32Bits(x)==BitConverter.SingleToInt32Bits(y):System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(p.First.Value,p.Second.Value)));
 public static void MeleeStage(string root,Func<string,Memory<byte>,PARAM> parse,string inputMod,string output){
  Directory.CreateDirectory(output);string input=Path.Combine(inputMod,"regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Name(f)=="AtkParam_Pc.param");var table=parse("AtkParam_Pc",file.Bytes);
  var reference=table.Rows.Single(r=>r.ID==300000711);var expected=new PARAM.Row(reference);MeleeFields(expected);
  Check(MeleeSame(reference,expected),"Native Lightning Ram melee/skill categories changed");
  var changed=new[]{Owner+60,Owner+61,Owner+62};
  foreach(int id in changed)MeleeFields(table.Rows.Single(r=>r.ID==id));
  file.Bytes=table.Write();string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(path);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in reopened.Files){var wanted=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback mismatch");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(originals[f.ID]),"Unrelated table changed");}
  var before=parse("AtkParam_Pc",originals[file.ID]);var after=parse("AtkParam_Pc",reopened.Files.Single(f=>f.ID==file.ID).Bytes);Check(before.Rows.Count==after.Rows.Count,"Row count changed");
  foreach(var old in before.Rows){var actual=after.Rows.Single(r=>r.ID==old.ID);if(!changed.Contains(old.ID)){Check(MeleeSame(old,actual),"Unrelated attack changed");continue;}var wanted=new PARAM.Row(old);MeleeFields(wanted);Check(MeleeSame(wanted,actual),"Unexpected attack field changed");}
  File.WriteAllBytes(Path.Combine(output,"installed-attacks.param"),reopened.Files.Single(f=>f.ID==file.ID).Bytes.ToArray());
  File.WriteAllBytes(Path.Combine(output,"installed-effects.param"),reopened.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes.ToArray());
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(path),changed,fields=MeleeNames,nativeReference=300000711,unrelatedRowsAndTablesUnchanged=true,otherAttackFieldsUnchanged=true,attacks=after.Rows.Where(r=>changed.Contains(r.ID)||r.ID==300000711).Select(r=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Ram melee/skill categories match native; all damage/poise/rescue/impact fields and unrelated data preserved");
 }
}
