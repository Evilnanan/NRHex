using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class EchoCast {
 public const int Owner=95003500;
 public const float SpellFpRate=2.5f;
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 public static void OwnerEffect(PARAM.Row r){Set(r,"magicConsumptionRate",SpellFpRate);Set(r,"miracleConsumptionRate",SpellFpRate);}
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool p,string why){if(!p)throw new InvalidDataException(why);}
 public static void CostResources(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes,bool stage){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/echo-cost");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Compression==RegulationCompression.Format,"Unexpected regulation compression");
  var file=binder.Files.Single(entry=>Name(entry)=="SpEffectParam.param");var before=file.Bytes.ToArray();
  var table=parse("SpEffectParam",before);var owner=table.Rows.Single(row=>row.ID==Owner);
  var fields=new[]{"magicConsumptionRate","miracleConsumptionRate"};
  Check(Convert.ToSingle(owner["artsConsumptionRate"].Value)==1f,"Echo must not change skill FP consumption");
  if(!stage){
   Check(fields.All(field=>Convert.ToSingle(owner[field].Value)==SpellFpRate),"Installed echo FP rates differ from build configuration");
   File.WriteAllText(Path.Combine(directory,"installed.json"),JsonSerializer.Serialize(new{installedSha256=Hash(input),spellFpRate=SpellFpRate,artsFpRate=1,installedReadbackPassed=true},new JsonSerializerOptions{WriteIndented=true}));
   Console.WriteLine("PASS: installed echo spell FP rates are 2.5; skill FP rate remains 1");return;
  }
  Check(fields.All(field=>Convert.ToSingle(owner[field].Value) is 2.25f or SpellFpRate),"Unexpected previous echo FP rate");
  var changes=fields.Select(field=>new{field,before=Convert.ToSingle(owner[field].Value),after=SpellFpRate}).ToArray();
  OwnerEffect(owner);file.Bytes=table.Write();
  string output=Path.Combine(directory,"staged");Directory.CreateDirectory(output);string target=Path.Combine(output,"regulation.bin");
  SFUtil.EncryptNightreignRegulation(target,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(target);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var pair in binder.Files.Zip(actual.Files))Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Parameter binder readback failed");
  var oldRows=parse("SpEffectParam",before).Rows;var newRows=parse("SpEffectParam",actual.Files.Single(entry=>Name(entry)=="SpEffectParam.param").Bytes).Rows;
  Check(oldRows.Count==newRows.Count,"Echo tuning changed row count");
  foreach(var pair in oldRows.Zip(newRows)){
   if(pair.First.ID==Owner)OwnerEffect(pair.First);
   Check(pair.First.Name==pair.Second.Name&&CageAI.Same(pair.First,pair.Second),"Echo tuning changed unrelated row or field");
  }
  HexTexts.Stage(root,hexes,Path.Combine(output,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(target),owner=Owner,spellFpRate=SpellFpRate,artsFpRate=1,changes,unrelatedRowsAndFieldsPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: echo spell FP rates staged at 2.5; all unrelated parameter fields preserved");
 }
 public static void ClawProbe(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var bullets=parse("Bullet",native.Files.Single(entry=>Name(entry)=="Bullet.param").Bytes);
  var attacks=parse("AtkParam_Pc",native.Files.Single(entry=>Name(entry)=="AtkParam_Pc.param").Bytes);
  var selected=bullets.Rows.Where(row=>row.ID is >=10682000 and <10682100 or >=10683000 and <10683100).ToArray();
  object Dump(PARAM.Row row)=>new{row.ID,fields=row.Cells.Where(cell=>cell!=null).ToDictionary(cell=>cell.Def.InternalName,cell=>cell.Value)};
  var directory=Path.Combine(root,".work/echo-cost");Directory.CreateDirectory(directory);
  File.WriteAllText(Path.Combine(directory,"native-claws.json"),JsonSerializer.Serialize(new{version=native.Version,bullets=selected.Select(Dump),attacks=attacks.Rows.Where(row=>selected.Any(bullet=>Convert.ToInt32(bullet["atkId_Bullet"].Value)==row.ID)).Select(Dump)},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native Beast Claw and Gurranq's Beast Claw bullet/attack fields exported");
 }
 public sealed record Area(int Magic,int Root,int AreaBullet,int Replay);
 public static Area[] ReadAreas(string root){
  using var d=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"data/balance/echo-cast-areas.json")));
  Check(d.RootElement.GetProperty("version").GetInt32()==10350000,"Echo area version mismatch");
  var rows=d.RootElement.GetProperty("areas").EnumerateArray().Select(x=>new Area(x.GetProperty("magic").GetInt32(),x.GetProperty("root").GetInt32(),x.GetProperty("area").GetInt32(),x.GetProperty("replay").GetInt32())).ToArray();
  Check(rows.Length==11&&rows.Select(x=>x.Magic).SequenceEqual(rows.Select(x=>x.Magic).Distinct().Order())&&rows.Select(x=>x.Replay).Distinct().Count()==rows.Length&&rows.All(x=>x.Replay is >=95003510 and <=95003599),"Invalid echo area registry");return rows;
 }
 public static void Areas(string root,Func<string,int,int,PARAM.Row> clone,Dictionary<string,PARAM> tables){
  PARAM.Row Row(string table,int id)=>tables[table].Rows.Single(r=>r.ID==id);
  int I(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
  foreach(var p in ReadAreas(root)){
   var cast=Row("Bullet",p.Root);var source=Row("Bullet",p.AreaBullet);
   Check(p.Root==p.AreaBullet||I(cast,"HitBulletID")==p.AreaBullet,"Group effect chain changed");
   Check(Convert.ToSingle(source["life"].Value)>0&&I(source,"FollowType")==0&&I(source,"isSyncBulletCulcDumypolyPos")==0&&I(source,"numShoot")==1&&I(source,"HitBulletID")==-1&&I(source,"intervalCreateBulletId")==-1,"Unsupported group area chain");
   Check(new[]{"initVellocity","minVellocity","maxVellocity","gravityInRange","gravityOutRange","accelInRange","accelOutRange","externalForce"}.All(f=>Convert.ToSingle(source[f].Value)==0),"Group area must remain stationary");
   var nativeAttack=Row("AtkParam_Pc",I(source,"atkId_Bullet"));
   Check(I(nativeAttack,"friendlyTarget")==1&&new[]{"atkPhys","atkMag","atkFire","atkThun","atkDark"}.All(f=>I(nativeAttack,f)==0),"Group area is not a native friendly support attack");
   var attack=clone("AtkParam_Pc",nativeAttack.ID,p.Replay);Set(attack,"selfTarget",1);
   var bullet=clone("Bullet",p.AreaBullet,p.Replay);Set(bullet,"atkId_Bullet",p.Replay);
   // A delayed area is a new hit pass, never the original cast's shared ledger.
   Set(bullet,"isUseSharedHitList",0);
   if(I(source,"sfxId_Bullet")<0&&I(cast,"sfxId_Bullet")>=0)Set(bullet,"sfxId_Bullet",I(cast,"sfxId_Bullet"));
   var effect=Row("SpEffectParam",I(source,"spEffectId0"));
   Check(I(effect,"effectTargetFriendlyTarget")==1&&I(effect,"effectTargetOpposeTarget")==0,"Support effect relationship changed");
   if(I(effect,"effectTargetSelfTarget")==0){
    var copy=clone("SpEffectParam",effect.ID,p.Replay);Set(copy,"effectTargetSelfTarget",1);Set(bullet,"spEffectId0",p.Replay);
   }
  }
 }
 public static void VerifyAreas(string root,Dictionary<string,PARAM> actual){
  var expected=new List<(string Table,PARAM.Row Row)>();
  Areas(root,(name,source,id)=>{var copy=new PARAM.Row(actual[name].Rows.Single(r=>r.ID==source)){ID=id};expected.Add((name,copy));return copy;},actual);
  foreach(var(name,row)in expected)Check(CageAI.Same(actual[name].Rows.Single(r=>r.ID==row.ID),row),"Echo area field readback mismatch: "+name+" "+row.ID);
 }
 public static void AreaResources(string root,Func<string,Memory<byte>,PARAM> parse,bool stage){
  var input=Path.Combine(root,"dist/mod/regulation.bin");var b=SFUtil.DecryptNightreignRegulation(input);
  var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"Bullet","AtkParam_Pc","SpEffectParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new List<int>());
  var output=Path.Combine(root,"work/echo-casting/areas");Directory.CreateDirectory(output);
  if(stage){
   Areas(root,(name,source,id)=>{var table=tables[name];Check(!table.Rows.Any(r=>r.ID==id),"Echo area row collision");var row=new PARAM.Row(table.Rows.Single(r=>r.ID==source)){ID=id};table.Rows.Add(row);added[name].Add(id);return row;},tables);
   foreach(var name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==name+".param").Bytes=tables[name].Write();}
   var target=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(target);
   Check(actual.Files.Count==b.Files.Count&&actual.Version==b.Version&&actual.Compression==b.Compression,"Echo area binder metadata changed");
   foreach(var file in actual.Files){var intended=b.Files.Single(f=>f.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Echo area binder readback failed");var name=names.SingleOrDefault(n=>Name(file)==n+".param");if(name==null){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}
    var old=parse(name,before[file.ID]);var now=parse(name,file.Bytes);Check(now.Rows.Count==old.Rows.Count+added[name].Count,"Echo area row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing row changed");tables[name]=now;
   }
   VerifyAreas(root,tables);
   File.WriteAllText(Path.Combine(output,"parameters.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(target),addedRows=added,profiles=ReadAreas(root),existingRowsAndUnrelatedTablesUnchanged=true,stagedContainerReadbackPassed=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  }else{
   VerifyAreas(root,tables);
   File.WriteAllText(Path.Combine(output,"installed.json"),JsonSerializer.Serialize(new{installedSha256=Hash(input),installedReadbackPassed=true,profiles=ReadAreas(root),gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  }
  Console.WriteLine("PASS: 11 group echo areas; native ally amounts/radii retained, self included, private hit records; "+(stage?"staged and original rows preserved":"installed rows verified"));
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var input=Path.Combine(root,"dist/mod/regulation.bin");var output=Path.Combine(root,"work/echo-casting/staged");Directory.CreateDirectory(output);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int source,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),"Echo row collision");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==source)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new (string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);OwnerEffect(s);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  var history=RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length).Where(id=>!tables["SpEffectParam"].Rows.Any(r=>r.ID==id)).ToArray();
  RewardCycle.Parameters((name,source,id)=>id==RewardCycle.Reset?tables[name].Rows.Single(r=>r.ID==id):Add(name,source,id),history);
  // Parameters also declares the reset effect, already present in current dist.
  foreach(var name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==name+".param").Bytes=tables[name].Write();}
  var target=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(target);
  Check(actual.Files.Count==b.Files.Count&&actual.Version==b.Version&&actual.Compression==b.Compression,"Binder metadata changed");
  foreach(var f in actual.Files){Check(f.Bytes.Span.SequenceEqual(b.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Binder roundtrip mismatch");var name=names.SingleOrDefault(n=>Name(f)==n+".param");if(name==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(name,before[f.ID]);var now=parse(name,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[name].Count,"Row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing row changed");}
  RewardCatalog.Write(Path.Combine(output,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(output,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(target),owner=Owner,spellFpRate=SpellFpRate,artsFpRate=Convert.ToSingle(s["artsConsumptionRate"].Value),addedRows=added,historyBytes=(RewardCatalog.HistoricalOwners.Length+7)/8,historyBit=RewardCatalog.HistoryBit(Owner),unrelatedRowsAndTablesUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
 }
}
