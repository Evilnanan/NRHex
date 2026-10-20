using SoulsFormats;
using System.Text.Json;

static partial class ContractHeart {
 public const int Contract=95002600, Heart=95002700;
 public const int SelfDamage=Contract+1, Receipt=Heart+1, Used=Heart+2, Cooldown=Heart+3, Return=Heart+4, Second=Heart+5;
 public const int InitialLot=95127000, ReturnLot=95127010;
 static readonly int[] BranchBullets={10335000,13335000,13335010};
 static readonly int[] EventIds={9100970,9100971,9100972};
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v){r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());}
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  // Native on-hit effects select the attacker. The DLL consumes this marker
  // and executes one native HP cost plus the separate hit-effect notification.
  var pulse=clone("SpEffectParam",8600105,SelfDamage);
  Set(pulse,"changeHpPoint",0);Set(pulse,"changeHpRate",0f);Set(pulse,"bCurrHPIndependeMaxHP",0);Set(pulse,"isDisableNetSync",1);
  HeartMist.Support(clone);
 }
 public static void Owners(PARAM.Row contract){Set(contract,"stateInfo",2103);Set(contract,"atkOccurrenceSpEffectId",SelfDamage);foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(contract,f,0);}
 public static void Branches(PARAM bullets){
  foreach(int id in BranchBullets){var r=bullets.Rows.Single(r=>r.ID==id);Check(Convert.ToInt32(r["spEffectIDForShooter"].Value)==0,"Native branch shooter effect changed");}
 }
 public static void Lots(PARAM original,Func<string,int,int,PARAM.Row> clone){
  var template=original.Rows.First(r=>Convert.ToInt32(r["lotItemCategory01"].Value)==1);
  foreach(int id in new[]{InitialLot,ReturnLot}){
   var r=clone("ItemLotParam_map",template.ID,id);
   for(int i=1;i<=8;i++){Set(r,$"lotItemId{i:00}",-1);Set(r,$"lotItemCategory{i:00}",0);Set(r,$"lotItemBasePoint{i:00}",0);Set(r,$"lotItemNum{i:00}",0);}
   Set(r,"lotItemId01",3350);Set(r,"lotItemCategory01",1);Set(r,"lotItemBasePoint01",1000);Set(r,"lotItemNum01",id==InitialLot?2:1);Set(r,"getItemFlagId",0);Set(r,"cumulateNumFlagId",0);Set(r,"cumulateNumMax",0);
  }
 }
 public static void Events(EMEVD ev)=>HeartMist.Events(ev);
 public static void ExportChecks(string root,Func<string,Memory<byte>,PARAM> parse,string? regulation=null,string? eventFile=null){
  var dir=Path.Combine(root,".work/contract-heart/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(regulation??Path.Combine(root,"dist/mod/regulation.bin"));
  var tables=new[]{"SpEffectParam","ItemLotParam_map","Bullet"}.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  foreach(int id in BranchBullets)Check(Convert.ToInt32(tables["Bullet"].Rows.Single(r=>r.ID==id)["spEffectIDForShooter"].Value)==0,"Retired branch notification remains");
  File.WriteAllText(Path.Combine(dir,"mechanics.json"),JsonSerializer.Serialize(new{effects=tables["SpEffectParam"].Rows.Where(r=>r.ID>=Contract&&r.ID<=Second).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),lots=tables["ItemLotParam_map"].Rows.Where(r=>r.ID is InitialLot or ReturnLot).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})},new JsonSerializerOptions{WriteIndented=true}));
  var ev=EMEVD.Read(eventFile??Path.Combine(root,"dist/mod/event/common.emevd.dcx"));
  File.WriteAllText(Path.Combine(dir,"events.json"),JsonSerializer.Serialize(ev.Events.Where(e=>EventIds.Contains((int)e.ID)).Select(e=>new{e.ID,e.RestBehavior,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})}),new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native branch notifications restored; contract/retired-heart fixtures exported");
 }
 public static void StageDamageMarker(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/contract-damage/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var f=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var sp=parse("SpEffectParam",f.Bytes);var old=parse("SpEffectParam",f.Bytes);
  var pulse=sp.Rows.Single(r=>r.ID==SelfDamage);Check(Convert.ToInt32(pulse["effectTargetAttacker"].Value)==1&&Convert.ToInt32(pulse["changeHpPoint"].Value)==0,"Contract pulse route changed");
  Set(pulse,"changeHpRate",0f);Set(pulse,"isDisableNetSync",1);f.Bytes=sp.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var result=SFUtil.DecryptNightreignRegulation(output);
  Check(result.Version==binder.Version&&result.Files.Count==binder.Files.Count&&result.Compression==binder.Compression,"Binder metadata changed");
  foreach(var member in result.Files){var intended=binder.Files.Single(v=>v.ID==member.ID);Check(member.Name==intended.Name&&member.Flags==intended.Flags&&member.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(member.ID!=f.ID)Check(member.Bytes.Span.SequenceEqual(before[member.ID]),"Unrelated table changed");}
  var actual=parse("SpEffectParam",result.Files.Single(v=>v.ID==f.ID).Bytes);Check(old.Rows.Count==actual.Rows.Count,"Unexpected effect row count");
  foreach(var p in old.Rows.Zip(actual.Rows)){var r=new PARAM.Row(p.Second);if(r.ID==SelfDamage)foreach(string field in new[]{"changeHpRate","isDisableNetSync"})Set(r,field,p.First[field].Value);Check(CageAI.Same(p.First,r),"Unrelated effect field changed");}
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedEffect=SelfDamage,changedFields=new[]{"changeHpRate","isDisableNetSync"},nativeDamageCriteria=12,otherFieldsRowsAndTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  ExportChecks(root,parse,output);
  Console.WriteLine("PASS: Contract request is now a neutral local marker; delayed HP deduction removed; all other fields/rows/tables preserved");
 }
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var tables=new[]{"SpEffectParam","EquipParamGoods","Bullet","AtkParam_Pc","ItemLotParam_enemy","ItemLotParam_map"}.ToDictionary(n=>n,n=>parse(n,original.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var report=new Dictionary<string,object>();
  int[] effects={6820400,6820401,6820500,6820501,8000000,1753,1760,1763,1765,1604004,8600100,8600105,8600200,8600201,8600202,8600205,8600206,8600207,7122000,7122001,7170100,7170101,8610700,8610701,8610702,511086};
  report["SpEffectParam"]=tables["SpEffectParam"].Rows.Where(r=>effects.Contains(r.ID)).Select(Dump).ToArray();
  report["EquipParamGoods"]=tables["EquipParamGoods"].Rows.Where(r=>r.ID==3350).Select(Dump).ToArray();
  report["Bullet"]=tables["Bullet"].Rows.Where(r=>r.ID is 10335000 or 10335001).Select(Dump).ToArray();
  report["AtkParam_Pc"]=tables["AtkParam_Pc"].Rows.Where(r=>r.ID is 7 or 11101701).Select(Dump).ToArray();
  report["ItemLotParam_map"]=tables["ItemLotParam_map"].Rows.Take(12).Select(Dump).ToArray();
  report["BranchLots"]=tables["ItemLotParam_enemy"].Rows.Where(r=>Enumerable.Range(1,8).Any(i=>Convert.ToInt32(r[$"lotItemId{i:00}"].Value)==3350)).Select(Dump).ToArray();
  var dir=Path.Combine(root,".work/contract-heart");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"native-parameters.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Read-only native contract/self-damage and Bewitching Branch parameters exported");
 }
}
