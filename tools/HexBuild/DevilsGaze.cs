using SoulsFormats;
using System.Text.Json;

static class DevilsGaze {
 public const int Owner=95002200,FirstBullet=95002250;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row row,string field,object value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 static void HitPhases(PARAM.Row bullet,int part){
  // The original warning uses dummy NPC attack 1. For an environmental
  // all-team source it must not submit hits before the damaging child exists.
  // Native visual-only chains also use atkId_Bullet=-1 (e.g. 5210190).
  if(part<2)Set(bullet,"atkId_Bullet",-1);
  Set(bullet,"isHitBothTeam",part==2?1:0);
  // An explosion gets its own hit record, never a warning/other circle's record.
  Set(bullet,"isUseSharedHitList",0);
 }
 public static void Mechanics(Dictionary<string,PARAM> tables,Func<string,int,int,PARAM.Row> add){
  var native=tables["Bullet"].Rows.Single(r=>r.ID==7560366);
  Check(Convert.ToSingle(native["life"].Value)==3.95f&&Convert.ToInt32(native["HitBulletID"].Value)==7560367,"Libra slow circle changed");
  var main=tables["AtkParam_Npc"].Rows.Single(r=>r.ID==7560221);
  Check(Convert.ToInt32(main["atkDark"].Value)==300&&Convert.ToInt32(main["spAttributeMixcraft"].Value)==13,"Libra holy attack changed");
  // Retain the native raw attack. Base/depth/recipient multipliers are applied
  // to the per-hit input copy at runtime, before native mitigation.
  for(int depth=0;depth<=5;depth++){
   int start=FirstBullet+depth*10,attackId=95002200+depth;
   var attack=add("AtkParam_Npc",7560221,attackId);
   Set(attack,"friendlyTarget",1);Set(attack,"selfTarget",1);
   for(int part=0;part<3;part++){
    var bullet=add("Bullet",7560366+part,start+part);
    HitPhases(bullet,part);
    Set(bullet,"HitBulletID",part==2?7560369:start+part+1);
    if(part==2)Set(bullet,"atkId_Bullet",attackId);
   }
  }
 }
 public static void StageHitFix(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/devils-gaze-hitfix");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="Bullet.param");
  var table=parse("Bullet",member.Bytes);var baseline=parse("Bullet",member.Bytes);
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var changes=new List<object>();var rows=new List<object>();
  for(int depth=0;depth<=5;depth++)for(int part=0;part<3;part++){
   int id=FirstBullet+depth*10+part;var row=table.Rows.Single(r=>r.ID==id);var old=baseline.Rows.Single(r=>r.ID==id);
   Check(part==2?Convert.ToInt32(row["HitBulletID"].Value) is -1 or 7560369:Convert.ToInt32(row["HitBulletID"].Value)==id+1,"Unexpected circle chain");
   Check(Convert.ToInt32(row["atkId_Bullet"].Value)==(part==2?95002200+depth:1)||part<2&&Convert.ToInt32(row["atkId_Bullet"].Value)==-1,"Unexpected circle attack");
   HitPhases(row,part);
   foreach(string field in new[]{"atkId_Bullet","isHitBothTeam","isUseSharedHitList"}){
    var prev=old[field].Value;var next=row[field].Value;
    if(!Equals(prev,next))changes.Add(new{id,field,previous=prev,next});
    old[field].Value=next;
   }
   Check(CageAI.Same(row,old),"Unrelated circle field changed");
   rows.Add(new{row.ID,fields=row.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)});
  }
  member.Bytes=table.Write();string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Files.Count==binder.Files.Count,"Binder member count changed");
  foreach(var f in final.Files){var expected=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Readback mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");}
  var actual=parse("Bullet",final.Files.Single(f=>f.ID==member.ID).Bytes);
  Check(actual.Rows.Count==baseline.Rows.Count&&actual.Rows.Zip(baseline.Rows).All(v=>CageAI.Same(v.First,v.Second)),"Unrelated bullet changed");
  File.WriteAllText(Path.Combine(directory,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changes,rows,onlyPrivateBulletHitFieldsChanged=true,attackDamageMadnessDelayRadiusResourcesUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: warning phases have no attack, explosions use independent hits; all unrelated fields/tables preserved");
 }
 public static void Tune(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/devils-gaze-tuning");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);
  string[] names={"AtkParam_Npc"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()==n+".param").Bytes));
  var member=binder.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()=="AtkParam_Npc.param");
  var baseline=parse("AtkParam_Npc",member.Bytes);var changed=new List<object>();
  for(int depth=0;depth<=5;depth++){
   int id=95002200+depth;var row=tables["AtkParam_Npc"].Rows.Single(r=>r.ID==id);
   int old=Convert.ToInt32(row["atkDark"].Value),next=300;
   // Accept only the known prior package or an already normalized package.
   Check(old==new[]{252,328,390,464,640,743}[depth]||old==next,"Unexpected installed damage");
   Set(row,"atkDark",next);Set(baseline.Rows.Single(r=>r.ID==id),"atkDark",next);
   changed.Add(new{id,old,next});
  }
  member.Bytes=tables["AtkParam_Npc"].Write();
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Version==binder.Version&&final.Compression==binder.Compression&&final.Files.Count==binder.Files.Count,"Regulation container changed");
  foreach(var file in final.Files){var expected=binder.Files.Single(f=>f.ID==file.ID);Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Member readback mismatch");}
  var actual=parse("AtkParam_Npc",final.Files.Single(f=>f.ID==member.ID).Bytes);
  Check(actual.Rows.Count==baseline.Rows.Count&&actual.Rows.Zip(baseline.Rows).All(v=>CageAI.Same(v.First,v.Second)),"Unrelated attack field changed");
  File.WriteAllText(Path.Combine(directory,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),rawHolyAttack=300,changes=changed,onlySixHolyAttackFieldsChanged=true,bulletVisualRadiusMadnessUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: six Gaze rows retain raw 300 holy attack; all other attack fields, bullets and resources unchanged");
 }
 public static void VerifyRaw(string root,Func<string,Memory<byte>,PARAM> parse){
  var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var table=parse("AtkParam_Npc",binder.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()=="AtkParam_Npc.param").Bytes);
  Check(Convert.ToInt32(table.Rows.Single(r=>r.ID==7560221)["atkDark"].Value)==300,"Native Libra base attack changed");
  for(int depth=0;depth<=5;depth++)Check(Convert.ToInt32(table.Rows.Single(r=>r.ID==Owner+depth)["atkDark"].Value)==300,"Gaze raw base attack mismatch; install matching regulation and DLL");
  Console.WriteLine("PASS: installed Gaze attacks use raw 300 holy damage at all six stages");
 }
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string directory=Path.Combine(root,".work/devils-gaze/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Installed regulation version changed");
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy","Bullet","AtkParam_Npc","ChaosMatchingCorrectParam"};
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var baseline=names.ToDictionary(n=>n,n=>parse(n,before[binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int source,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"ID collision: {n} {id}");var row=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(row);added[n].Add(id);return row;}
  var h=hexes.Single(h=>h.Id==Owner);
  var effect=Add("SpEffectParam",8000000,Owner);Set(effect,"maxHpRate",1f);Set(effect,"spCategory",20);Set(effect,"iconId",h.SmallIcon);Set(effect,"spEffectTextId_1",Owner);Set(effect,"displayIconForPermamentEffect",0);
  var attach=Add("AttachEffectParam",95002100,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"attachTextId",Owner);Set(attach,"overrideBaseEffectId",Owner);Set(attach,"statusIconId",h.SmallIcon);
  var permanent=Add("PermanentBuffParam",95002100,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002100,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002100),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  Mechanics(tables,Add);
  foreach(var entry in RewardCatalog.Compatibility(hexes))for(int slot=0;slot<3;slot++){
   int id=entry.Lot+slot,target=RewardCatalog.Table(entry.Owners[slot]);
   if(id<95268400)Check(Convert.ToInt32(tables["ItemLotParam_enemy"].Rows.Single(r=>r.ID==id)["lotItemId01"].Value)==target,"Old ball changed");
   else{var row=Add("ItemLotParam_enemy",95200000+slot,id);Set(row,"lotItemId01",target);}
  }
  RewardCatalog.Write(Path.Combine(directory,"hex_pool.rs"),hexes);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes=tables[n].Write();}
  var destination=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(destination,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(destination);
  foreach(var f in final.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(!names.Contains(Path.GetFileNameWithoutExtension(f.Name)))Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated member changed");}
  foreach(string n in names){var actual=parse(n,final.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);Check(actual.Rows.Count==baseline[n].Rows.Count+added[n].Count,"Row count mismatch");var old=baseline[n].Rows.OrderBy(r=>r.ID).ToArray();var retained=actual.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID).ToArray();Check(old.Length==retained.Length&&old.Zip(retained).All(p=>CageAI.Same(p.First,p.Second)),"Preexisting row changed");}
  HexTexts.Stage(root,hexes,Path.Combine(directory,"texts"),allRewards:true);
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(destination),abilityCount=hexes.Length,added=added.ToDictionary(k=>k.Key,k=>k.Value.Order().ToArray()),originalRowsUnchanged=true,delaySeconds=4.45,holyDamage=tables["AtkParam_Npc"].Rows.Where(r=>r.ID>=95002200&&r.ID<=95002205).Select(r=>new{r.ID,damage=r["atkDark"].Value}),madnessEffectsUnchanged=true,nativePickupChild=7560369,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Demon gaze native slow circle/depth attacks, stable rewards and bilingual texts staged/read back");
 }
 public static void StageResources(string root){
  string source=Path.Combine(root,".work/devils-gaze/resources/sfxbnd_c7560.ffxbnd.dcx"),installed=Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");
  var common=BND4.Read(installed);var old=common.Files.ToArray();var boss=BND4.Read(source).Files.ToDictionary(f=>Path.GetFileName(f.Name),StringComparer.OrdinalIgnoreCase);
  var shared=BND4.Read(Path.Combine(root,".work/storm-vfx-01757/extracted/sfx/sfxbnd_commoneffects.ffxbnd.dcx")).Files.Select(f=>Path.GetFileName(f.Name)).ToHashSet(StringComparer.OrdinalIgnoreCase);
  var byName=common.Files.ToDictionary(f=>Path.GetFileName(f.Name),StringComparer.OrdinalIgnoreCase);var added=new List<BinderFile>();
  void Add(string name,int group){if(byName.ContainsKey(name))return;Check(boss.ContainsKey(name),"Missing dependency: "+name);var src=boss[name];int id=common.Files.Where(f=>f.ID>=group&&f.ID<group+100000).Select(f=>f.ID).DefaultIfEmpty(group-1).Max()+1;Check(id<group+100000,"Binder ID overflow");var f=new BinderFile(src.Flags,id,src.Name,src.Bytes.ToArray());common.Files.Add(f);byName.Add(name,f);added.Add(f);}
  foreach(int id in new[]{675685,675687,675683,675600,675601,675602,675603}){
   string stem=$"f{id:000000000}";Add(stem+".fxr",0);Add(stem+".ffxreslist",400000);
   foreach(var reference in System.Text.Encoding.UTF8.GetString(boss[stem+".ffxreslist"].Bytes.Span).Split(['\r','\n'],StringSplitOptions.RemoveEmptyEntries)){
    string r=reference.Trim(),ext=Path.GetExtension(r).ToLowerInvariant();Check(ext is ".tif" or ".sib","Unknown dependency type");string name=Path.GetFileNameWithoutExtension(r)+(ext==".tif"?".tpf":".flver");if(!shared.Contains(name))Add(name,ext==".tif"?100000:200000);
   }
  }
  common.Files.Sort((a,b)=>a.ID.CompareTo(b.ID));string destination=Path.Combine(root,".work/devils-gaze/staged/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(destination)!);common.Write(destination);
  var final=BND4.Read(destination);foreach(var f in old){var actual=final.Files.Single(v=>v.ID==f.ID);Check(actual.Name==f.Name&&actual.Flags==f.Flags&&actual.Bytes.Span.SequenceEqual(f.Bytes.Span),"Existing effect member changed");}
  File.WriteAllText(Path.Combine(root,".work/devils-gaze/staged/resources-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(installed),outputSha256=Hash(destination),added=added.Select(f=>new{f.Name,bytes=f.Bytes.Length}),outputBytes=new FileInfo(destination).Length,existingResourcesPreserved=true},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: compact Libra resources merged; {added.Count} members added, original {old.Length} retained");
 }
 public static void Resources(string root,string smith,string game){
  StormVfx.Extract(root,smith,game);
  var header=BHD5.Read(File.ReadAllBytes(Path.Combine(root,".work/storm-vfx-01757/data0.bhd")),BHD5.Game.EldenRing);
  ulong Hash(string name){ulong h=0;foreach(char c in name.ToLowerInvariant())h=unchecked(h*0x85+c);return h;}
  var entry=header.Buckets.SelectMany(b=>b).Single(e=>e.FileNameHash==Hash("/sfx/sfxbnd_c7560.ffxbnd.dcx"));
  using var bdt=File.OpenRead(Path.Combine(game,"data0.bdt"));
  string dir=Path.Combine(root,".work/devils-gaze/resources");Directory.CreateDirectory(dir);
  var bytes=entry.ReadFile(bdt);File.WriteAllBytes(Path.Combine(dir,"sfxbnd_c7560.ffxbnd.dcx"),bytes);
  var bnd=BND4.Read(bytes);
  File.WriteAllText(Path.Combine(dir,"members.json"),JsonSerializer.Serialize(bnd.Files.Select(f=>new{f.ID,f.Name,size=f.Bytes.Length})));
  foreach(int id in new[]{675578,675579,675600,675683,675685,675686,675687,675688}){
   var stem=$"f{id:000000000}";
   foreach(var file in bnd.Files.Where(f=>Path.GetFileName(f.Name).StartsWith(stem)))File.WriteAllBytes(Path.Combine(dir,Path.GetFileName(file.Name)),file.Bytes.ToArray());
  }
  Console.WriteLine("Selected Libra FXR/resource lists extracted from freshly decrypted archive");
 }
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var directory=Path.Combine(root,".work/devils-gaze");Directory.CreateDirectory(directory);
  object Dump(PARAM.Row row)=>new{row.ID,fields=row.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var names=new[]{"Bullet","AtkParam_Npc","BehaviorParam","NpcParam","SpEffectParam","SpEffectVfxParam","ChaosMatchingCorrectParam"};
  foreach(var name in names)if(!original.Files.Any(f=>Path.GetFileName(f.Name)==name+".param"))throw new InvalidDataException("Missing native table: "+name+"; available "+string.Join(",",original.Files.Select(f=>Path.GetFileName(f.Name)).Where(n=>n.Contains("Behavior")||n.Contains("AtkParam")||n.Contains("ChaosMatching"))));
  var tables=names.ToDictionary(n=>n,n=>parse(n,original.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var corrections=tables["ChaosMatchingCorrectParam"].Rows.Select(Dump).ToArray();
  var effects=tables["SpEffectParam"].Rows.Where(row=>row.ID is >=45890 and <=45899||row.ID is >=48000 and <50000||row.ID is >=4800000 and <4800200||row.ID is >=877601 and <=877605||row.ID is 26010 or 26013 or 7760).Select(Dump).ToArray();
  var data=tables.Where(k=>k.Key!="SpEffectParam"&&k.Key!="ChaosMatchingCorrectParam").ToDictionary(k=>k.Key,k=>k.Value.Rows.Where(row=>row.ID.ToString().StartsWith("7560")||(k.Key=="SpEffectVfxParam"&&row.ID is >=54460 and <=54468)||(k.Key=="Bullet"&&row.ID is >=7560000 and <7563000)||(k.Key=="BehaviorParam"&&row.ID/1000==275600)||(k.Key=="Bullet"&&Convert.ToInt32(row["sfxId_Bullet"].Value) is >=675570 and <=675589)).Select(Dump).ToArray());
  var dummy=tables["AtkParam_Npc"].Rows.Where(row=>row.ID is 0 or 1).Select(Dump).ToArray();
  var visualOnly=tables["Bullet"].Rows.Where(row=>Convert.ToInt32(row["atkId_Bullet"].Value)==-1&&Convert.ToInt32(row["HitBulletID"].Value)>0).Take(15).Select(Dump).ToArray();
  File.WriteAllText(Path.Combine(directory,"native.json"),JsonSerializer.Serialize(new{data,effects,corrections,dummy,visualOnly}));
  Console.WriteLine("Read-only Libra projectile/attack/behavior/NPC/effect/depth data exported");
 }
}
