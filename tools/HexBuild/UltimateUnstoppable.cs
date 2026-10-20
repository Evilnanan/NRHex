using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class UltimateUnstoppable {
 public const int Owner=95003900,Buff=Owner+1,Source=1650;
 static readonly string[] CutFields={"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"};
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var r=clone("SpEffectParam",Source,Buff);
  Set(r,"effectEndurance",15f);Set(r,"spEffectTextId_1",Buff);foreach(var f in CutFields)Set(r,f,1f);
 }
 static bool Removal(EMEVD.Instruction i,int effect)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==effect;
 public static void Events(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   Check(e.Parameters.Count==0,"Unexpected parameterized run cleanup");
   Check(!e.Instructions.Any(i=>Removal(i,Owner)||Removal(i,Buff)),"Unstoppable cleanup already present");
   int index=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(index>0&&e.Instructions.Take(index).Any(i=>i.Bank==3),"Guarded player-ready cleanup anchor missing");
   e.Instructions.InsertRange(index,new[]{Owner,Buff}.Select(effect=>new EMEVD.Instruction(2004,21,new object[]{20000u,effect})));
  }
 }
 static object Validate(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var origin=Table(native,"SpEffectParam");var actual=Table(current,"SpEffectParam");var source=origin.Rows.Single(r=>r.ID==Source);var buff=actual.Rows.Single(r=>r.ID==Buff);
  Check(CageAI.Same(source,actual.Rows.Single(r=>r.ID==Source)),"Original Endure changed");
  var expected=new PARAM.Row(source){ID=Buff};Set(expected,"effectEndurance",15f);Set(expected,"spEffectTextId_1",Buff);foreach(var f in CutFields)Set(expected,f,1f);
  Check(CageAI.Same(expected,buff),"Unstoppable differs from the reviewed native Endure clone");
  foreach(var f in CutFields)Check(Convert.ToSingle(buff[f].Value)==1f,"Unexpected damage reduction: "+f);
  Check(Convert.ToInt32(buff["spCategory"].Value)==1001&&Convert.ToInt32(buff["categoryPriority"].Value)==0&&Convert.ToInt32(buff["defFlickPower"].Value)==35,"Native Endure reaction metadata changed");
  Check(Convert.ToInt32(buff["saveCategory"].Value)==-1&&Convert.ToInt32(buff["vfxId"].Value)==8570&&Convert.ToInt32(buff["iconId"].Value)==20408,"Native transient/visual metadata mismatch");
  var owner=actual.Rows.Single(r=>r.ID==Owner);Check(Convert.ToInt32(owner["iconId"].Value)==22297&&Convert.ToInt32(owner["saveCategory"].Value)==10&&Convert.ToSingle(owner["effectEndurance"].Value)==-1f&&CutFields.All(f=>Convert.ToSingle(owner[f].Value)==1f),"Neutral permanent owner mismatch");
  Check(RewardCatalog.HistoryBit(Owner)==36&&RewardCatalog.HistoricalOwners.Length>=37,"History must append without reindexing");
  var table=Table(current,"ItemTableParam").Rows.Single(r=>r.ID==RewardCatalog.Table(Owner));Check(Convert.ToInt32(table["itemId"].Value)==Owner&&Convert.ToInt32(table["itemCategory"].Value)==1&&Convert.ToInt32(table["chanceWeight"].Value)==1000,"Reward table mismatch");
  var good=Table(current,"EquipParamGoods").Rows.Single(r=>r.ID==Owner);Check(Convert.ToInt32(good["goodsType"].Value)==11&&Convert.ToInt32(good["vagrantItemLotId"].Value)==Owner&&Convert.ToInt32(good["rarity"].Value)==3&&Convert.ToInt32(good["iconId"].Value)==20297,"Native ability grant mismatch");
  var attach=Table(current,"AttachEffectParam").Rows.Single(r=>r.ID==Owner);var perm=Table(current,"PermanentBuffParam").Rows.Single(r=>r.ID==Owner);
  Check(Convert.ToInt32(attach["statusIconId"].Value)==22297&&Convert.ToInt32(attach["passiveSpEffectId_1"].Value)==Owner&&Convert.ToInt32(perm["spEffectId"].Value)==Owner&&Convert.ToInt32(perm["iconId"].Value)==21297,"Ability effect/icon wiring mismatch");
  var visual=Table(current,"SpEffectVfxParam").Rows.Single(r=>r.ID==8570);Check(CageAI.Same(visual,Table(native,"SpEffectVfxParam").Rows.Single(r=>r.ID==8570))&&Convert.ToInt32(visual["midstSfxId"].Value)==440111,"Native Endure visual changed");
  return new{owner=Owner,buff=Buff,source=Source,seconds=15,historyBit=36,damageRates=CutFields.ToDictionary(f=>f,f=>buff[f].Value),nativeVfx=8570,nativeSfx=440111};
 }
 static void ValidateEvents(EMEVD ev){foreach(int id in new[]{1111,9100941})foreach(int effect in new[]{Owner,Buff})Check(ev.Events.Single(e=>e.ID==id).Instructions.Count(i=>Removal(i,effect))==1,"Missing/duplicate Unstoppable reset");}
 static void Fixture(string directory,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  var sp=parse("SpEffectParam",current.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes);sp.Rows=sp.Rows.Where(r=>new[]{Owner,Buff,Source,UltimateAwakening.Owner,UltimateAwakening.Buff,SwordPoise.Effect,SwordPoise.Source}.Contains(r.ID)).ToList();Directory.CreateDirectory(directory);File.WriteAllBytes(Path.Combine(directory,"installed-effects.param"),sp.Write());
 }
 public static void Verify(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));Validate(native,current,parse);Fixture(Path.Combine(root,"work/ultimate-unstoppable"),current,parse);
  string path=Path.Combine(root,"dist/mod/event/common.emevd.dcx");
  var raw=Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_RAW");EMEVD ev;
  if(!OperatingSystem.IsWindows()&&raw!=null){Check(Hash(path).Equals(Environment.GetEnvironmentVariable("NRHEX_HEARTSTEALER_COMMON_SHA256"),StringComparison.OrdinalIgnoreCase),"Common-event raw fixture does not match current package");ev=EMEVD.Read(raw);}else ev=EMEVD.Read(path);
  ValidateEvents(ev);Console.WriteLine("PASS: installed Ultimate Unstoppable native Endure reactions without damage reduction, reward wiring, VFX and guarded run resets");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,"work/ultimate-unstoppable/staged");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var originals=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Unstoppable ID collision: {name}/{id}");var r=new PARAM.Row(originals[name].Rows.First(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
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
  foreach(var old in oldEv.Events){var actual=actualEv.Events.Single(e=>e.ID==old.ID);Check(Cages.Same(ev.Events.Single(e=>e.ID==old.ID),actual),"Event readback mismatch");if(old.ID is 1111 or 9100941)actual.Instructions.RemoveAll(i=>Removal(i,Owner)||Removal(i,Buff));Check(Cages.Same(old,actual),"Unrelated event instructions/parameters changed");}
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,addedRows=added.ToDictionary(p=>p.Key,p=>p.Value.Order().ToArray()),rowCounts=tables.ToDictionary(p=>p.Key,p=>p.Value.Rows.Count),audit,existingRowsAndUnrelatedTablesUnchanged=true,onlyTwoGuardedCleanupEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Ultimate Unstoppable native Endure clone, append-only reward and two guarded resets staged");
 }
}
