using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class HexBalance {
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string field,float value)=>r[field].Value=Convert.ChangeType(value,r[field].Value.GetType());
 public static void Stage(string root,string inputMod,string output,Func<string,Memory<byte>,PARAM> parse,bool escapeOnly=false){
  Directory.CreateDirectory(output);
  string input=Path.Combine(inputMod,"regulation.bin"),target=Path.Combine(output,"regulation.bin");
  Check(Path.GetFullPath(input)!=Path.GetFullPath(target),"Staging must preserve the input package");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");
  var old=parse("SpEffectParam",file.Bytes);var rows=parse("SpEffectParam",file.Bytes);
  var updates=new Dictionary<int,Dictionary<string,float>>{
   [Marking.Owner]=new(){["characterSkillCooldownReduction"]=Marking.CooldownRate},
   [Erosion.Owner]=Erosion.AttackRates.ToDictionary(f=>f,f=>Erosion.AttackRate),
   [UltimateAwakening.Buff]=new(){["effectEndurance"]=UltimateAwakening.Duration}
  };
  if(escapeOnly){
   updates=new(){[95001201]=Erosion.AttackRates.ToDictionary(f=>f,f=>.7f),[95001202]=new(){["effectEndurance"]=70f},[95000300]=Erosion.AttackRates.ToDictionary(f=>f,f=>GlassCannon.AttackRate)};
   foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})updates[95001201][f]=1;
   var buff=rows.Rows.Single(r=>r.ID==95001201);
   Check(Convert.ToSingle(buff["effectEndurance"].Value)==10f,"Escape duration changed");
   foreach(string f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Check(Convert.ToSingle(buff[f].Value)==.4f,"Escape damage negation changed");
  }
  var changes=new List<object>();
  foreach(var(id,fields)in updates){
   var row=rows.Rows.Single(r=>r.ID==id);
   foreach(var(field,value)in fields){
    float previous=Convert.ToSingle(row[field].Value);
    float prior=escapeOnly?(id==95000300?1.2f:id==95001202?60f:Erosion.AttackRates.Contains(field)?1f:0f):id==Marking.Owner?1f:id==Erosion.Owner?.8f:15f;
    Check(previous==value||previous==prior,$"Unexpected balance input {id}/{field}: {previous}");
    Set(row,field,value);changes.Add(new{id,field,before=previous,after=value});
   }
  }
  file.Bytes=rows.Write();SFUtil.EncryptNightreignRegulation(target,binder,binder.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(target);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var pair in binder.Files.Zip(actual.Files)){
   var a=pair.First;var b=pair.Second;
   Check(a.ID==b.ID&&a.Name==b.Name&&a.Flags==b.Flags&&a.Bytes.Span.SequenceEqual(b.Bytes.Span),"Regulation readback mismatch");
   if(b.ID!=file.ID)Check(b.Bytes.Span.SequenceEqual(before[b.ID]),"Unrelated parameter table changed");
  }
  var after=parse("SpEffectParam",actual.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(after.Rows.Count==old.Rows.Count,"Row count changed");
  foreach(var pair in old.Rows.Zip(after.Rows)){
   var expected=new PARAM.Row(pair.First);
   if(updates.TryGetValue(expected.ID,out var fields))foreach(var(field,value)in fields)Set(expected,field,value);
   // MMV contains unrelated non-finite floating-point values; compare cells
   // directly instead of serializing them as JSON.
   Check(expected.ID==pair.Second.ID&&expected.Name==pair.Second.Name&&expected.Cells.Count==pair.Second.Cells.Count&&expected.Cells.Zip(pair.Second.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&Equals(p.First.Value,p.Second.Value)),"Unrelated effect row/field changed");
  }
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(output,"texts"),allRewards:true,onlyIds:escapeOnly?new[]{95001200,95000300}:new[]{Marking.Owner,Erosion.Owner,Sniper.Owner,UltimateAwakening.Owner},inputMod:inputMod);
  if(escapeOnly)File.WriteAllText(Path.Combine(output,"effects.json"),JsonSerializer.Serialize(after.Rows.Where(r=>r.ID>=95001200&&r.ID<=95001204).Select(r=>new{id=r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)})));
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{input,inputSha256=Hash(input),outputSha256=Hash(target),changes,onlySevenFieldsChanged=!escapeOnly,escapeOnly,otherRowsAndTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine(escapeOnly?"PASS: Escape attack x0.7 / 70s cooldown and Glass Cannon attack x1.25; fourteen fields only; unrelated data preserved":"PASS: Slide Kick cooldown x1.5, Erosion attack x0.7, Awakening 10s; seven fields only, bilingual text verified");
 }
}
