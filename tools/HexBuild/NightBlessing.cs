using SoulsFormats;
using System.Text.Json;
static partial class NightBlessing{
 public const int Owner=95002500;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  for(int i=1;i<=11;i++){
   var r=clone("SpEffectParam",8000000,Owner+i);
   Set(r,"maxHpRate",1f);Set(r,"spCategory",20);Set(r,"iconId",-1);Set(r,"spEffectTextId_1",-1);Set(r,"displayIconForPermamentEffect",0);Set(r,"saveCategory",i==5?10:-1);
   Set(r,"effectEndurance",i<=2?15f:i==7?0f:-1f);
   if(i==5){Set(r,"dontDeleteOnDead",1);Set(r,"isDisableNetSync",1);}
   if(i<=2){Set(r,"changeHpRate",-4f);Set(r,"changeHpPoint",i==1?-30:-60);Set(r,"motionInterval",1f);Set(r,"bCurrHPIndependeMaxHP",0);Set(r,"replaceSpEffectId",Owner+7+i);Set(r,"isDisableNetSync",1);}
   if(i==6||i>=8)Set(r,"isDisableNetSync",1);
   if(i>=10){Set(r,"changeHpRate",2f);Set(r,"changeHpPoint",i==10?15:30);Set(r,"motionInterval",1f);Set(r,"bCurrHPIndependeMaxHP",0);}
   if(i==3){foreach(string f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(r,f,1.2f);foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(r,f,1);Set(r,"iconId",22206);Set(r,"spEffectTextId_1",Owner+3);}
   if(i==4){Set(r,"vfxId",1643100);Set(r,"vfxId1",1642000);}
  }
 }
 public static void ValidateNative(BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var sp=parse("SpEffectParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  foreach(int id in new[]{99810,99811}){var r=sp.Rows.Single(r=>r.ID==id);Check(Convert.ToSingle(r["changeHpRate"].Value)==2f&&Convert.ToInt32(r["changeHpPoint"].Value)==(id==99810?15:30)&&Convert.ToSingle(r["motionInterval"].Value)==1f&&Convert.ToInt32(r["bCurrHPIndependeMaxHP"].Value)==0,"Native rain base damage changed");}
  var v=sp.Rows.Single(r=>r.ID==7012201);Check(Convert.ToInt32(v["vfxId"].Value)==1643100&&Convert.ToInt32(v["vfxId1"].Value)==1642000,"Native low-HP regeneration visuals changed");
  var heal=sp.Rows.Single(r=>r.ID==100);Check(Convert.ToSingle(heal["changeHpRate"].Value)==-100f&&Convert.ToSingle(heal["changeMpRate"].Value)==-100f,"Grace command template no longer restores HP/FP");
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  ValidateNative(original,parse);
  string directory=Path.Combine(root,".work/night-blessing/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Installed regulation version changed");
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"ID collision: {n} {id}");var row=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(row);added[n].Add(id);return row;}
  var h=hexes.Single(h=>h.Id==Owner);
  var effect=Add("SpEffectParam",8000000,Owner);Set(effect,"maxHpRate",1f);Set(effect,"spCategory",20);Set(effect,"iconId",h.SmallIcon);Set(effect,"spEffectTextId_1",Owner);Set(effect,"displayIconForPermamentEffect",0);
  var attach=Add("AttachEffectParam",95002300,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"attachTextId",Owner);Set(attach,"overrideBaseEffectId",Owner);Set(attach,"statusIconId",h.SmallIcon);
  var permanent=Add("PermanentBuffParam",95002300,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002300,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002300),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  Mechanics(Add);
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(tables["ItemLotParam_enemy"].Rows.Any(r=>r.ID==id))Check(Convert.ToInt32(tables["ItemLotParam_enemy"].Rows.Single(r=>r.ID==id)["lotItemId01"].Value)==target,"Old ball changed");
   else{var row=Add("ItemLotParam_enemy",95200000+slot,id);Set(row,"lotItemId01",target);}
  }
  RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  var destination=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(destination,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(destination);
  foreach(var f in final.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated member changed");}
  foreach(string n in names){var actual=parse(n,final.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);Check(actual.Rows.Count==baseline[n].Rows.Count+added[n].Count,"Row count mismatch");var old=baseline[n].Rows.OrderBy(r=>r.ID).ToArray();var retained=actual.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();Check(old.Length==retained.Length&&old.Zip(retained).All(p=>CageAI.Same(p.First,p.Second)),"Preexisting row changed");}
  Assets(root);
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"catalog.json"),JsonSerializer.Serialize(hexes,new JsonSerializerOptions{WriteIndented=true}));
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(destination),abilityCount=hexes.Length,added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),originalRowsUnchanged=true,nativeRainAndHealing=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Night blessing parameters, stable rewards and bilingual texts staged/read back");
 }
 static bool SameEventData(TAE.Event a,TAE.Event b){
  var x=a.GetParameterBytes(false);var y=b.GetParameterBytes(false);if(x.SequenceEqual(y))return true;
  // TAE reads the aligned span up to the next parameter block. Rewriting the
  // two edited clips may add another eight zero padding bytes to these types.
  int shortSize=Math.Min(x.Length,y.Length),longSize=Math.Max(x.Length,y.Length);var shorter=x.Length==shortSize?x:y;var longer=x.Length==longSize?x:y;
  int minimum=a.Type==227?8:a.Type==0?24:int.MaxValue;
  return shortSize>=minimum&&(shortSize-minimum)%8==0&&(longSize-shortSize)%8==0&&shorter.SequenceEqual(longer.Take(shortSize))&&longer.Skip(shortSize).All(v=>v==0);
 }
 public static void Assets(string root){
  string dir=Path.Combine(root,".work/night-blessing/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx");
  var binder=BND4.Read(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="a00.tae");var tae=TAE.Read(member.Bytes);
  foreach(long id in new long[]{81400,81401}){
   var a=tae.Animations.Single(a=>a.ID==id);
   a.Events.RemoveAll(e=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false))==Owner+7);
  }
  member.Bytes=tae.Write();string animOut=Path.Combine(dir,"c0000.anibnd.dcx");binder.Write(animOut);
  var result=BND4.Read(animOut);foreach(var f in result.Files){Check(f.Bytes.Span.SequenceEqual(binder.Files.Single(x=>x.ID==f.ID).Bytes.Span),"TAE readback differs");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated TAE changed");}
  bool Marker(TAE.Event e)=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false))==Owner+7;
  var old=TAE.Read(before[member.ID]);var current=TAE.Read(result.Files.Single(f=>f.ID==member.ID).Bytes);Check(old.Animations.Count==current.Animations.Count,"Animation count changed");
  var differences=old.Animations.Zip(current.Animations).SelectMany(pair=>pair.First.Events.Where(e=>!Marker(e)).Zip(pair.Second.Events.Where(e=>!Marker(e))).Where(v=>!SameEventData(v.First,v.Second)).Select(v=>new{animation=pair.First.ID,type=v.First.Type,before=Convert.ToHexString(v.First.GetParameterBytes(false)),after=Convert.ToHexString(v.Second.GetParameterBytes(false))})).ToArray();
  File.WriteAllText(Path.Combine(dir,"animation-padding-diff.json"),JsonSerializer.Serialize(differences));
  foreach(var pair in old.Animations.Zip(current.Animations)){Check(pair.First.ID==pair.Second.ID,"Animation identity changed");var a=pair.First.Events.Where(e=>!Marker(e)).ToArray();var b=pair.Second.Events.Where(e=>!Marker(e)).ToArray();Check(a.Length==b.Length&&a.Zip(b).All(v=>v.First.Type==v.Second.Type&&v.First.StartTime==v.Second.StartTime&&v.First.EndTime==v.Second.EndTime&&SameEventData(v.First,v.Second)),$"Original animation event changed: {pair.First.ID}");Check(!pair.Second.Events.Any(Marker),"Retired gesture marker remains");}
  string rel="script/talk/m00_00_00_00.talkesdbnd.dcx";
  string installed=Path.Combine(root,"dist/mod",rel),talkInput=File.Exists(installed)?installed:Path.Combine(root,"assets/original",rel);
  var talk=BND4.Read(talkInput);var originalTalk=talk.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var t=talk.Files.Single(f=>Path.GetFileName(f.Name)=="t000001000.esd");var esd=ESD.Read(t.Bytes);
  var cmds=esd.StateGroups[2147483623][1].EntryCommands;
  byte[] Arg(int v)=>new byte[]{0x82}.Concat(BitConverter.GetBytes(v)).Append((byte)0xa1).ToArray();
  Check(cmds.Count is 12 or 13&&cmds.Take(12).Select(c=>c.CommandID).All(id=>id==62)&&cmds.Take(12).All(c=>c.CommandBank==1),"Grace restore helper changed");
  int[] expected={100,101,102,103,104,105,106,107,108,109,990000020,46284};
  for(int i=0;i<12;i++)Check(cmds[i].Arguments.Count==1&&cmds[i].Arguments[0].SequenceEqual(Arg(expected[i])),"Grace restore effect sequence changed");
  if(cmds.Count==13){Check(cmds[12].CommandBank==1&&cmds[12].CommandID==62&&cmds[12].Arguments.Single().SequenceEqual(Arg(Owner+6)),"Foreign command at grace patch site");cmds.RemoveAt(12);}
  string beforeEsd=JsonSerializer.Serialize(esd);
  var command=new ESD.CommandCall{CommandBank=1,CommandID=62};command.Arguments.Add(Arg(Owner+6));cmds.Add(command);
  t.Bytes=esd.Write();string talkOut=Path.Combine(dir,"m00_00_00_00.talkesdbnd.dcx");talk.Write(talkOut);
  var read=BND4.Read(talkOut);foreach(var f in read.Files){Check(f.Bytes.Span.SequenceEqual(talk.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Talk binder readback differs");if(f.ID!=t.ID)Check(f.Bytes.Span.SequenceEqual(originalTalk[f.ID]),"Unrelated talk script changed");}
  var readEsd=ESD.Read(read.Files.Single(f=>f.ID==t.ID).Bytes);var added=readEsd.StateGroups[2147483623][1].EntryCommands;Check(added.Last().CommandBank==1&&added.Last().CommandID==62&&added.Last().Arguments.Single().SequenceEqual(Arg(Owner+6)),"Grace pulse readback differs");added.RemoveAt(added.Count-1);Check(JsonSerializer.Serialize(readEsd)==beforeEsd,"Original grace logic changed");
  File.WriteAllText(Path.Combine(dir,"assets-validation.json"),JsonSerializer.Serialize(new{animationInputSha256=Hash(input),animationOutputSha256=Hash(animOut),retiredGestureClips=new[]{81400,81401},graceInput=talkInput,graceInputSha256=Hash(talkInput),graceOutputSha256=Hash(talkOut),graceInputInstalled=File.Exists(installed),originalAnimationEventsPreserved=true,originalGraceCommandsPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: retired gesture markers removed and grace restoration pulse retained; original behavior preserved");
 }
}
