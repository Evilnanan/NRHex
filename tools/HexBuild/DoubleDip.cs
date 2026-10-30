using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class DoubleDip {
 public const int Owner=95004700,Receipt=Owner+1,Ready=Owner+2,FirstMarker=96210000,GrantEvent=9101130;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static Hex[] Targets(Hex[] hexes)=>hexes.Where(h=>h.Id!=Owner).ToArray();
 static int Marker(Hex h)=>FirstMarker+RewardCatalog.HistoryBit(h.Id);
 static int[] Helpers(Hex[] hexes)=>Targets(hexes).Select(Marker).Append(Receipt).Append(Ready).ToArray();
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,Hex[] hexes){
  foreach(int id in Helpers(hexes)){var r=clone("SpEffectParam",8000000,id);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);}
 }
 static EMEVD.Instruction I(int b,int id,params object[] args)=>new(b,id,args);
 static EMEVD.Instruction Has(int group,int id,bool yes=true)=>I(4,5,(sbyte)group,20000u,id,(byte)(yes?1:0),(sbyte)0,1f);
 static bool Removal(EMEVD.Instruction i,int effect)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==effect;
 static EMEVD.Event Worker(bool legacyPopup=false){
  var e=new EMEVD.Event(GrantEvent,EMEVD.Event.RestBehaviorType.Restart);
  e.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Owner),Has(1,Ready),Has(1,Receipt,false),Has(1,RewardCycle.Reset,false),Has(1,0),Has(1,0,false),I(0,0,(sbyte)0,(byte)1,(sbyte)1),I(2004,8,20000u,0),I(2003,124,0),Has(0,0),I(1000,4,(byte)1)]);
  if(!legacyPopup)e.Instructions.RemoveAt(9);
  var bindings=legacyPopup?new[]{(5,8,0),(6,8,4),(8,4,4),(9,0,4),(10,8,4)}:new[]{(5,8,0),(6,8,4),(8,4,4),(9,8,4)};
  foreach(var(at,target,source)in bindings)e.Parameters.Add(new EMEVD.Parameter{InstructionIndex=at,TargetStartByte=target,SourceStartByte=source,ByteCount=4,UnkID=source/4+1});
  return e;
 }
 public static void Events(EMEVD ev,Hex[] hexes){
  Check(!ev.Events.Any(e=>e.ID==GrantEvent),"Double the Joy worker collision");var parent=ev.Events.Single(e=>e.ID==0);Check(parent.Parameters.Count==0,"Parameterized common constructor");
  var targets=Targets(hexes);for(int i=targets.Length-1;i>=0;i--)parent.Instructions.Insert(0,I(2000,0,i,GrantEvent,Marker(targets[i]),targets[i].Id));ev.Events.Add(Worker());
  foreach(int id in new[]{1111,9100941}){var e=ev.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0,"Parameterized lifecycle");int at=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;Check(at>0&&e.Instructions.Take(at).Any(i=>i.Bank==3),"Missing guarded lifecycle anchor");e.Instructions.InsertRange(at,Helpers(hexes).Prepend(Owner).Select(effect=>I(2004,21,20000u,effect)));}
 }
 static string Common(string root){var input=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(input).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Raw common does not match installed package");return raw;}return input;}
 static void ValidateEvents(EMEVD events,Hex[] hexes){
  var worker=events.Events.Single(e=>e.ID==GrantEvent);Check(Cages.Same(worker,Worker()),"Parameterized native grant/popup worker differs from expected instructions or bindings");
  var inits=events.Events.Single(e=>e.ID==0).Instructions.Where(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length>=8&&BitConverter.ToInt32(i.ArgData,4)==GrantEvent).ToArray();var targets=Targets(hexes);Check(inits.Length==targets.Length,"Worker initialization count mismatch");
  for(int slot=0;slot<targets.Length;slot++){
   var h=targets[slot];var init=inits.Single(i=>BitConverter.ToInt32(i.ArgData)==slot);Check(init.Layer==null&&init.ArgData.SequenceEqual(I(2000,0,slot,GrantEvent,Marker(h),h.Id).ArgData),"Invalid worker instance arguments");
   var args=init.ArgData.AsSpan(8).ToArray();var bound=Worker();foreach(var p in bound.Parameters)Array.Copy(args,p.SourceStartByte,bound.Instructions[(int)p.InstructionIndex].ArgData,p.TargetStartByte,p.ByteCount);
   Check(BitConverter.ToInt32(bound.Instructions[5].ArgData,8)==Marker(h)&&BitConverter.ToInt32(bound.Instructions[6].ArgData,8)==h.Id&&bound.Instructions[6].ArgData[12]==0&&BitConverter.ToInt32(bound.Instructions[8].ArgData,4)==h.Id&&BitConverter.ToInt32(bound.Instructions[9].ArgData,8)==h.Id,"Bound grant uses wrong marker or owner");
  }
  foreach(int id in new[]{1111,9100941})foreach(int effect in Helpers(hexes).Prepend(Owner))Check(events.Events.Single(e=>e.ID==id).Instructions.Count(i=>Removal(i,effect))==1,"Missing guarded reset removal");
 }
 public static void StagePopup(string root){
  var input=Common(root);var before=EMEVD.Read(input);var ev=EMEVD.Read(input);
  var old=ev.Events.Single(e=>e.ID==GrantEvent);
  Check(Cages.Same(old,Worker(true))||Cages.Same(old,Worker()),"Unknown direct-grant worker; refusing to replace");
  ev.Events[ev.Events.IndexOf(old)]=Worker();
  var dir=Path.Combine(root,"work/double-dip-popup/staged");Directory.CreateDirectory(dir);var output=Path.Combine(dir,"common.emevd.dcx");ev.Write(output);
  var final=EMEVD.Read(output);ValidateEvents(final,HexCatalog.Load(root));
  Check(final.Format==before.Format&&final.Compression==before.Compression&&final.Events.Count==before.Events.Count&&final.StringData.SequenceEqual(before.StringData)&&final.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in before.Events)Check(Cages.Same(final.Events.Single(x=>x.ID==e.ID),e.ID==GrantEvent?Worker():e),"Unrelated event changed");
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedEvent=GrantEvent,removedRecordOnlyOpcode="2003[124]",nativePopupQueuedByDllAfterOwnership=true,allUnrelatedEventsUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: direct-grant worker staged without record-only popup opcode; four bindings and all unrelated events verified");
 }
 static void Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var effects=Table(current,"SpEffectParam");var original=Table(native,"SpEffectParam").Rows.Single(r=>r.ID==8000000);var h=hexes.Single(h=>h.Id==Owner);
  var expected=new PARAM.Row(original){ID=Owner};foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(expected,f,v);Check(CageAI.Same(expected,effects.Rows.Single(r=>r.ID==Owner)),"Owner differs from neutral reward template");
  foreach(int id in Helpers(hexes)){var r=new PARAM.Row(original){ID=id};foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);Check(CageAI.Same(r,effects.Rows.Single(row=>row.ID==id)),"Invalid hidden persistent selection/receipt effect");}
  var good=Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==Owner);var attach=Table(current,"AttachEffectParam").Rows.Single(r=>r.ID==Owner);var permanent=Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==Owner);var table=Table(current,"ItemTableParam").Rows.Single(r=>r.ID==RewardCatalog.Table(Owner));
  Check(Convert.ToInt32(good["goodsType"].Value)==11&&Convert.ToInt32(good["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(good["iconId"].Value)==h.LargeIcon-1000&&Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(attach["statusIconId"].Value)==h.SmallIcon&&Convert.ToInt32(permanent["spEffectId"].Value)==Owner&&Convert.ToInt32(permanent["iconId"].Value)==h.LargeIcon&&Convert.ToInt32(table["itemId"].Value)==Owner&&Convert.ToInt32(table["chanceWeight"].Value)==1000,"Reward wiring mismatch");
  foreach(var target in Targets(hexes))Check(Convert.ToInt32(Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==target.Id)["spEffectId"].Value)==target.Id&&Convert.ToInt32(effects.Rows.Single(r=>r.ID==target.Id)["saveCategory"].Value)==10,"Direct gift target differs from actual native claim owner");
  Check(RewardCatalog.HistoryBit(Owner)==44&&RewardCatalog.HistoricalOwners[43]==95004600,"History registry must only append");
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,bool staged=false){var hexes=HexCatalog.Load(root);Validate(native,SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin")),parse,hexes);ValidateEvents(EMEVD.Read(staged?Path.Combine(root,"work/double-dip-popup/staged/common.emevd.dcx"):Common(root)),hexes);Console.WriteLine("PASS: Mystery Box neutral reward, stable hidden selection and parameterized direct-grant workers read back; popup belongs to DLL");}
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,"work/double-dip/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"Double the Joy ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);Mechanics(Add,hexes);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){var intended=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter roundtrip failed");var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count&&old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing parameter row changed");}Validate(native,actual,parse,hexes);
  string eventInput=Common(root);var ev=EMEVD.Read(eventInput);var previous=EMEVD.Read(eventInput);Events(ev,hexes);string eventOutput=Path.Combine(dir,"common.emevd.dcx");ev.Write(eventOutput);var final=EMEVD.Read(eventOutput);ValidateEvents(final,hexes);Check(final.Format==previous.Format&&final.Compression==previous.Compression&&final.Events.Count==previous.Events.Count+1&&final.StringData.SequenceEqual(previous.StringData)&&final.LinkedFileOffsets.SequenceEqual(previous.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in previous.Events){var now=final.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(now,ev.Events.Single(x=>x.ID==e.ID)),"Event roundtrip mismatch");if(e.ID is 1111 or 9100941)now.Instructions.RemoveAll(i=>Helpers(hexes).Prepend(Owner).Any(id=>Removal(i,id)));if(e.ID==0)now.Instructions.RemoveAll(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length>=8&&BitConverter.ToInt32(i.ArgData,4)==GrantEvent);Check(Cages.Same(e,now),"Unrelated event changed");}
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{version="0.17.253",inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=44,historyBytes=6,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),workerInstances=Targets(hexes).Length,parameterizedWorkerBindingsChecked=true,directOwnerEffectsMatchNativePermanentBuffClaims=true,noRewardNodesOrInventoryGrants=true,allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("PASS: Double the Joy direct grants, popup bindings and append-only catalog staged; original rows preserved");
 }
}
