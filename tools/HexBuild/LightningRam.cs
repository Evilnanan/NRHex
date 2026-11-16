using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static partial class LightningRam {
 public const int Owner=95005600;
 public static readonly int[] Markers=[Owner+1];
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var fast=clone("SpEffectParam",8000000,Owner+1);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",-1),("effectEndurance",-1f),("dontDeleteOnDead",0),("isDisableNetSync",0),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(fast,f,v);
  var attack=clone("AtkParam_Pc",69300,Owner+60);
  foreach(var c in attack.Cells){
   string n=c.Def.InternalName;
   if((n.StartsWith("atk")&&n!="atkObj")||n.StartsWith("guardAtk")||n=="regainableSlotDamage"||n=="reviveHighHpCutRate")Set(attack,n,0);
   if(n.StartsWith("spEffectId"))Set(attack,n,-1);
  }
  foreach(var(f,v)in new(string,object)[]{("atkPhys",1),("atkSuperArmor",6f),("atkSuperArmorCorrection",100),("ultimateChargeCorrection",30),("reviveDamage",0f),("reviveDamageCorrection",0),("atkAttribute",0),("spAttribute",0),("selfTarget",0),("friendlyTarget",0),("opposeTarget",1),("dmgLevel",7),("dmgLevel_vsPlayer",7),("knockbackDist",1f),("mapHitType",1)})Set(attack,f,v);
  MeleeFields(attack);
  var extreme=clone("AtkParam_Pc",69300,Owner+61);foreach(var c in attack.Cells)Set(extreme,c.Def.InternalName,c.Value);
  var fastAttack=clone("AtkParam_Pc",69300,Owner+62);foreach(var c in attack.Cells)Set(fastAttack,c.Def.InternalName,c.Value);
  ContactPower(attack,12,20,false);ContactPower(fastAttack,18,30,false);ContactPower(extreme,30,50,true);
  var bullet=clone("Bullet",7720606,Owner+50);
  foreach(var(f,v)in new(string,object)[]{("atkId_Bullet",Owner+60),("sfxId_Bullet",-1),("sfxId_Hit",-1),("sfxId_Flick",-1),("life",0.04f),("lifeRandomRange",0f),("hitRadius",0.75f),("hitRadiusMax",0.75f),("initVellocity",0f),("minVellocity",0f),("maxVellocity",0f),("gravityInRange",0f),("gravityOutRange",0f),("accelInRange",0f),("accelOutRange",0f),("HitBulletID",-1),("intervalCreateBulletId",-1),("numShoot",1),("launchConditionType",0),("isUseSharedHitList",1),("dmgHitRecordLifeTime",0.1f),("isHitBothTeam",0),("isPenetrateMap",1),("isCheckWall_byCenterRay",1),("staminaDamp",0),("knockbackDamp",0),("spEffectIDForShooter",-1),("spEffectId0",-1),("spEffectId1",-1),("spEffectId2",-1),("spEffectId3",-1),("spEffectId4",-1)})Set(bullet,f,v);
  // Suppress the native contact only while the swept high-speed contact owns
  // damage. Low-speed rolls still resolve the unchanged native attacks.
  var harmless=clone("AtkParam_Pc",300000710,Owner+69);
  foreach(var c in harmless.Cells){string n=c.Def.InternalName;if((n.StartsWith("atk")&&n!="atkObj")||n.StartsWith("guardAtk")||n.StartsWith("dmgLevel")||n=="regainableSlotDamage"||n=="reviveHighHpCutRate")Set(harmless,n,0);if(n.StartsWith("spEffectId"))Set(harmless,n,-1);}
  foreach(string f in new[]{"opposeTarget","friendlyTarget","selfTarget"})Set(harmless,f,0);
  var behavior=clone("BehaviorParam_PC",300000710,Owner+90);Set(behavior,"refId",Owner+69);
 }
 static void ContactPower(PARAM.Row row,float poise,float rescue,bool extreme){
  Set(row,"atkSuperArmor",poise);Set(row,"atkSuperArmorCorrection",100);
  Set(row,"reviveDamage",rescue);Set(row,"reviveDamageCorrection",100);
  Set(row,"atkBehaviorId",extreme?12:0);
 }
 public static void PowerStage(string root,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string input=Path.Combine(root,mmv?"dist/mmv/mod/regulation.bin":"dist/mod/regulation.bin");
  string dir=Path.Combine(root,"work/lightning-ram-power",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="AtkParam_Pc.param");var table=parse("AtkParam_Pc",file.Bytes);
  var regular=table.Rows.Single(r=>r.ID==Owner+60);var extreme=table.Rows.Single(r=>r.ID==Owner+61);
  Check(Convert.ToInt32(regular["atkBehaviorId"].Value)==0&&Convert.ToInt32(extreme["atkBehaviorId"].Value)==12,"Unexpected Ram attack rows");
  Check(!table.Rows.Any(r=>r.ID==Owner+62),"Fast attack ID collision");
  var fast=new PARAM.Row(regular){ID=Owner+62};table.Rows.Add(fast);
  ContactPower(regular,12,20,false);ContactPower(fast,18,30,false);ContactPower(extreme,30,50,true);
  table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=table.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder changed");
  string[] allowed={"atkSuperArmor","atkSuperArmorCorrection","reviveDamage","reviveDamageCorrection"};
  foreach(var f in actual.Files){
   Check(f.Bytes.Span.SequenceEqual(b.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Readback mismatch");
   if(f.ID!=file.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse("AtkParam_Pc",before[f.ID]);var now=parse("AtkParam_Pc",f.Bytes);
   Check(now.Rows.Count==old.Rows.Count+1,"Unexpected row count");
   foreach(var r in old.Rows){var nr=now.Rows.Single(x=>x.ID==r.ID);Check(nr.Name==r.Name,"Name changed");foreach(var c in r.Cells)if(!(r.ID==Owner+60||r.ID==Owner+61)||!allowed.Contains(c.Def.InternalName))Check(Value(c.Value)==Value(nr[c.Def.InternalName].Value),"Unrelated attack field changed");}
   foreach(var row in new[]{regular,fast,extreme})Check(Same(row,now.Rows.Single(r=>r.ID==row.ID)),"Tier readback mismatch");
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),addedAttack=Owner+62,tiers=new[]{new{speed=6,attack=Owner+60,poise=12,rescue=20},new{speed=10,attack=Owner+62,poise=18,rescue=30},new{speed=15,attack=Owner+61,poise=30,rescue=50}},onlyPowerFieldsAndOneRowChanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Ram power tiers roundtrip; only four fields on two private rows and one new private row changed");
 }
 public static void Extreme(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string input=Path.Combine(root,mmv?"dist/mmv/mod/regulation.bin":"dist/mod/regulation.bin");
  string dir=Path.Combine(root,"work/lightning-ram-enemy-surface",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  var original=parse("AtkParam_Pc",native.Files.Single(f=>Name(f)=="AtkParam_Pc.param").Bytes);
  foreach(int id in new[]{700000001,700000006,700000100,700000101})Check(Convert.ToInt32(original.Rows.Single(r=>r.ID==id)["atkBehaviorId"].Value)==12,"Native ultimate behavior changed");
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="AtkParam_Pc.param");var table=parse("AtkParam_Pc",file.Bytes);
  var source=table.Rows.Single(r=>r.ID==Owner+60);var extreme=new PARAM.Row(source){ID=Owner+61};Set(extreme,"atkBehaviorId",12);
  Check(Convert.ToInt32(source["atkBehaviorId"].Value)==0,"Unexpected regular ram behavior");
  var existing=table.Rows.SingleOrDefault(r=>r.ID==extreme.ID);Check(existing==null||Same(existing,extreme),"Extreme attack ID collision");
  if(existing==null)table.Rows.Add(extreme);table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=table.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder changed");
  foreach(var f in actual.Files){
   Check(f.Bytes.Span.SequenceEqual(b.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Readback mismatch");
   if(f.ID!=file.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse("AtkParam_Pc",before[f.ID]);var now=parse("AtkParam_Pc",f.Bytes);
   Check(now.Rows.Count==old.Rows.Count+(existing==null?1:0),"Unexpected row count");
   Check(old.Rows.Where(r=>r.ID!=extreme.ID).OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>r.ID!=extreme.ID).OrderBy(r=>r.ID)).All(pair=>Same(pair.First,pair.Second)),"Existing attack changed");
   Check(Same(extreme,now.Rows.Single(r=>r.ID==extreme.ID)),"Extreme attack mismatch");
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),addedAttack=Owner+61,nativeUltimateBehavior=12,onlyOneRowAdded=true,existingRowsPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: extreme Ram attack uses native ultimate stagger; all existing rows and other tables preserved");
 }
 public static void Physical(string root,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string input=Path.Combine(root,mmv?"dist/mmv/mod/regulation.bin":"dist/mod/regulation.bin");
  string dir=Path.Combine(root,"work/lightning-ram-physical",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var changes=new Dictionary<string,(int id,string field,int value)>{["Bullet.param"]=(Owner+50,"isPenetrateMap",1),["AtkParam_Pc.param"]=(Owner+60,"mapHitType",1)};
  foreach(var(name,change)in changes){var f=b.Files.Single(f=>Name(f)==name);var t=parse(name[..^6],f.Bytes);var row=t.Rows.Single(r=>r.ID==change.id);Set(row,change.field,change.value);
   if(name=="Bullet.param")Check(Convert.ToInt32(row["isCheckWall_byCenterRay"].Value)==1,"Wall occlusion must remain enabled");
   else Check(Convert.ToInt32(row["atkObj"].Value)>0,"Object damage required");f.Bytes=t.Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression,"Binder changed");
  foreach(var f in actual.Files){Check(f.Bytes.Span.SequenceEqual(b.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Readback mismatch");
   if(!changes.TryGetValue(Name(f),out var change)){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   string name=Name(f)[..^6];var old=parse(name,before[f.ID]);var now=parse(name,f.Bytes);Check(old.Rows.Count==now.Rows.Count,"Rows changed");
   foreach(var(r,nr)in old.Rows.Zip(now.Rows)){Check(r.ID==nr.ID&&r.Name==nr.Name,"Row identity changed");foreach(var c in r.Cells)if(r.ID!=change.id||c.Def.InternalName!=change.field)Check(Value(c.Value)==Value(nr[c.Def.InternalName].Value),"Unrelated field changed");}
   Check(Convert.ToInt32(now.Rows.Single(r=>r.ID==change.id)[change.field].Value)==change.value,"Changed field mismatch");
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),onlyTwoFieldsChanged=true,wallOcclusionRetained=true},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: stationary contact carrier survives geometry overlap; precise asset attack and wall occlusion preserved");
 }
 public static void Objects(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string input=Path.Combine(root,mmv?"dist/mmv/mod/regulation.bin":"dist/mod/regulation.bin");
  string dir=Path.Combine(root,"work/lightning-ram-objects",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="AtkParam_Pc.param");var table=parse("AtkParam_Pc",file.Bytes);
  var originals=parse("AtkParam_Pc",native.Files.Single(f=>Name(f)=="AtkParam_Pc.param").Bytes);
  var changes=new List<object>();
  foreach(var(id,source)in new[]{(Owner+60,69300),(Owner+69,300000710)}){
   var row=table.Rows.Single(r=>r.ID==id);var value=originals.Rows.Single(r=>r.ID==source)["atkObj"].Value;
   Check(Convert.ToInt32(value)>0,"Native object attack must be positive");
   changes.Add(new{id,source,before=row["atkObj"].Value,after=value});Set(row,"atkObj",value);
  }
  file.Bytes=table.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);Check(actual.Version==b.Version&&actual.Compression==b.Compression,"Binder changed");
  foreach(var f in actual.Files){
   Check(f.Bytes.Span.SequenceEqual(b.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Readback mismatch");
   if(f.ID!=file.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse("AtkParam_Pc",before[f.ID]);var now=parse("AtkParam_Pc",f.Bytes);Check(old.Rows.Count==now.Rows.Count,"Rows changed");
   foreach(var(r,nr)in old.Rows.Zip(now.Rows)){Check(r.ID==nr.ID&&r.Name==nr.Name,"Row identity changed");foreach(var c in r.Cells)if(r.ID!=Owner+60&&r.ID!=Owner+69||c.Def.InternalName!="atkObj")Check(Value(c.Value)==Value(nr[c.Def.InternalName].Value),"Unrelated field changed");}
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changes,onlyTwoObjectAttackFieldsChanged=true},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine(JsonSerializer.Serialize(changes));
 }
 // MMV legitimately contains non-finite floats; compare their exact bits.
 static string Value(object? v)=>v switch{
  float n=>"f32:"+BitConverter.SingleToInt32Bits(n).ToString("X8"),
  double n=>"f64:"+BitConverter.DoubleToInt64Bits(n).ToString("X16"),
  _=>JsonSerializer.Serialize(v)
 };
 static bool Same(PARAM.Row a,PARAM.Row b,bool names=true)=>a.ID==b.ID&&(!names||a.Name==b.Name)&&a.Cells.Select(c=>(c.Def.InternalName,Value(c.Value))).SequenceEqual(b.Cells.Select(c=>(c.Def.InternalName,Value(c.Value))));
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Events(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0,"Parameterized lifecycle");
   int at=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(at>0&&e.Instructions.Take(at).Any(i=>i.Bank==3),"Missing guarded lifecycle anchor");
   Check(!e.Instructions.Any(i=>Removal(i,Owner)),"Duplicate owner cleanup");
   e.Instructions.InsertRange(at,new[]{Owner}.Concat(Markers).Select(id=>new EMEVD.Instruction(2004,21,new object[]{20000u,id})));
  }
 }
 static bool Removal(EMEVD.Instruction i,int id)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==id;
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes,bool mmv=false){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,mmv?"work/lightning-ram/mmv":"work/lightning-ram/staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","Bullet","AtkParam_Pc","BehaviorParam_PC"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"LightningRam ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0),("isDisableNetSync",0)})Set(s,f,v);
  Mechanics(Add);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Check(RewardCatalog.HistoryBit(Owner)==51,"History must append at bit 51");
  int marker=DoubleDip.FirstMarker+RewardCatalog.HistoryBit(Owner);
  var hidden=Add("SpEffectParam",8000000,marker);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(hidden,f,v);
  foreach(int id in RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length).Where(id=>!tables["SpEffectParam"].Rows.Any(r=>r.ID==id))){
   var record=Add("SpEffectParam",8000000,id);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(record,f,v);
  }
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){
   var intended=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter roundtrip failed");
   var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count&&old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>Same(pair.First,pair.Second)),"Existing parameter row changed");
  }
  string eventInput=Path.Combine(mod,"event/common.emevd.dcx");var ev=EMEVD.Read(eventInput);var previous=EMEVD.Read(eventInput);Events(ev);
  var parent=ev.Events.Single(e=>e.ID==0);Check(parent.Parameters.Count==0,"Parameterized constructor");
  var inits=parent.Instructions.Where(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length>=8&&BitConverter.ToInt32(i.ArgData,4)==DoubleDip.GrantEvent).ToArray();
  Check(inits.Length==hexes.Length-2&&inits.All(i=>i.ArgData.Length==16)&&!inits.Any(i=>BitConverter.ToInt32(i.ArgData,12)==Owner),"Unexpected Mystery Box worker instances");
  parent.Instructions.Insert(0,new(2000,0,new object[]{inits.Length,DoubleDip.GrantEvent,marker,Owner}));
  foreach(int id in new[]{1111,9100941}){var e=ev.Events.Single(e=>e.ID==id);int at=e.Instructions.FindIndex(i=>Removal(i,Owner));e.Instructions.Insert(at+1,new(2004,21,new object[]{20000u,marker}));}
  string eventOutput=Path.Combine(dir,"common.emevd.dcx");ev.Write(eventOutput);var final=EMEVD.Read(eventOutput);
  Check(final.Format==previous.Format&&final.Compression==previous.Compression&&final.Events.Count==previous.Events.Count&&final.StringData.SequenceEqual(previous.StringData)&&final.LinkedFileOffsets.SequenceEqual(previous.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in previous.Events){
   var now=final.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(now,ev.Events.Single(x=>x.ID==e.ID)),"Event roundtrip mismatch");
   if(e.ID is 1111 or 9100941){Check(now.Instructions.Count(i=>Removal(i,Owner))==1&&now.Instructions.Count(i=>Removal(i,marker))==1,"Missing cleanup");Check(Markers.All(id=>now.Instructions.Count(i=>Removal(i,id))==1),"Missing mastery cleanup");now.Instructions.RemoveAll(i=>Removal(i,Owner)||Removal(i,marker)||Markers.Any(id=>Removal(i,id)));}
   if(e.ID==0)now.Instructions.RemoveAt(0);Check(Cages.Same(e,now),"Unrelated event changed");
  }
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=51,historyBytes=7,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Lightning Ram owner and contact attack, append-only reward, Mystery Box instance and guarded cleanup staged");
 }


}
