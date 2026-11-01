using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class BalloonTargetDamage {
 public const int Attack=95001760,Effect=95001702,Source=3269;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Configure(PARAM.Row attack,PARAM.Row effect){
  // An ordinary, immediate HP pulse delivered by the native attack effect list.
  // State 0 avoids ailment buildup/guard correction; positive HP rate is damage.
  Set(attack,"spEffectId0",Effect);Set(effect,"soul",0);
  Set(effect,"changeHpRate",7f);Set(effect,"effectTargetOpposeTarget",1);
 }
 static void Equal(PARAM.Row a,PARAM.Row b)=>Check(a.ID==b.ID&&a.Name==b.Name&&a.Cells.Count==b.Cells.Count&&a.Cells.Zip(b.Cells).All(p=>p.First.Def.InternalName==p.Second.Def.InternalName&&System.Collections.StructuralComparisons.StructuralEqualityComparer.Equals(p.First.Value,p.Second.Value)),"Unexpected row/field change");
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string input=Path.Combine(root,mmv?"dist/mmv/mod/regulation.bin":"dist/mod/regulation.bin");
  string dir=Path.Combine(root,".work/balloon7-independent",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  var bnd=SFUtil.DecryptNightreignRegulation(input);var before=bnd.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AtkParam_Pc"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,bnd.Files.Single(f=>Name(f)==n+".param").Bytes));
  var effects=tables["SpEffectParam"];var attacks=tables["AtkParam_Pc"];
  var source=effects.Rows.Single(r=>r.ID==Source);
  Check(Convert.ToSingle(source["effectEndurance"].Value)==0&&Convert.ToInt32(source["stateInfo"].Value)==0&&Convert.ToInt32(source["bCurrHPIndependeMaxHP"].Value)==0&&Convert.ToInt32(source["changeHpPoint"].Value)==0,"Immediate HP template changed");
  Check(!effects.Rows.Any(r=>r.ID==Effect),"Balloon damage effect already exists");
  var attack=attacks.Rows.Single(r=>r.ID==Attack);Check(Convert.ToInt32(attack["spEffectId0"].Value)==-1,"Balloon effect slot occupied");
  var effect=new PARAM.Row(source){ID=Effect,Name=""};int index=effects.Rows.FindIndex(r=>r.ID>Effect);effects.Rows.Insert(index<0?effects.Rows.Count:index,effect);Configure(attack,effect);
  foreach(string n in names)bnd.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,bnd,bnd.Compression);
  var read=SFUtil.DecryptNightreignRegulation(output);Check(read.Version==bnd.Version&&read.Compression==bnd.Compression&&read.Files.Count==bnd.Files.Count,"Binder metadata changed");
  foreach(var pair in bnd.Files.Zip(read.Files)){
   var a=pair.First;var b=pair.Second;Check(a.ID==b.ID&&a.Name==b.Name&&a.Flags==b.Flags&&a.Bytes.Span.SequenceEqual(b.Bytes.Span),"Binder readback mismatch");
   string n=Path.GetFileNameWithoutExtension(Name(b));
   if(!names.Contains(n)){Check(b.Bytes.Span.SequenceEqual(before[b.ID]),"Unrelated table changed");continue;}
   var old=parse(n,before[b.ID]);var current=parse(n,b.Bytes);
   Check(current.Rows.Count==old.Rows.Count+(n=="SpEffectParam"?1:0),"Unexpected row count");
   var retained=current.Rows.Where(r=>n!="SpEffectParam"||r.ID!=Effect).ToArray();
   foreach(var pairRow in old.Rows.Zip(retained)){var expected=new PARAM.Row(pairRow.First);if(n=="AtkParam_Pc"&&expected.ID==Attack)Set(expected,"spEffectId0",Effect);Equal(expected,pairRow.Second);}
   if(n=="SpEffectParam"){Equal(effect,current.Rows.Single(r=>r.ID==Effect));File.WriteAllBytes(Path.Combine(dir,"effects.param"),b.Bytes.ToArray());}
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),attack=Attack,effect=Effect,source=Source,percentMaxHp=7,independentImmediateHpDamage=true,changedAttackFields=new[]{"spEffectId0"},templateOverrides=new[]{"soul","changeHpRate","effectTargetOpposeTarget"},otherRowsFieldsTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Balloon independent 7% HP pulse; only private attack effect slot and one new effect row changed");
 }
}
