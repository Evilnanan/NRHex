using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class Metal {
 public const int Owner=95004600,Good=3520,Lot=95546000,GrantEvent=9101120;
 static readonly int[] Markers={Owner+1,Owner+2};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,PARAM lots){
  foreach(int id in Markers){var row=clone("SpEffectParam",8000000,id);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",id==Owner+2?10:-1),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(row,f,v);}
  var template=lots.Rows.First(r=>Convert.ToInt32(r["lotItemCategory01"].Value)==1);
  var lot=clone("ItemLotParam_map",template.ID,Lot);
  for(int n=1;n<=8;n++){Set(lot,$"lotItemId{n:00}",-1);Set(lot,$"lotItemCategory{n:00}",0);Set(lot,$"lotItemBasePoint{n:00}",0);Set(lot,$"lotItemNum{n:00}",0);}
  foreach(var(f,v)in new[]{("lotItemId01",Good),("lotItemCategory01",1),("lotItemBasePoint01",1000),("lotItemNum01",1),("getItemFlagId",0),("cumulateNumFlagId",0),("cumulateNumMax",0)})Set(lot,f,v);
 }
 static EMEVD.Instruction I(int b,int id,params object[] args)=>new(b,id,args);
 static EMEVD.Instruction Has(int id)=>I(4,5,(sbyte)1,20000u,id,(byte)1,(sbyte)0,1f);
 static bool Removal(EMEVD.Instruction i,int effect)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==effect;
 public static void Events(EMEVD events){
  Check(!events.Events.Any(e=>e.ID==GrantEvent),"Metal grant event collision");
  var parent=events.Events.Single(e=>e.ID==0);Check(parent.Parameters.Count==0,"Parameterized common constructor");parent.Instructions.Insert(0,I(2000,0,0,GrantEvent,0));
  var grant=new EMEVD.Event(GrantEvent,EMEVD.Event.RestBehaviorType.Restart);
  grant.Instructions.AddRange([I(2000,2,(byte)0),Has(Owner),Has(Owner+1),I(0,0,(sbyte)0,(byte)1,(sbyte)1),I(2004,21,20000u,Owner+1),I(2003,4,Lot),I(1000,4,(byte)1)]);events.Events.Add(grant);
  foreach(int id in new[]{1111,9100941}){
   var e=events.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0,"Parameterized lifecycle");
   int at=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;Check(at>0&&e.Instructions.Take(at).Any(i=>i.Bank==3),"Missing guarded lifecycle anchor");
   e.Instructions.InsertRange(at,Markers.Prepend(Owner).Select(id=>I(2004,21,20000u,id)));
  }
 }
 static string Common(string root){
  var input=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");
  if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(input).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Raw common does not match installed package");return raw;}
  return input;
 }
 static void Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var goods=Table(current,"EquipParamGoods");var original=Table(native,"EquipParamGoods").Rows.Single(r=>r.ID==Good);
  Check(CageAI.Same(original,goods.Rows.Single(r=>r.ID==Good)),"Shared Ironjar Aromatic row changed");
  Check(Convert.ToInt32(original["isConsume"].Value)==1&&Convert.ToInt32(original["consumeMP"].Value)==0&&Convert.ToInt32(original["maxNum"].Value)==2&&Convert.ToInt32(original["goodsUseAnim"].Value)==93&&Convert.ToInt32(original["refId_default"].Value)==10352000,"Unexpected native Ironjar definition");
  foreach(string n in new[]{"Bullet","SpEffectParam"}){
   var source=Table(native,n);var actual=Table(current,n);
   foreach(int id in n=="Bullet"?new[]{10352000}:new[]{503520})Check(CageAI.Same(source.Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"Shared Ironjar effect changed");
  }
  var h=hexes.Single(h=>h.Id==Owner);var iconReference=hexes.Single(h=>h.Id==95000200);Check(h.LargeIcon==iconReference.LargeIcon&&h.SmallIcon==iconReference.SmallIcon,"Metal must share Celestial Body icons");
  foreach(var(n,f,v)in new[]{("SpEffectParam","iconId",h.SmallIcon),("AttachEffectParam","statusIconId",h.SmallIcon),("PermanentBuffParam","iconId",h.LargeIcon),("EquipParamGoods","iconId",h.LargeIcon-1000)})Check(Convert.ToInt32(Table(current,n).Rows.Single(r=>r.ID==iconReference.Id)[f].Value)==v,"Celestial Body icon reference differs: "+n);
  var s=Table(current,"SpEffectParam").Rows.Single(r=>r.ID==Owner);
  var expected=new PARAM.Row(Table(native,"SpEffectParam").Rows.Single(r=>r.ID==8000000)){ID=Owner};foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(expected,f,v);Check(CageAI.Same(expected,s),"Owner differs from neutral template");
  foreach(int id in Markers){var marker=Table(current,"SpEffectParam").Rows.Single(r=>r.ID==id);Check(Convert.ToInt32(marker["saveCategory"].Value)==(id==Owner+2?10:-1)&&Convert.ToInt32(marker["dontDeleteOnDead"].Value)==1&&Convert.ToInt32(marker["iconId"].Value)==-1,"Invalid gift marker");}
  var lot=Table(current,"ItemLotParam_map").Rows.Single(r=>r.ID==Lot);Check(Convert.ToInt32(lot["lotItemId01"].Value)==Good&&Convert.ToInt32(lot["lotItemNum01"].Value)==1&&Convert.ToInt32(lot["lotItemCategory01"].Value)==1,"Grant must contain one native Ironjar");
  var reward=goods.Rows.Single(r=>r.ID==Owner);var attach=Table(current,"AttachEffectParam").Rows.Single(r=>r.ID==Owner);var permanent=Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==Owner);var table=Table(current,"ItemTableParam").Rows.Single(r=>r.ID==RewardCatalog.Table(Owner));
  Check(Convert.ToInt32(reward["goodsType"].Value)==11&&Convert.ToInt32(reward["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(reward["iconId"].Value)==h.LargeIcon-1000&&Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(attach["statusIconId"].Value)==h.SmallIcon&&Convert.ToInt32(permanent["spEffectId"].Value)==Owner&&Convert.ToInt32(permanent["iconId"].Value)==h.LargeIcon&&Convert.ToInt32(table["itemId"].Value)==Owner&&Convert.ToInt32(table["chanceWeight"].Value)==1000,"Reward wiring mismatch");
  Check(RewardCatalog.HistoryBit(Owner)==43&&RewardCatalog.HistoricalOwners[42]==95004500,"History registry must only append");
 }
 static void ValidateEvents(EMEVD events){
  var grant=events.Events.Single(e=>e.ID==GrantEvent);Check(grant.RestBehavior==EMEVD.Event.RestBehaviorType.Restart&&grant.Parameters.Count==0&&grant.Instructions.Count==7,"Unexpected grant event");
  var instructions=new[]{I(2000,2,(byte)0),Has(Owner),Has(Owner+1),I(0,0,(sbyte)0,(byte)1,(sbyte)1),I(2004,21,20000u,Owner+1),I(2003,4,Lot),I(1000,4,(byte)1)};
  Check(grant.Instructions.Zip(instructions).All(p=>p.First.Bank==p.Second.Bank&&p.First.ID==p.Second.ID&&p.First.Layer==p.Second.Layer&&p.First.ArgData.SequenceEqual(p.Second.ArgData)),"Grant instruction mismatch");
  foreach(int id in new[]{1111,9100941})foreach(int effect in Markers.Prepend(Owner))Check(events.Events.Single(e=>e.ID==id).Instructions.Count(i=>Removal(i,effect))==1,"Missing guarded lifecycle cleanup");
  Check(events.Events.Single(e=>e.ID==0).Instructions.Count(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length==12&&BitConverter.ToInt32(i.ArgData,4)==GrantEvent)==1,"Grant event not initialized once");
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));Validate(native,current,parse,HexCatalog.Load(root));ValidateEvents(EMEVD.Read(Common(root)));
  string dir=Path.Combine(root,"work/metal");Directory.CreateDirectory(dir);var goods=parse("EquipParamGoods",current.Files.Single(f=>Name(f)=="EquipParamGoods.param").Bytes);goods.Rows=goods.Rows.Where(r=>r.ID is Good or 3500 or 3550).ToList();File.WriteAllBytes(Path.Combine(dir,"installed-goods.param"),goods.Write());
  Console.WriteLine("PASS: Metal original Ironjar/effects, neutral owner, reward wiring, gift lot and guarded lifecycle read back");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,"work/metal/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_map"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"Metal ID collision: {n}/{id}");var row=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(row);added[n].Add(id);return row;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Mechanics(Add,origins["ItemLotParam_map"]);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM row capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var file in actual.Files){var intended=b.Files.Single(f=>f.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Packed parameter readback failed");var n=names.SingleOrDefault(n=>Name(file)==n+".param");if(n==null){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}var old=parse(n,before[file.ID]);var now=parse(n,file.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count&&old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Existing parameter row changed");}Validate(native,actual,parse,hexes);
  var eventInput=Common(root);var ev=EMEVD.Read(eventInput);var previous=EMEVD.Read(eventInput);Events(ev);string eventOutput=Path.Combine(dir,"common.emevd.dcx");ev.Write(eventOutput);var final=EMEVD.Read(eventOutput);ValidateEvents(final);
  Check(final.Format==previous.Format&&final.Compression==previous.Compression&&final.Events.Count==previous.Events.Count+1&&final.StringData.SequenceEqual(previous.StringData)&&final.LinkedFileOffsets.SequenceEqual(previous.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in previous.Events){var now=final.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(now,ev.Events.Single(x=>x.ID==e.ID)),"Event roundtrip failed");if(e.ID is 1111 or 9100941)now.Instructions.RemoveAll(i=>Markers.Prepend(Owner).Any(id=>Removal(i,id)));if(e.ID==0)now.Instructions.RemoveAll(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length==12&&BitConverter.ToInt32(i.ArgData,4)==GrantEvent);Check(Cages.Same(e,now),"Unrelated event changed");}
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{version="0.17.251",inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=43,historyBytes=6,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Order().ToArray()),existingRowsAndUnrelatedTablesUnchanged=true,originalIronjarUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Metal original-item gift, append-only rewards and lifecycle staged; all existing parameters preserved");
 }
}
