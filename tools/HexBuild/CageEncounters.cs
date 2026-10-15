using System.Collections;
using System.Numerics;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Nodes;
using SoulsFormats;

// The selection is an encounter. Actors remain independent native enemies.
static class CageEncounters {
 public record Member(int index,string sourceName,uint entity,int npc,int think,int alternateThink,string model,bool combat,float[] offset);
 public record Bar(int member,int name);
 public record Ride(int rider,int mount,string sourceName);
 public record Group(int choice,string map,uint target,string name,string sourceSha256,string sourceEventSha256,string sourceCommonEventSha256,Member[] members,Bar[] bars,Ride[] riding,JsonElement reward,int? birthAnimation);
 public record Catalog(int encounterCount,int actorCount,Group[] groups);
 public static Catalog Load(string root)=>JsonSerializer.Deserialize<Catalog>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-encounters.json")))!;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p))).ToLowerInvariant();
 static int N(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Source(string root,Group g){var p=Path.Combine(root,"assets/original/map/mapstudio",g.map+".msb.dcx");Check(Hash(p)==g.sourceSha256&&Hash(Path.Combine(root,"assets/original/event",g.map+".emevd.dcx"))==g.sourceEventSha256&&Hash(Path.Combine(root,"assets/original/event/common_func.emevd.dcx"))==g.sourceCommonEventSha256,"Group source changed: "+g.name);return p;}
 public static void AddNpcs(string root,Dictionary<string,PARAM> tables){
  var c=Load(root);Check(c.encounterCount==50&&c.actorCount==84&&c.groups.Select(g=>g.choice).SequenceEqual(Enumerable.Range(37,13)),"Unexpected group catalog");
  Check(c.groups.SelectMany(g=>g.members).Select(m=>m.index).SequenceEqual(Enumerable.Range(37,47)),"Group actor indices must be contiguous");
  foreach(var g in c.groups){
   var map=MSB_NR.Read(Source(root,g));
   foreach(var m in g.members){
    var part=map.Parts.Enemies.Single(p=>p.Name==m.sourceName);Check(part.EntityData.EntityID==m.entity&&part.NpcParamId==m.npc&&part.NpcThinkParamId==m.think&&part.ModelName==m.model,"Group actor identity changed");
    Check(part.EntityData.RandomAppearParamID==0,"Group member has an unresolved random placement condition");
    int id=95300000+m.index;var npc=tables["NpcParam"].Rows.Single(r=>r.ID==m.npc);var copy=new PARAM.Row(npc){ID=id};Set(copy,"disableRespawn",0);
    Replace(tables["NpcParam"],copy);
    var think=tables["NpcThinkParam"].Rows.Single(r=>r.ID==m.think);var ai=new PARAM.Row(think){ID=id};
    var baseline=g.map.StartsWith("m4")&&!g.map.StartsWith("m46")&&m.alternateThink>0?tables["NpcThinkParam"].Rows.Single(r=>r.ID==m.alternateThink):think;
    Check(Equals(think["logicId"].Value,baseline["logicId"].Value)&&Equals(think["battleGoalID"].Value,baseline["battleGoalID"].Value),"Group alternate AI changes combat script");
    foreach(var (f,limit) in CageAI.Limits){double v=Math.Min(limit,Math.Min(Convert.ToDouble(think[f].Value),Convert.ToDouble(baseline[f].Value)));Check(v>=0&&double.IsFinite(v),"Invalid group AI range");Set(ai,f,v);}
    Replace(tables["NpcThinkParam"],ai);
   }
  }
 }
 static void Replace(PARAM t,PARAM.Row row){var old=t.Rows.SingleOrDefault(r=>r.ID==row.ID);if(old==null)t.Rows.Add(row);else t.Rows[t.Rows.IndexOf(old)]=row;}
 static EMEVD.Instruction I(int bank,int id,params object[] a)=>new(bank,id,a);
 static EMEVD.Instruction Flag(int group,uint id,bool on)=>I(3,0,(sbyte)group,(byte)(on?1:0),(byte)0,id);
 static EMEVD.Instruction Shared(uint id,bool on)=>I(2003,69,(byte)0,id,(byte)(on?1:0));
 static EMEVD.Instruction Local(uint id,bool on)=>I(2003,66,(byte)0,id,(byte)(on?1:0));
 static EMEVD.Instruction Wait(int g)=>I(0,0,(sbyte)0,(byte)1,(sbyte)g);
 static EMEVD.Instruction Restart()=>I(1000,4,(byte)1);
 static EMEVD.Instruction Hp(int g,uint actor,int comparison,int value)=>I(4,14,(sbyte)g,actor,(sbyte)comparison,value,(sbyte)0,1f);
 static EMEVD.Instruction Anim(uint actor,int id,bool loop=false)=>I(2003,18,actor,id,(byte)(loop?1:0),(byte)0,(byte)0,(byte)0,1f);
 static uint Actor(uint prefix,Member m)=>prefix+9000+(uint)m.index;
 static uint Shown(uint prefix,Member m)=>prefix+7750+(uint)m.index;
 static uint Dead(uint prefix,Member m)=>prefix+7900+(uint)m.index;
 static uint Target(uint prefix,Group g)=>prefix+5900+(uint)g.choice;
 static uint RewardFlag(uint prefix,Group g)=>prefix+7850+(uint)g.choice;
 static void Conditions(EMEVD.Event e,uint p,Group g){
  e.Instructions.AddRange([Flag(1,p+7404,true),Flag(1,p+7500+(uint)g.choice,true),I(4,0,(sbyte)1,20000u,(byte)0,(sbyte)0,1f),I(3,3,(sbyte)1,(byte)1,20000u,Target(p,g),50f,1)]);
  // Dead members need no new intro on reconnect. Every living member must
  // have completed this peer's reveal before its group is put on screen.
  foreach(var (m,i) in g.members.Where(m=>m.combat).Select((m,i)=>(m,i))){
   int child=-i-1;e.Instructions.AddRange([Flag(child,Shown(p,m),true),Flag(child,Dead(p,m),true),I(0,0,(sbyte)1,(byte)1,(sbyte)child),Hp(-10,Actor(p,m),2,0)]);
  }
  e.Instructions.Add(I(0,0,(sbyte)1,(byte)1,(sbyte)-10));
 }
 static void EndConditions(EMEVD.Event e,uint p,Group g){
  foreach(var m in g.members.Where(m=>m.combat))e.Instructions.Add(Flag(5,Dead(p,m),true));
  e.Instructions.AddRange([I(0,0,(sbyte)-12,(byte)1,(sbyte)5),Flag(-12,p+7404,false),Flag(-12,p+7500+(uint)g.choice,false),I(4,0,(sbyte)-12,20000u,(byte)1,(sbyte)0,1f),I(3,3,(sbyte)-12,(byte)0,20000u,Target(p,g),100f,1)]);
  // Removal of a live member invalidates this local display, rather than
  // silently keeping a cached bar associated with unloaded actors.
  foreach(var (m,i) in g.members.Where(m=>m.combat).Select((m,i)=>(m,i))){int child=i+6;e.Instructions.AddRange([Flag(child,Dead(p,m),false),Flag(child,Shown(p,m),false),I(0,0,(sbyte)-12,(byte)1,(sbyte)child)]);}
 }
 static EMEVD.Event Hud(uint p,Group g){
  if(g.bars.Length==1)return SharedHud(p,g);
  var e=new EMEVD.Event(p+9800+(uint)g.choice,EMEVD.Event.RestBehaviorType.Restart);e.Instructions.Add(I(2000,2,(byte)0));
  Conditions(e,p,g);e.Instructions.Add(Wait(1));
  // Native three-bar encounters reserve BOTH native flags. Reserve the whole
  // set without yielding; one event owns every slot it will later release.
  e.Instructions.AddRange([Flag(2,5000,false),Flag(2,5001,false),I(0,0,(sbyte)-13,(byte)1,(sbyte)2)]);EndConditions(e,p,g);e.Instructions.AddRange([I(0,0,(sbyte)-13,(byte)1,(sbyte)-12),Wait(-13)]);
  Conditions(e,p,g);e.Instructions.AddRange([Flag(1,5000,false),Flag(1,5001,false),I(1000,2,(byte)1,(byte)0,(sbyte)1),Local(5000,true),Local(5001,true)]);
  foreach(var (b,slot) in g.bars.Select((b,s)=>(b,s))){uint target=b.member<0?Target(p,g):Actor(p,g.members[b.member]);if(b.member>=0)e.Instructions.Add(I(1003,1,(byte)2,(byte)1,(byte)0,Dead(p,g.members[b.member])));e.Instructions.AddRange([I(2003,11,(sbyte)1,target,(short)slot,b.name),I(2003,102,(sbyte)1,b.name,target)]);}
  EndConditions(e,p,g);e.Instructions.AddRange([Wait(-12),I(1000,7,(byte)1,(byte)0,(sbyte)5),I(1001,0,Cages.VictoryDelaySeconds)]);
  foreach(var (b,slot) in g.bars.Select((b,s)=>(b,s)))e.Instructions.Add(I(2003,11,(sbyte)0,b.member<0?Target(p,g):Actor(p,g.members[b.member]),(short)slot,b.name));
  e.Instructions.AddRange([Local(5000,false),Local(5001,false),I(1001,1,1),Restart()]);return e;
 }
 static EMEVD.Event SharedHud(uint p,Group g){
  var bar=g.bars.Single();Check(bar.member<0,"One-bar group must target its native group");uint target=Target(p,g);
  var e=new EMEVD.Event(p+9800+(uint)g.choice,EMEVD.Event.RestBehaviorType.Restart);e.Instructions.Add(I(2000,2,(byte)0));
  Conditions(e,p,g);e.Instructions.Add(Wait(1));
  e.Instructions.AddRange([Flag(-14,5000,false),Flag(-15,5001,false),I(0,0,(sbyte)-13,(byte)1,(sbyte)-14),I(0,0,(sbyte)-13,(byte)1,(sbyte)-15)]);
  EndConditions(e,p,g);e.Instructions.AddRange([I(0,0,(sbyte)-13,(byte)1,(sbyte)-12),Wait(-13)]);
  Conditions(e,p,g);e.Instructions.AddRange([I(1000,2,(byte)1,(byte)0,(sbyte)1),I(1000,7,(byte)3,(byte)0,(sbyte)-14),Local(5000,true),I(2003,11,(sbyte)1,target,(short)0,bar.name),I(1000,3,(byte)2),Local(5001,true),I(2003,11,(sbyte)1,target,(short)1,bar.name),I(2003,102,(sbyte)1,bar.name,target)]);
  EndConditions(e,p,g);e.Instructions.AddRange([Wait(-12),I(1000,7,(byte)1,(byte)0,(sbyte)5),I(1001,0,Cages.VictoryDelaySeconds),I(1000,7,(byte)3,(byte)0,(sbyte)-14),I(2003,11,(sbyte)0,target,(short)0,bar.name),Local(5000,false),I(1000,3,(byte)2),I(2003,11,(sbyte)0,target,(short)1,bar.name),Local(5001,false),I(1001,1,1),Restart()]);return e;
 }
 public static EMEVD.Event Birth(uint p,Group g,Member m,bool stableSnapshot=true){
  uint actor=Actor(p,m),shown=Shown(p,m),choice=p+7500+(uint)g.choice;
  var e=new EMEVD.Event(p+9900+(uint)m.index,EMEVD.Event.RestBehaviorType.Restart);
  e.Instructions.AddRange([I(2000,2,(byte)0),Local(shown,false),Flag(1,p+7403,true),Flag(1,choice,true),Flag(5,p+7404,true),Flag(6,p+7404,false),I(0,0,(sbyte)-5,(byte)1,(sbyte)5),I(0,0,(sbyte)-5,(byte)1,(sbyte)6),I(0,0,(sbyte)1,(byte)1,(sbyte)-5),Wait(1),I(2004,29,actor,(byte)0),I(4,7,(sbyte)2,actor,(byte)1,(sbyte)0,1f),Hp(2,actor,2,0),Wait(2)]);
  Cages.Multiplayer(e,actor);int skip=e.Instructions.Count;e.Instructions.Add(I(1000,7,(byte)0,(byte)1,(sbyte)5));
  e.Instructions.AddRange([I(2004,5,actor,(byte)0),I(2006,3,2,actor,900,690047),I(1001,0,4f)]);
  if(g.birthAnimation.HasValue&&m.combat)e.Instructions.Add(Anim(actor,g.birthAnimation.Value,true));
  e.Instructions[skip]=I(1000,7,(byte)(e.Instructions.Count-skip-1),(byte)1,(sbyte)5);
  e.Instructions.AddRange([I(2004,5,actor,(byte)1),Local(shown,true),Flag(-2,choice,false),I(4,7,(sbyte)-2,actor,(byte)0,(sbyte)0,1f),Wait(-2),Local(shown,false),I(1001,1,1),Restart()]);if(stableSnapshot){Cages.StabilizeBirth(e);Cages.LocalGeneration(e,actor,p+3800+(uint)m.index,m.combat?Dead(p,m):null);CageUsage.Intro(e);CageCoop.Birth(e,actor);}return e;
 }
 static EMEVD.Event Death(uint p,Group g,Member m){
  var e=new EMEVD.Event(p+9100+(uint)m.index,EMEVD.Event.RestBehaviorType.Restart);
  e.Instructions.AddRange([I(2000,2,(byte)0),Cages.HostOnly(),Flag(1,p+7500+(uint)g.choice,true),Flag(1,p+7404,true),Wait(1),Hp(0,Actor(p,m),5,0),Shared(Dead(p,m),true),Flag(0,p+7500+(uint)g.choice,false),I(1001,1,1),Restart()]);return e;
 }
 static EMEVD.Event Battle(uint p,Group g,Dictionary<string,PARAM> tables){
  uint choice=p+7500+(uint)g.choice,cooling=p+7550+(uint)g.choice,red=p+7650+(uint)g.choice,decided=p+7700+(uint)g.choice;
  var e=new EMEVD.Event(p+9710+(uint)g.choice,EMEVD.Event.RestBehaviorType.Restart);e.Instructions.Add(I(2000,2,(byte)0));
  // Per-peer startup loading is owned by Birth; battle begins with host authority.
  e.Instructions.AddRange([Cages.HostOnly(),Shared(cooling,false)]);
  e.Instructions.AddRange([Flag(1,p+7403,true),Flag(1,choice,true),Wait(1)]);
  foreach(var m in g.members)e.Instructions.AddRange([I(2004,29,Actor(p,m),(byte)0),I(4,7,(sbyte)2,Actor(p,m),(byte)1,(sbyte)0,1f),Hp(2,Actor(p,m),2,0)]);
  e.Instructions.Add(Wait(2));foreach(var m in g.members){e.Instructions.Add(I(2004,28,Actor(p,m),8192));Cages.Multiplayer(e,Actor(p,m));}
  // The library calls TeamFight +0x0c "LeaderEntityID", but the native getter
  // e3a470 remaps it as an event flag; 37f310 tests and sets this claim flag.
  // Rearm only AFTER every newly generated actor is alive, so old corpses
  // cannot produce a second reward during the cleanup/next-spawn gap.
  e.Instructions.Add(Shared(RewardFlag(p,g),false));foreach(var m in g.members.Where(m=>m.combat))e.Instructions.Add(Shared(Dead(p,m),false));Cages.Difficulty(e,Target(p,g),g.members[0].entity);
  var first=g.members.First(m=>m.combat);Cages.Mutate(e,Actor(p,first),tables["NpcParam"].Rows.Single(r=>r.ID==95300000+first.index),tables,Cages.MutationMaps(tables),red,decided);
  e.Instructions.Add(I(1003,1,(byte)(g.members.Length-1),(byte)0,(byte)0,red));
  foreach(var m in g.members.Where(m=>m!=first))e.Instructions.Add(I(2004,8,Actor(p,m),Cages.Mutation));
  foreach(var m in g.members)e.Instructions.AddRange([I(2004,1,Actor(p,m),(byte)0),I(2004,34,Actor(p,m),(byte)1,(sbyte)0)]);
  e.Instructions.Add(I(1001,0,CageUsage.IntroSeconds));foreach(var m in g.members)e.Instructions.AddRange([I(2004,39,Actor(p,m),(byte)1),I(2004,1,Actor(p,m),(byte)1)]);
  e.Instructions.AddRange([Shared(p+7404,true),I(2010,12,28u,(byte)0),Anim(p+1200,110,true),Anim(p+1210,1,true),I(4,23,(sbyte)0,Target(p,g),(sbyte)5,0,(sbyte)0,1f),I(1001,0,Cages.VictoryDelaySeconds),I(2010,2,Target(p,g),5,888880000),I(2007,19,1030),I(2003,101,1),I(2003,31,8145u,4u,7u),Shared(cooling,true),Anim(p+1210,10,true),I(2003,114,(byte)6,(byte)0),Shared(red,false),Shared(decided,false),Shared(p+7404,false),Shared(choice,false),Shared(p+7403,false),Shared(p+7402,false),Shared(p+7401,false),I(1001,0,Cages.CleanupDelaySeconds-Cages.VictoryDelaySeconds)]);
  foreach(var m in g.members)e.Instructions.AddRange([I(2004,29,Actor(p,m),(byte)1),I(2004,5,Actor(p,m),(byte)0)]);
  e.Instructions.AddRange([Shared(cooling,false),I(1001,1,1),Restart()]);CageCoop.Battle(e,p,g.members.Select(m=>Actor(p,m)).ToArray());return e;
 }
 static void Remove(object param,object entry){foreach(var property in param.GetType().GetProperties()){if(property.GetValue(param) is IList l&&l.Contains(entry)){l.Remove(entry);return;}}throw new InvalidDataException("MSB collection not found");}
 static void Add(MSB_NR map,MSB_NR.Event entry)=>map.Events.Add(entry);
 public static void Build(string root,Dictionary<string,PARAM> tables,string? output=null,string? basis=null){
  Directory.CreateDirectory(Path.Combine(root,".work/cage-groups"));
  var c=Load(root);output??=Path.Combine(root,"dist/mod");Directory.CreateDirectory(Path.Combine(output,"map/mapstudio"));Directory.CreateDirectory(Path.Combine(output,"event"));
  var genSourceMap=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio/m52_42_00_00.msb.dcx"));var genSource=genSourceMap.Events.Generators.Single(g=>g.EntityID==52423810);var regionSource=genSourceMap.Regions.GetEntries().Single(r=>r.Name==genSource.SpawnRegionNames[0]);var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block:00}_00_00";uint p=(uint)(4600+block)*10000;
   basis??=Path.Combine(root,"dist/mod");
   string inputMap=Path.Combine(basis,"map/mapstudio",name+".msb.dcx"),inputEvent=Path.Combine(basis,"event",name+".emevd.dcx"),outMap=Path.Combine(output,"map/mapstudio",name+".msb.dcx"),outEvent=Path.Combine(output,"event",name+".emevd.dcx");
   string inputMapHash=Hash(inputMap),inputEventHash=Hash(inputEvent);
   var map=MSB_NR.Read(inputMap);var beforeMap=map.Write();var models=map.Models.GetEntries().Select(m=>m.Name).ToHashSet();var before=EMEVD.Read(inputEvent);var events=EMEVD.Read(inputEvent);var ctor=events.Events.Single(e=>e.ID==0);var oldCtor=JsonSerializer.Serialize(Cages.Dump(ctor));
   Check(!map.Parts.Enemies.Any(e=>e.EntityData.EntityID>=p+9037&&e.EntityData.EntityID<p+9100),"Group patch requires a single-boss base; use the full build to regenerate an installed group version");
   var selectors=events.Events.Where(e=>e.ID==p+9700).ToArray();Check(selectors.Length==1&&Cages.Same(selectors[0],Cages.Selector(p,37)),"Unexpected selector base");events.Events[events.Events.IndexOf(selectors[0])]=Cages.Selector(p,c.encounterCount);
   // Correct the bound name for existing single-actor bars too.
   var singleBosses=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
   foreach(var (boss,index) in singleBosses.Select((b,i)=>(b,i))){var hud=events.Events.Single(e=>e.ID==p+9800+(uint)index);var link=hud.Instructions.Single(i=>i.Bank==2003&&i.ID==102);int old=BitConverter.ToInt32(link.ArgData,4);Check(old==-1||old==boss.healthBarName,"Unexpected single HUD association");BitConverter.GetBytes(boss.healthBarName).CopyTo(link.ArgData,4);}
   var originalFlags=EMEVD.Read(Path.Combine(root,"assets/original/event",name+".emevd.dcx"));var flags=c.groups.SelectMany(g=>new[]{p+7500+(uint)g.choice,p+7550+(uint)g.choice,p+7650+(uint)g.choice,p+7700+(uint)g.choice,RewardFlag(p,g)}.Concat(g.members.SelectMany(m=>new[]{Shown(p,m),Dead(p,m)}))).ToHashSet();
   CageNetwork.CheckDraftFlags(originalFlags,p,c.encounterCount);
   Check(flags.All(f=>f/1000%10==7)&&!originalFlags.Events.Any(e=>flags.Contains((uint)e.ID)||e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>flags.Contains(BitConverter.ToUInt32(i.ArgData,n*4))))),"Group state collides with original events or unallocated flags");
   var addedEvents=new List<EMEVD.Event>();var actorReports=new List<object>();var rewardReports=new List<object>();
   float yaw=map.Regions.GetEntries().Single(r=>r.Name=="NRHex_cage_spawn_0").Rotation.Y;
   foreach(var g in c.groups){
    var src=MSB_NR.Read(Source(root,g));var copied=new List<MSB_NR.Part.Enemy>();
    foreach(var m in g.members){
     uint actor=Actor(p,m);var enemy=(MSB_NR.Part.Enemy)src.Parts.Enemies.Single(e=>e.Name==m.sourceName).DeepCopy();enemy.Name=$"{m.model}_{9500+m.index}";enemy.InstanceID=9500+m.index;enemy.EntityData.EntityID=actor;enemy.EntityData.EntityGroupIDs=new uint[8];if(m.combat)enemy.EntityData.EntityGroupIDs[0]=Target(p,g);enemy.EntityData.Variation=-1;enemy.EntityData.RandomAppearParamID=0;enemy.NpcParamId=95300000+m.index;enemy.NpcThinkParamId=95300000+m.index;enemy.Position=new Vector3(m.offset[0],m.offset[1],m.offset[2]);enemy.Rotation=new Vector3(0,yaw,0);enemy.HitPartName=null;enemy.PatrolRouteName=null;map.Parts.Enemies.Add(enemy);copied.Add(enemy);
     if(!map.Models.GetEntries().Any(v=>v.Name==m.model))map.Models.Add(src.Models.GetEntries().Single(v=>v.Name==m.model).DeepCopy());
     var region=regionSource.DeepCopy();region.Name=$"NRHex_cage_spawn_{m.index}";region.RegionID=100+m.index;region.Position=enemy.Position;region.Rotation=enemy.Rotation;region.Common_EntityID=p+3900+(uint)m.index;map.Regions.Add(region);
     var gen=(MSB_NR.Event.Generator)genSource.DeepCopy();gen.Name=$"NRHex_cage_generator_{m.index}";gen.EventID=100+m.index;gen.EntityID=p+3800+(uint)m.index;gen.MaxNum=1;gen.MinGenNum=1;gen.MaxGenNum=1;gen.SpawnRegionNames=new string[8];gen.SpawnRegionNames[0]=region.Name;gen.SpawnPartNames=new string[32];gen.SpawnPartNames[0]=enemy.Name;map.Events.Generators.Add(gen);addedEvents.Add(Birth(p,g,m));if(m.combat)addedEvents.Add(Death(p,g,m));
     actorReports.Add(new{m.index,g.choice,entity=actor,m.combat,shown=Shown(p,m),dead=Dead(p,m),npc=enemy.NpcParamId,think=enemy.NpcThinkParamId,position=m.offset,sourceNpc=m.npc,sourceThink=m.think});
    }
    foreach(var r in g.riding){var ride=(MSB_NR.Event.Riding)src.Events.GetEntries().Single(e=>e.Name==r.sourceName&&e is MSB_NR.Event.Riding).DeepCopy();ride.Name=$"NRHex_cage_riding_{g.choice}_{r.rider}";ride.EventID=200+g.members[r.rider].index;ride.RiderPartName=copied[r.rider].Name;ride.MountPartName=copied[r.mount].Name;Add(map,ride);}
    var nativeReward=(MSB_NR.Event.TeamFight)src.Events.GetEntries().Single(e=>e is MSB_NR.Event.TeamFight t&&t.TargetEntityGroupID==g.target);var team=(MSB_NR.Event.TeamFight)nativeReward.DeepCopy();
    Check(JsonSerializer.SerializeToElement(nativeReward).EnumerateObject().All(v=>!g.reward.TryGetProperty(v.Name,out var expected)||JsonElement.DeepEquals(expected,v.Value)),"Group reward evidence changed");
    team.Name=$"NRHex_cage_reward_{g.choice}";team.EventID=300+g.choice;team.TargetEntityGroupID=(int)Target(p,g);
    team.LeaderEntityID=(int)RewardFlag(p,g);
    if(g.map.StartsWith("m48_")||g.map.StartsWith("m49_")){Check(team.Unk18 is 6850000 or 6851000&&team.Unk20==team.Unk18+1000,"Missing tower group weapon override");team.FirstItemLot=team.Unk18;team.UnkItemLot=team.Unk18;team.Unk14=0;team.Unk18=-1;team.Unk1C=0;team.Unk20=-1;}
    foreach(int lot in new[]{team.FirstItemLot,team.SecondItemLot,team.UnkItemLot}.Where(v=>v>0)){
     var rows=tables["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=lot&&r.ID<lot+10).ToArray();
     Check(rows.Length>0&&rows.All(r=>N(r,"getItemFlagId")==0)&&rows.Any(r=>Enumerable.Range(1,8).Any(i=>N(r,$"lotItemCategory{i:00}")==7&&N(r,$"lotItemBasePoint{i:00}")>0)),"Group reward must refer to a native repeatable ability-ball table");
    }
    Add(map,team);rewardReports.Add(new{g.choice,native=JsonSerializer.SerializeToElement(nativeReward),copy=JsonSerializer.SerializeToElement(team)});addedEvents.Add(Battle(p,g,tables));addedEvents.Add(Hud(p,g));
   }
   foreach(var e in addedEvents){Check(!events.Events.Any(v=>v.ID==e.ID),"Group event collision");events.Events.Add(e);ctor.Instructions.Insert(0,I(2000,0,0,(uint)e.ID,0));}
   map.Write(outMap);events.Write(outEvent);var checkedMap=MSB_NR.Read(outMap);var checkedEvents=EMEVD.Read(outEvent);
   Check(checkedMap.Parts.Enemies.Count==map.Parts.Enemies.Count&&checkedMap.Events.Generators.Count==84,"Group map readback count");
   foreach(var m in c.groups.SelectMany(g=>g.members)){var enemy=checkedMap.Parts.Enemies.Single(e=>e.EntityData.EntityID==Actor(p,m));Check(enemy.NpcParamId==95300000+m.index&&enemy.NpcThinkParamId==95300000+m.index&&enemy.EntityData.Variation==-1&&enemy.Position==new Vector3(m.offset[0],m.offset[1],m.offset[2]),"Group actor binding readback");}
   foreach(var g in c.groups){
    foreach(var m in g.members)Check(checkedMap.Parts.Enemies.Single(e=>e.EntityData.EntityID==Actor(p,m)).EntityData.EntityGroupIDs.SequenceEqual(new uint[]{m.combat?Target(p,g):0,0,0,0,0,0,0,0}),"Group membership readback");
    foreach(var r in g.riding){var ride=checkedMap.Events.GetEntries().OfType<MSB_NR.Event.Riding>().Single(e=>e.Name==$"NRHex_cage_riding_{g.choice}_{r.rider}");Check(ride.RiderPartName==checkedMap.Parts.Enemies.Single(e=>e.EntityData.EntityID==Actor(p,g.members[r.rider])).Name&&ride.MountPartName==checkedMap.Parts.Enemies.Single(e=>e.EntityData.EntityID==Actor(p,g.members[r.mount])).Name,"Riding relation readback");}
    var reward=checkedMap.Events.GetEntries().OfType<MSB_NR.Event.TeamFight>().Single(e=>e.Name==$"NRHex_cage_reward_{g.choice}");Check(JsonElement.DeepEquals(JsonSerializer.SerializeToElement(reward),JsonSerializer.SerializeToElement(map.Events.GetEntries().Single(e=>e.Name==reward.Name),reward.GetType())),"Group reward readback");
   }
   foreach(var e in checkedMap.Parts.Enemies.Where(e=>e.EntityData.EntityID>=p+9037&&e.EntityData.EntityID<p+9084).ToArray())checkedMap.Parts.Enemies.Remove(e);
   foreach(var e in checkedMap.Events.GetEntries().Where(e=>e.Name.StartsWith("NRHex_cage_")&&(e.Name.StartsWith("NRHex_cage_reward_")||e.Name.StartsWith("NRHex_cage_riding_")||e is MSB_NR.Event.Generator v&&v.EntityID>=p+3837)).ToArray())Remove(checkedMap.Events,e);
   foreach(var r in checkedMap.Regions.GetEntries().Where(r=>r.Common_EntityID>=p+3937&&r.Common_EntityID<p+3984).ToArray())Remove(checkedMap.Regions,r);
   foreach(var m in checkedMap.Models.GetEntries().Where(m=>!models.Contains(m.Name)).ToArray())Remove(checkedMap.Models,m);
   Check(checkedMap.Write().SequenceEqual(beforeMap),"Group patch changed preexisting map content");
   Check(events.Events.Count==checkedEvents.Events.Count&&events.Events.Zip(checkedEvents.Events).All(x=>Cages.Same(x.First,x.Second)),"Group event readback failed");
   Check(before.Format==checkedEvents.Format&&before.Compression==checkedEvents.Compression&&before.StringData.SequenceEqual(checkedEvents.StringData)&&before.LinkedFileOffsets.SequenceEqual(checkedEvents.LinkedFileOffsets),"Group event metadata changed");
   var stripped=EMEVD.Read(outEvent);stripped.Events.RemoveAll(e=>addedEvents.Any(v=>v.ID==e.ID));var strippedCtor=stripped.Events.Single(e=>e.ID==0);strippedCtor.Instructions.RemoveRange(0,addedEvents.Count);Check(JsonSerializer.Serialize(Cages.Dump(strippedCtor))==oldCtor,"Group constructor changed existing initializers");
   foreach(var e in before.Events.Where(e=>e.ID!=0&&e.ID!=p+9700&&(e.ID<p+9800||e.ID>=p+9837)))Check(Cages.Same(e,checkedEvents.Events.Single(v=>v.ID==e.ID)),"Group patch changed unrelated event");
   reports.Add(new{mapName=name,inputMapSha256=inputMapHash,inputEventSha256=inputEventHash,mapSha256=Hash(outMap),eventSha256=Hash(outEvent),actorReports,rewardReports,selector=Cages.Dump(checkedEvents.Events.Single(e=>e.ID==p+9700)),singleEvents=checkedEvents.Events.Where(e=>e.ID==p+9700||e.ID>=p+9710&&e.ID<p+9747||e.ID>=p+9800&&e.ID<p+9837||e.ID>=p+9900&&e.ID<p+9937).Select(Cages.Dump),events=addedEvents.Select(e=>Cages.Dump(checkedEvents.Events.Single(v=>v.ID==e.ID))),preexistingMapContentPreserved=true,unrelatedEventsPreserved=true});
  }
  var parameters=c.groups.SelectMany(g=>g.members).Select(m=>{var npc=tables["NpcParam"].Rows.Single(r=>r.ID==95300000+m.index);var source=tables["NpcParam"].Rows.Single(r=>r.ID==m.npc);Check(CageCoop.SameNativeNpc(npc,source),"Group NPC combat/scaling/reward changed");return new{m.index,m.npc,clone=95300000+m.index,fields=npc.Cells.Where(v=>v!=null&&new[]{"hp","getSoul","rewardItemLot_2","multiPlayCorrectionParamId","chaosMatchingCorrectParamId","chaosMatchingSpEffectSetParamId","chaosMatchingRewardLotId","excludeGroupRewardCheck"}.Contains(v.Def.InternalName)).ToDictionary(v=>v.Def.InternalName,v=>v.Value),ai=CageAI.Limits.Keys.ToDictionary(f=>f,f=>tables["NpcThinkParam"].Rows.Single(r=>r.ID==95300000+m.index)[f].Value)};});
  File.WriteAllText(Path.Combine(root,".work/cage-groups/built.json"),JsonSerializer.Serialize(new{version="0.17.27",c.encounterCount,c.actorCount,output,reports,parameters,allGroupMembersRetainNativeStats=true,nativeTeamFightRewards=true,ridingRelationsRetained=true},new JsonSerializerOptions{WriteIndented=true}));
  if(Path.GetFullPath(basis!)==Path.GetFullPath(output)){
   string path=Path.Combine(root,".work/cage/built.json");var single=JsonNode.Parse(File.ReadAllText(path))!;var group=JsonNode.Parse(File.ReadAllText(Path.Combine(root,".work/cage-groups/built.json")))!;
   foreach(var r in single["reports"]!.AsArray()){var match=group["reports"]!.AsArray().Single(g=>g!["mapName"]!.GetValue<string>()==r!["mapName"]!.GetValue<string>());r!["events"]=match!["singleEvents"]!.DeepClone();r["outputSha256"]=match["eventSha256"]!.DeepClone();r["mapSha256"]=match["mapSha256"]!.DeepClone();r["encounterCount"]=c.encounterCount;}
   File.WriteAllText(path,single.ToJsonString(new JsonSerializerOptions{WriteIndented=true}));
  }
 }
}
