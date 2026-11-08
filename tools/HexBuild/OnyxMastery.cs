using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
static class OnyxMastery {
 public const int Owner=95005300;
 public const float MeteorRange=20f, MeteorSpeed=15f;
 public const int MeteorPhysical=50, MeteorMagic=50;
 public const int Receipt=Owner+1, Finish=6955300, Started=6955301, Lot=95553000, Behavior=95005390, Judge=3998;
 public static readonly int[] Markers=[Receipt,Finish,Started];
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,PARAM lots){
  foreach(int id in Markers){var r=clone("SpEffectParam",8000000,id);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",id==Receipt?10:-1),("effectEndurance",id==Receipt?-1f:0.15f),("dontDeleteOnDead",id==Receipt?1:0),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);}
  var attack=clone("AtkParam_Pc",303400807,Owner+60);
  foreach(var(f,v)in new(string,object)[]{("atkPhys",MeteorPhysical),("atkMag",MeteorMagic),("atkSuperArmor",10f),("dmgLevel",7),("dmgLevel_vsPlayer",7),("knockbackDist",1f)})Set(attack,f,v);
  foreach(var(source,id)in new[]{(3600810,Owner+50),(3600811,Owner+51),(3600812,Owner+52),(3600860,Owner+53),(3600861,Owner+54)}){
   var r=clone("Bullet",source,id);
   Set(r,"HitBulletID",id is Owner+50 or Owner+51 or Owner+53?id+1:-1);
   // One native terminal child per carrier: do not expire a 0.05s carrier
   // before its one-second periodic launch can run.
   if(id==Owner+52){Set(r,"life",0.05f);Set(r,"HitBulletID",Owner+53);Set(r,"intervalCreateBulletId",-1);Set(r,"intervalCreateTimeMin",0f);Set(r,"intervalCreateTimeMax",0f);Set(r,"intervalCreateWaitTime",0f);Set(r,"sfxId_Bullet",636003);}
   if(id==Owner+53){
    Set(r,"launchConditionType",0);
    // The NPC carrier used limited steering plus deliberate aim scatter.
    // A player-owned meteor inherits the lock descriptor but must aim at it
    // from the elevated portal, including close and low targets.
    Set(r,"lockShootLimitAng",180);Set(r,"hormingOffsetRange",0f);Set(r,"targetYOffsetRange",0f);
    Check(Convert.ToSingle(r["initVellocity"].Value)==MeteorSpeed&&Convert.ToSingle(r["maxVellocity"].Value)==MeteorSpeed,"Meteor speed changed");
    Set(r,"life",MeteorRange/MeteorSpeed);Set(r,"lifeRandomRange",0f);
   }
   if(id>=Owner+53){Set(r,"atkId_Bullet",Owner+60);Set(r,"isUseSharedHitList",1);Set(r,"dmgHitRecordLifeTime",0.2f);}
  }
  var behavior=clone("BehaviorParam_PC",300805900,Behavior);
  Set(behavior,"refId",Owner+50);Set(behavior,"behaviorJudgeId",Judge);Set(behavior,"stamina",20);
  var lot=clone("ItemLotParam_map",lots.Rows.First(r=>Convert.ToInt32(r["lotItemCategory01"].Value)==2&&Convert.ToInt32(r["lotItemId01"].Value)>0).ID,Lot);
  for(int n=1;n<=8;n++){Set(lot,$"lotItemId{n:00}",-1);Set(lot,$"lotItemCategory{n:00}",0);Set(lot,$"lotItemBasePoint{n:00}",0);Set(lot,$"lotItemNum{n:00}",0);}
  // Category 6 generates the native custom weapon, including its random
  // attachment table. Category 2 only grants the bare EquipParamWeapon.
  foreach(var(f,v)in new[]{("lotItemId01",8010000),("lotItemCategory01",6),("lotItemBasePoint01",1000),("lotItemNum01",1),("getItemFlagId",0),("cumulateNumFlagId",0),("cumulateNumMax",0)})Set(lot,f,v);
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
 public static void RefreshPlanted(string sourceMod,string outputMod,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(Path.Combine(outputMod,"chr"));
  var b=SFUtil.DecryptNightreignRegulation(Path.Combine(sourceMod,"regulation.bin"));var original=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var f=b.Files.Single(f=>Name(f)=="Bullet.param");var p=parse("Bullet",f.Bytes);
  var changes=new Dictionary<int,Dictionary<string,object>>{
   [Owner+52]=new(){{"life",0.05f},{"HitBulletID",Owner+53},{"intervalCreateBulletId",-1},{"intervalCreateTimeMin",0f},{"intervalCreateTimeMax",0f},{"intervalCreateWaitTime",0f}},
   [Owner+53]=new(){{"launchConditionType",0},{"lockShootLimitAng",180},{"hormingOffsetRange",0f},{"targetYOffsetRange",0f},{"life",MeteorRange/MeteorSpeed},{"lifeRandomRange",0f}}
  };
  foreach(var (id,fields) in changes){var row=p.Rows.Single(r=>r.ID==id);foreach(var(field,value)in fields)Set(row,field,value);}
  f.Bytes=p.Write();
  var attackFile=b.Files.Single(x=>Name(x)=="AtkParam_Pc.param");var attacks=parse("AtkParam_Pc",attackFile.Bytes);
  var attack=attacks.Rows.Single(r=>r.ID==Owner+60);Set(attack,"atkPhys",MeteorPhysical);Set(attack,"atkMag",MeteorMagic);attackFile.Bytes=attacks.Write();
  string output=Path.Combine(outputMod,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var read=SFUtil.DecryptNightreignRegulation(output);foreach(var x in read.Files)Check(x.Bytes.Span.SequenceEqual(b.Files.Single(y=>y.ID==x.ID).Bytes.Span),"Planted parameters readback differs");
  var before=parse("Bullet",original[f.ID]);var now=parse("Bullet",f.Bytes);
  foreach(var old in before.Rows){var row=now.Rows.Single(r=>r.ID==old.ID);foreach(var c in old.Cells)if(!changes.TryGetValue(old.ID,out var fields)||!fields.ContainsKey(c.Def.InternalName))Check(Value(c.Value)==Value(row[c.Def.InternalName].Value),"Unrelated bullet field changed");}
  foreach(var x in read.Files.Where(x=>x.ID!=f.ID&&x.ID!=attackFile.ID))Check(x.Bytes.Span.SequenceEqual(original[x.ID]),"Unrelated table changed");
  var oldAttacks=parse("AtkParam_Pc",original[attackFile.ID]);var newAttacks=parse("AtkParam_Pc",read.Files.Single(x=>x.ID==attackFile.ID).Bytes);
  Check(oldAttacks.Rows.Count==newAttacks.Rows.Count,"Attack row count changed");
  foreach(var (old,row) in oldAttacks.Rows.Zip(newAttacks.Rows)){Check(old.ID==row.ID,"Attack row order changed");foreach(var c in old.Cells){
   if(old.ID==Owner+60&&c.Def.InternalName is "atkPhys" or "atkMag")Check(Convert.ToInt32(row[c.Def.InternalName].Value)==(c.Def.InternalName=="atkPhys"?MeteorPhysical:MeteorMagic),"Meteor base damage differs");
   else Check(Value(c.Value)==Value(row[c.Def.InternalName].Value),"Unrelated attack field changed");
  }}
  var meteor=now.Rows.Single(r=>r.ID==Owner+53);
  Check(Math.Abs(Convert.ToSingle(meteor["life"].Value)*Convert.ToSingle(meteor["initVellocity"].Value)-MeteorRange)<0.001f&&Convert.ToSingle(meteor["minVellocity"].Value)==MeteorSpeed&&Convert.ToSingle(meteor["maxVellocity"].Value)==MeteorSpeed,"Meteor flight budget differs");
  var magic=parse("Magic",b.Files.Single(x=>Name(x)=="Magic.param").Bytes).Rows.Single(r=>r.ID==4700);
  Check(Convert.ToInt32(magic["effectSfxId"].Value)==530124,"Review target Meteorite casting effect");
  var motions=BND4.Read(Path.Combine(sourceMod,"chr/c0000.anibnd.dcx"));var taeFile=motions.Files.Single(f=>Name(f)=="a841.tae");var tae=TAE.Read(taeFile.Bytes);
  Check(tae.Animations.Count(a=>a.ID is 40003 or 40004)==2,"Expected existing private channel actions");
  Check(tae.Animations.Single(a=>a.ID==40003).Events.Count(e=>e.Type==2)==5,"Unexpected existing channel");
  tae.Animations.RemoveAll(a=>a.ID is 40003 or 40004);taeFile.Bytes=tae.Write();motions.Write(Path.Combine(outputMod,"chr/c0000.anibnd.dcx"));
  Animations(outputMod,Path.Combine(outputMod,"chr/c0000.anibnd.dcx"),addStart:false);
  Console.WriteLine("PASS: private carrier terminal launch and planted TAE updated; unrelated parameters preserved");
 }
 public static void RewardStage(string root,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,"work/onyx-fix",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  string input=Path.Combine(mod,"regulation.bin");var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  PARAM Table(string name)=>parse(name,b.Files.Single(f=>Name(f)==name+".param").Bytes);
  var custom=Table("EquipParamCustomWeapon").Rows.Single(r=>r.ID==8010000);
  Check(Convert.ToInt32(custom["targetWeaponId"].Value)==8010000&&Convert.ToInt32(custom["swordArtsTableId"].Value)==1041&&Convert.ToInt32(custom["weaponLevel"].Value)==0&&Convert.ToInt32(custom["isCursed"].Value)==0,"Native Onyx generation row changed");
  int table=Convert.ToInt32(custom["attachEffectTableId_1"].Value);
  Check(table==501200200&&Convert.ToInt32(custom["attachEffectTableId_2"].Value)==-1&&Convert.ToInt32(custom["attachEffectTableId_3"].Value)==-1,"Expected one native ordinary-mode affix slot");
  // Tables legitimately contain repeated IDs; keep every target-specific row.
  var pool=Table("AttachEffectTableParam").Rows.Where(r=>r.ID==table).ToArray();
  var candidates=pool.Where(r=>Convert.ToInt32(r["chanceWeight"].Value)>0).ToArray();
  Check(candidates.Length>1&&candidates.All(r=>Convert.ToInt32(r["attachEffectId"].Value)>0),"Native affix pool is missing or contains an empty positive-weight reward");
  var file=b.Files.Single(f=>Name(f)=="ItemLotParam_map.param");var lots=Table("ItemLotParam_map");var row=lots.Rows.Single(r=>r.ID==Lot);var old=new PARAM.Row(row);
  Check(Convert.ToInt32(row["lotItemId01"].Value)==8010000&&Convert.ToInt32(row["lotItemCategory01"].Value) is 2 or 6,"Unexpected mastery gift");
  Set(row,"lotItemCategory01",6);file.Bytes=lots.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);var now=parse("ItemLotParam_map",actual.Files.Single(f=>Name(f)=="ItemLotParam_map.param").Bytes);
  Check(Same(row,now.Rows.Single(r=>r.ID==Lot)),"Gift readback differs");
  now.Rows.Single(r=>r.ID==Lot)["lotItemCategory01"].Value=old["lotItemCategory01"].Value;
  var original=parse("ItemLotParam_map",before[file.ID]);Check(now.Rows.Count==original.Rows.Count&&now.Rows.Zip(original.Rows).All(p=>Same(p.First,p.Second)),"Unrelated lot row or field changed");
  foreach(var f in actual.Files.Where(f=>f.ID!=file.ID))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{version="0.17.294",giftLot=Lot,category=6,customWeapon=8010000,targetWeapon=8010000,ordinaryAffixSlots=1,affixTable=table,positiveWeightCandidates=candidates.Length,deepNightSlotsUnchanged=true,onlyGiftCategoryChanged=true,inputSha256=Hash(input),outputSha256=Hash(output),gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: native custom Onyx reward with {candidates.Length} weighted affix candidates; only private lot category changed");
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
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,mmv?"work/onyx-mastery/mmv":"work/onyx-mastery/staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","Bullet","AtkParam_Pc","BehaviorParam_PC","ItemLotParam_map"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"OnyxMastery ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  Mechanics(Add,origins["ItemLotParam_map"]);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Check(RewardCatalog.HistoryBit(Owner)==49,"History must append at bit 49");
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
  Animations(mod,Path.Combine(dir,"c0000.anibnd.dcx"));
  Resources(root,mod,Path.Combine(dir,"sfxbnd_commoneffects_dlc01.ffxbnd.dcx"));
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=49,historyBytes=7,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Onyx mastery owner, weapon gift and meteor art, append-only reward, Mystery Box instance and guarded cleanup staged");
 }


 public static bool AddedAnimation(string name,long id)=>name=="a841.tae"&&id is 40003 or 40004;
 public static void Animations(string mod,string output,bool addStart=true){
  var b=BND4.Read(Path.Combine(mod,"chr/c0000.anibnd.dcx"));var f=b.Files.Single(f=>Name(f)=="a841.tae");var t=TAE.Read(f.Bytes);var before=TAE.Read(f.Bytes);
  Check(!t.Animations.Any(a=>a.ID is 40003 or 40004),"Onyx loop already present or conflicts");
  var startup=t.Animations.Single(a=>a.ID==40000);var bullet=startup.Events.First(e=>e.Type==2&&BitConverter.ToInt32(e.GetParameterBytes(false),8)==3900);
  Check(bullet.StartTime==2.7f,"Onyx native wave timing changed");
  void Add(TAE.Animation a,int type,float start,float end,byte[] bytes){var group=new TAE.EventGroup(type);a.EventGroups.Add(group);a.Events.Add(new TAE.Event(start,end,type,0,bytes,false){Group=group});}
  byte[] Effect(int id){var d=new byte[16];BitConverter.GetBytes(id).CopyTo(d,0);return d;}
  if(addStart)Add(startup,302,bullet.StartTime,100f,Effect(Started));
  foreach(int id in new[]{40003,40004}){
   var a=new TAE.Animation(id,new TAE.Animation.AnimMiniHeader.Standard{ImportsHKX=false,IsLoopByDefault=false,ImportHKXSourceAnimID=-1},$"a841_{id:D6}.hkt");
   // Keep native controls aligned to the settled recovery at 3.3s.
   // The impact recoil is played only on channel entry. Never repeat the wave,
   // hitboxes, sounds or onset effect from the original startup.
   float origin=id==40003?2.7f:3.3f;
   bool Control(TAE.Event e)=>id==40003?e.Type is 16 or 225 or 712:e.Type is 0 or 16 or 112 or 129 or 225 or 300 or 712 or 10150;
   foreach(var e in startup.Events.Where(e=>Control(e)&&e.EndTime>origin)){
    var group=new TAE.EventGroup(e.Type);a.EventGroups.Add(group);
    float start=id==40003?0:Math.Max(0,e.StartTime-origin);
    float end=id==40003?3.2f:Math.Min(1.7f,e.EndTime-origin);
    if(end>start)a.Events.Add(new TAE.Event(start,end,e.Type,e.Unk04,e.GetParameterBytes(false),false){Group=group});
   }
   if(id==40003){
    var turn=startup.Events.Single(e=>e.Type==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==7&&e.StartTime<2.7f&&e.EndTime>3.3f);
    Add(a,0,0,3.2f,turn.GetParameterBytes(false).ToArray());
    var spell=TAE.Read(b.Files.Single(x=>Name(x)=="a415.tae").Bytes).Animations.Single(x=>x.ID==45012);
    var wind=spell.Events.Single(e=>e.Type==96&&BitConverter.ToInt32(e.GetParameterBytes(false))==530125);
    var windBytes=wind.GetParameterBytes(false).ToArray();
    Check(BitConverter.ToInt32(windBytes,4)==900&&BitConverter.ToInt32(windBytes,8)==2,"Native Meteorite body binding changed");
    Add(a,96,0,3.1f,windBytes);
    // Resolve native Magic/effectSfxId once at build time. Do not replay event
    // 115: that would depend on whichever spell happens to occupy the slot.
    var aura=windBytes.ToArray();BitConverter.GetBytes(530124).CopyTo(aura,0);BitConverter.GetBytes(3).CopyTo(aura,8);
    Add(a,96,0,3.1f,aura);
    for(int i=0;i<5;i++){var d=bullet.GetParameterBytes(false).ToArray();BitConverter.GetBytes(58).CopyTo(d,0);BitConverter.GetBytes(Judge).CopyTo(d,8);d[19]=0;Add(a,2,0.4f+i*0.6f,0.5f+i*0.6f,d);}
    Add(a,302,3.1f,100f,Effect(Finish));
   }
   t.Animations.Add(a);
  }
  t.Animations.Sort((a,b)=>a.ID.CompareTo(b.ID));f.Bytes=t.Write();
  b.Write(output);var actual=BND4.Read(output);Check(actual.Files.Count==b.Files.Count,"Animation binder count changed");
  foreach(var x in actual.Files)Check(x.Bytes.Span.SequenceEqual(b.Files.Single(y=>y.ID==x.ID).Bytes.Span),"Animation binder readback failed");
  var loops=TAE.Read(actual.Files.Single(f=>Name(f)=="a841.tae").Bytes);
  var held=loops.Animations.Single(a=>a.ID==40003);var recovery=loops.Animations.Single(a=>a.ID==40004);
  Check(held.Events.Count(e=>e.Type==2)==5,"Meteor count changed");
  Check(held.Events.Any(e=>e.Type==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==7&&e.StartTime==0&&e.EndTime==3.2f),"Channel turning must stay disabled");
  Check(held.Events.Where(e=>e.Type==96).Select(e=>BitConverter.ToInt32(e.GetParameterBytes(false))).Order().SequenceEqual(new[]{530124,530125}),"Native Meteorite visuals missing");
  Check(!recovery.Events.Any(e=>e.Type is 1 or 2 or 64 or 115 or 302),"Recovery must not repeat attacks, spells or channel markers");
  var expectedRecovery=startup.Events.Where(e=>e.Type is 0 or 16 or 112 or 129 or 225 or 300 or 712 or 10150).Where(e=>e.EndTime>3.3f&&e.StartTime<5f).ToArray();
  Check(recovery.Events.Count==expectedRecovery.Length,"Native recovery controls missing");
  foreach(var (expected,actualEvent)in expectedRecovery.Zip(recovery.Events)){
   Check(actualEvent.Type==expected.Type&&Math.Abs(actualEvent.StartTime-Math.Max(0,expected.StartTime-3.3f))<0.00001f&&Math.Abs(actualEvent.EndTime-Math.Min(1.7f,expected.EndTime-3.3f))<0.00001f,"Recovery timing differs from native tail");
   var x=expected.GetParameterBytes(false);var y=actualEvent.GetParameterBytes(false);
   Check(x.Take(Math.Min(x.Length,y.Length)).SequenceEqual(y.Take(Math.Min(x.Length,y.Length)))&&x.Skip(y.Length).All(v=>v==0)&&y.Skip(x.Length).All(v=>v==0),"Native recovery control payload differs");
  }
  foreach(var old in before.Animations){
   var now=loops.Animations.Single(a=>a.ID==old.ID);
   Check(old.AnimFileName==now.AnimFileName&&JsonSerializer.Serialize(old.MiniHeader,old.MiniHeader.GetType())==JsonSerializer.Serialize(now.MiniHeader,now.MiniHeader.GetType()),"Existing Onyx animation header changed");
   Check(now.Events.Count==old.Events.Count+(addStart&&old.ID==40000?1:0),"Existing Onyx event count changed");
   foreach(var pair in old.Events.Zip(now.Events)){
    var a=pair.First;var c=pair.Second;var x=a.GetParameterBytes(false);var y=c.GetParameterBytes(false);
    // TAE serialization may adjust trailing zero alignment, not event fields.
    Check(a.Type==c.Type&&a.Unk04==c.Unk04&&a.StartTime==c.StartTime&&a.EndTime==c.EndTime&&a.Group?.GroupType==c.Group?.GroupType&&x.Take(Math.Min(x.Length,y.Length)).SequenceEqual(y.Take(Math.Min(x.Length,y.Length)))&&x.Skip(y.Length).All(v=>v==0)&&y.Skip(x.Length).All(v=>v==0),"Existing Onyx event payload changed");
   }
  }

 }
 public static void Resources(string root,string mod,string output){
  var b=BND4.Read(Path.Combine(mod,"sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx"));
  var boss=BND4.Read(Path.Combine(root,"assets/original/sfx/sfxbnd_c3600.ffxbnd.dcx"));
  foreach(int id in new[]{636000,636001,636003})foreach(var(ext,group)in new[]{("fxr",0),("ffxreslist",400000)}){
   string name=$"f{id:D9}.{ext}";var source=boss.Files.Single(f=>Name(f)==name);var old=b.Files.SingleOrDefault(f=>Name(f)==name);
   if(old!=null){Check(old.Bytes.Span.SequenceEqual(source.Bytes.Span),"Onyx SFX collision: "+name);continue;}
   int slot=b.Files.Where(f=>f.ID>=group&&f.ID<group+100000).Max(f=>f.ID)+1;
   b.Files.Add(new BinderFile(source.Flags,slot,source.Name,source.Bytes.ToArray()));
  }
  b.Files.Sort((a,c)=>a.ID.CompareTo(c.ID));b.Write(output);
  var actual=BND4.Read(output);Check(actual.Files.Count==b.Files.Count&&actual.Files.Zip(b.Files).All(p=>p.First.Bytes.Span.SequenceEqual(p.Second.Bytes.Span)),"SFX roundtrip failed");
 }
 public static void DashMap(string input,string output,Func<string,Memory<byte>,PARAM> parse){
  var b=SFUtil.DecryptNightreignRegulation(input);var p=parse("BehaviorParam_PC",b.Files.Single(f=>Name(f)=="BehaviorParam_PC.param").Bytes);
  var anim=BND4.Read(Path.Combine(Path.GetDirectoryName(input)!,"chr/c0000.anibnd.dcx"));
  var t=TAE.Read(anim.Files.Single(f=>Name(f)=="a40.tae").Bytes);
  var judges=t.Animations.Where(a=>a.ID is 30200 or 30210 or 32200 or 32210 or 34200).SelectMany(a=>a.Events.Where(e=>e.Type==1)).Select(e=>BitConverter.ToInt32(e.GetParameterBytes(false),8)).ToHashSet();
  Check(judges.SetEquals(new[]{120,125,320,325,860,865}),"Curved greatsword dash actions changed");
  var ids=p.Rows.Where(r=>r.ID<300000000&&judges.Contains(Convert.ToInt32(r["behaviorJudgeId"].Value))&&Convert.ToInt32(r["refType"].Value)==0).Select(r=>r.ID).Distinct().Order().ToArray();
  Check(ids.Contains(100400120)&&ids.Contains(100400125),"Native curved greatsword dash attacks absent");
  // Native behavior selectors map ordinary/charged R2 to 305xx/325xx and
  // jumping R2 to 3123x..3127x/3323x..3327x. Guard counters are separate.
  var heavyJudges=t.Animations.Where(a=>a.ID is >=30500 and <30600 or >=32500 and <32600 or >=31230 and <=31270 or >=33230 and <=33270)
   .SelectMany(a=>a.Events.Where(e=>e.Type==1)).Select(e=>BitConverter.ToInt32(e.GetParameterBytes(false),8)).ToHashSet();
  Check(heavyJudges.SetEquals(new[]{100,105,110,115,116,160,300,305,310,315,316,360}),"Curved greatsword heavy actions changed");
  Check(!heavyJudges.Overlaps(judges),"Dash and stationary heavy judges overlap");
  var heavy=p.Rows.Where(r=>r.ID<300000000&&heavyJudges.Contains(Convert.ToInt32(r["behaviorJudgeId"].Value))&&Convert.ToInt32(r["refType"].Value)==0).Select(r=>r.ID).Distinct().Order().ToArray();
  Check(new[]{100400100,100400105,100400300,100400305,100400160,100400360}.All(heavy.Contains),"Native heavy references absent");
  File.WriteAllText(output,"// Generated from target curved greatsword TAE judges and BehaviorParam_PC references.\npub const DASH_BEHAVIORS:&[i32]=&["+string.Join(",",ids)+"];\npub const HEAVY_BEHAVIORS:&[i32]=&["+string.Join(",",heavy)+"];\n");
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,bool mmv=false,bool tune=false){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),input=Path.Combine(mod,"regulation.bin");
  var b=SFUtil.DecryptNightreignRegulation(input);var originals=new Dictionary<string,PARAM>();var tables=new Dictionary<string,PARAM>();
  PARAM Table(string n){if(!tables.TryGetValue(n,out var p)){p=parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);tables[n]=p;}return p;}
  PARAM Original(string n){if(!originals.TryGetValue(n,out var p)){p=parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes);originals[n]=p;}return p;}
  var expected=new Dictionary<(string,int),PARAM.Row>();
  PARAM.Row Clone(string n,int from,int id){var r=new PARAM.Row(Original(n).Rows.First(r=>r.ID==from)){ID=id};expected.Add((n,id),r);return r;}
  Mechanics(Clone,Original("ItemLotParam_map"));
  if(tune){
   // Upgrade only the prototype's private contact records; original rows and
   // source attack classification are invariant.
   foreach(int id in new[]{Owner+53,Owner+54})foreach(string field in new[]{"launchConditionType","dmgHitRecordLifeTime"}){
    var r=Table("Bullet").Rows.Single(r=>r.ID==id);r[field].Value=expected[("Bullet",id)][field].Value;
   }
   b.Files.Single(f=>Name(f)=="Bullet.param").Bytes=Table("Bullet").Write();SFUtil.EncryptNightreignRegulation(input,b,b.Compression);
  }
  foreach(var(key,row)in expected)Check(Same(Table(key.Item1).Rows.Single(r=>r.ID==key.Item2),row,false),$"Onyx private row changed: {key}");
  foreach(var n in expected.Keys.Select(k=>k.Item1).Distinct())foreach(var r in Original(n).Rows.Where(r=>n=="Bullet"?r.ID is >=3600800 and <=3600861:n=="BehaviorParam_PC"?r.ID is >=300805900 and <=300805904:n=="AtkParam_Pc"?r.ID is >=303400800 and <=303400807:false))Check(Same(Table(n).Rows.Single(x=>x.ID==r.ID),r,false),$"Native Onyx row changed: {n}/{r.ID}");
  var bnd=BND4.Read(Path.Combine(mod,"chr/c0000.anibnd.dcx"));var tae=TAE.Read(bnd.Files.Single(f=>Name(f)=="a841.tae").Bytes);
  foreach(int id in new[]{40003,40004}){
   var a=tae.Animations.Single(a=>a.ID==id);Check(a.MiniHeader is TAE.Animation.AnimMiniHeader.Standard m&&!m.ImportsHKX&&a.AnimFileName==$"a841_{id:D6}.hkt","Onyx planted motion missing");
   Check(!a.Events.Any(e=>e.Type is 64 or 115),"Onyx channel must not read spell slots");
  }
  var shots=tae.Animations.Single(a=>a.ID==40003).Events.Where(e=>e.Type==2).ToArray();
  Check(shots.Length==5&&shots.Select(e=>e.StartTime).Distinct().Count()==5&&shots.All(e=>BitConverter.ToInt32(e.GetParameterBytes(false),8)==Judge),"Onyx must fire five distinct weapon events");
  Check(tae.Animations.Single(a=>a.ID==40003).Events.Any(e=>e.Type==302&&BitConverter.ToInt32(e.GetParameterBytes(false))==Finish&&e.StartTime>shots.Max(x=>x.StartTime)),"Missing bounded channel exit");
  Check(tae.Animations.Single(a=>a.ID==40000).Events.Any(e=>e.Type==302&&BitConverter.ToInt32(e.GetParameterBytes(false))==Started)&&!tae.Animations.Single(a=>a.ID==40005).Events.Any(e=>e.Type==302&&BitConverter.ToInt32(e.GetParameterBytes(false))==Started),"Empty-FP art must not enter channel");
  string report=Path.Combine(root,mmv?"dist/mmv/onyx-mastery-validation.json":"dist/reports/onyx-mastery-resources-validation.json");
  File.WriteAllText(report,JsonSerializer.Serialize(new{version="0.17.290",owner=Owner,privateRows=expected.Count,weapon=8010000,giftLot=Lot,meteorCount=5,meteorIntervalSeconds=0.6,initialFpCost=27,perMeteorBaseStamina=20,rawPhysical=MeteorPhysical,rawMagic=MeteorMagic,sourceClassification="weapon art",nativeOnyxRowsUnchanged=true,spellSlotEventsAbsent=true,emptyFpChannelBlocked=true,regulationSha256=Hash(input),animationSha256=Hash(Path.Combine(mod,"chr/c0000.anibnd.dcx")),gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Onyx private parameters, original rows, five meteor events, native motion imports and empty-FP boundary verified");
 }

}
