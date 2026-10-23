using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class Wraith {
 public const int Owner=95004300;
 public static readonly int[] Originals={3080,3010,3011};
 public static int[] Helpers=>Enumerable.Range(1,9).Concat(Enumerable.Range(11,60)).Concat(Enumerable.Range(90,3)).Select(i=>Owner+i).ToArray();
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone,PARAM nativeLots){
  foreach(int id in Helpers){var r=clone("SpEffectParam",8000000,id);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",id>=Owner+11?10:-1),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);}
  var template=nativeLots.Rows.First(r=>Convert.ToInt32(r["lotItemCategory01"].Value)==1);
  for(int i=0;i<3;i++){
   var lot=clone("ItemLotParam_map",template.ID,95543000+i*10);
   for(int n=1;n<=8;n++){Set(lot,$"lotItemId{n:00}",-1);Set(lot,$"lotItemCategory{n:00}",0);Set(lot,$"lotItemBasePoint{n:00}",0);Set(lot,$"lotItemNum{n:00}",0);}
   foreach(var(f,v)in new[]{("lotItemId01",Originals[i]),("lotItemCategory01",1),("lotItemBasePoint01",1000),("lotItemNum01",1),("getItemFlagId",0),("cumulateNumFlagId",0),("cumulateNumMax",0)})Set(lot,f,v);
  }
 }
 static EMEVD.Instruction I(int b,int i,params object[] args)=>new(b,i,args);
 static EMEVD.Instruction Has(int group,int id)=>I(4,5,(sbyte)group,20000u,id,(byte)1,(sbyte)0,1f);
 public static void Events(EMEVD ev){
  var parent=ev.Events.Single(e=>e.ID==0);Check(parent.Parameters.Count==0,"Parameterized common constructor");
  for(int stage=0;stage<3;stage++){
   int id=9101100+stage;Check(!ev.Events.Any(e=>e.ID==id),"Wraith event collision");parent.Instructions.Insert(0,I(2000,0,0,id,0));
   var e=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
   e.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Owner),Has(1,Owner+1+stage),I(0,0,(sbyte)0,(byte)1,(sbyte)1),I(2004,21,20000u,Owner+1+stage),I(2003,4,95543000+stage*10),I(1000,4,(byte)1)]);ev.Events.Add(e);
  }
  for(int stage=0;stage<2;stage++){
   int id=9101103+stage;Check(!ev.Events.Any(e=>e.ID==id),"Wraith cleanup collision");parent.Instructions.Insert(0,I(2000,0,0,id,0));
   var e=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
   e.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Owner),Has(1,Owner+7+stage),I(0,0,(sbyte)0,(byte)1,(sbyte)1),I(2004,21,20000u,Owner+7+stage),I(2003,24,(byte)3,Originals[stage],1),I(1000,4,(byte)1)]);ev.Events.Add(e);
  }
  foreach(int id in new[]{1111,9100941}){var e=ev.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0,"Parameterized lifecycle");int at=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;Check(at>0,"Missing guarded reset");e.Instructions.InsertRange(at,Helpers.Prepend(Owner).Select(id=>I(2004,21,20000u,id)));}
 }
 public static void GoodsTexts(string input,string output){
  var b=BND4.Read(input);var before=BND4.Read(input);
  foreach(string name in new[]{"GoodsName.fmg","GoodsInfo.fmg","GoodsCaption.fmg"}){
   var file=b.Files.Single(f=>Name(f)==name);var fmg=FMG.Read(file.Bytes);
   if(fmg.Entries.RemoveAll(e=>e.ID>=Owner+80&&e.ID<=Owner+82)>0)file.Bytes=fmg.Write();
  }
  string target=output+".wraith";b.Write(target);var actual=BND4.Read(target);Check(actual.Compression==before.Compression&&actual.Version==before.Version&&actual.Files.Count==before.Files.Count,"Goods text binder changed");
  foreach(var file in actual.Files){var old=before.Files.Single(f=>f.ID==file.ID);Check(file.Name==old.Name&&file.Flags==old.Flags,"Goods member metadata changed");if(new[]{"GoodsName.fmg","GoodsInfo.fmg","GoodsCaption.fmg"}.Contains(Name(file))){var a=FMG.Read(file.Bytes);var o=FMG.Read(old.Bytes);var expected=o.Entries.Where(e=>e.ID<Owner+80||e.ID>Owner+82).ToArray();Check(a.Entries.Count==expected.Length&&expected.Zip(a.Entries).All(pair=>pair.First.ID==pair.Second.ID&&pair.First.Text==pair.Second.Text),"Existing goods texts changed");}else Check(file.Bytes.Span.SequenceEqual(old.Bytes.Span),"Unrelated text member changed");}
  File.Move(target,output,true);
 }
 static void Validate(BND4 b,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(string n)=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  var goods=Table("EquipParamGoods");var effects=Table("SpEffectParam");
  foreach(int id in Helpers){var r=effects.Rows.Single(r=>r.ID==id);Check(Convert.ToInt32(r["saveCategory"].Value)==(id>=Owner+11?10:-1)&&Convert.ToInt32(r["dontDeleteOnDead"].Value)==1&&Convert.ToInt32(r["iconId"].Value)==-1,"Invalid saved/request marker");}
  var lots=Table("ItemLotParam_map");
  for(int i=0;i<3;i++){Check(!goods.Rows.Any(r=>r.ID==Owner+80+i),"Private tool still present");var g=goods.Rows.Single(r=>r.ID==Originals[i]);Check(Convert.ToInt32(g["maxNum"].Value)==1&&Convert.ToInt32(g["maxRepositoryNum"].Value)==0&&Convert.ToInt32(g["consumeMP"].Value)>0&&Convert.ToInt32(g["isConsume"].Value)==0&&Convert.ToInt32(g["bagcraftCategory"].Value)==9&&Convert.ToInt32(g["fh_unknown_09c"].Value)==1,"Invalid native tool");var lot=lots.Rows.Single(r=>r.ID==95543000+i*10);Check(Convert.ToInt32(lot["lotItemId01"].Value)==Originals[i]&&Convert.ToInt32(lot["lotItemCategory01"].Value)==1&&Convert.ToInt32(lot["lotItemNum01"].Value)==1,"Grant must use native tool");}
  Check(RewardCatalog.HistoryBit(Owner)==40&&RewardCatalog.HistoricalOwners.Length>=41&&RewardCatalog.HistoricalOwners[38]==95004100&&RewardCatalog.HistoricalOwners[39]==95004200,"Append-only history mismatch");
  foreach(int id in RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length))Check(effects.Rows.Any(r=>r.ID==id),"Missing packed history byte");
 }
 public static void Verify(string root,Func<string,Memory<byte>,PARAM> parse,bool staged=false){
  var b=SFUtil.DecryptNightreignRegulation(Path.Combine(root,staged?"work/wraith-native/staged/regulation.bin":"dist/mod/regulation.bin"));Validate(b,parse);
  var goods=parse("EquipParamGoods",b.Files.Single(f=>Name(f)=="EquipParamGoods.param").Bytes);goods.Rows=goods.Rows.Where(r=>Originals.Contains(r.ID)||r.ID>=Owner+80&&r.ID<=Owner+82).ToList();var dir=Path.Combine(root,"work/wraith");Directory.CreateDirectory(dir);File.WriteAllBytes(Path.Combine(dir,"installed-goods.param"),goods.Write());
  Console.WriteLine("PASS: Wraith uses original tools and native FP costs; no duplicate goods; sixth packed history byte");
 }
 public static void StageNative(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  var dir=Path.Combine(root,"work/wraith-native/staged");Directory.CreateDirectory(dir);
  var input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var goodsFile=binder.Files.Single(f=>Name(f)=="EquipParamGoods.param");var goods=parse("EquipParamGoods",goodsFile.Bytes);
  foreach(int id in Originals){var original=parse("EquipParamGoods",native.Files.Single(f=>Name(f)=="EquipParamGoods.param").Bytes).Rows.Single(r=>r.ID==id);Check(CageAI.Same(goods.Rows.Single(r=>r.ID==id),original),"Native tool differs from game input");}
  Check(goods.Rows.RemoveAll(r=>r.ID>=Owner+80&&r.ID<=Owner+82)==3,"Expected three legacy private tools");goodsFile.Bytes=goods.Write();
  var lotsFile=binder.Files.Single(f=>Name(f)=="ItemLotParam_map.param");var lots=parse("ItemLotParam_map",lotsFile.Bytes);
  for(int stage=0;stage<3;stage++){var lot=lots.Rows.Single(r=>r.ID==95543000+stage*10);Check(Convert.ToInt32(lot["lotItemId01"].Value)==Owner+80+stage,"Unexpected previous tool grant");Set(lot,"lotItemId01",Originals[stage]);}lotsFile.Bytes=lots.Write();
  var output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Parameter binder metadata changed");
  foreach(var file in actual.Files){
   var intended=binder.Files.Single(f=>f.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter roundtrip failed");
   string name=Name(file);if(name!="EquipParamGoods.param"&&name!="ItemLotParam_map.param"){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated parameter table changed");continue;}
   string table=name.Replace(".param","");var previous=parse(table,before[file.ID]);var current=parse(table,file.Bytes);
   if(table=="EquipParamGoods")previous.Rows.RemoveAll(r=>r.ID>=Owner+80&&r.ID<=Owner+82);
   else for(int stage=0;stage<3;stage++)Set(previous.Rows.Single(r=>r.ID==95543000+stage*10),"lotItemId01",Originals[stage]);
   Check(previous.Rows.Count==current.Rows.Count&&previous.Rows.Zip(current.Rows).All(pair=>CageAI.Same(pair.First,pair.Second)),"Unrelated row or field changed");
  }
  Validate(actual,parse);
  var eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var events=EMEVD.Read(eventInput);var originalEvents=EMEVD.Read(eventInput);
  for(int stage=0;stage<2;stage++){
   var entry=events.Events.Single(e=>e.ID==9101103+stage);Check(entry.Instructions.Count==7&&entry.Parameters.Count==0,"Unexpected removal event structure");
   var instruction=entry.Instructions[5];var expected=I(2003,24,(byte)3,Owner+80+stage,1);
   Check(instruction.Bank==expected.Bank&&instruction.ID==expected.ID&&instruction.ArgData.SequenceEqual(expected.ArgData)&&instruction.Layer==expected.Layer,"Unexpected previous tool removal");
   entry.Instructions[5]=I(2003,24,(byte)3,Originals[stage],1);
  }
  var eventOutput=Path.Combine(dir,"common.emevd.dcx");events.Write(eventOutput);var readback=EMEVD.Read(eventOutput);
  Check(readback.Events.Count==originalEvents.Events.Count&&readback.Format==originalEvents.Format&&readback.Compression==originalEvents.Compression&&readback.StringData.SequenceEqual(originalEvents.StringData)&&readback.LinkedFileOffsets.SequenceEqual(originalEvents.LinkedFileOffsets),"Event metadata changed");
  foreach(var entry in readback.Events){Check(Cages.Same(entry,events.Events.Single(e=>e.ID==entry.ID)),"Event roundtrip failed");var original=originalEvents.Events.Single(e=>e.ID==entry.ID);if(entry.ID is 9101103 or 9101104)original.Instructions[5]=I(2003,24,(byte)3,Originals[(int)entry.ID-9101103],1);Check(Cages.Same(entry,original),"Unrelated event changed");}
  foreach(string language in new[]{"zhocn","engus"}){var textOutput=Path.Combine(dir,"msg",language,"item_dlc01.msgbnd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(textOutput)!);GoodsTexts(Path.Combine(root,"dist/mod/msg",language,"item_dlc01.msgbnd.dcx"),textOutput);}
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{owner=Owner,nativeGoods=Originals,removedPrivateGoods=Enumerable.Range(80,3).Select(id=>Owner+id).ToArray(),inputSha256=Hash(input),outputSha256=Hash(output),originalGoodsUnchanged=true,onlyThreeGrantIdsChanged=true,onlyTwoRemovalIdsChanged=true,privateGoodsTextsRemoved=true,unrelatedContentPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native tool grants/removals staged; three duplicate goods and their texts removed; unrelated content preserved");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var dir=Path.Combine(root,"work/wraith/staged");Directory.CreateDirectory(dir);var b=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_map"};var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));var orig=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),"Wraith parameter collision");var r=new PARAM.Row(orig[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Mechanics(Add,orig["ItemLotParam_map"]);
  var history=RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length).Where(id=>!tables["SpEffectParam"].Rows.Any(r=>r.ID==id)).ToArray();RewardCycle.Parameters((n,from,id)=>id==RewardCycle.Reset?tables[n].Rows.Single(r=>r.ID==id):Add(n,from,id),history);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM row capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  var output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var file in actual.Files){var intended=b.Files.Single(f=>f.ID==file.ID);Check(file.Name==intended.Name&&file.Flags==intended.Flags&&file.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter serialization mismatch");var n=names.SingleOrDefault(n=>Name(file)==n+".param");if(n==null){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}var previous=parse(n,before[file.ID]);var now=parse(n,file.Bytes);Check(now.Rows.Count==previous.Rows.Count+added[n].Count&&previous.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing rows changed");}Validate(actual,parse);
  var eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var ev=EMEVD.Read(eventInput);var old=EMEVD.Read(eventInput);Events(ev);var eventOutput=Path.Combine(dir,"common.emevd.dcx");ev.Write(eventOutput);var readback=EMEVD.Read(eventOutput);Check(readback.Format==old.Format&&readback.Compression==old.Compression&&readback.StringData.SequenceEqual(old.StringData)&&readback.LinkedFileOffsets.SequenceEqual(old.LinkedFileOffsets),"Event metadata changed");foreach(var e in old.Events){var now=readback.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(ev.Events.Single(x=>x.ID==e.ID),now),"Event roundtrip failed");if(e.ID is 1111 or 9100941)now.Instructions.RemoveAll(i=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&Helpers.Prepend(Owner).Contains(BitConverter.ToInt32(i.ArgData,4)));if(e.ID==0)now.Instructions.RemoveAll(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length>=8&&BitConverter.ToInt32(i.ArgData,4)is >=9101100 and <=9101104);Check(Cages.Same(e,now),"Unrelated event changed");}
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});foreach(string lang in new[]{"zhocn","engus"}){var file=Path.Combine(dir,"texts",lang,"item_dlc01.msgbnd.dcx");GoodsTexts(file,file);}
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{owner=Owner,historyBit=40,historyBytes=(RewardCatalog.HistoricalOwners.Length+7)/8,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),originalRowsAndUnrelatedTablesPreserved=true,originalEventsPreserved=true,nativeGoods=Originals,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("PASS: Wraith owner, original tools, native grant/removal events and append-only history staged");
 }
}
