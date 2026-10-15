using SoulsFormats;
using System.Text.Json;

static partial class RedEnvelope {
 public const int FirstReceipt=95000103,RewardReset=95000106;
 public static readonly (int Source,int Target)[] RewardLots={(6100000,95410000),(6110200,95410010)};
 public static void Rewards(Func<string,int,int,PARAM.Row> clone){
  foreach(int id in Enumerable.Range(FirstReceipt,4)){
   var r=clone("SpEffectParam",8000000,id);
   foreach(var (f,v) in new (string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
  }
  foreach(var (source,target) in RewardLots)for(int i=0;i<3;i++)clone("ItemLotParam_enemy",source+i,target+i);
 }
 public static void RewardEvents(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   int at=e.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==8&&BitConverter.ToInt32(i.ArgData,4)==RewardCycle.Reset);
   Check(at>=0&&!e.Instructions.Any(i=>i.Bank==2004&&i.ID==8&&BitConverter.ToInt32(i.ArgData,4)==RewardReset),"Red Envelope guarded reset missing/duplicated");
   var added=Enumerable.Range(FirstReceipt,4).Select(v=>new EMEVD.Instruction(2004,21,new object[]{20000u,v})).Append(new EMEVD.Instruction(2004,8,new object[]{20000u,RewardReset})).ToArray();
   e.Instructions.InsertRange(at,added);foreach(var p in e.Parameters)if(p.InstructionIndex>=at)p.InstructionIndex+=added.Length;
  }
 }
 static void VerifyRewards(BND4 native,BND4 current,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(BND4 b,string n)=>parse(n,b.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);
  var npcs=Table(native,"NpcParam");
  foreach(var (npc,lot) in new[]{(21000020,6100000),(42700020,6100000),(47701210,6110200),(49801020,6110200)})Check(Convert.ToInt32(npcs.Rows.Single(r=>r.ID==npc)["rewardItemLot_2"].Value)==lot,"Native outdoor boss binding changed");
  var original=Table(native,"ItemLotParam_enemy");var actual=Table(current,"ItemLotParam_enemy");
  foreach(var (source,target) in RewardLots)for(int i=0;i<3;i++){
   var copy=new PARAM.Row(actual.Rows.Single(r=>r.ID==target+i)){ID=source+i};
   Check(CageAI.Same(original.Rows.Single(r=>r.ID==source+i),copy),"Bonus lottery differs from native outdoor reward");
  }
  var effects=Table(current,"SpEffectParam");
  foreach(int id in Enumerable.Range(FirstReceipt,4)){
   var r=effects.Rows.Single(r=>r.ID==id);
   foreach(var (f,v) in new (string,object)[]{("maxHpRate",1f),("maxMpRate",1f),("stateInfo",0),("soul",0),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Check(Convert.ToDouble(r[f].Value)==Convert.ToDouble(v),$"Nonneutral receipt {id}/{f}");
  }
  Check(Convert.ToInt32(effects.Rows.Single(r=>r.ID==95000102)["soul"].Value)==88888,"Original rune payout changed");
 }
 public static void CheckRewards(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  VerifyRewards(native,SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin")),parse);
  var ev=EMEVD.Read(Path.Combine(root,"dist/mod/event/common.emevd.dcx"));
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   foreach(int effect in Enumerable.Range(FirstReceipt,4))Check(e.Instructions.Count(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==effect)==1,"Missing/duplicate reward reset removal");
   Check(e.Instructions.Count(i=>i.Bank==2004&&i.ID==8&&BitConverter.ToInt32(i.ArgData,4)==RewardReset)==1,"Missing reward reset notification");
  }
  Console.WriteLine("PASS: installed Red Envelope lotteries exactly match both native outdoor reward groups; neutral saved receipts, guarded reset and original 88888 rune payout verified");
 }
 public static void StageRewards(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/red-envelope-rewards/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(dir,"regulation.bin");
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","ItemLotParam_enemy"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string name,int from,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),"Reward ID collision");var r=new PARAM.Row(tables[name].Rows.Single(r=>r.ID==from)){ID=id};tables[name].Rows.Add(r);added[name].Add(id);return r;}
  Rewards(Add);
  foreach(string name in names){tables[name].Rows=tables[name].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes=tables[name].Write();}
  SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==b.Version&&final.Compression==b.Compression&&final.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in final.Files){var expected=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  foreach(string name in names){int file=b.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").ID;var old=parse(name,before[file]);var now=parse(name,final.Files.Single(f=>f.ID==file).Bytes);Check(now.Rows.Count==old.Rows.Count+added[name].Count,"Row count mismatch");foreach(var pair in old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[name].Contains(r.ID)).OrderBy(r=>r.ID)))Check(CageAI.Same(pair.First,pair.Second),"Existing row changed");}
  VerifyRewards(native,final,parse);
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");
  var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);RewardEvents(ev);ev.Write(eventOutput);var current=EMEVD.Read(eventOutput);
  Check(current.Format==oldEv.Format&&current.Compression==oldEv.Compression&&current.Events.Count==oldEv.Events.Count&&current.StringData.SequenceEqual(oldEv.StringData)&&current.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");
  Check(ev.Events.All(e=>Cages.Same(e,current.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
  var events=current.Events.Where(e=>e.ID is 1111 or 9100941).Select(e=>JsonSerializer.SerializeToElement(Cages.Dump(e))).ToArray();
  foreach(var e in oldEv.Events){var n=current.Events.Single(v=>v.ID==e.ID);if(e.ID is 1111 or 9100941){int at=n.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==FirstReceipt);Check(at>=0,"Reset block missing");n.Instructions.RemoveRange(at,5);foreach(var p in n.Parameters)if(p.InstructionIndex>=at+5)p.InstructionIndex-=5;}Check(Cages.Same(e,n),"Unrelated event changed");}
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(dir,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),events,previousEvents=oldEv.Events.Where(e=>e.ID is 1111 or 9100941).Select(Cages.Dump),allExistingRowsUnchanged=true,nativeOutdoorLotteriesPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: two native outdoor lotteries, persistent receipts, guarded resets and bilingual descriptions staged; all existing rows/events preserved");
 }
}
