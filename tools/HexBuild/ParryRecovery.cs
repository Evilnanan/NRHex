using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class ParryRecovery {
 public const int Counter=95002404,Pulse=95002406,Ready=95002407,Source=3269,Prayer=1631,Visual=8630;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void ConfigureCounter(PARAM.Row r){Set(r,"stateInfo",2103);Set(r,"atkOccurrenceSpEffectId",Pulse);}
 static void ConfigurePulse(PARAM.Row r){Set(r,"soul",0);Set(r,"changeHpRate",-35f);Set(r,"changeStaminaRate",-65f);Set(r,"vfxId",Visual);Set(r,"effectTargetAttacker",1);}
 static void ConfigureReady(PARAM.Row r){Set(r,"soul",0);Set(r,"effectEndurance",-1f);Set(r,"spCategory",20);}
 public static void Support(Func<string,int,int,PARAM.Row> clone){ConfigurePulse(clone("SpEffectParam",Source,Pulse));ConfigureReady(clone("SpEffectParam",Source,Ready));}
 public static void Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse,bool requireReady=true){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var source=Table(native,"SpEffectParam");var rows=Table(current,"SpEffectParam");
  var expected=new PARAM.Row(source.Rows.Single(r=>r.ID==322000)){ID=Counter};
  foreach(string f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(expected,f,2f);
  Set(expected,"spCategory",20);ConfigureCounter(expected);
  Check(CageAI.Same(expected,rows.Rows.Single(r=>r.ID==Counter)),"Counter must retain native category-103 filter and x2 damage");
  var pulse=new PARAM.Row(source.Rows.Single(r=>r.ID==Source)){ID=Pulse};ConfigurePulse(pulse);
  Check(CageAI.Same(pulse,rows.Rows.Single(r=>r.ID==Pulse)),"Unexpected recovery pulse fields");
  if(requireReady){
   var ready=new PARAM.Row(source.Rows.Single(r=>r.ID==Source)){ID=Ready};ConfigureReady(ready);
   Check(CageAI.Same(ready,rows.Rows.Single(r=>r.ID==Ready)),"Unexpected single-use recovery opportunity fields");
   Check(Convert.ToInt32(ready["saveCategory"].Value)==-1&&Convert.ToInt32(ready["stateInfo"].Value)==0&&Convert.ToInt32(ready["vfxId"].Value)==-1,"Opportunity must be transient and invisible");
  }
  Check(Convert.ToSingle(pulse["effectEndurance"].Value)==0&&Convert.ToInt32(pulse["saveCategory"].Value)==-1&&Convert.ToInt32(pulse["stateInfo"].Value)==0,"Recovery must be immediate, transient and not spawn Prayerful Strike bullets");
  foreach(string f in new[]{"changeHpPoint","changeMpRate","changeMpPoint","changeStaminaPoint"})Check(Convert.ToSingle(pulse[f].Value)==0,"Unrequested recovery: "+f);
  Check(Convert.ToInt32(source.Rows.Single(r=>r.ID==Prayer)["vfxId"].Value)==Visual,"Prayerful Strike visual moved");
  var vfx=Table(native,"SpEffectVfxParam").Rows.Single(r=>r.ID==Visual);
  Check(Convert.ToInt32(vfx["initSfxId"].Value)==525033&&Convert.ToInt32(vfx["initDmyId"].Value)==220,"Prayerful Strike visual changed");
  Check(CageAI.Same(vfx,Table(current,"SpEffectVfxParam").Rows.Single(r=>r.ID==Visual)),"Shared Prayerful Strike VFX changed");
  foreach(int id in new[]{Source,Prayer,1630,1632,1635,1636,322000,8600200})Check(CageAI.Same(source.Rows.Single(r=>r.ID==id),rows.Rows.Single(r=>r.ID==id)),"Original effect changed: "+id);
 }
 static void Fixture(string dir,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(dir);var p=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  p.Rows=p.Rows.Where(r=>new[]{Counter,Pulse,Ready,Prayer,Source}.Contains(r.ID)).ToList();File.WriteAllBytes(Path.Combine(dir,"installed-effects.param"),p.Write());
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));Validate(native,current,parse);Fixture(Path.Combine(root,"work/parry-recovery"),current,parse);
  Console.WriteLine("PASS: installed Clash counter x2, native on-hit 35% HP / 65% stamina pulse and Prayerful Strike visual");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/parry-recovery/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var old=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="SpEffectParam.param");var before=parse("SpEffectParam",file.Bytes);var rows=parse("SpEffectParam",file.Bytes);
  Validate(native,b,parse,requireReady:false);Check(!rows.Rows.Any(r=>r.ID==Ready),"Recovery opportunity ID collision");
  var nativeRows=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);
  var ready=new PARAM.Row(nativeRows.Rows.Single(r=>r.ID==Source)){ID=Ready};ConfigureReady(ready);rows.Rows.Add(ready);rows.Rows=rows.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=rows.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var pair in b.Files.Zip(final.Files)){
   Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Regulation readback mismatch");
   if(pair.Second.ID!=file.ID)Check(pair.Second.Bytes.Span.SequenceEqual(old[pair.Second.ID]),"Unrelated table changed");
  }
  var actual=parse("SpEffectParam",final.Files.Single(f=>f.ID==file.ID).Bytes);Check(actual.Rows.Count==before.Rows.Count+1,"Unexpected row count");
  foreach(var previous in before.Rows){var expected=new PARAM.Row(previous);Check(CageAI.Same(expected,actual.Rows.Single(r=>r.ID==expected.ID)),"Unrelated effect changed");}
  Validate(native,final,parse);Fixture(dir,final,parse);
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Parry.Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),counter=Counter,pulse=Pulse,hpPercent=35,staminaPercent=65,visual=Visual,sfx=525033,ready=Ready,onlyRecoveryOpportunityAdded=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Clash single-use recovery opportunity staged; original rows and Prayerful Strike resources preserved");
 }
}
