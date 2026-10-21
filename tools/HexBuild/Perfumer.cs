using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class Perfumer {
 public const int Owner=95004100;
 static readonly (int Source,int Target)[] Goods={(3520,Owner+20),(3500,Owner+21),(3550,Owner+22)};
 static readonly int[] Cleanup={Owner};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){foreach(var e in Goods){var r=clone("EquipParamGoods",e.Source,e.Target);Set(r,"isConsume",0);}}
 static bool Removal(EMEVD.Instruction i,int effect)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==effect;
 public static void Events(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   Check(e.Parameters.Count==0,"Unexpected parameterized run cleanup");
   Check(!e.Instructions.Any(i=>Cleanup.Any(id=>Removal(i,id))),"Perfumer cleanup already present");
   int index=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(index>0&&e.Instructions.Take(index).Any(i=>i.Bank==3),"Guarded player-ready cleanup anchor missing");
   e.Instructions.InsertRange(index,Cleanup.Select(effect=>new EMEVD.Instruction(2004,21,new object[]{20000u,effect})));
  }
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var origin=Table(native,"SpEffectParam");var actual=Table(current,"SpEffectParam");
  foreach(var e in Goods){var source=Table(native,"EquipParamGoods").Rows.Single(r=>r.ID==e.Source);var expected=new PARAM.Row(source){ID=e.Target};Set(expected,"isConsume",0);Check(CageAI.Same(expected,Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==e.Target)),"Private non-consuming aromatic mismatch");Check(CageAI.Same(source,Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==e.Source)),"Native aromatic good changed");}
  foreach(var id in new[]{503500,503501,503520,503521,503522,503523,503529,503550,503559,708690,708691,708695,708710,708720,708725})Check(CageAI.Same(origin.Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"Native aromatic buff changed");
  var owner=actual.Rows.Single(r=>r.ID==Owner);Check(Convert.ToInt32(owner["iconId"].Value)==22229&&Convert.ToInt32(owner["saveCategory"].Value)==10&&Convert.ToSingle(owner["effectEndurance"].Value)==-1f,"Neutral permanent owner mismatch");
  Check(RewardCatalog.HistoryBit(Owner)==38&&RewardCatalog.HistoricalOwners.Length==39,"History must append without reindexing");
  var table=Table(current,"ItemTableParam").Rows.Single(r=>r.ID==RewardCatalog.Table(Owner));Check(Convert.ToInt32(table["itemId"].Value)==Owner&&Convert.ToInt32(table["itemCategory"].Value)==1&&Convert.ToInt32(table["chanceWeight"].Value)==1000,"Reward table mismatch");
  var good=Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==Owner);Check(Convert.ToInt32(good["goodsType"].Value)==11&&Convert.ToInt32(good["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(good["rarity"].Value)==3&&Convert.ToInt32(good["iconId"].Value)==20229,"Native ability grant mismatch");
  var attach=Table(current,"AttachEffectParam").Rows.Single(r=>r.ID==Owner);var perm=Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==Owner);
  Check(Convert.ToInt32(attach["statusIconId"].Value)==22229&&Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(perm["spEffectId"].Value)==Owner&&Convert.ToInt32(perm["iconId"].Value)==21229,"Ability effect/icon wiring mismatch");
  return new{owner=Owner,goods=Goods.Select(e=>new{source=e.Source,target=e.Target}),intervalSeconds=30,nativeBuffDurationsPreserved=true,historyBit=38,weights=new[]{1,1,1},noConsecutiveRepeat=false,noConsecutiveIronjar=true,baseNativeAromatics=true};
 }
 static void ValidateEvents(EMEVD ev){foreach(int id in new[]{1111,9100941})foreach(int effect in Cleanup)Check(ev.Events.Single(e=>e.ID==id).Instructions.Count(i=>Removal(i,effect))==1,"Missing/duplicate Perfumer reset");}
 static void Fixture(string directory,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  Directory.CreateDirectory(directory);var goods=parse("EquipParamGoods",current.Files.Single(f=>Name(f)=="EquipParamGoods.param").Bytes);goods.Rows=goods.Rows.Where(r=>Goods.Any(e=>r.ID==e.Source||r.ID==e.Target)).ToList();File.WriteAllBytes(Path.Combine(directory,"installed-goods.param"),goods.Write());
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));var audit=Validate(native,current,parse);Fixture(Path.Combine(root,"work/perfumer"),current,parse);File.WriteAllText(Path.Combine(root,"work/perfumer/installed-validation.json"),JsonSerializer.Serialize(audit,new JsonSerializerOptions{WriteIndented=true}));
  string path=Path.Combine(root,"dist/mod/event/common.emevd.dcx");
  var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");EMEVD ev;
  if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(path).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Common-event raw fixture does not match current package");ev=EMEVD.Read(raw);}else ev=EMEVD.Read(path);
  ValidateEvents(ev);Console.WriteLine("PASS: installed Perfumer native-use goods, original buffs, rewards and guarded resets");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/perfumer/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Perfumer ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Mechanics(Add);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM row capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var expected=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count,"Row count mismatch");Check(old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(p=>CageAI.Same(p.First,p.Second)),"Existing parameter row changed");}
  var audit=Validate(native,final,parse);Fixture(dir,final,parse);RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);Events(ev);ev.Write(eventOutput);var actualEv=EMEVD.Read(eventOutput);ValidateEvents(actualEv);
  Check(actualEv.Format==oldEv.Format&&actualEv.Compression==oldEv.Compression&&actualEv.Events.Count==oldEv.Events.Count&&actualEv.StringData.SequenceEqual(oldEv.StringData)&&actualEv.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");
  foreach(var old in oldEv.Events){var actual=actualEv.Events.Single(e=>e.ID==old.ID);Check(Cages.Same(ev.Events.Single(e=>e.ID==old.ID),actual),"Event readback mismatch");if(old.ID is 1111 or 9100941)actual.Instructions.RemoveAll(i=>Cleanup.Any(id=>Removal(i,id)));Check(Cages.Same(old,actual),"Unrelated event instructions/parameters changed");}
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Order().ToArray()),rowCounts=tables.ToDictionary(p=>p.Key,p=>p.Value.Rows.Count),audit,existingRowsAndUnrelatedTablesUnchanged=true,onlyTwoGuardedCleanupEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Perfumer native-use goods, unchanged native buffs, append-only reward and guarded reset staged");
 }
}
