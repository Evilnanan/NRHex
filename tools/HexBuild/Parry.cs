using SoulsFormats;
using System.Text.Json;
static class Parry {
 public const int Owner=95002400;
 public const int GuardJudge=95002400;
 public const float ChainSeconds=5f/30f;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Support(Func<string,int,int,PARAM.Row> clone){
  Counter(clone);
  ParryFollowup.Support(clone);
  foreach(int id in new[]{Owner+1,Owner+2,Owner+3}){
   var row=clone("SpEffectParam",8000000,id);
   Set(row,"maxHpRate",1f);Set(row,"effectEndurance",id==Owner+1?ChainSeconds:.2f);
   Set(row,"saveCategory",-1);Set(row,"spCategory",20);Set(row,"iconId",-1);
   Set(row,"displayIconForPermamentEffect",0);Set(row,"spEffectTextId_1",-1);
   if(id==Owner+1)continue;
   Set(row,"vfxId",id);
   var fx=clone("SpEffectVfxParam",503701,id);
   Check(Convert.ToInt32(fx["initSfxId"].Value)==-1&&Convert.ToInt32(fx["midstSfxId"].Value)==-1,"Executor visual template changed");
   Set(fx,"materialParamId",-1);Set(fx,"initSfxId",462800);Set(fx,"initDmyId",id==Owner+2?20:21);
  }
 }
 static void Counter(Func<string,int,int,PARAM.Row> clone){
  // Curved Sword Talisman: native guard-counter attack category, all elements.
  var row=clone("SpEffectParam",322000,Owner+4);
  Check(Convert.ToInt32(row["magicSubCategoryChange1"].Value)==103,"Native guard-counter category changed");
  foreach(string f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"}){
   Check(Math.Abs(Convert.ToSingle(row[f].Value)-1.2f)<.00001f,"Native guard-counter multiplier changed");Set(row,f,2f);
  }
  Set(row,"spCategory",20);
 }
 public static void ProbeCounterCost(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  PARAM Table(string n)=>parse(n,original.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);
  var a=Table("AtkParam_Pc");var b=Table("BehaviorParam_PC");
  var ids=a.Rows.Where(r=>Enumerable.Range(1,5).Any(i=>Convert.ToInt32(r["subCategory"+i].Value)==103)).Select(r=>r.ID).ToHashSet();
  var counters=b.Rows.Where(r=>Convert.ToInt32(r["refType"].Value)==0&&ids.Contains(Convert.ToInt32(r["refId"].Value))).Select(r=>new{r.ID,judge=r["behaviorJudgeId"].Value,attack=r["refId"].Value,stamina=r["stamina"].Value}).ToArray();
  // Verify actual serialized field offsets, not an unversioned XML layout.
  var row=a.Rows[0];a.Rows=new(){row};row["hit0_Radius"].Value=BitConverter.Int32BitsToSingle(0x3f123456);
  var before=a.Write();var marker=BitConverter.GetBytes(0x3f123456);
  int start=Enumerable.Range(0,before.Length-4).Single(i=>before.AsSpan(i,4).SequenceEqual(marker));
  var offsets=new List<int>();foreach(int i in Enumerable.Range(1,5)){
   var cell=row["subCategory"+i];var old=cell.Value;cell.Value=(byte)(Convert.ToByte(old)^0xff);var after=a.Write();
   offsets.Add(Enumerable.Range(0,before.Length).Single(j=>before[j]!=after[j])-start);cell.Value=old;
  }
  Check(offsets.SequenceEqual(new[]{0x4e,0x4f,0x1c8,0x1c9,0x1ca}),"Counter attack categories moved");
  Check(ids.Count>0&&counters.Length>0,"No native counter behaviors found");
  string directory=Path.Combine(root,".work/parry-counter");Directory.CreateDirectory(directory);
  File.WriteAllText(Path.Combine(directory,"parameters.json"),JsonSerializer.Serialize(new{offsets,attackCount=ids.Count,counters,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: native counter categories / serialized offsets; {ids.Count} attacks and {counters.Length} behaviors");
 }
 public static void StageCounter(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/parry/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var table=parse("SpEffectParam",file.Bytes);var baseline=parse("SpEffectParam",file.Bytes);
  Counter((n,source,id)=>{Check(!table.Rows.Any(r=>r.ID==id),"Counter ID already exists");var row=new PARAM.Row(table.Rows.Single(r=>r.ID==source)){ID=id};table.Rows.Add(row);return row;});
  table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=table.Write();
  string output=Path.Combine(dir,"counter-regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var result=SFUtil.DecryptNightreignRegulation(output);
  foreach(var f in result.Files){Check(f.Bytes.Span.SequenceEqual(binder.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Counter readback differs");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var actual=parse("SpEffectParam",result.Files.Single(f=>f.ID==file.ID).Bytes);
  Check(actual.Rows.Count==baseline.Rows.Count+1,"Unexpected row count");
  foreach(var row in baseline.Rows)Check(CageAI.Same(row,actual.Rows.Single(r=>r.ID==row.ID)),"Preexisting row changed");
  var added=actual.Rows.Single(r=>r.ID==Owner+4);var native=baseline.Rows.Single(r=>r.ID==322000);
  string[] changed={"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate","spCategory"};
  foreach(var cell in added.Cells.Where(c=>!changed.Contains(c.Def.InternalName)))Check(Equals(cell.Value,native[cell.Def.InternalName].Value),"Native guard-counter filter/lifecycle changed");
  File.WriteAllText(Path.Combine(dir,"counter-parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),added=Owner+4,nativeTemplate=322000,attackCategory=103,multiplier=2f,originalRowsUnchanged=true,nativeFiltersUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native category-103 counter multiplier x2 staged/read back; all existing parameters preserved");
 }
 public static void StageSupport(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/parry/staged");Directory.CreateDirectory(dir);
  var b=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
  var a=TAE.Read(b.Files.Single(f=>Path.GetFileName(f.Name)=="a00.tae").Bytes).Animations.Single(a=>a.ID==19480);
  var timing=a.Events.Single(e=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false))==707000);
  Check(timing.StartTime==0&&Math.Abs(timing.EndTime-ChainSeconds)<.00001f,"Native follow-up timing changed");
  Check(a.Events.Any(e=>e.Type==96&&BitConverter.ToInt32(e.GetParameterBytes(false))==462800),"Native deflect spark changed");
  var common=BND4.Read(Path.Combine(root,".work/storm-vfx-01757/extracted/sfx/sfxbnd_commoneffects.ffxbnd.dcx"));
  Check(common.Files.Any(f=>Path.GetFileName(f.Name)=="f000462800.fxr"),"Deflect spark not in native common resources");
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","SpEffectVfxParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Clone(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"Support ID collision: {n} {id}");var row=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(row);added[n].Add(id);return row;}
  Support(Clone);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"guard-regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var result=SFUtil.DecryptNightreignRegulation(output);
  foreach(var f in result.Files){Check(f.Bytes.Span.SequenceEqual(binder.Files.Single(x=>x.ID==f.ID).Bytes.Span),"Support readback differs");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  foreach(string n in names){var actual=parse(n,result.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);foreach(var row in baseline[n].Rows)Check(CageAI.Same(row,actual.Rows.Single(r=>r.ID==row.ID)),"Preexisting parameter row changed");}
  File.WriteAllText(Path.Combine(dir,"guard-parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),chainSeconds=ChainSeconds,sfx=462800,nativeCommonResource=true,originalRowsUnchanged=true,added=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native deflect spark and 5/30-second continuation verified; 5 support rows staged/read back");
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string directory=Path.Combine(root,".work/parry/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Installed regulation version changed");
  string[] names={"SpEffectParam","SpEffectVfxParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy"};
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
  Support(Add);
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
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"catalog.json"),JsonSerializer.Serialize(hexes,new JsonSerializerOptions{WriteIndented=true}));
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(destination),abilityCount=hexes.Length,added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),originalRowsUnchanged=true,nativeGuard=true,poiseScaledWindow=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Parry attack guard parameters, stable rewards and bilingual texts staged/read back");
 }
 public static void BuildActions(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  const int first=14000,minTier=2,tiers=7,last=24000;
  int[] totalMilliseconds={67,100,133,167,200,233,267};
  var work=Path.Combine(root,".work/parry/staged");Directory.CreateDirectory(work);
  PARAM Table(string n)=>parse(n,original.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);
  Check(!Table("SpEffectParam").Rows.Any(r=>Convert.ToInt32(r["stateInfo"].Value)>=first&&Convert.ToInt32(r["stateInfo"].Value)<last),"Private state range collides with native state");
  var weapons=Table("EquipParamWeapon").Rows.Where(r=>Convert.ToSingle(r["saWeaponDamage"].Value)>0).OrderBy(r=>r.ID).ToArray();
  var attacks=Table("AtkParam_Pc").Rows.ToDictionary(r=>r.ID);
  var behaviors=Table("BehaviorParam_PC").Rows.Where(r=>Convert.ToInt32(r["refType"].Value)==0&&attacks.ContainsKey(Convert.ToInt32(r["refId"].Value))).OrderBy(r=>r.ID).ToArray();
  var validJudges=behaviors.Select(r=>Convert.ToInt32(r["behaviorJudgeId"].Value)).ToHashSet();
  string input=Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx");var binder=BND4.Read(input);
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var keyset=new SortedSet<(int Judge,byte Source,short Required,byte Extra)>();
  int alignmentOnlyChanges=0;
  bool SameEvent(TAE.Event a,TAE.Event b,bool firstEvent){
   if(a.Type!=b.Type||a.StartTime!=b.StartTime||a.EndTime!=b.EndTime||a.Unk04!=b.Unk04)return false;
   var x=a.GetParameterBytes(false);var y=b.GetParameterBytes(false);if(x.SequenceEqual(y))return true;
   // TAE's raw reader includes inter-record alignment in parameter bytes.
   // Moving from six to seven events per attack shifts the first record's
   // alignment. Preserve its ENTIRE old payload, allowing exactly eight
   // appended zero bytes there only; never remove or change any old byte.
   if(!firstEvent||y.Length!=x.Length+8||!x.SequenceEqual(y.Take(x.Length))||y.Skip(x.Length).Any(v=>v!=0))return false;
   alignmentOnlyChanges++;return true;
  }
  var parsed=binder.Files.Where(f=>f.Name.EndsWith(".tae")).ToDictionary(f=>f.ID,f=>TAE.Read(f.Bytes));
  bool Generated(TAE.Event e)=>e.Type==0&&e.GetParameterBytes(false).Length>=16&&BitConverter.ToInt16(e.GetParameterBytes(false),14)>=first&&BitConverter.ToInt16(e.GetParameterBytes(false),14)<last;
  bool Guard(TAE.Event e)=>e.Type==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==3;
  bool Weapon(TAE.Event e){if(e.Type!=1||e.EndTime<=e.StartTime||!float.IsFinite(e.StartTime)||!float.IsFinite(e.EndTime))return false;var b=e.GetParameterBytes(false);return b.Length>=16&&BitConverter.ToInt32(b) is >=0 and <=3&&b[13]<=2&&(validJudges.Contains(BitConverter.ToInt32(b,8))||validJudges.Contains(BitConverter.ToInt32(b,8)%1000));}
  (int,byte,short,byte) Key(TAE.Event e){var b=e.GetParameterBytes(false);return(BitConverter.ToInt32(b,8),b[13],BitConverter.ToInt16(b,14),(byte)(b.Length>16&&b[16]==1?1:0));}
  var guardTemplate=parsed.Values.SelectMany(t=>t.Animations).Where(a=>a.ID==30900).SelectMany(a=>a.Events).First(e=>Guard(e)&&BitConverter.ToInt32(e.GetParameterBytes(false),8)==460);
  Check(guardTemplate.Group!=null&&guardTemplate.Group!.GroupType==0,"Native guard event group changed");
  // Rebuilding replaces only our tagged events; native and earlier Mod events survive.
  foreach(var t in parsed.Values)foreach(var a in t.Animations){a.Events.RemoveAll(e=>Generated(e));if(a.Events.Any(Guard))continue;foreach(var e in a.Events.Where(Weapon)){var k=Key(e);Check(k.Item3<first||k.Item3>=last,"Attack condition collides with private range");keyset.Add(k);}}
  var keys=keyset.ToArray();Check(first+keys.Length*tiers<=last,"Too many guarded attack conditions");
  var indices=keys.Select((k,i)=>(k,i)).ToDictionary(v=>v.k,v=>v.i);int added=0,clips=0;
  foreach(var f in binder.Files.Where(f=>parsed.ContainsKey(f.ID))){var t=parsed[f.ID];foreach(var a in t.Animations){
   if(a.Events.Any(Guard))continue;var selected=a.Events.Where(Weapon).ToArray();if(selected.Length==0)continue;clips++;
   var group=a.EventGroups.FirstOrDefault(g=>g.GroupType==0);
   if(group==null){group=new TAE.EventGroup(guardTemplate.Group!.GroupType){GroupData=guardTemplate.Group!.GroupData};a.EventGroups.Add(group);}
   foreach(var e in selected)for(int index=0;index<tiers;index++){
    // Configure offsets in milliseconds, without frame-unit quantization.
    int totalMs=totalMilliseconds[index],beforeMs=(totalMs+1)/3,afterMs=totalMs-beforeMs;
    var b=guardTemplate.GetParameterBytes(false).ToArray();BitConverter.GetBytes(GuardJudge).CopyTo(b,8);BitConverter.GetBytes((short)(first+indices[Key(e)]*tiers+index)).CopyTo(b,14);
    a.Events.Add(new TAE.Event(Math.Max(0,e.StartTime-beforeMs/1000f),e.StartTime+afterMs/1000f,0,guardTemplate.Unk04,b,false){Group=group});added++;
   }
  }f.Bytes=t.Write();}
  string output=Path.Combine(work,"c0000.anibnd.dcx");binder.Write(output);var readback=BND4.Read(output);
  Check(readback.Files.Count==binder.Files.Count,"Animation binder member count changed");
  foreach(var f in readback.Files){var expected=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Animation binder readback mismatch");if(!parsed.ContainsKey(f.ID))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Non-TAE member changed");
   else{var old=TAE.Read(before[f.ID]);var now=TAE.Read(f.Bytes);Check(old.Animations.Count==now.Animations.Count,"Animation count changed");foreach(var pair in old.Animations.Zip(now.Animations)){var a=pair.First;var b=pair.Second;Check(a.ID==b.ID&&a.AnimFileName==b.AnimFileName,"Animation identity changed");var ae=a.Events.Where(e=>!Generated(e)).ToArray();var be=b.Events.Where(e=>!Generated(e)).ToArray();Check(ae.Length==be.Length&&ae.Zip(be).Select((v,i)=>SameEvent(v.First,v.Second,i==0)).All(v=>v),"Preexisting animation event changed");}}
  }
  // Independent readback: enumerate original attack onsets, not newly written
  // windows. Explicit millisecond splits catch duration-based and unit errors.
  int verifiedWindows=0;
  int[] pre={22,33,44,56,67,78,89},post={45,67,89,111,133,155,178};
  foreach(var f in readback.Files.Where(f=>parsed.ContainsKey(f.ID))){
   var old=TAE.Read(before[f.ID]);var current=TAE.Read(f.Bytes);
   foreach(var pair in old.Animations.Zip(current.Animations)){
    var originalEvents=pair.First.Events.Where(e=>!Generated(e)).ToArray();
    var expected=new List<(int State,float Start,float End)>();
    if(!originalEvents.Any(Guard))foreach(var e in originalEvents.Where(Weapon))for(int i=0;i<pre.Length;i++)
     expected.Add((first+indices[Key(e)]*tiers+i,Math.Max(0,e.StartTime-pre[i]/1000f),e.StartTime+post[i]/1000f));
    var actual=pair.Second.Events.Where(Generated).Select(e=>(State:(int)BitConverter.ToInt16(e.GetParameterBytes(false),14),Start:e.StartTime,End:e.EndTime));
    var want=expected.OrderBy(e=>e.State).ThenBy(e=>e.Start).ThenBy(e=>e.End).ToArray();
    var got=actual.OrderBy(e=>e.State).ThenBy(e=>e.Start).ThenBy(e=>e.End).ToArray();
    Check(want.Length==got.Length&&want.Zip(got).All(v=>v.First.State==v.Second.State&&Math.Abs(v.First.Start-v.Second.Start)<.00001f&&Math.Abs(v.First.End-v.Second.End)<.00001f),"Onset guard window readback mismatch");
    verifiedWindows+=got.Length;
   }
  }
  Check(verifiedWindows==added,"Not all guard windows verified");
  string Float(object v)=>Convert.ToSingle(v).ToString("R",System.Globalization.CultureInfo.InvariantCulture)+"f32";
  var text="// Generated from guarded original parameters and installed TAE; do not edit.\n"+
   $"pub const FIRST:u16={first};\npub const MIN_TIER:usize={minTier};\npub const TIERS:usize={tiers};\npub const CONDITIONS:&[(i32,u8,u16,u8)]=&[\n"+string.Join("\n",keys.Select(k=>$" ({k.Judge},{k.Source},{unchecked((ushort)k.Required)},{k.Extra}),"))+"\n];\n"+
   "pub const WEAPONS:&[(i32,f32)]=&[\n"+string.Join("\n",weapons.Select(r=>$" ({r.ID},{Float(r["saWeaponDamage"].Value)}),"))+"\n];\n"+
   "pub const BEHAVIORS:&[(i32,f32)]=&[\n"+string.Join("\n",behaviors.Select(r=>$" ({r.ID},{Float(attacks[Convert.ToInt32(r["refId"].Value)]["atkSuperArmorCorrection"].Value)}),"))+"\n];\n";
  File.WriteAllText(Path.Combine(work,"parry_data.rs"),text);
  File.WriteAllText(Path.Combine(work,"actions-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),clips,added,verifiedWindows,alignmentOnlyChanges,conditionCount=keys.Length,firstState=first,endState=first+keys.Length*tiers,minTier,tiers,timeUnit="milliseconds",totalMilliseconds,beforeMilliseconds=pre,afterMilliseconds=post,anchor="each weapon attack event start",tier="clamp(ceil(basePoise/2),2,8)",weaponRows=weapons.Length,behaviorRows=behaviors.Length,guardJudge=GuardJudge,nativeGuardJudge=460,guardEventType=0,guardFlag=3,onlyWeaponBehaviorEvents=true,originalGuardClipsUntouched=true,originalEventsPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {clips} attack clips, {added} conditional guard windows, {keys.Length} attack conditions; original events preserved");
 }

}
