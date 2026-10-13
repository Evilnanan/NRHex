using System.Security.Cryptography;
using System.Text.Json;
using System.Collections;
using SoulsFormats;

// Map-unit IDs are deliberately relative to each original gaol template.
// The native map-unit instancer remaps entities AND event flags together.
static class Cages {
 public const int Owner=95001600, Hollow=95001601, Mutation=95001602, OpenEvent=9100961;
 public record MutationMap(int modifier,uint flag,int slots,int[] counts,float[] probabilities);
 static int Number(PARAM.Row row,string field)=>Convert.ToInt32(row[field].Value);
 static void Set(PARAM.Row row,string field,object value)=>row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
 public record TowerSource(string map,int sourceNpc,int cloneNpc,uint entity,int entityOffset,uint firstFlag,int firstLot,uint secondFlag,int secondLot,int alternateThink);
 public static TowerSource[] TowerSources(string root,Boss[] bosses){
  return bosses.Where(b=>!b.map.StartsWith("m46_")).Select(b=>{
   var path=Path.Combine(root,"assets/original/map/mapstudio",b.map+".msb.dcx");Check(Hash(path)==b.sourceSha256,"Tower source hash changed");
   var bytes=DCX.Decompress(path).ToArray();
   // The installed SoulsFormats omits the tail of NR Enemy type data.
   // Read it from the original bytes, anchored to the unique entity, and
   // validate the preceding think/NPC fields before interpreting the tail.
   var offsets=Enumerable.Range(0,bytes.Length-0xc0).Where(i=>i%4==0&&BitConverter.ToUInt32(bytes,i)==b.entity).ToArray();
   Check(offsets.Length==1,"Ambiguous tower entity");int at=offsets[0];
   Check(BitConverter.ToInt32(bytes,at+0x58)==b.think&&BitConverter.ToInt32(bytes,at+0x5c)==b.npc,"Tower enemy type-data layout changed");
   var r=new TowerSource(b.map,b.npc,95300000+Array.IndexOf(bosses,b),b.entity,at,BitConverter.ToUInt32(bytes,at+0xa0),BitConverter.ToInt32(bytes,at+0xa4),BitConverter.ToUInt32(bytes,at+0xa8),BitConverter.ToInt32(bytes,at+0xac),BitConverter.ToInt32(bytes,at+0xb0));
   Check(r.firstFlag==b.entity-500&&r.secondFlag==r.firstFlag+1&&r.firstLot is 6850000 or 6851000&&r.secondLot==r.firstLot+1000&&r.alternateThink>0,"Unexpected native tower reward binding");
   var ctor=EMEVD.Read(Path.Combine(root,"assets/original/event",b.map+".emevd.dcx")).Events.Single(e=>e.ID==0);
   Check(ctor.Instructions.Any(i=>i.Bank==1012&&i.ID==1&&i.ArgData.Take(3).SequenceEqual(new byte[]{1,0,2})),"Tower variation-2 event guard changed");
   foreach(uint flag in new[]{r.firstFlag,r.secondFlag})Check(ctor.Instructions.Any(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length==20&&BitConverter.ToInt32(i.ArgData,4)==90015070&&BitConverter.ToUInt32(i.ArgData,16)==flag),"Native tower reward flag initializer missing");
   return r;
  }).ToArray();
 }
 public static int Reward(Boss boss,TowerSource[] towers)=>towers.SingleOrDefault(t=>t.sourceNpc==boss.npc)?.firstLot??boss.nativeRewardLot;
 public static void MutationParameters(Dictionary<string,PARAM> tables,TowerSource[] towers){
  var effects=tables["SpEffectParam"];
  if(!effects.Rows.Any(r=>r.ID==Mutation)){
   var marker=new PARAM.Row(effects.Rows.Single(r=>r.ID==8000000)){ID=Mutation};
   foreach(var (field,value) in new (string,object)[]{("maxHpRate",1f),("saveCategory",-1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(marker,field,value);
   effects.Rows.Add(marker);
  }
  foreach(var row in tables["NpcParam"].Rows.Where(r=>r.ID>=95300000&&r.ID<95300037)){
   var tower=towers.SingleOrDefault(t=>t.cloneNpc==row.ID);
   if(tower!=null){
    var lot=tables["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=tower.firstLot&&r.ID<tower.firstLot+10).ToArray();
    Check(lot.Length==3&&lot.All(r=>Number(r,"getItemFlagId")==0),"Native tower weapon reward must remain repeatable");
    Set(row,"rewardItemLot_2",tower.firstLot);Set(row,"chaosMatchingRewardLotId",tower.firstLot);
   }
   if(Number(row,"chaosMatchingRewardLotId")==-1)Set(row,"chaosMatchingRewardLotId",Number(row,"rewardItemLot_2"));
  }
 }
 public static MutationMap[] MutationMaps(Dictionary<string,PARAM> tables){
  var candidates=tables["ChaosMatchingMutationEnemyTableParam"].Rows.Where(r=>Number(r,"categoryId")==120).ToArray();
  return tables["ChaosMatchingMutationCategoryParam"].Rows.Where(r=>Number(r,"categoryId")==120).OrderBy(r=>Number(r,"modifierMapId")).Select(row=>{
   int modifier=Number(row,"modifierMapId");
   uint flag=tables["LotBaseMapPatternFlag"].Rows.Where(r=>Number(r,"modifier")==modifier&&Number(r,"eventFlag")==7600+modifier-10).Select(r=>Convert.ToUInt32(r["eventFlag"].Value)).Distinct().Single();
   // Convert quotas into independent cage rolls. This is a table-based
   // approximation, not the native world's placement-dependent marginal rate.
   int slots=candidates.Where(r=>Number(r,"modifierMapId1")==0||new[]{"modifierMapId1","modifierMapId2","modifierMapId3"}.Any(f=>Number(r,f)==modifier)).Sum(r=>Number(r,"mutationCount"));
   var counts=Enumerable.Range(1,5).Select(depth=>Number(row,$"depth{depth}Count")).ToArray();
   Check(slots>0&&counts.All(count=>count>=0&&count<=slots),"Invalid native mutation quota conversion");
   return new MutationMap(modifier,flag,slots,counts,counts.Select(count=>(float)count/slots).ToArray());
  }).ToArray();
 }
 public static void Mutate(EMEVD.Event encounter,uint entity,PARAM.Row npc,Dictionary<string,PARAM> tables,MutationMap[] maps,uint red,uint decided){
  // Only the host rolls. A decision persists across reloads and is cleared
  // at victory, before a later encounter may select this boss again.
  encounter.Instructions.AddRange([I(1003,101,(byte)8,(byte)1,(byte)0,decided),I(1003,13,(byte)8,(byte)1),SetFlag(red,false),I(1001,1,1)]);
  var correction=tables["ChaosMatchingCorrectParam"].Rows.Single(r=>r.ID==Number(npc,"chaosMatchingCorrectParamId"));
  var deep=correction.Cells.Where(c=>c!=null&&c.Def.InternalName.StartsWith("spEffect")).Select(c=>Convert.ToInt32(c.Value)).ToArray();
  Check(deep.Length==5&&deep.All(id=>id>0),"Cage boss needs five native depth effects");
  // All category-120 quotas are zero at depth 1 and equal at depths 2..5.
  Check(maps.Length==6&&maps.All(m=>m.counts[0]==0&&m.counts.Skip(1).Distinct().Count()==1),"Native depth quota structure changed; regenerate depth branches");
  foreach(int effect in deep.Skip(1))encounter.Instructions.Add(I(4,5,(sbyte)-4,entity,effect,(byte)1,(sbyte)0,1f));
  encounter.Instructions.Add(I(1000,101,(byte)6,(byte)0,(sbyte)-4));
  foreach(var map in maps)encounter.Instructions.Add(I(3,42,(sbyte)-5,map.flag,map.probabilities[1]));
  encounter.Instructions.AddRange([I(1000,101,(byte)6,(byte)0,(sbyte)-5),SetFlag(red,true),I(1014,6),SetFlag(decided,true),I(1014,8),Flag(0,decided,true),I(1003,101,(byte)9,(byte)0,(byte)0,red),I(2004,8,entity,Mutation),I(1014,9)]);
 }
 public const float VictoryDelaySeconds=5f, CleanupDelaySeconds=20f;
 public record Boss(string map,uint entity,int npc,int think,string model,string name,int healthBarName,int nativeRewardLot,string sourceSha256);
 public record BirthModel(string model,bool nativeFog,bool towerWalk,int[] fogDummies);
 public record BirthOverride(int npc,string model,int animation,bool loop,string map,uint entity,int eventId,bool cancelled=false);
 public record BirthIdleReset(int npc,string model,int animation);
 public record BirthCatalog(int fogSfx,int nativeDummy,int towerFogDefault,int towerFogDancer,int towerDummy,float towerWaitSeconds,BirthModel[] models,BirthOverride[] nativeOverrides,BirthIdleReset[] idleResets);
 static void Check(bool ok,string reason){if(!ok)throw new InvalidDataException(reason);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
 public static Boss[] AddNpcs(string root,Dictionary<string,PARAM> tables,Func<string,int,int,PARAM.Row> clone,Action<PARAM.Row,string,object> set){
  var bosses=JsonSerializer.Deserialize<Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
  CageAI.Add(root,tables["NpcThinkParam"],bosses,(source,id)=>clone("NpcThinkParam",source,id));
  Check(bosses.Length==37&&bosses.Select(b=>b.npc).Distinct().Count()==bosses.Length,"Unexpected cage boss catalog");
  for(int i=0;i<bosses.Length;i++){
   var b=bosses[i];Check((b.map.StartsWith("m46_")&&int.Parse(b.map.Substring(4,2))>=50)||new[]{"m48_10_00_00","m48_30_00_00","m48_40_00_00","m49_10_00_00","m49_17_00_00","m49_18_00_00","m49_20_00_00"}.Contains(b.map),"Only original field/gaol and selected Hollow tower bosses can enter the cage pool");
   var source=Path.Combine(root,"assets/original/map/mapstudio",b.map+".msb.dcx");Check(Hash(source)==b.sourceSha256,"Cage source map mismatch: "+b.map);
   var enemy=MSB_NR.Read(source).Parts.Enemies.Single(e=>e.EntityData.EntityID==b.entity);
   Check(enemy.ModelName==b.model&&enemy.NpcParamId==b.npc&&enemy.NpcThinkParamId==b.think,"Cage boss identity mismatch: "+b.name);
   if(!b.map.StartsWith("m46_")){
    var ctor=EMEVD.Read(Path.Combine(root,"assets/original/event",b.map+".emevd.dcx")).Events.Single(e=>e.ID==0);
    Check(ctor.Instructions.Any(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length>=32&&BitConverter.ToInt32(i.ArgData,4)==90015023&&BitConverter.ToUInt32(i.ArgData,24)==b.entity&&BitConverter.ToInt32(i.ArgData,28)==b.healthBarName),"Tower boss health-bar name must come from its native display event, not its BGM ID");
   }
   var row=clone("NpcParam",b.npc,95300000+i);set(row,"disableRespawn",0);
   var original=tables["NpcParam"].Rows.Single(r=>r.ID==b.npc);
   Check(row.Cells.Where(c=>c!=null&&c.Def.InternalName!="disableRespawn").All(c=>c.Value is byte[] bytes?bytes.SequenceEqual((byte[])original[c.Def.InternalName].Value):Equals(c.Value,original[c.Def.InternalName].Value)),"Cage NPC changed combat/reward fields");
   Check(Convert.ToInt32(row["rewardItemLot_2"].Value)==b.nativeRewardLot,"Cage reward must retain its native difficulty table");
   Check(Convert.ToInt32(row["getSoul"].Value)>0,"Cage boss lacks native rune reward");
   var rewards=tables["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=b.nativeRewardLot&&r.ID<b.nativeRewardLot+10).ToArray();
   Check(rewards.Any(r=>Convert.ToInt32(r["lotItemCategory01"].Value)==7)&&rewards.All(r=>Convert.ToUInt32(r["getItemFlagId"].Value)==0),"Native boss loot must support repeated awards");
  }
  CageEncounters.AddNpcs(root,tables);
  return bosses;
 }
 static EMEVD.Instruction I(int bank,int id,params object[] args)=>new(bank,id,args);
 static EMEVD.Instruction Flag(int group,uint id,bool on)=>I(3,0,(sbyte)group,(byte)(on?1:0),(byte)0,id);
 static EMEVD.Instruction SetFlag(uint id,bool on)=>I(2003,69,(byte)0,id,(byte)(on?1:0));
 static EMEVD.Instruction LocalFlag(uint id,bool on)=>I(2003,66,(byte)0,id,(byte)(on?1:0));
 static EMEVD.Instruction Wait(int group)=>I(0,0,(sbyte)0,(byte)1,(sbyte)group);
 static EMEVD.Instruction Restart()=>I(1000,4,(byte)1);
 static EMEVD.Instruction Has(int group,int effect,bool on)=>I(4,5,(sbyte)group,20000u,effect,(byte)(on?1:0),(sbyte)0,1f);
 static EMEVD.Instruction Anim(uint entity,int animation,bool loop=false)=>I(2003,18,entity,animation,(byte)(loop?1:0),(byte)0,(byte)0,(byte)0,1f);
 public static EMEVD.Instruction HostOnly()=>I(1003,14,(byte)0,(byte)1);
 public static void Multiplayer(EMEVD.Event e,uint entity){
  // Native 90015022: the 7999 marker belongs to this generated actor's life.
  e.Instructions.AddRange([I(1004,0,(byte)2,entity,7999,(byte)1,(sbyte)0,1),I(2009,4,entity),I(2004,8,entity,7999)]);
 }
 public static void Difficulty(EMEVD.Event encounter,uint entity,uint sourceEntity){
  // Mirror the choice in native 90045004 at each fresh spawn. Keep its real
  // effects, including attack scaling and the ancient-dragon visual variant.
  int visual=sourceEntity==46800810u?99146:99145;
  encounter.Instructions.AddRange([I(1003,101,(byte)2,(byte)1,(byte)0,7505u),I(2004,8,entity,99150),I(2004,8,entity,visual),I(1000,103,(byte)4),I(1014,2),I(1,7,(sbyte)3,(byte)0,(byte)0,(byte)0,(byte)16,(byte)9,(byte)0),I(1000,101,(byte)3,(byte)0,(sbyte)3),I(2004,8,entity,99152),I(2004,8,entity,visual),I(1000,103,(byte)4),I(1014,3),I(2004,8,entity,99153),I(1014,4)]);
 }
 static EMEVD.Event HealthBar(uint id,uint entity,int name,uint spawned,uint choice,uint shown){
  var e=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
  // Replica AI state is not a HUD prerequisite. Use the shared birth state,
  // the locally loaded live actor and the local player's distance instead.
  e.Instructions.AddRange([I(2000,2,(byte)0),I(1014,0),Flag(1,spawned,true),Flag(1,choice,true),Flag(1,shown,true),I(4,7,(sbyte)1,entity,(byte)1,(sbyte)0,1f),I(4,14,(sbyte)1,entity,(sbyte)2,0,(sbyte)0,1f),I(4,0,(sbyte)1,20000u,(byte)0,(sbyte)0,1f),I(3,3,(sbyte)1,(byte)1,20000u,entity,50f,1),Wait(1)]);
  // Native 90015000 reserves local flags 5000/5001 for HUD slots 0/1.
  // Wait for a free slot OR an ended encounter; never clear another boss's bar.
  e.Instructions.AddRange([Flag(-5,5000,false),Flag(-6,5001,false),I(0,0,(sbyte)-7,(byte)1,(sbyte)-5),I(0,0,(sbyte)-7,(byte)1,(sbyte)-6),Flag(-7,spawned,false),Flag(-7,choice,false),Flag(-7,shown,false),I(4,0,(sbyte)-7,entity,(byte)1,(sbyte)0,1f),I(4,0,(sbyte)-7,20000u,(byte)1,(sbyte)0,1f),I(4,7,(sbyte)-7,entity,(byte)0,(sbyte)0,1f),I(3,3,(sbyte)-7,(byte)0,20000u,entity,50f,1),Wait(-7),I(1003,2,(byte)1,(byte)0,(byte)0,spawned),I(1003,2,(byte)1,(byte)0,(byte)0,choice),Flag(2,shown,true),I(4,14,(sbyte)2,entity,(sbyte)2,0,(sbyte)0,1f),I(4,0,(sbyte)2,20000u,(byte)0,(sbyte)0,1f),I(4,7,(sbyte)2,entity,(byte)1,(sbyte)0,1f),I(3,3,(sbyte)2,(byte)1,20000u,entity,50f,1),I(1000,2,(byte)1,(byte)0,(sbyte)2),I(1000,7,(byte)3,(byte)0,(sbyte)-5),LocalFlag(5000,true),I(2003,11,(sbyte)1,entity,(short)0,name),I(1000,3,(byte)2),LocalFlag(5001,true),I(2003,11,(sbyte)1,entity,(short)1,name)]);
  // Match the native HUD association as well as its display request.
  e.Instructions.Add(I(2003,102,(sbyte)1,name,entity));
  e.Instructions.AddRange([Flag(-12,shown,false),I(3,3,(sbyte)-12,(byte)0,20000u,entity,100f,1),I(4,0,(sbyte)-11,entity,(byte)1,(sbyte)0,1f),I(0,0,(sbyte)-12,(byte)1,(sbyte)-11),I(4,0,(sbyte)-12,20000u,(byte)1,(sbyte)0,1f),I(4,7,(sbyte)-12,entity,(byte)0,(sbyte)0,1f),Flag(-12,spawned,false),Flag(-12,choice,false),Wait(-12),I(1000,7,(byte)2,(byte)0,(sbyte)-11),I(1001,0,5f),I(1000,3,(byte)1),I(1001,0,1f),I(1000,7,(byte)3,(byte)0,(sbyte)-5),LocalFlag(5000,false),I(2003,11,(sbyte)0,entity,(short)0,name),I(1000,3,(byte)2),LocalFlag(5001,false),I(2003,11,(sbyte)0,entity,(short)1,name),I(1001,1,1),Restart()]);
  return e;
 }
 static EMEVD.Event Birth(uint id,uint entity,Boss boss,BirthCatalog births,uint ready,uint choice,uint spawned,uint shown,bool stableSnapshot=true){
  var e=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
  var birth=births.models.Single(m=>m.model==boss.model);
  int fog=boss.model=="c7920"?births.towerFogDancer:births.towerFogDefault;
  // Run once on EVERY peer. Synchronized encounter progression must not stand
  // in for local SFX playback. Skip the intro when loading an ongoing fight.
  // Snapshot ongoing/fresh BEFORE waiting for this peer's actor to load.
  // A slow peer still plays its own fresh intro even after the host starts AI.
  e.Instructions.AddRange([I(2000,2,(byte)0),LocalFlag(shown,false),Flag(1,ready,true),Flag(1,choice,true),Flag(5,spawned,true),Flag(6,spawned,false),I(0,0,(sbyte)-5,(byte)1,(sbyte)5),I(0,0,(sbyte)-5,(byte)1,(sbyte)6),I(0,0,(sbyte)1,(byte)1,(sbyte)-5),Wait(1),I(2004,29,entity,(byte)0),I(4,7,(sbyte)2,entity,(byte)1,(sbyte)0,1f),I(4,14,(sbyte)2,entity,(sbyte)2,0,(sbyte)0,1f),Wait(2)]);
  Multiplayer(e,entity);
  int skip=e.Instructions.Count;e.Instructions.Add(I(1000,7,(byte)0,(byte)1,(sbyte)5));
  e.Instructions.AddRange([I(2004,5,entity,(byte)0),I(2006,3,2,entity,births.towerDummy,fog),I(1001,0,births.towerWaitSeconds)]);
  var birthOverride=births.nativeOverrides.SingleOrDefault(o=>o.npc==boss.npc);
  var idleReset=births.idleResets.SingleOrDefault(r=>r.npc==boss.npc);
  if(idleReset!=null)e.Instructions.Add(Anim(entity,idleReset.animation));
  else if(birthOverride!=null){
   // Keep the native reference, but a cancelled intro must not fall through
   // to another animation inferred from the same model.
   if(!birthOverride.cancelled)e.Instructions.Add(Anim(entity,birthOverride.animation,birthOverride.loop));
  }
  else if(birth.towerWalk)e.Instructions.Add(Anim(entity,20026));
  else if(birth.nativeFog)e.Instructions.Add(Anim(entity,20025,true));
  e.Instructions[skip]=I(1000,7,(byte)(e.Instructions.Count-skip-1),(byte)1,(sbyte)5);
  e.Instructions.AddRange([I(2004,5,entity,(byte)1),LocalFlag(shown,true),Flag(-2,choice,false),I(4,7,(sbyte)-2,entity,(byte)0,(sbyte)0,1f),Wait(-2),LocalFlag(shown,false),I(1001,1,1),Restart()]);
  if(stableSnapshot){StabilizeBirth(e);LocalGeneration(e,entity,entity-5200,entity-1100);CageUsage.Intro(e);}
  return e;
 }
 // Native condition results can latch while the main wait is still false.
 // Read spawned only AFTER this encounter is selected, and branch BEFORE
 // waiting for its local actor. Control flow itself preserves that snapshot.
 public static void StabilizeBirth(EMEVD.Event e){
  var old=e.Instructions.ToArray();Check(e.Parameters.Count==0,"Parameterized birth cannot be rewritten");
  Check(old.Length>20&&old[4].Bank==3&&old[4].ID==0&&old[4].ArgData[0]==5&&old[9].Bank==0&&old[9].ID==0,"Unexpected old birth snapshot header");
  Check(old.Skip(4).Take(5).Select(i=>(i.Bank,i.ID)).SequenceEqual(new[]{(3,0),(3,0),(0,0),(0,0),(0,0)}),"Unexpected snapshot graph");
  uint spawned=BitConverter.ToUInt32(old[4].ArgData,4);
  int skip=Array.FindIndex(old,i=>i.Bank==1000&&i.ID==7&&i.ArgData[2]==5);
  Check(skip>14&&old[skip].ArgData[1]==1,"Unexpected birth compiled skip");
  int reveal=skip+1+old[skip].ArgData[0];Check(reveal<old.Length&&old[reveal].Bank==2004&&old[reveal].ID==5&&old[reveal].ArgData[4]==1,"Unexpected reveal branch");
  e.Instructions.Clear();e.Instructions.AddRange(old.Take(4));e.Instructions.Add(old[9]);
  e.Instructions.Add(I(1003,101,(byte)1,(byte)1,(byte)0,spawned));
  e.Instructions.AddRange(old.Skip(10).Take(skip-10));
  e.Instructions.AddRange(old.Skip(skip+1).Take(reveal-skip-1));
  e.Instructions.AddRange([I(1000,103,(byte)2),I(1014,1)]);
  // The ongoing branch loads/activates the actor without replaying its intro.
  e.Instructions.AddRange(old.Skip(10).Take(skip-10));
  e.Instructions.Add(I(1014,2));e.Instructions.AddRange(old.Skip(reveal));
 }
 // The native generator resolves its MSB SpawnPart identities and resets an
 // existing map actor; it does not allocate an unrelated NPC per peer. Keep
 // peer loading separate from host selection/settlement. Network behavior
 // still requires cooperative acceptance on the installed game version.
 public static void LocalGeneration(EMEVD.Event e,uint actor,uint generator,uint? dead=null){
  Check(e.Parameters.Count==0,"Unexpected parameterized cage birth");
  var old=e.Instructions.ToArray();Check(old.Count(i=>i.Bank==2004&&i.ID==29&&BitConverter.ToUInt32(i.ArgData,0)==actor&&i.ArgData[4]==0)==2,"Expected two local load branches");
  e.Instructions.Clear();
  foreach(var i in old){
   e.Instructions.Add(i);
   if(dead.HasValue&&i.Bank==1014&&i.ID==1)e.Instructions.Add(I(1003,101,(byte)3,(byte)1,(byte)0,dead.Value));
   if(i.Bank==2004&&i.ID==29&&BitConverter.ToUInt32(i.ArgData,0)==actor&&i.ArgData[4]==0)e.Instructions.Add(I(2003,54,generator));
  }
  if(dead.HasValue){
   uint choice=BitConverter.ToUInt32(old[3].ArgData,4);
   e.Instructions.AddRange([I(1014,3),Flag(0,choice,false),I(1001,1,1),Restart()]);
  }
  PeerLifecycle(e,actor);
 }
 // Loading and presentation must have one writer per peer. Battle's former
 // startup could unload an actor after Birth had already requested it, then
 // terminate on the client. Streaming out is NOT a new encounter either.
 public static void PeerLifecycle(EMEVD.Event e,uint actor){
  Check(e.Parameters.Count==0&&e.Instructions[0].Bank==2000&&e.Instructions[0].ID==2,"Unexpected local birth");
  uint choice=BitConverter.ToUInt32(e.Instructions[3].ArgData,4);
   var original=e.Instructions.ToArray();e.Instructions.Clear();int loadWaits=0;
  foreach(var i in original){
   if(i.Bank==0&&i.ID==0&&i.ArgData[0]==0&&i.ArgData[2]==2){
     loadWaits++;
    e.Instructions.AddRange([I(0,0,(sbyte)-3,(byte)1,(sbyte)2),Flag(-3,choice,false),Wait(-3),I(1003,101,(byte)4,(byte)0,(byte)0,choice)]);
   }else e.Instructions.Add(i);
  }
   Check(loadWaits==2,"Expected fresh and reconnect load waits");
   e.Instructions.InsertRange(1,[I(2004,29,actor,(byte)1),I(2004,5,actor,(byte)0),I(2004,1,actor,(byte)0)]);
  int count=e.Instructions.RemoveAll(i=>i.Bank==4&&i.ID==7&&i.ArgData[0]==unchecked((byte)-2)&&BitConverter.ToUInt32(i.ArgData,4)==actor);
  Check(count==1,"Missing birth stream-out restart condition");
  // Match host corpse cleanup before restarting the local preparation. A
  // different encounter may start during this wait without touching it.
  int end=e.Instructions.FindIndex(i=>i.Bank==1000&&i.ID==4);
  Check(end>0,"Missing birth restart");e.Instructions.Insert(end,I(1001,0,CleanupDelaySeconds-VictoryDelaySeconds));
  e.Instructions.AddRange([I(1014,4),I(1001,1,1),Restart()]);
  PeerReplicaLoading(e,actor);
 }
 // Presentation cannot wait for a replica's first positive-HP packet. Native
 // 90065000/90065031 establish network updates before their load/presentation
 // waits. Selection/dead flags identify this life; HP remains a HUD/battle gate.
 public static void PeerReplicaLoading(EMEVD.Event e,uint actor){
  Check(e.Parameters.Count==0,"Parameterized replica birth");
  int removed=e.Instructions.RemoveAll(i=>i.Bank==4&&i.ID==14&&i.ArgData[0]==2&&BitConverter.ToUInt32(i.ArgData,4)==actor);
  Check(removed==2,"Expected fresh and ongoing replica HP waits");
  int loads=0,reveals=0;var old=e.Instructions.ToArray();e.Instructions.Clear();
  foreach(var i in old){
   if(i.Bank==2004&&i.ID==29&&BitConverter.ToUInt32(i.ArgData,0)==actor&&i.ArgData[4]==0){
    e.Instructions.Add(I(2004,34,actor,(byte)1,(sbyte)0));loads++;
   }
   e.Instructions.Add(i);
   if(i.Bank==2004&&i.ID==5&&BitConverter.ToUInt32(i.ArgData,0)==actor&&i.ArgData[4]==1){
    e.Instructions.AddRange([I(2004,39,actor,(byte)1),I(2004,1,actor,(byte)1)]);reveals++;
   }
  }
  Check(loads==2&&reveals==1,"Unexpected replica birth branches");
 }
 public static void StagePeerLifecycle(string root){
  string output=Path.Combine(root,".work/audit-01734/events");Directory.CreateDirectory(output);
   var bosses=JsonSerializer.Deserialize<Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
   var births=JsonSerializer.Deserialize<BirthCatalog>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-birth.json")))!;
   var groups=CageEncounters.Load(root);
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block:00}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),target=Path.Combine(output,name+".emevd.dcx");uint p=(uint)(4600+block)*10000;
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changes=new List<object>();var changed=new HashSet<long>();
   foreach(var e in next.Events.Where(e=>e.ID>=p+9710&&e.ID<p+9760)){
    changes.Add(new{e.ID,before=Dump(before.Events.Single(x=>x.ID==e.ID))});
    int host=e.Instructions.FindIndex(i=>i.Bank==1003&&i.ID==14);
    Check(host>=4&&(host-1)%3==0,"Unexpected battle preparation");
    for(int i=1;i<host;i+=3){Check(e.Instructions[i].Bank==2004&&e.Instructions[i].ID==29&&e.Instructions[i].ArgData[4]==1&&e.Instructions[i+1].Bank==2004&&e.Instructions[i+1].ID==5&&e.Instructions[i+1].ArgData[4]==0&&e.Instructions[i+2].Bank==2004&&e.Instructions[i+2].ID==1&&e.Instructions[i+2].ArgData[4]==0,"Unexpected battle load commands");}
    e.Instructions.RemoveRange(1,host-1);changed.Add(e.ID);
   }
   foreach(var e in next.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984)){
    changes.Add(new{e.ID,before=Dump(before.Events.Single(x=>x.ID==e.ID))});PeerLifecycle(e,p+9000+(uint)(e.ID-p-9900));changed.Add(e.ID);
   }
   Check(changed.Count==134,"Incomplete peer lifecycle patch");next.Write(target);var read=EMEVD.Read(target);
    var regenerated=Enumerable.Range(0,bosses.Length).Select(i=>Birth(p+9900+(uint)i,p+9000+(uint)i,bosses[i],births,p+7403,p+7500+(uint)i,p+7404,p+7600+(uint)i))
     .Concat(groups.groups.SelectMany(g=>g.members.Select(m=>CageEncounters.Birth(p,g,m)))).ToArray();
    Check(regenerated.Length==84&&regenerated.All(e=>Same(e,read.Events.Single(x=>x.ID==e.ID))),"Full-build birth differs from staged lifecycle");
   Check(read.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&read.Events.All(e=>Same(e,next.Events.Single(x=>x.ID==e.ID))),"Peer lifecycle readback mismatch");
   Check(read.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Same(e,before.Events.Single(x=>x.ID==e.ID))),"Unrelated event changed");
   Check(read.Format==before.Format&&read.Compression==before.Compression&&read.StringData.SequenceEqual(before.StringData)&&read.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Event metadata changed");
   reports.Add(new{mapName=name,inputSha256=Hash(input),outputSha256=Hash(target),changes,events=read.Events.Where(e=>changed.Contains(e.ID)).Select(Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/audit-01734/cages.json"),JsonSerializer.Serialize(reports,new JsonSerializerOptions{WriteIndented=true}));
 }
 public static void SingleDeaths(EMEVD.Event e,uint actor,uint dead){
  var old=e.Instructions.ToArray();Check(!old.Any(i=>i.Bank==2003&&i.ID==69&&BitConverter.ToUInt32(i.ArgData,4)==dead),"Single death marker already installed");
  int load=0,death=0;e.Instructions.Clear();
  foreach(var i in old){
   e.Instructions.Add(i);
   if(i.Bank==0&&i.ID==0&&i.ArgData[0]==0&&i.ArgData[2]==2){e.Instructions.Add(SetFlag(dead,false));load++;}
   if(i.Bank==4&&i.ID==14&&i.ArgData[0]==0&&BitConverter.ToUInt32(i.ArgData,4)==actor&&i.ArgData[8]==0){e.Instructions.Add(SetFlag(dead,true));death++;}
  }
  Check(load==1&&death==1,"Single spawn/death control flow changed");
 }
 public static void StagePeerGeneration(string root){
  string output=Path.Combine(root,".work/fixes-01731/events");Directory.CreateDirectory(output);
  var groups=CageEncounters.Load(root);var reports=new List<object>();
  var native=EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx")).Events.Single(e=>e.ID==90065031);
  Check(native.Instructions[0].Bank==2000&&native.Instructions[0].ID==2&&native.Instructions[0].ArgData[0]==0&&native.Instructions.Count(i=>i.Bank==2003&&i.ID==54)==1&&!native.Instructions.Any(i=>i.Bank==1003&&i.ID==14),"Native peer-local generator contract changed");
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block:00}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),target=Path.Combine(output,name+".emevd.dcx");uint p=(uint)(4600+block)*10000;
   var map=MSB_NR.Read(Path.Combine(root,"dist/mod/map/mapstudio",name+".msb.dcx"));
   var generators=map.Events.Generators.Where(g=>g.EntityID>=p+3800&&g.EntityID<p+3884).ToArray();
   Check(generators.Length==84&&generators.All(g=>g.GenType==3&&g.InitialSpawnCount==0&&g.LimitNum==-1&&g.MaxNum==1&&g.MinGenNum==1&&g.MaxGenNum==1&&g.SpawnPartNames.Count(n=>!string.IsNullOrEmpty(n))==1),"Unexpected per-actor native generator capacity");
   foreach(var g in generators)Check(map.Parts.Enemies.Single(a=>a.Name==g.SpawnPartNames[0]).EntityData.EntityID==g.EntityID+5200,"Generator selected a different actor");
   var before=EMEVD.Read(input);var patched=EMEVD.Read(input);var changed=new HashSet<long>();var changes=new List<object>();
   foreach(var e in patched.Events.Where(e=>e.ID>=p+9710&&e.ID<p+9760)){
    Check(e.Parameters.Count==0&&e.Instructions.Any(i=>i.Bank==1003&&i.ID==14&&i.ArgData.Take(2).SequenceEqual(new byte[]{0,1})),"Missing host battle authority");
    changes.Add(new{e.ID,before=JsonSerializer.SerializeToElement(Dump(e))});int expected=e.ID<p+9747?1:groups.groups.Single(g=>p+9710+g.choice==e.ID).members.Length;
    int count=e.Instructions.RemoveAll(i=>i.Bank==2003&&i.ID==54);Check(count==expected,"Unexpected host generator calls");
    if(e.ID<p+9747){uint index=(uint)e.ID-p-9710;SingleDeaths(e,p+9000+index,p+7900+index);}changed.Add(e.ID);
   }
   foreach(var e in patched.Events.Where(e=>e.ID>=p+9900&&e.ID<p+9984)){
    changes.Add(new{e.ID,before=JsonSerializer.SerializeToElement(Dump(e))});uint index=(uint)e.ID-p-9900;var member=groups.groups.SelectMany(g=>g.members).SingleOrDefault(m=>m.index==index);
    Check(!e.Instructions.Any(i=>i.Bank==2003&&i.ID==54),"Birth generation already patched");
    LocalGeneration(e,p+9000+index,p+3800+index,index<37||member?.combat==true?p+7900+index:null);changed.Add(e.ID);
   }
   Check(changed.Count==134,"Expected 50 battles and 84 births");patched.Write(target);var read=EMEVD.Read(target);
   Check(read.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&read.Events.All(e=>Same(e,patched.Events.Single(v=>v.ID==e.ID))),"Peer generation readback failed");
   Check(read.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Same(e,before.Events.Single(v=>v.ID==e.ID))),"Unrelated cage event changed");
   Check(read.Format==before.Format&&read.Compression==before.Compression&&read.StringData.SequenceEqual(before.StringData)&&read.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Cage metadata changed");
   reports.Add(new{mapName=name,inputSha256=Hash(input),outputSha256=Hash(target),unrelatedEventsAndConstructorUnchanged=true,generatorCount=generators.Length,eachGeneratorHasOneActorAndMaximumOneLiveUnit=true,changes,events=read.Events.Where(e=>changed.Contains(e.ID)).Select(Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/fixes-01731/cages.json"),JsonSerializer.Serialize(new{nativeGeneratorEvent=Dump(native),reports},new JsonSerializerOptions{WriteIndented=true}));
 }
 public static void StageFogPatch(string root){
  string output=Path.Combine(root,".work/cage-fog-01729");Directory.CreateDirectory(Path.Combine(output,"events"));
  var bosses=JsonSerializer.Deserialize<Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
  var births=JsonSerializer.Deserialize<BirthCatalog>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-birth.json")))!;
  var groups=CageEncounters.Load(root);var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string mapName=$"m46_{block:00}_00_00";uint p=(uint)(4600+block)*10000;
   string input=Path.Combine(root,"dist/mod/event",mapName+".emevd.dcx"),target=Path.Combine(output,"events",mapName+".emevd.dcx");
   var before=EMEVD.Read(input);var patched=EMEVD.Read(input);var changes=new List<object>();var changed=new HashSet<long>();
   var legacy=Enumerable.Range(0,bosses.Length).Select(i=>Birth(p+9900+(uint)i,p+9000+(uint)i,bosses[i],births,p+7403,p+7500+(uint)i,p+7404,p+7600+(uint)i,false))
    .Concat(groups.groups.SelectMany(g=>g.members.Select(m=>CageEncounters.Birth(p,g,m,false))));
   foreach(var old in legacy){
    var current=patched.Events.Single(e=>e.ID==old.ID);Check(Same(current,old),"Unexpected installed birth event "+old.ID);
    changes.Add(new{old.ID,before=JsonSerializer.SerializeToElement(Dump(old))});StabilizeBirth(old);patched.Events[patched.Events.IndexOf(current)]=old;changed.Add(old.ID);
   }
   Check(changed.Count==84,"Expected every single/group/mount birth");patched.Write(target);var check=EMEVD.Read(target);
   Check(check.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&check.Events.All(e=>Same(e,patched.Events.Single(v=>v.ID==e.ID))),"Fog patch failed readback");
   Check(check.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Same(e,before.Events.Single(v=>v.ID==e.ID))),"Fog patch changed another event");
   Check(check.Format==before.Format&&check.Compression==before.Compression&&check.StringData.SequenceEqual(before.StringData)&&check.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Fog patch changed metadata");
   reports.Add(new{mapName,inputSha256=Hash(input),outputSha256=Hash(target),unrelatedEventsAndConstructorUnchanged=true,changes,events=check.Events.Where(e=>changed.Contains(e.ID)).Select(Dump)});
  }
  File.WriteAllText(Path.Combine(output,"patch.json"),JsonSerializer.Serialize(new{stagedOnly=true,installed=false,reports},new JsonSerializerOptions{WriteIndented=true}));
 }
 public static bool Same(EMEVD.Event a,EMEVD.Event b)=>a.ID==b.ID&&a.RestBehavior==b.RestBehavior&&a.Name==b.Name&&a.Instructions.Count==b.Instructions.Count&&a.Instructions.Zip(b.Instructions).All(p=>p.First.Bank==p.Second.Bank&&p.First.ID==p.Second.ID&&p.First.Layer==p.Second.Layer&&p.First.ArgData.SequenceEqual(p.Second.ArgData))&&JsonSerializer.Serialize(a.Parameters)==JsonSerializer.Serialize(b.Parameters);
 public static object Dump(EMEVD.Event e)=>new{e.ID,e.RestBehavior,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})};
 public static EMEVD.Event Selector(uint prefix,int count){
  uint request=prefix+7402,ready=prefix+7403,choices=prefix+7500,cooling=prefix+7550,draft=prefix+7100;
  var selector=new EMEVD.Event(prefix+9700,EMEVD.Event.RestBehaviorType.Restart);
  // Native E0D9B0 encodes all ON writes before all OFF writes. Clearing the
  // shared choice range and selecting within it in one batch makes the peer
  // clear the selected flag last. Draw locally; publish ONLY the accepted ON.
  // The finished encounter owns its shared OFF, never the next lottery.
  selector.Instructions.AddRange([I(2000,2,(byte)0),HostOnly(),Flag(0,request,true),I(1003,101,(byte)0,(byte)1,(byte)0,ready),SetFlag(prefix+7400,true),SetFlag(prefix+7401,true),I(1001,1,1),I(2003,22,draft,draft+(uint)count-1,(byte)0),I(2003,17,draft,draft+(uint)count-1,(byte)1)]);
  // Native goto searches only AFTER the current instruction (0x5943b4).
  // Reject cooling actors by jumping forward to a restart, never backwards.
  for(uint index=0;index<count;index++)selector.Instructions.AddRange([I(1003,1,(byte)1,(byte)0,(byte)0,draft+index),I(1003,101,(byte)1,(byte)1,(byte)0,cooling+index)]);
  for(uint index=0;index<count;index++)selector.Instructions.AddRange([I(1003,1,(byte)1,(byte)0,(byte)0,draft+index),SetFlag(choices+index,true)]);
  selector.Instructions.AddRange([SetFlag(ready,true),I(1014,0),Flag(0,ready,false),Restart(),I(1014,1),Restart()]);
  return selector;
 }
 public static void StageBirthPatch(string root){
  string output=Path.Combine(root,".work/cage-birth-fix");Directory.CreateDirectory(Path.Combine(output,"events"));
  var bosses=JsonSerializer.Deserialize<Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
  var births=JsonSerializer.Deserialize<BirthCatalog>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-birth.json")))!;
  int index=Array.FindIndex(bosses,b=>b.npc==25001020&&b.model=="c2500");
  Check(index==20&&births.nativeOverrides.Single(o=>o.npc==25001020).cancelled,"Expected cancelled Tree Crucible intro");
  var legacyBirths=births with{nativeOverrides=births.nativeOverrides.Select(o=>o with{cancelled=false}).ToArray()};
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string mapName=$"m46_{block:00}_00_00";uint prefix=(uint)(4600+block)*10000;
   string input=Path.Combine(root,"dist/mod/event",mapName+".emevd.dcx"),target=Path.Combine(output,"events",mapName+".emevd.dcx");
   var before=EMEVD.Read(input);var patched=EMEVD.Read(input);
   EMEVD.Event Make(BirthCatalog catalog)=>Birth(prefix+9900+(uint)index,prefix+9000+(uint)index,bosses[index],catalog,prefix+7403,prefix+7500+(uint)index,prefix+7404,prefix+7600+(uint)index);
   var expected=Make(births);var legacy=Make(legacyBirths);
   var current=patched.Events.Single(e=>e.ID==expected.ID);
   Check(Same(current,legacy)||Same(current,expected),"Unexpected Tree Crucible birth event; refusing incremental patch");
   Check(expected.Instructions.All(i=>i.Bank!=2003||i.ID!=18)&&legacy.Instructions.Count==expected.Instructions.Count+1,"Intro cancellation must remove exactly one forced animation");
   patched.Events[patched.Events.IndexOf(current)]=expected;patched.Write(target);
   var check=EMEVD.Read(target);
   Check(check.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&check.Events.All(e=>Same(e,e.ID==expected.ID?expected:before.Events.Single(v=>v.ID==e.ID))),"Birth patch changed an unrelated event or its order");
   Check(check.Format==before.Format&&check.Compression==before.Compression&&check.StringData.SequenceEqual(before.StringData)&&check.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Birth patch changed EMEVD metadata");
   reports.Add(new{mapName,inputSha256=Hash(input),outputSha256=Hash(target),onlyTreeCrucibleBirthChanged=true,birth=Dump(expected)});
  }
  File.WriteAllText(Path.Combine(output,"patch.json"),JsonSerializer.Serialize(new{stagedOnly=true,installed=false,reports},new JsonSerializerOptions{WriteIndented=true}));
 }
 public static void StageSelectorPatch(string root){
  string output=Path.Combine(root,".work/cage-selector-fix");Directory.CreateDirectory(Path.Combine(output,"events"));
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string mapName=$"m46_{block:00}_00_00";uint prefix=(uint)(4600+block)*10000;
   string input=Path.Combine(root,"dist/mod/event",mapName+".emevd.dcx"),target=Path.Combine(output,"events",mapName+".emevd.dcx");
   var before=EMEVD.Read(input);var patched=EMEVD.Read(input);var expected=Selector(prefix,37);
   var legacy=Selector(prefix,37);legacy.Instructions.RemoveRange(legacy.Instructions.Count-2,2);legacy.Instructions.Insert(2,I(1014,1));
   var current=patched.Events.Single(e=>e.ID==prefix+9700);
   Check(Same(current,legacy)||Same(current,expected),"Unexpected cage selector; refusing incremental patch");
   patched.Events[patched.Events.IndexOf(current)]=expected;patched.Write(target);
   var check=EMEVD.Read(target);
   Check(check.Events.Count==before.Events.Count&&check.Events.All(e=>Same(e,e.ID==expected.ID?expected:before.Events.Single(v=>v.ID==e.ID))),"Selector patch changed an unrelated event");
   Check(check.Format==before.Format&&check.Compression==before.Compression&&check.StringData.SequenceEqual(before.StringData)&&check.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Selector patch changed EMEVD metadata");
   foreach(var ev in check.Events.Where(e=>e.ID>=prefix+9700&&e.ID<prefix+9970)){
    for(int pc=0;pc<ev.Instructions.Count;pc++){
     var ins=ev.Instructions[pc];
     if(ins.Bank==1003&&ins.ID==101)Check(ev.Instructions.Skip(pc+1).Any(i=>i.Bank==1014&&i.ID==ins.ArgData[0]),"Cage goto has no forward label");
    }
   }
   reports.Add(new{mapName,inputSha256=Hash(input),outputSha256=Hash(target),request=prefix+7402,ready=prefix+7403,choices=prefix+7500,cooling=prefix+7550,onlySelectorChanged=true,allCageFlagGotosHaveForwardTargets=true,selector=Dump(expected)});
  }
  File.WriteAllText(Path.Combine(output,"patch.json"),JsonSerializer.Serialize(new{stagedOnly=true,installed=false,reports},new JsonSerializerOptions{WriteIndented=true}));
 }
 static void RemoveEntry(object parameter,object entry){
  var list=parameter.GetType().GetProperties().Select(p=>p.GetIndexParameters().Length==0?p.GetValue(parameter):null).OfType<IList>().Single(l=>l.Contains(entry));list.Remove(entry);
 }
 public static void Terrain(EMEVD common){
  Check(!common.Events.Any(e=>e.ID==9100960),"Cage terrain event collision");
  common.Events.Single(e=>e.ID==0).Instructions.Insert(0,I(2000,0,0,9100960,0));
  var e=new EMEVD.Event(9100960,EMEVD.Event.RestBehaviorType.Restart);
  e.Instructions.AddRange([I(2000,2,(byte)0),Flag(0,7604,true),I(2004,8,20000u,Hollow),Flag(0,7604,false),I(2004,21,20000u,Hollow),Restart()]);
  common.Events.Add(e);
 }
 public static void Build(string root,Boss[] bosses,Dictionary<string,PARAM> serialized,string? output=null){
  output??=Path.Combine(root,"dist/mod");Directory.CreateDirectory(Path.Combine(output,"event"));
  Check(Enumerable.Range(95300000,bosses.Length).All(id=>serialized["NpcThinkParam"].Rows.Any(r=>r.ID==id)),"Missing independent cage AI rows");
  var towers=TowerSources(root,bosses);
  var mutationMaps=MutationMaps(serialized);
  var births=JsonSerializer.Deserialize<BirthCatalog>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-birth.json")))!;
  Check(births.fogSfx==690031&&births.nativeDummy==960&&births.towerFogDefault==690047&&births.towerFogDancer==690046&&births.towerDummy==900&&births.towerWaitSeconds==4&&bosses.All(b=>births.models.Count(m=>m.model==b.model)==1)&&births.models.All(m=>m.fogDummies.Contains(births.towerDummy)),"Unsupported cage birth reference");
  string Original(string name)=>Path.Combine(root,"assets/original/event",name+".emevd.dcx");
  string Output(string name)=>Path.Combine(output,"event",name+".emevd.dcx");
  var nativeDeath=EMEVD.Read(Original("common_func")).Events.Single(e=>e.ID==90045002);
  var nativeHud=EMEVD.Read(Original("common_func")).Events.Single(e=>e.ID==90015000);
  Check(nativeHud.Instructions[0].Bank==2000&&nativeHud.Instructions[0].ID==2&&nativeHud.Instructions[0].ArgData[0]==0&&new[]{(28,5000u),(32,5001u),(67,5000u),(71,5001u)}.All(p=>nativeHud.Instructions[p.Item1].Bank==2003&&nativeHud.Instructions[p.Item1].ID==66&&BitConverter.ToUInt32(nativeHud.Instructions[p.Item1].ArgData,4)==p.Item2),"Native local HUD slot allocation changed");
  Check(nativeDeath.Instructions.Count==15&&nativeDeath.Instructions[3].Bank==4&&nativeDeath.Instructions[4].Bank==1001&&nativeDeath.Instructions[4].ID==0&&BitConverter.ToSingle(nativeDeath.Instructions[4].ArgData,0)==VictoryDelaySeconds&&!nativeDeath.Instructions.Any(i=>i.Bank==2004&&(i.ID==5||i.ID==29))&&CleanupDelaySeconds>VictoryDelaySeconds,"Native gaol victory delay must not be treated as enemy removal");
  var nativeDifficulty=EMEVD.Read(Original("common_func")).Events.Single(e=>e.ID==90045004);
  Check(nativeDifficulty.Instructions.Count==42&&nativeDifficulty.Instructions[9].Bank==3&&nativeDifficulty.Instructions[9].ID==0&&nativeDifficulty.Instructions[9].ArgData[1]==0&&BitConverter.ToUInt32(nativeDifficulty.Instructions[9].ArgData,4)==7505&&nativeDifficulty.Instructions[13].ArgData[1]==1&&BitConverter.ToUInt32(nativeDifficulty.Instructions[13].ArgData,4)==7505&&nativeDifficulty.Instructions[14].Bank==1&&nativeDifficulty.Instructions[14].ID==7&&nativeDifficulty.Instructions[14].ArgData.Take(7).SequenceEqual(new byte[]{2,0,0,0,16,9,0}),"Unsupported native gaol progression rule");
  foreach(var (index,effect) in new[]{(26,99150),(28,99145),(30,99146),(33,99152),(40,99153)})Check(nativeDifficulty.Instructions[index].Bank==2004&&nativeDifficulty.Instructions[index].ID==8&&BitConverter.ToInt32(nativeDifficulty.Instructions[index].ArgData,4)==effect,"Native gaol progression effect mismatch");
  foreach(var reset in births.idleResets){
   var boss=bosses.Single(b=>b.npc==reset.npc&&b.model==reset.model);
   Check(reset.animation==0&&births.idleResets.Count(r=>r.npc==reset.npc)==1&&!births.nativeOverrides.Any(o=>o.npc==reset.npc),"Cage idle reset must be unique and use normal idle");
   var originalCtor=EMEVD.Read(Original(boss.map)).Events.Single(e=>e.ID==0);
   Check(!originalCtor.Instructions.Any(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length>=24&&((BitConverter.ToInt32(i.ArgData,4)==90045001&&BitConverter.ToUInt32(i.ArgData,20)==boss.entity)||(BitConverter.ToInt32(i.ArgData,4)==90065910&&BitConverter.ToUInt32(i.ArgData,16)==boss.entity))),"Idle reset unexpectedly overrides a native birth event");
  }
  foreach(var birthOverride in births.nativeOverrides){
   Check(bosses.Count(b=>b.npc==birthOverride.npc&&b.model==birthOverride.model&&b.map==birthOverride.map&&b.entity==birthOverride.entity)==1&&births.nativeOverrides.Count(o=>o.npc==birthOverride.npc)==1,"Cage birth override identity mismatch");
   var originalCtor=EMEVD.Read(Original(birthOverride.map)).Events.Single(e=>e.ID==0);
   Check(originalCtor.Instructions.Any(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length==32&&BitConverter.ToInt32(i.ArgData,4)==birthOverride.eventId&&BitConverter.ToUInt32(i.ArgData,20)==birthOverride.entity),"Cage birth override lacks a native initializer");
   var originalBirth=EMEVD.Read(Original("common_func")).Events.Single(e=>e.ID==birthOverride.eventId);
   Check(birthOverride.eventId==90045001&&originalBirth.Instructions[8].Bank==2003&&originalBirth.Instructions[8].ID==18&&BitConverter.ToInt32(originalBirth.Instructions[8].ArgData,4)==birthOverride.animation&&originalBirth.Instructions[8].ArgData[8]==(birthOverride.loop?1:0)&&originalBirth.Parameters.Any(p=>p.InstructionIndex==8&&p.TargetStartByte==0&&p.SourceStartByte==12&&p.ByteCount==4),"Cage birth override differs from its native animation or entity binding");
  }
  var tower=EMEVD.Read(Original("common_func")).Events.Single(e=>e.ID==90065910);
  Check(tower.Instructions[39].Bank==2006&&tower.Instructions[39].ID==3&&BitConverter.ToInt32(tower.Instructions[39].ArgData,0)==2&&BitConverter.ToInt32(tower.Instructions[39].ArgData,8)==900&&tower.Instructions[42].Bank==1&&tower.Instructions[42].ID==0&&BitConverter.ToSingle(tower.Instructions[42].ArgData,4)==4f&&tower.Instructions[43].Bank==2003&&tower.Instructions[43].ID==18&&BitConverter.ToInt32(tower.Instructions[43].ArgData,4)==20026,"Unsupported native tower birth sequence");
  foreach(var (mapName,fogId) in new[]{("m48_30_00_00",births.towerFogDancer),("m48_40_00_00",births.towerFogDefault)}){
   var ctor=EMEVD.Read(Original(mapName)).Events.Single(e=>e.ID==0);
   Check(ctor.Instructions.Any(i=>i.Bank==2000&&i.ID==6&&i.ArgData.Length>=28&&BitConverter.ToInt32(i.ArgData,4)==90065910&&BitConverter.ToInt32(i.ArgData,24)==fogId),"Tower fog ID differs from the native initializer");
  }
  var common=EMEVD.Read(Original("common_func"));var before=EMEVD.Read(Original("common_func"));
  // Clone only the native open event. Other mods/nonowners retain the original.
  var native=common.Events.Single(e=>e.ID==90045000);
  Check(native.Instructions.Count==26&&native.Parameters.Count==9&&native.Instructions[10].Bank==3&&native.Instructions[10].ID==4&&native.Instructions[21].Bank==2003&&native.Instructions[21].ID==24,"Unsupported native gaol open control flow");
  var open=CageUsage.Open(native);
  common.Events.Add(open);common.Write(Output("common_func"));
  var checkCommon=EMEVD.Read(Output("common_func"));
  Check(checkCommon.Events.Count==before.Events.Count+1&&before.Events.All(e=>Same(e,checkCommon.Events.Single(v=>v.ID==e.ID)))&&Same(open,checkCommon.Events.Single(e=>e.ID==OpenEvent)),"Cage open clone changed unrelated common events");
  var generatorMap=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio/m52_42_00_00.msb.dcx"));
  var generatorSource=generatorMap.Events.Generators.Single(g=>g.EntityID==52423810);
  Check(generatorSource.GenType==3&&generatorSource.InitialSpawnCount==0&&generatorSource.LimitNum==-1,"Unsupported repeatable native generator template");
  var spawnSource=generatorMap.Regions.GetEntries().Single(r=>r.Name==generatorSource.SpawnRegionNames[0]);
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string mapName=$"m46_{block:00}_00_00";uint prefix=(uint)(4600+block)*10000;
   var eventMap=EMEVD.Read(Original(mapName));var originalEvents=EMEVD.Read(Original(mapName));var ctor=eventMap.Events.Single(e=>e.ID==0);
   uint used=prefix+7400,busy=prefix+7401,request=prefix+7402,ready=prefix+7403,spawned=prefix+7404,choices=prefix+7500,cooling=prefix+7550,shown=prefix+7600,red=prefix+7650,decided=prefix+7700;
   // The native flag manager has no map category for the 6000 segment.
   // Writing it silently fails even though the rest of the interaction runs.
   Check(new[]{used,busy,request,ready,spawned,choices,choices+(uint)bosses.Length-1,cooling,cooling+(uint)bosses.Length-1,shown,shown+(uint)bosses.Length-1}.All(f=>f/1000%10==7),"Cage state must use an allocated map flag category");
   var stateFlags=new[]{used,busy,request,ready,spawned}.Concat(new[]{choices,cooling,shown,red,decided,prefix+7900}.SelectMany(first=>Enumerable.Range((int)first,bosses.Length).Select(v=>(uint)v))).ToHashSet();
   Check(stateFlags.All(flag=>flag/1000%10==7),"Mutation flags must use allocated storage");
   Check(!originalEvents.Events.Any(e=>stateFlags.Contains((uint)e.ID)||e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>stateFlags.Contains(BitConverter.ToUInt32(i.ArgData,n*4))))),"Cage state collides with a native map event/argument");
   var map=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio",mapName+".msb.dcx"));var originalMap=map.Write();var originalModels=map.Models.GetEntries().Select(m=>m.Name).ToHashSet();
   var stone=map.Parts.Assets.Single(p=>p.EntityData.EntityID==prefix+1200);
   // Bake facing into both transforms: map instancing rotates them together,
   // and generator creation no longer overwrites a pending runtime turn.
   float birthYaw=MathF.Atan2(stone.Position.X,stone.Position.Z)*180f/MathF.PI+180f;
   Check(!eventMap.Events.Any(e=>e.ID>=prefix+9700&&e.ID<prefix+9970),"Cage map event collision");
   foreach(var ins in ctor.Instructions.Where(i=>i.Bank==2000&&i.ID==6&&BitConverter.ToInt32(i.ArgData,4)==90045000)){
    var bytes=ins.ArgData.ToArray();Check(bytes.Length==24,"Unexpected native gaol initializer");
    var extra=new byte[40];bytes.CopyTo(extra,0);BitConverter.GetBytes(OpenEvent).CopyTo(extra,4);BitConverter.GetBytes(used).CopyTo(extra,24);BitConverter.GetBytes(busy).CopyTo(extra,28);BitConverter.GetBytes(request).CopyTo(extra,32);BitConverter.GetBytes(prefix+CageUsage.CounterOffset).CopyTo(extra,36);ins.ArgData=extra;
   }
   CageUsage.CheckCounter(originalEvents,prefix);
   var selector=Selector(prefix,bosses.Length);
   CageNetwork.CheckDraftFlags(originalEvents,prefix,bosses.Length);
   eventMap.Events.Add(selector);ctor.Instructions.Insert(0,I(2000,0,0,(uint)selector.ID,0));
   for(int index=0;index<bosses.Length;index++){
    var b=bosses[index];uint entity=prefix+9000+(uint)index,generator=prefix+3800+(uint)index,choice=choices+(uint)index,actorCooling=cooling+(uint)index;
    var src=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio",b.map+".msb.dcx"));
    var enemy=(MSB_NR.Part.Enemy)src.Parts.Enemies.Single(p=>p.EntityData.EntityID==b.entity).DeepCopy();
    enemy.Name=$"{b.model}_{9500+index}";enemy.InstanceID=9500+index;enemy.EntityData.EntityID=entity;enemy.EntityData.EntityGroupIDs=new uint[8];enemy.EntityData.Variation=-1;enemy.EntityData.RandomAppearParamID=0;enemy.Position=new(0,0,0);enemy.Rotation=new(0,birthYaw,0);enemy.HitPartName=null;enemy.PatrolRouteName=null;enemy.NpcParamId=95300000+index;enemy.NpcThinkParamId=95300000+index;
    Check(!map.Parts.GetEntries().Any(p=>p.Name==enemy.Name),"Cage enemy name collision");map.Parts.Enemies.Add(enemy);
    if(!map.Models.GetEntries().Any(m=>m.Name==b.model))map.Models.Add(src.Models.GetEntries().Single(m=>m.Name==b.model).DeepCopy());
    var spawn=spawnSource.DeepCopy();spawn.Name=$"NRHex_cage_spawn_{index}";spawn.RegionID=100+index;spawn.Position=enemy.Position;spawn.Rotation=enemy.Rotation;spawn.Common_EntityID=prefix+3900+(uint)index;map.Regions.Add(spawn);
    var gen=(MSB_NR.Event.Generator)generatorSource.DeepCopy();gen.Name=$"NRHex_cage_generator_{index}";gen.EntityID=generator;gen.EventID=100+index;gen.MaxNum=1;gen.MinGenNum=1;gen.MaxGenNum=1;gen.SpawnRegionNames=new string[8];gen.SpawnRegionNames[0]=spawn.Name;gen.SpawnPartNames=new string[32];gen.SpawnPartNames[0]=enemy.Name;map.Events.Generators.Add(gen);
    var encounter=new EMEVD.Event(prefix+9710+index,EMEVD.Event.RestBehaviorType.Restart);
    encounter.Instructions.AddRange([I(2000,2,(byte)0),HostOnly(),SetFlag(actorCooling,false),Flag(1,ready,true),Flag(1,choice,true),Wait(1),I(2004,29,entity,(byte)0),I(4,7,(sbyte)2,entity,(byte)1,(sbyte)0,1f),I(4,14,(sbyte)2,entity,(sbyte)2,0,(sbyte)0,1f),Wait(2)]);
    encounter.Instructions.Add(I(2004,28,entity,8192));
    Multiplayer(encounter,entity);
    Difficulty(encounter,entity,b.entity);
    Mutate(encounter,entity,serialized["NpcParam"].Rows.Single(r=>r.ID==95300000+index),serialized,mutationMaps,red+(uint)index,decided+(uint)index);
    encounter.Instructions.AddRange([I(2004,1,entity,(byte)0),I(2004,34,entity,(byte)1,(sbyte)0),I(1001,0,CageUsage.IntroSeconds),I(2004,39,entity,(byte)1)]);
    encounter.Instructions.AddRange([I(2004,1,entity,(byte)1),SetFlag(spawned,true),I(2010,12,28u,(byte)0),Anim(prefix+1200,110,true),Anim(prefix+1210,1,true),I(4,14,(sbyte)0,entity,(sbyte)0,0,(sbyte)0,1f),I(1001,0,VictoryDelaySeconds),I(2010,2,entity,5,888880000),I(2007,19,1030),I(2003,101,1)]);
    // Dispatch the original 8145 -> common 1400 -> 7060000 relic reward chain,
    // including its native seven-stack cap and per-player reward bookkeeping.
    encounter.Instructions.Add(I(2003,31,8145u,4u,7u));
    // Do not set either original open/completion flag: that would release grace.
    encounter.Instructions.AddRange([SetFlag(actorCooling,true),Anim(prefix+1210,10,true),I(2003,114,(byte)6,(byte)0),SetFlag(red+(uint)index,false),SetFlag(decided+(uint)index,false),SetFlag(spawned,false),SetFlag(choice,false),SetFlag(ready,false),SetFlag(request,false),SetFlag(busy,false)]);
    // Native gaols finish victory at 5s without removing the actor. Release the
    // shared gaol now, but only touch this old actor during the delayed cleanup.
    encounter.Instructions.AddRange([I(1001,0,CleanupDelaySeconds-VictoryDelaySeconds),I(2004,29,entity,(byte)1),I(2004,5,entity,(byte)0),SetFlag(actorCooling,false),I(1001,1,1),Restart()]);
    SingleDeaths(encounter,entity,prefix+7900+(uint)index);eventMap.Events.Add(encounter);ctor.Instructions.Insert(0,I(2000,0,0,(uint)encounter.ID,0));
    var hud=HealthBar(prefix+9800+(uint)index,entity,b.healthBarName,spawned,choice,shown+(uint)index);
    eventMap.Events.Add(hud);ctor.Instructions.Insert(0,I(2000,0,0,(uint)hud.ID,0));
    var presentation=Birth(prefix+9900+(uint)index,entity,b,births,ready,choice,spawned,shown+(uint)index);
    eventMap.Events.Add(presentation);ctor.Instructions.Insert(0,I(2000,0,0,(uint)presentation.ID,0));
   }
   string mapOut=Path.Combine(output,"map/mapstudio",mapName+".msb.dcx");Directory.CreateDirectory(Path.GetDirectoryName(mapOut)!);map.Write(mapOut);eventMap.Write(Output(mapName));
   var reread=MSB_NR.Read(mapOut);var eventsRead=EMEVD.Read(Output(mapName));
   var birthLayouts=reread.Parts.Enemies.Where(p=>p.EntityData.EntityID>=prefix+9000&&p.EntityData.EntityID<prefix+9000+bosses.Length).Select(p=>new{entity=p.EntityData.EntityID,yaw=p.Rotation.Y,spawnYaw=reread.Regions.GetEntries().Single(r=>r.Name=="NRHex_cage_spawn_"+(p.EntityData.EntityID-prefix-9000)).Rotation.Y}).ToArray();
   Check(birthLayouts.All(p=>MathF.Abs(p.yaw-birthYaw)<.0001f&&MathF.Abs(p.spawnYaw-birthYaw)<.0001f),"Cage facing did not round-trip");
   Check(reread.Parts.Enemies.Count==MSB_NR.Read(originalMap).Parts.Enemies.Count+bosses.Length&&reread.Events.Generators.Count==bosses.Length,"Cage map did not round-trip");
   reread.Parts.Enemies.RemoveAll(p=>p.EntityData.EntityID>=prefix+9000&&p.EntityData.EntityID<prefix+9000+bosses.Length);
   reread.Events.Generators.RemoveAll(g=>g.Name.StartsWith("NRHex_cage_generator_"));
   foreach(var region in reread.Regions.GetEntries().Where(r=>r.Name.StartsWith("NRHex_cage_spawn_")).ToArray())RemoveEntry(reread.Regions,region);
   foreach(var model in reread.Models.GetEntries().Where(m=>!originalModels.Contains(m.Name)).ToArray())RemoveEntry(reread.Models,model);
   Check(reread.Write().SequenceEqual(originalMap),"Cage additions changed original map entries");
   Check(eventsRead.Events.Count==originalEvents.Events.Count+bosses.Length*3+1&&originalEvents.Events.Where(e=>e.ID!=0).All(e=>Same(e,eventsRead.Events.Single(v=>v.ID==e.ID)))&&eventMap.Events.All(e=>Same(e,eventsRead.Events.Single(v=>v.ID==e.ID))),"Cage event round-trip or unrelated event changed");
   Check(eventsRead.Format==originalEvents.Format&&eventsRead.Compression==originalEvents.Compression&&eventsRead.StringData.SequenceEqual(originalEvents.StringData)&&eventsRead.LinkedFileOffsets.SequenceEqual(originalEvents.LinkedFileOffsets),"Cage EMEVD metadata changed");
   reports.Add(new{mapName,birthYaw,stonePosition=new{stone.Position.X,stone.Position.Y,stone.Position.Z},birthLayouts,sourceSha256=Hash(Original(mapName)),outputSha256=Hash(Output(mapName)),mapSha256=Hash(mapOut),used,busy,request,ready,spawned,choices,cooling,shown,red,decided,openInitializers=eventsRead.Events.Single(e=>e.ID==0).Instructions.Where(i=>i.Bank==2000&&i.ID==6&&BitConverter.ToInt32(i.ArgData,4)==OpenEvent).Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)}),events=eventsRead.Events.Where(e=>e.ID>=prefix+9700&&e.ID<prefix+9970).Select(Dump)});
  }
  Directory.CreateDirectory(Path.Combine(root,".work/cage"));
  var npcs=bosses.Select((b,i)=>{var r=serialized["NpcParam"].Rows.Single(r=>r.ID==95300000+i);return new{r.ID,b.npc,b.nativeRewardLot,runes=r["getSoul"].Value,disableRespawn=r["disableRespawn"].Value,rewardItemLot=r["rewardItemLot_2"].Value};});
  var difficultyEffects=new[]{99150,99152,99153}.Select(id=>{var r=serialized["SpEffectParam"].Rows.Single(r=>r.ID==id);return new{r.ID,maxHpRate=r["maxHpRate"].Value,attackPowerRate=r["physicsAttackPowerRate"].Value,replaceSpEffectId=r["replaceSpEffectId"].Value};});
  File.WriteAllText(Path.Combine(root,".work/cage/built.json"),JsonSerializer.Serialize(new{open=Dump(open),terrain=Dump(EMEVD.Read(Output("common")).Events.Single(e=>e.ID==9100960)),reports,bosses,npcs,towers,difficultyEffects,originalMapEntriesReconstructed=true,originalCommonFunctionEventsPreserved=true,originalMapNonconstructorEventsPreserved=true},new JsonSerializerOptions{WriteIndented=true}));
  var mutationNpcs=bosses.Select((b,i)=>{
   var npc=serialized["NpcParam"].Rows.Single(r=>r.ID==95300000+i);
   var set=serialized["SpEffectSetParam"].Rows.Single(r=>r.ID==Number(npc,"chaosMatchingSpEffectSetParamId"));
   var effects=set.Cells.Where(c=>c!=null).Select(c=>Convert.ToInt32(c.Value)).Where(id=>id>0).ToArray();
   Check(effects.Length==2&&effects[0]>=4480&&effects[0]<=4483&&effects[1]>=7220&&effects[1]<=7240,"Unexpected native mutation set");
   return new{npc=npc.ID,b.name,sourceNpc=b.npc,deepEffects=serialized["ChaosMatchingCorrectParam"].Rows.Single(r=>r.ID==Number(npc,"chaosMatchingCorrectParamId")).Cells.Where(c=>c!=null&&c.Def.InternalName.StartsWith("spEffect")).Select(c=>Convert.ToInt32(c.Value)),effectSet=set.ID,effects,rewardLot=Number(npc,"chaosMatchingRewardLotId"),towerRewardFallback=!b.map.StartsWith("m46_")};
  });
  object Row(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var mutationEffects=serialized["SpEffectParam"].Rows.Where(r=>r.ID>=7220&&r.ID<=7240&&r.ID%10==0).Select(Row);
  var rewardLots=mutationNpcs.Select(n=>n.rewardLot).Concat(towers.Select(t=>t.firstLot)).ToHashSet();
  var loot=serialized["ItemLotParam_enemy"].Rows.Where(r=>rewardLots.Any(l=>r.ID>=l&&r.ID<l+10)).Select(Row);
  var marker=serialized["SpEffectParam"].Rows.Single(r=>r.ID==Mutation);
  File.WriteAllText(Path.Combine(root,".work/cage/mutation.json"),JsonSerializer.Serialize(new{marker=Mutation,markerFields=Row(marker),maps=mutationMaps,npcs=mutationNpcs,towers,mutationEffects,loot,probabilityConversion="Native category-120 quota / sum of applicable native mutationCount; independent rolls, approximate placement-independent rate",nativeMutationFlag="ChrIns+0x1fb bit 0",nativeEffectSource="ChrIns+0x5d8 -> 0x4fff30",nativeRewardSelector="0xe95360 -> NpcParam+0x338",towerRewardPolicy="First native tower reward binding; no tower floor placement in cages; same weapon lot for normal/red, native red rune multiplier retained"},new JsonSerializerOptions{WriteIndented=true}));
  CageEncounters.Build(root,serialized,output,output);
  Console.WriteLine($"Cage: {bosses.Length} single bosses and 13 native group encounters, four repeatable templates; original key/grace events retained");
 }
}
