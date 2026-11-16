using SoulsFormats;
using System.Text.Json;

static class SpiritCharge {
 public const int Owner=95001800,Charge=95001805;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static bool Same(PARAM.Row a,PARAM.Row b)=>a.ID==b.ID&&a.Cells.Count==b.Cells.Count&&a.Cells.Zip(b.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&(p.First.Value is float x&&p.Second.Value is float y?BitConverter.SingleToInt32Bits(x)==BitConverter.SingleToInt32Bits(y):System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(p.First.Value,p.Second.Value)));
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Configure(Func<string,int,int,PARAM.Row> add){
  // Native on-kill relic 7090000 -> 7090001/2. The instant collector
  // applies ultimateArtGaugeChargeRate before the native percentage setter.
  var pulse=add("SpEffectParam",7090001,Charge);
  Check(Convert.ToSingle(pulse["effectEndurance"].Value)==0f&&Convert.ToSingle(pulse["ultimateArtGauge"].Value)==5f&&Convert.ToInt32(pulse["ultimateArtGaugeRecovery"].Value)==0,"Native on-kill charge template changed");
  Set(pulse,"ultimateArtGauge",25f);
  Set(pulse,"isDisableNetSync",1); // each beneficiary settles its own native reward
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,string inputMod,string output){
  Directory.CreateDirectory(output);
  string input=Path.Combine(inputMod,"regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);
  Check(!table.Rows.Any(r=>r.ID==Charge),"Spirit charge ID already exists");
  var relic=table.Rows.Single(r=>r.ID==7090000);
  Check(Convert.ToInt32(relic["stateInfo"].Value)==199&&Convert.ToInt32(relic["applyIdOnGetSoul"].Value)==7090001&&Convert.ToInt32(relic["applyIdOnKillSp"].Value)==7090002,"Native on-kill relic changed");
  Configure((name,source,id)=>{Check(name=="SpEffectParam","Unexpected table");var row=new PARAM.Row(table.Rows.Single(r=>r.ID==source)){ID=id};table.Rows.Add(row);return row;});
  table.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));file.Bytes=table.Write();
  string path=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(path);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in reopened.Files){var wanted=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback mismatch");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(originals[f.ID]),"Unrelated table changed");}
  var before=parse("SpEffectParam",originals[file.ID]);var after=parse("SpEffectParam",reopened.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(after.Rows.Count==before.Rows.Count+1,"Unexpected row count");
  foreach(var row in before.Rows)Check(Same(row,after.Rows.Single(r=>r.ID==row.ID)),"Existing effect changed");
  var actual=new PARAM.Row(after.Rows.Single(r=>r.ID==Charge)){ID=7090001};Set(actual,"ultimateArtGauge",5f);Set(actual,"isDisableNetSync",before.Rows.Single(r=>r.ID==7090001)["isDisableNetSync"].Value);
  Check(Same(actual,before.Rows.Single(r=>r.ID==7090001)),"Unexpected inherited effect fields");
  File.WriteAllBytes(Path.Combine(output,"installed-effects.param"),reopened.Files.Single(f=>f.ID==file.ID).Bytes.ToArray());
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(output,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:inputMod);
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(path),added=Charge,nativeSource=7090001,ultimateArtGauge=25,localOnly=true,existingRowsAndOtherTablesUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native +25% Spirit charge pulse added; all existing parameters preserved; bilingual text staged");
 }
}
