using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class StaticCharge {
 public const int Owner=95005100;
 public static readonly int[] Markers={Owner+1,Owner+2};
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var ready=clone("SpEffectParam",8000000,Owner+1);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",-1),("effectEndurance",-1f),("iconId",20506),("spEffectTextId_1",Owner+1),("displayIconForPermamentEffect",1),("vfxId",-1)})Set(ready,f,v);
  var mark=clone("SpEffectParam",8000000,Owner+2);
  // Native GOLD Dragonbolt Blessing visuals only: no resistance, rebound or
  // damage modifiers inherited from the original prayer.
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",-1),("effectEndurance",2f),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0),("vfxId",1697100)})Set(mark,f,v);
  for(int i=0;i<4;i++){
   var attack=clone("AtkParam_Pc",69300,Owner+60+i);
   bool wave=(i&1)!=0,leak=i>=2;
   foreach(var(f,v)in new(string,object)[]{("atkPhys",0),("atkMag",0),("atkFire",0),("atkThun",1),("atkDark",0),("atkSuperArmor",30f),("atkSuperArmorCorrection",100),("reviveDamage",30f),("reviveDamageCorrection",100),("ultimateChargeCorrection",100),("dmgLevel",wave?2:3),("dmgLevel_vsPlayer",wave?2:3),("selfTarget",leak?1:0),("opposeTarget",leak?0:1),("friendlyTarget",0)})Set(attack,f,v);
  }
  // Each discharge has one root and three native water-wave stages. Leakage
  // uses its own rows so allowing self contact never affects outgoing bolts.
  int[] source={10693002,10693006,10693007,10693008};
  for(int mode=0;mode<2;mode++)for(int i=0;i<source.Length;i++){
   int id=Owner+50+mode*4+i;
   var row=clone("Bullet",source[i],id);
   foreach(var(f,v)in new(string,object)[]{("atkId_Bullet",Owner+60+mode*2+(i==0?0:1)),("HitBulletID",i==3?-1:id+1),("launchConditionType",0),("isUseSharedHitList",1),("dmgHitRecordLifeTime",10f),("isHitBothTeam",mode)})Set(row,f,v);
  }
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
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,mmv?"work/static-charge/mmv":"work/static-charge/staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","Bullet","AtkParam_Pc"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"StaticCharge ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  Mechanics(Add);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Check(RewardCatalog.HistoryBit(Owner)==47,"History must append at bit 47");
  int marker=DoubleDip.FirstMarker+RewardCatalog.HistoryBit(Owner);
  var hidden=Add("SpEffectParam",8000000,marker);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(hidden,f,v);
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
   if(e.ID is 1111 or 9100941){Check(now.Instructions.Count(i=>Removal(i,Owner))==1&&now.Instructions.Count(i=>Removal(i,marker))==1,"Missing cleanup");Check(Markers.All(id=>now.Instructions.Count(i=>Removal(i,id))==1),"Missing charged-visual cleanup");now.Instructions.RemoveAll(i=>Removal(i,Owner)||Removal(i,marker)||Markers.Any(id=>Removal(i,id)));}
   if(e.ID==0)now.Instructions.RemoveAt(0);Check(Cages.Same(e,now),"Unrelated event changed");
  }
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=47,historyBytes=6,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: StaticCharge movement charge owner, append-only reward, Mystery Box instance and guarded cleanup staged");
 }

 public static void Rework(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes,bool mmv){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,"work/lightning-rework",mmv?"mmv":"staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","Bullet","AtkParam_Pc"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));
  var touched=names.ToDictionary(n=>n,n=>new HashSet<int>());
  Check(tables["SpEffectParam"].Rows.Any(r=>r.ID==Owner)&&tables["Bullet"].Rows.Any(r=>r.ID==Owner+53),"Missing static-charge baseline");
  Check(!tables["SpEffectParam"].Rows.Any(r=>r.ID==Owner+2)&&!tables["Bullet"].Rows.Any(r=>r.ID>=Owner+54&&r.ID<=Owner+57)&&!tables["AtkParam_Pc"].Rows.Any(r=>r.ID>=Owner+61&&r.ID<=Owner+63),"Rework IDs already occupied");
  PARAM.Row Replace(string n,int from,int id){
   Check(touched[n].Add(id),"Duplicate row edit");
   var existing=tables[n].Rows.SingleOrDefault(r=>r.ID==id);
   if(existing!=null)tables[n].Rows.Remove(existing);
   var row=new PARAM.Row(origins[n].Rows.Single(r=>r.ID==from)){ID=id};tables[n].Rows.Add(row);return row;
  }
  Mechanics(Replace);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){
   var expected=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Parameter roundtrip mismatch");
   var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);
   var unchanged=now.Rows.Where(r=>!touched[n].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();var prior=old.Rows.Where(r=>!touched[n].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();
   Check(prior.Length==unchanged.Length&&prior.Zip(unchanged).All(p=>Same(p.First,p.Second)),"Unrelated row changed");
   Check(now.Rows.Count==old.Rows.Count+touched[n].Count(id=>!old.Rows.Any(r=>r.ID==id)),"Unexpected row count");
  }
  string eventInput=Path.Combine(mod,"event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var previous=EMEVD.Read(eventInput);
  foreach(int id in new[]{1111,9100941}){var e=ev.Events.Single(e=>e.ID==id);int at=e.Instructions.FindIndex(i=>Removal(i,Owner+1));Check(at>=0&&!e.Instructions.Any(i=>Removal(i,Owner+2)),"Missing cleanup anchor or duplicate marker");e.Instructions.Insert(at+1,new(2004,21,new object[]{20000u,Owner+2}));}
  ev.Write(eventOutput);var final=EMEVD.Read(eventOutput);
  Check(final.Format==previous.Format&&final.Compression==previous.Compression&&final.Events.Count==previous.Events.Count&&final.StringData.SequenceEqual(previous.StringData)&&final.LinkedFileOffsets.SequenceEqual(previous.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in previous.Events){var now=final.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(now,ev.Events.Single(x=>x.ID==e.ID)),"Event roundtrip mismatch");if(e.ID is 1111 or 9100941){Check(now.Instructions.RemoveAll(i=>Removal(i,Owner+2))==1,"Missing marker cleanup");}Check(Cages.Same(e,now),"Unrelated event changed");}
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),touchedRows=touched.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),unrelatedRowsAndTablesUnchanged=true,onlyMarkerCleanupAdded=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Lightning Conduit rework staged; unrelated rows, events and texts preserved");
 }

 public static void MeleeStage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes,bool mmv){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod");
  string dir=Path.Combine(root,"work/lightning-melee",mmv?"mmv":"standalone");
  Directory.CreateDirectory(dir);
  string input=Path.Combine(mod,"regulation.bin"),output=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var file=binder.Files.Single(f=>Name(f)=="AtkParam_Pc.param");
  var table=parse("AtkParam_Pc",file.Bytes);
  var wave=table.Rows.Single(r=>r.ID==Owner+63);
  var original=new PARAM.Row(wave);
  Check(Convert.ToInt32(wave["selfTarget"].Value)==1 && Convert.ToInt32(wave["friendlyTarget"].Value)==0,"Unexpected leakage wave target mask");
  Set(wave,"opposeTarget",0);
  file.Bytes=table.Write();
  SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Files.Count==binder.Files.Count && actual.Version==binder.Version && actual.Compression==binder.Compression,"Regulation roundtrip mismatch");
  foreach(var f in actual.Files)Check(f.Bytes.Span.SequenceEqual(binder.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Binder member roundtrip mismatch");
  var verified=parse("AtkParam_Pc",actual.Files.Single(f=>f.ID==file.ID).Bytes);
  foreach(var id in new[]{Owner+62,Owner+63}){
   var attack=verified.Rows.Single(r=>r.ID==id);
   Check(Convert.ToInt32(attack["selfTarget"].Value)==1 && Convert.ToInt32(attack["friendlyTarget"].Value)==0 && Convert.ToInt32(attack["opposeTarget"].Value)==0,"Leakage must only target the owner");
  }
  Set(original,"opposeTarget",0);
  Check(Same(original,verified.Rows.Single(r=>r.ID==Owner+63)),"Unexpected leakage wave change");
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output)}));
 }

 public static void DelayStage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes,bool mmv){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,"work/lightning-delay2",mmv?"mmv":"staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Name(f)=="SpEffectParam.param");var table=parse("SpEffectParam",file.Bytes);
  var mark=table.Rows.Single(r=>r.ID==Owner+2);Check(Convert.ToSingle(mark["effectEndurance"].Value)==1f,"Expected one-second mark baseline");Set(mark,"effectEndurance",2f);file.Bytes=table.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){
   var expected=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Parameter roundtrip mismatch");
   if(f.ID!=file.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse("SpEffectParam",before[f.ID]);var now=parse("SpEffectParam",f.Bytes);Check(old.Rows.Count==now.Rows.Count,"Unexpected row count");
   foreach(var row in now.Rows){var previous=old.Rows.Single(r=>r.ID==row.ID);if(row.ID==Owner+2){Check(Convert.ToSingle(row["effectEndurance"].Value)==2f,"Mark duration mismatch");Set(row,"effectEndurance",1f);}Check(Same(previous,row),"Unrelated row or field changed");}
  }
  string eventInput=Path.Combine(mod,"event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");File.Copy(eventInput,eventOutput,true);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),changedRow=Owner+2,changedField="effectEndurance",beforeSeconds=1,afterSeconds=2,allOtherFieldsRowsTablesAndEventsUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: two-second mark staged; all other parameters and events unchanged");
 }
}
