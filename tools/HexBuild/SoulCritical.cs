using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
static class SoulCritical {
 public const int Owner=95005200;
 public static readonly int[] Markers=Array.Empty<int>();
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  var attack=clone("AtkParam_Pc",69300,Owner+60);
  foreach(var(f,v)in new(string,object)[]{("atkPhys",1),("atkMag",1),("atkFire",0),("atkThun",0),("atkDark",0),("atkPhysCorrection",0),("atkMagCorrection",0),("atkFireCorrection",0),("atkThunCorrection",0),("atkDarkCorrection",0),("atkSuperArmor",10f),("atkSuperArmorCorrection",100),("reviveDamage",30f),("reviveDamageCorrection",100),("ultimateChargeCorrection",100),("dmgLevel",5),("dmgLevel_vsPlayer",5),("knockbackDist",1.2f),("spAttribute",0),("throwFlag",0),("throwTypeId",0),("throwDamageAttribute",0),("selfTarget",0),("opposeTarget",1),("friendlyTarget",0),("subCategory1",0),("subCategory2",0),("subCategory3",0),("subCategory4",0),("subCategory5",0)})Set(attack,f,v);
  for(int i=0;i<5;i++)Set(attack,"spEffectId"+i,-1);
  var bullet=clone("Bullet",7720606,Owner+50);
  foreach(var(f,v)in new(string,object)[]{("atkId_Bullet",Owner+60),("HitBulletID",-1),("launchConditionType",0),("isUseSharedHitList",1),("dmgHitRecordLifeTime",1f),("isHitBothTeam",0)})Set(bullet,f,v);
  Prelude(clone);
 }
 public static void Prelude(Func<string,int,int,PARAM.Row> clone){
  var r=clone("Bullet",7720606,Owner+51);
  foreach(var(f,v)in new(string,object)[]{("atkId_Bullet",-1),("sfxId_Bullet",677210),("life",0.5f),("HitBulletID",-1),("hitRadius",0f),("hitRadiusMax",0f),("spreadTime",0f),("intervalCreateBulletId",-1),("intervalCreateWaitTime",0f),("intervalCreateTimeMin",0f),("intervalCreateTimeMax",0f),("launchConditionType",0),("isHitBothTeam",0)})Set(r,f,v);
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
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,mmv?"work/soul-critical/mmv":"work/soul-critical/staged"),input=Path.Combine(mod,"regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","Bullet","AtkParam_Pc"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"SoulCritical ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  Mechanics(Add);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Check(RewardCatalog.HistoryBit(Owner)==48,"History must append at bit 48");
  int marker=DoubleDip.FirstMarker+RewardCatalog.HistoryBit(Owner);
  var hidden=Add("SpEffectParam",8000000,marker);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(hidden,f,v);
  foreach(int id in RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length).Where(id=>!tables["SpEffectParam"].Rows.Any(r=>r.ID==id))){
   var record=Add("SpEffectParam",8000000,id);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(record,f,v);
  }
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
  PARAM ReadTable(string n)=>tables.TryGetValue(n,out var p)?p:parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes);
  Generate(root,mod,ReadTable,Path.Combine(dir,"soul_critical_data.rs"));
  Resources(root,mod,dir);
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner},inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=48,historyBytes=7,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Soul Critical owner and native shockwave, append-only reward, Mystery Box instance and guarded cleanup staged");
 }

 public static void TargetMap(string root,string overlay,Func<string,Memory<byte>,PARAM> parse,string? destination=null){
  var mod=Path.Combine(overlay,"mod");var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(mod,"regulation.bin"));var tables=new Dictionary<string,PARAM>();
  PARAM Table(string n){if(!tables.TryGetValue(n,out var p)){p=parse(n,binder.Files.Single(f=>Name(f)==n+".param").Bytes);tables[n]=p;}return p;}
  Generate(root,mod,Table,destination??Path.Combine(overlay,"soul_critical_data.rs"));
 }
 public static void Generate(string root,string mod,Func<string,PARAM> table,string output){
  int I(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
  double V(PARAM.Row r,string f)=>Convert.ToDouble(r[f].Value);
  var attacks=table("AtkParam_Pc").Rows.Where(r=>I(r,"throwFlag")==2).GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>{Check(g.All(r=>Same(r,g.First(),false)),"Ambiguous critical attack row "+g.Key);return g.First();});
  var bulletGroups=table("Bullet").Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>g.ToArray());
  var bullets=bulletGroups.ToDictionary(g=>g.Key,g=>g.Value[0]);
  var behaviorGroups=table("BehaviorParam_PC").Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>g.ToArray());
  var behaviors=behaviorGroups.ToDictionary(g=>g.Key,g=>g.Value[0]);
  var weapons=table("EquipParamWeapon").Rows;
  string taePath=Path.Combine(mod,"chr/c0000.anibnd.dcx");if(!File.Exists(taePath))taePath=Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx");
  var binder=BND4.Read(taePath);var animations=new Dictionary<(int,long),TAE.Animation>();
  foreach(var file in binder.Files.Where(f=>f.Name.EndsWith(".tae"))){var name=Name(file);var match=Regex.Match(name,@"^a(\d+)\.tae$");if(!match.Success)continue;int group=int.Parse(match.Groups[1].Value);foreach(var a in TAE.Read(file.Bytes).Animations)animations[(group,a.ID)]=a;}
  TAE.Animation? Resolve(int group,long id,HashSet<(int,long)> seen){if(!seen.Add((group,id))||!animations.TryGetValue((group,id),out var a))return null;
   if(a.MiniHeader is TAE.Animation.AnimMiniHeader.ImportOtherAnim import)return Resolve((int)(import.ImportFromAnimID/1000000),import.ImportFromAnimID%1000000,seen);return a;}
  var pairs=new HashSet<(int variation,int motion)>();
  // Native skill/ultimate source tools use a placeholder straight-sword motion
  // with shield variation 3000. Exclude those auxiliary descriptors, not MMV weapons.
  foreach(var w in weapons){if(!(w.ID>=60000000&&w.ID<=65000000&&I(w,"wepmotionCategory")==23&&I(w,"behaviorVariationId")==3000))pairs.Add((I(w,"behaviorVariationId"),I(w,"wepmotionCategory")));if(I(w,"heroID")>0&&I(w,"heroBehaviorVariationID")>=0)pairs.Add((I(w,"heroBehaviorVariationID"),I(w,"heroMovesetOverrideID")>0?I(w,"heroMovesetOverrideID"):I(w,"wepmotionCategory")));}
  int[] AttackIds(PARAM.Row behavior){var result=new HashSet<int>();var seen=new HashSet<int>();
   void Attack(int id){if(attacks.TryGetValue(id,out var a)&&I(a,"throwFlag")==2)result.Add(id);}
   void Bullet(int id){if(!seen.Add(id)||!bullets.TryGetValue(id,out var b))return;Check(bulletGroups[id].All(r=>Same(r,b,false)),"Ambiguous referenced critical bullet "+id);Attack(I(b,"atkId_Bullet"));Bullet(I(b,"HitBulletID"));Bullet(I(b,"intervalCreateBulletId"));}
   if(I(behavior,"refType")==0)Attack(I(behavior,"refId"));else if(I(behavior,"refType")==1)Bullet(I(behavior,"refId"));return result.Order().ToArray();
  }
  double Score(int id){var r=attacks[id];return new[]{"Phys","Mag","Fire","Thun","Dark"}.Sum(e=>V(r,"atk"+e+"Correction"))+new[]{"Phys","Mag","Fire","Thun","Dark"}.Sum(e=>V(r,"atk"+e));}
  var entries=new SortedDictionary<(int attack,int animation),bool>();var evidence=new List<object>();var conflicts=new List<object>();
  foreach(var (variation,motion) in pairs.OrderBy(p=>p.variation).ThenBy(p=>p.motion))foreach(int anim in new[]{31700,31710,31720,31730,31740,31750,31760}){
   var animation=Resolve(motion,anim,new());if(animation==null)continue;
   var hits=new List<(float time,int judge,int[] attacks)>();
   foreach(var e in animation.Events.Where(e=>e.Type==304).OrderBy(e=>e.StartTime)){
    int judge=BitConverter.ToInt32(e.GetParameterBytes(false),4);
    if(behaviors.TryGetValue(100000000+variation*1000+judge,out var b)){Check(behaviorGroups[b.ID].All(r=>Same(r,b,false)),"Ambiguous critical behavior "+b.ID);var ids=AttackIds(b);if(ids.Length>0)hits.Add((e.StartTime,judge,ids));}
   }
   if(hits.Count==0)continue;
   // Pick the highest scheduled attack strength, with the later event winning
   // equal scores. Runtime uses actual FINAL damage, never this ranking score.
   int main=Enumerable.Range(0,hits.Count).OrderBy(i=>hits[i].attacks.Max(Score)).ThenBy(i=>hits[i].time).Last();
   for(int i=0;i<hits.Count;i++)foreach(int id in hits[i].attacks){var key=(id,anim);bool primary=i==main;
    if(entries.TryGetValue(key,out var old)&&old!=primary)conflicts.Add(new{variation,motion,anim,id,old,primary});else entries[key]=primary;
   }
   evidence.Add(new{variation,motion,animation=anim,mainIndex=main,hits=hits.Select(h=>new{h.time,h.judge,h.attacks,score=h.attacks.Max(Score)})});
  }
  string evidencePath=Path.Combine(root,"work/soul-critical/maps",Path.GetFileName(Path.GetDirectoryName(output))+".json");Directory.CreateDirectory(Path.GetDirectoryName(evidencePath)!);
  File.WriteAllText(evidencePath,JsonSerializer.Serialize(new{taeSha256=Hash(taePath),entries=entries.Count,conflicts,evidence},new JsonSerializerOptions{WriteIndented=true}));
  if(!Path.GetFullPath(output).StartsWith(Path.GetFullPath(Path.Combine(root,"src"))+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase))File.Copy(evidencePath,Path.ChangeExtension(output,"json"),true);
  Check(conflicts.Count==0,"Ambiguous Soul Critical main hits; inspect generated map report");Check(entries.Count>100,"Incomplete Soul Critical map");
  File.WriteAllText(output,"// Generated from this target's weapon, behavior, bullet, attack and resolved TAE data.\npub const HITS:&[(i32,i32,bool)]=&[\n"+string.Join("\n",entries.Select(e=>$" ({e.Key.attack},{e.Key.animation},{e.Value.ToString().ToLowerInvariant()}),"))+"\n];\n");
 }
 public static void Resources(string root,string mod,string output){
  string nativePath=Path.Combine(root,".work/soul-critical/sfxbnd_c7720.ffxbnd.dcx");
  string sharedPath=Path.Combine(root,".work/soul-critical/sfxbnd_commoneffects.ffxbnd.dcx");
  if(!File.Exists(nativePath)||!File.Exists(sharedPath)){
   string drive=Path.GetPathRoot(root)!;string smith=Environment.GetEnvironmentVariable("NRHEX_SMITHBOX")??Path.Combine(drive,"Program Files/Smithbox"),game=Environment.GetEnvironmentVariable("NRHEX_GAME")??Path.Combine(drive,"Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game");
   var assembly=System.Reflection.Assembly.LoadFrom(Path.Combine(smith,"Andre.Formats.dll"));
   System.Runtime.InteropServices.NativeLibrary.SetDllImportResolver(assembly,(n,_,_)=>n.Contains("bhd5_decrypt_rust")?System.Runtime.InteropServices.NativeLibrary.Load(Path.Combine(smith,"bhd5_decrypt_rust.dll")):IntPtr.Zero);
   var flags=System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic|System.Reflection.BindingFlags.Static;
   var keys=(Dictionary<string,string>)assembly.GetType("Andre.Formats.Util.ArchiveKeys")!.GetField("NightreignKeys",flags)!.GetValue(null)!;
   var key=keys.Single(k=>Path.GetFileNameWithoutExtension(k.Key).Equals("data0",StringComparison.OrdinalIgnoreCase)).Value;
   var decrypt=assembly.GetType("Andre.Formats.Util.NativeRsa")!.GetMethods().Single(m=>m.Name=="Decrypt"&&m.GetParameters().Length==3);
   var bytes=(byte[])decrypt.Invoke(null,new object[]{new Memory<byte>(File.ReadAllBytes(Path.Combine(game,"data0.bhd"))),key,2})!;
   var headers=BHD5.Read(bytes,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).ToDictionary(h=>h.FileNameHash);
   using var bdt=File.OpenRead(Path.Combine(game,"data0.bdt"));Directory.CreateDirectory(Path.GetDirectoryName(nativePath)!);
   foreach(string archivePath in new[]{nativePath,sharedPath}){ulong hash=0;foreach(char c in "/sfx/"+Path.GetFileName(archivePath))hash=unchecked(hash*0x85+c);File.WriteAllBytes(archivePath,headers[hash].ReadFile(bdt));}
  }
  var boss=BND4.Read(nativePath);string input=Path.Combine(mod,"sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");var b=BND4.Read(input);var old=b.Files.ToArray();
  var shared=BND4.Read(sharedPath).Files.Select(Name).ToHashSet();
  foreach(int effect in new[]{677224,677210})foreach(string ext in new[]{".fxr",".ffxreslist"}){
   var source=boss.Files.Single(f=>Name(f)==$"f{effect:D9}"+ext);var existing=b.Files.SingleOrDefault(f=>Name(f)==Name(source));
   if(existing!=null){Check(existing.Bytes.Span.SequenceEqual(source.Bytes.Span),"Conflicting Soul Critical FXR");continue;}
   int group=ext==".fxr"?0:400000;int id=b.Files.Where(f=>f.ID>=group&&f.ID<group+100000).Max(f=>f.ID)+1;
   b.Files.Add(new BinderFile(source.Flags,id,source.Name,source.Bytes.ToArray()));
  }
  foreach(int effect in new[]{677224,677210}){
  var list=b.Files.Single(f=>Name(f)==$"f{effect:D9}.ffxreslist");
  foreach(string line in System.Text.Encoding.UTF8.GetString(list.Bytes.Span).Split(['\r','\n'],StringSplitOptions.RemoveEmptyEntries)){
   string name=Path.GetFileNameWithoutExtension(line.Trim())+(Path.GetExtension(line.Trim())==".tif"?".tpf":".flver");Check(shared.Contains(name)||b.Files.Any(f=>Name(f)==name),"Missing Soul Critical FXR dependency: "+name);
  }
  }
  string path=Path.Combine(output,"sfxbnd_commoneffects_dlc01.ffxbnd.dcx");b.Files.Sort((a,c)=>a.ID.CompareTo(c.ID));b.Write(path);var final=BND4.Read(path);
  foreach(var f in old){var now=final.Files.Single(x=>x.ID==f.ID);Check(now.Name==f.Name&&now.Flags==f.Flags&&now.Bytes.Span.SequenceEqual(f.Bytes.Span),"Unrelated FXR changed");}
  foreach(var f in b.Files){var now=final.Files.Single(x=>x.ID==f.ID);Check(now.Bytes.Span.SequenceEqual(f.Bytes.Span),"FXR readback failed");}
  File.WriteAllText(Path.Combine(output,"sfx-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(path),nativeSha256=Hash(nativePath),added=final.Files.Count-old.Length,unrelatedMembersPreserved=true,dependenciesResolved=true,gameplayVerified=false}));
 }

 public static void PreludeStage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,"work/soul-leadin",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(Path.Combine(mod,"regulation.bin"));var old=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="Bullet.param");var p=parse("Bullet",file.Bytes);var before=parse("Bullet",file.Bytes);
  var original=parse("Bullet",native.Files.Single(f=>Name(f)=="Bullet.param").Bytes);
  PARAM.Row Clone(string n,int source,int id){Check(n=="Bullet"&&!p.Rows.Any(r=>r.ID==id),"Prelude row collision");var row=new PARAM.Row(original.Rows.Single(r=>r.ID==source)){ID=id};p.Rows.Add(row);return row;}
  Prelude(Clone);p.Rows=p.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=p.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);var now=parse("Bullet",actual.Files.Single(f=>Name(f)=="Bullet.param").Bytes);
  Check(now.Rows.Count==before.Rows.Count+1&&before.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>r.ID!=Owner+51).OrderBy(r=>r.ID)).All(pair=>Same(pair.First,pair.Second)),"Existing bullet changed");
  Check(Same(p.Rows.Single(r=>r.ID==Owner+51),now.Rows.Single(r=>r.ID==Owner+51)),"Prelude roundtrip mismatch");
  foreach(var f in actual.Files.Where(f=>Name(f)!="Bullet.param"))Check(f.Bytes.Span.SequenceEqual(old[f.ID]),"Unrelated table changed");
  Resources(root,mod,dir);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{version="0.17.293",preludeBullet=Owner+51,visual=677210,delaySeconds=0,lifeSeconds=0.5,intervalSeconds=0,childBullet=-1,preludeAttack=-1,allExistingRowsUnchanged=true,regulationSha256=Hash(output),gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: entry visual appended without a damaging child; existing shockwave and all other rows unchanged");
 }

 public static void EntryStage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string mod=Path.Combine(root,mmv?"dist/mmv/mod":"dist/mod"),dir=Path.Combine(root,"work/soul-entry",mmv?"mmv":"standalone");Directory.CreateDirectory(dir);
  string input=Path.Combine(mod,"regulation.bin");var b=SFUtil.DecryptNightreignRegulation(input);var old=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=b.Files.Single(f=>Name(f)=="Bullet.param");var p=parse("Bullet",file.Bytes);var before=parse("Bullet",file.Bytes);
  var original=parse("Bullet",native.Files.Single(f=>Name(f)=="Bullet.param").Bytes);PARAM.Row? expected=null;
  Prelude((n,source,id)=>expected=new PARAM.Row(original.Rows.Single(r=>r.ID==source)){ID=id});
  var row=p.Rows.Single(r=>r.ID==Owner+51);foreach(string field in new[]{"intervalCreateBulletId","intervalCreateWaitTime","intervalCreateTimeMin","intervalCreateTimeMax"})row[field].Value=expected![field].Value;
  Check(Same(row,expected!,false),"Entry visual has unexpected changes");file.Bytes=p.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);var now=parse("Bullet",actual.Files.Single(f=>Name(f)=="Bullet.param").Bytes);
  Check(now.Rows.Count==before.Rows.Count&&before.Rows.Where(r=>r.ID!=Owner+51).Zip(now.Rows.Where(r=>r.ID!=Owner+51)).All(pair=>Same(pair.First,pair.Second)),"Unrelated bullet changed");
  Check(Same(row,now.Rows.Single(r=>r.ID==Owner+51)),"Entry visual readback mismatch");
  foreach(var f in actual.Files.Where(f=>Name(f)!="Bullet.param"))Check(f.Bytes.Span.SequenceEqual(old[f.ID]),"Unrelated parameter table changed");
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{version="0.17.293",preludeBullet=Owner+51,visual=677210,preludeTrigger="critical entry",waveTrigger="accepted primary hit",childBullet=-1,delaySeconds=0,preludeAttack=-1,onlyFourPreludeFieldsChanged=true,inputSha256=Hash(input),outputSha256=Hash(output),gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: prelude child disabled; existing wave and all unrelated rows unchanged");
 }

}
