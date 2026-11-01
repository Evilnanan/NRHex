using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class DrinkingBalance {
 public const int Owner=95000500;
 public const float AttackRate=1.08f;
 static readonly int[] Layers=Enumerable.Range(1,79).Select(slot=>95005000+slot).Prepend(95000501).ToArray();
 static readonly string[] AttackFields={"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string inputMod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),input=Path.Combine(inputMod,"regulation.bin");
  string directory=Path.Combine(root,".work/drinking8",mmv?"mmv":"standalone");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");var original=parse("SpEffectParam",member.Bytes);var effects=parse("SpEffectParam",member.Bytes);
  var changes=new List<object>();
  foreach(int id in Layers)foreach(string field in AttackFields){
   var row=effects.Rows.Single(r=>r.ID==id);float old=Convert.ToSingle(row[field].Value),value=AttackRate;
   Check(old==value||old==1.05f,"Unexpected Drinking balance input");
   Set(row,field,value);changes.Add(new{id,field,before=old,after=value});
  }
  member.Bytes=effects.Write();string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var pair in binder.Files.Zip(final.Files)){
   var a=pair.First;var b=pair.Second;Check(a.ID==b.ID&&a.Name==b.Name&&a.Flags==b.Flags&&a.Bytes.Span.SequenceEqual(b.Bytes.Span),"Regulation readback mismatch");
   if(b.ID!=member.ID)Check(b.Bytes.Span.SequenceEqual(before[b.ID]),"Unrelated parameter table changed");
  }
  var current=parse("SpEffectParam",final.Files.Single(f=>f.ID==member.ID).Bytes);Check(current.Rows.Count==original.Rows.Count,"Effect row count changed");
  foreach(var pair in original.Rows.Zip(current.Rows)){
   var expected=new PARAM.Row(pair.First);if(Layers.Contains(expected.ID))foreach(string field in AttackFields)Set(expected,field,AttackRate);
   Check(expected.ID==pair.Second.ID&&expected.Name==pair.Second.Name&&expected.Cells.Count==pair.Second.Cells.Count&&expected.Cells.Zip(pair.Second.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&Equals(p.First.Value,p.Second.Value)),"Unrelated effect row or field changed");
  }
  var first=current.Rows.Single(r=>r.ID==Layers[0]);
  foreach(int id in Layers){
   var row=current.Rows.Single(r=>r.ID==id);Check(Convert.ToSingle(row["effectEndurance"].Value)==15f,"Drinking stack duration changed");
   Check(row.Cells.Zip(first.Cells).All(p=>Equals(p.First.Value,p.Second.Value)),"Drinking layers differ");
  }
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(directory,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:inputMod);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{input,inputSha256=Hash(input),outputSha256=Hash(output),changes,attackPerStack=AttackRate,layerCount=Layers.Length,otherFieldsRowsAndTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Drinking +8% attack per stack; all 80 layers updated, 400 rate fields only; bilingual text verified");
 }
}
