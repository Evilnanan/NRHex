using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class ColdJuice {
 public const int Owner=95005700, Sfx=950957, StartVisual=Owner+51;
 public static readonly int[] Markers=Enumerable.Range(1,7).Select(i=>Owner+i).ToArray();
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var self=clone("SpEffectParam",8450103,Owner+1);
  // stateInfo 260 enters the native buildup gate, not an unconditional proc.
  Set(self,"freezeAttackPower",99999);Set(self,"dontDeleteOnDead",0);Set(self,"isDisableNetSync",0);
  // Native weapon enchant entry conditions and opposite-hand categories stay intact.
  foreach(var (entry,enhance,entryBase,enhanceBase) in new[]{(Owner+2,Owner+3,825,826),(Owner+4,Owner+5,827,828)}){
   var start=clone("SpEffectParam",entryBase,entry);Set(start,"cycleOccurrenceSpEffectId",enhance);
   var buff=clone("SpEffectParam",enhanceBase,enhance);Set(buff,"effectEndurance",30f);Set(buff,"atkOccurrenceSpEffectId",Owner+7);Set(buff,"freezeAttackPower",30);Set(buff,"dontDeleteOnDead",0);
  }
  var frost=clone("SpEffectParam",1441000,Owner+6);Set(frost,"freezeAttackPower",20);
  var enchant=clone("SpEffectParam",829,Owner+7);Set(enchant,"freezeAttackPower",30);
  var attack=clone("AtkParam_Pc",12441002,Owner+60);
  foreach(var c in attack.Cells){var n=c.Def.InternalName;if((n.StartsWith("atk")&&n!="atkObj")||n.StartsWith("guardAtk")||n=="regainableSlotDamage"||n=="reviveHighHpCutRate")Set(attack,n,0);if(n.StartsWith("spEffectId"))Set(attack,n,-1);}
  foreach(var(f,v)in new(string,object)[]{("atkMag",40),("atkSuperArmor",2f),("atkSuperArmorCorrection",100),("atkAttribute",3),("atkType",3),("atkMaterial",6),("dmgLevel",1),("dmgLevel_vsPlayer",1),("selfTarget",0),("friendlyTarget",0),("opposeTarget",1),("reviveDamage",0f),("reviveDamageCorrection",0),("isDisableBothHandsAtkBonus",1)})Set(attack,f,v);
  var bullet=clone("Bullet",12441002,Owner+50);
  foreach(var(f,v)in new(string,object)[]{("atkId_Bullet",Owner+60),("sfxId_Bullet",334308),("life",7f),("hitRadius",3f),("hitRadiusMax",-1f),("dmgHitRecordLifeTime",0.4f),("isUseSharedHitList",1),("HitBulletID",-1),("intervalCreateBulletId",-1),("spEffectId0",Owner+6),("spEffectIDForShooter",0),("initVellocity",0f),("minVellocity",0f),("maxVellocity",0f),("gravityInRange",0f),("gravityOutRange",0f),("accelInRange",0f),("accelOutRange",0f),("homingAngle",0),("isPenetrateMap",1),("FollowType",0)})Set(bullet,f,v);
  ConfigureStartVisual(clone("Bullet",12441000,StartVisual));
 }
 static void ConfigureStartVisual(PARAM.Row row){
  Check(Convert.ToInt32(row["sfxId_Bullet"].Value)==334307,"Native storm start visual changed");
  // Retain the native 0.1s carrier and FXR's own appearance/termination rules.
  // Do not inherit its damage/frost or spawn a second native damage storm.
  foreach(var(f,v)in new(string,object)[]{("atkId_Bullet",-1),("HitBulletID",-1),("intervalCreateBulletId",-1),("spEffectIDForShooter",0),("hitRadius",0f),("hitRadiusMax",-1f)})Set(row,f,v);
  for(int i=0;i<5;i++)Set(row,"spEffectId"+i,0);
 }
 public static void StartVisualStage(BND4 native,string input,string output,Func<string,Memory<byte>,PARAM> parse){
  var b=SFUtil.DecryptNightreignRegulation(input);var previous=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="Bullet.param");var p=parse("Bullet",file.Bytes);var old=parse("Bullet",file.Bytes);
  var storm=p.Rows.Single(r=>r.ID==Owner+50);Check(Convert.ToInt32(storm["sfxId_Bullet"].Value) is Sfx or 334308,"Unexpected persistent storm visual");Set(storm,"sfxId_Bullet",334308);
  var source=parse("Bullet",native.Files.Single(f=>Name(f)=="Bullet.param").Bytes).Rows.Single(r=>r.ID==12441000);
  Check(!p.Rows.Any(r=>r.ID==StartVisual),"Start visual ID already exists; inspect before replacing");
  var start=new PARAM.Row(source){ID=StartVisual};ConfigureStartVisual(start);p.Rows.Add(start);p.Rows=p.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=p.Write();
  Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(output))!);SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){var intended=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter roundtrip mismatch");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(previous[f.ID]),"Unrelated table changed");}
  var readback=parse("Bullet",actual.Files.Single(f=>f.ID==file.ID).Bytes);
  Set(old.Rows.Single(r=>r.ID==Owner+50),"sfxId_Bullet",334308);
  Check(readback.Rows.Count==old.Rows.Count+1&&old.Rows.OrderBy(r=>r.ID).Zip(readback.Rows.Where(r=>r.ID!=StartVisual).OrderBy(r=>r.ID)).All(x=>Same(x.First,x.Second)),"Existing bullet changed beyond storm visual");
  Check(Same(readback.Rows.Single(r=>r.ID==StartVisual),start),"Visual carrier differs");
  File.WriteAllText(output+".validation.json",JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),newBullet=StartVisual,startSfx=334307,loopSfx=334308,noAttackOrChildren=true,onlyExistingFieldChanged="Bullet/95005750/sfxId_Bullet",gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native start visual carrier added; persistent visual restored to native; all other rows and tables unchanged");
 }
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
 public static void FrostStage(string input,string output,Func<string,Memory<byte>,PARAM> parse){
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="SpEffectParam.param");var p=parse("SpEffectParam",file.Bytes);
  var row=p.Rows.Single(r=>r.ID==Owner+1);
  Check(Convert.ToInt32(row["stateInfo"].Value)==260,"Self-frost state changed");
  Check(Convert.ToInt32(row["freezeAttackPower"].Value) is 0 or 99999,"Unexpected self-frost buildup");
  Set(row,"freezeAttackPower",99999);file.Bytes=p.Write();
  var bulletFile=b.Files.Single(f=>Name(f)=="Bullet.param");var bullets=parse("Bullet",bulletFile.Bytes);
  var storm=bullets.Rows.Single(r=>r.ID==Owner+50);Check(Convert.ToInt32(storm["sfxId_Bullet"].Value) is 334308 or Sfx,"Storm visual changed");
  Set(storm,"sfxId_Bullet",Sfx);bulletFile.Bytes=bullets.Write();
  Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(output))!);
  SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var after=SFUtil.DecryptNightreignRegulation(output);
  Check(after.Version==b.Version&&after.Compression==b.Compression&&after.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var f in after.Files){
   var original=b.Files.Single(o=>o.ID==f.ID);Check(f.Name==original.Name&&f.Flags==original.Flags,"Binder entry changed");
   if(f.ID!=file.ID&&f.ID!=bulletFile.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   string table=f.ID==file.ID?"SpEffectParam":"Bullet",field=f.ID==file.ID?"freezeAttackPower":"sfxId_Bullet";int id=f.ID==file.ID?Owner+1:Owner+50,value=f.ID==file.ID?99999:Sfx;
   var old=parse(table,before[f.ID]);var actual=parse(table,f.Bytes);
   Check(actual.Rows.Count==old.Rows.Count,"Effect count changed");
   Check(Convert.ToInt32(actual.Rows.Single(r=>r.ID==id)[field].Value)==value,"Field roundtrip failed");
   Set(old.Rows.Single(r=>r.ID==id),field,value);
   Check(old.Rows.Zip(actual.Rows).All(x=>Same(x.First,x.Second)),"Unexpected effect field changed");
  }
  File.WriteAllText(output+".validation.json",JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),effect=Owner+1,buildup=99999,bullet=Owner+50,sfx=Sfx,onlyTwoFieldsChanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: self-frost buildup and private storm audio visual corrected; other parameter fields preserved");
 }
 public static void Visual(string root,string input,string output){
  var b=BND4.Read(input);var old=b.Files.Select(f=>new BinderFile(f.Flags,f.ID,f.Name,f.Bytes.ToArray())).ToArray();var replaced=new HashSet<int>();
  foreach(var(ext,group)in new[]{("fxr",0),("ffxreslist",400000)}){
   string name=$"f{Sfx:000000000}.{ext}",asset=Path.Combine(root,"assets/cold-juice",name);var data=File.ReadAllBytes(asset);
   if(ext=="fxr")Check(Hash(asset)=="F7F36EEE917B5BF688DDB48A0A71105E421DAFBC198F8D329C8F92BB7D2D405A","Private storm FXR changed; revalidate native bytes");
   var existing=b.Files.SingleOrDefault(f=>Name(f)==name);
   if(existing!=null){
    if(!existing.Bytes.Span.SequenceEqual(data)){
     Check(ext=="fxr"&&Convert.ToHexString(SHA256.HashData(existing.Bytes.Span))=="DECCE27ABDCA20687D820D8BFA1A87536354FEF247231E3330E2C487F909FB05","Private storm FXR collision");
     existing.Bytes=data;replaced.Add(existing.ID);
    }
    continue;
   }
   var template=b.Files.First(f=>f.Name.EndsWith("."+ext));
   int id=b.Files.Where(f=>f.ID>=group&&f.ID<group+100000).Select(f=>f.ID).DefaultIfEmpty(group-1).Max()+1;Check(id<group+100000,"FXR ID range full");
   b.Files.Add(new BinderFile(template.Flags,id,template.Name[..(template.Name.Length-Name(template).Length)]+name,data));
  }
  b.Files.Sort((a,c)=>a.ID.CompareTo(c.ID));Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(output))!);b.Write(output);var actual=BND4.Read(output);
  Check(actual.Files.Count==b.Files.Count&&actual.Version==b.Version&&actual.Format==b.Format&&actual.Compression==b.Compression,"FXR binder roundtrip changed");
  foreach(var f in b.Files){var now=actual.Files.Single(x=>x.ID==f.ID);Check(f.Name==now.Name&&f.Flags==now.Flags&&f.Bytes.Span.SequenceEqual(now.Bytes.Span),"FXR roundtrip failed");}
  foreach(var f in old)if(!replaced.Contains(f.ID))Check(actual.Files.Single(x=>x.ID==f.ID).Bytes.Span.SequenceEqual(f.Bytes.Span),"Existing FXR changed");
  Console.WriteLine($"PASS: private storm sound 523877 installed; {replaced.Count} known private FXR replaced; all unrelated SFX members unchanged");
 }
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
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,mmv?"work/cold-juice/mmv":"work/cold-juice/staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","Bullet","AtkParam_Pc","BehaviorParam_PC"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"ColdJuice ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0),("isDisableNetSync",0)})Set(s,f,v);
  Mechanics(Add);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Check(RewardCatalog.HistoryBit(Owner)==52,"History must append at bit 52");
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
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=52,historyBytes=7,mechanics=tables.Where(t=>new[]{"SpEffectParam","Bullet","AtkParam_Pc"}.Contains(t.Key)).ToDictionary(t=>t.Key,t=>t.Value.Rows.Where(r=>r.ID>=Owner&&r.ID<Owner+100).Select(r=>new{r.ID,cells=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray()),addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Cold Juice owner and contact attack, append-only reward, Mystery Box instance and guarded cleanup staged");
 }


}
