using System.Collections;
using System.Numerics;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

// An additive MMV overlay. The existing cage event builders remain the source
// of lifecycle/network behavior; only their identities are relocated here.
static class MmvCages {
 public record Member(int index,string sourceName,uint entity,int npc,int think,string model,bool combat);
 public record Encounter(int index,string map,uint target,string name,int healthBarName,int bgm,string mapSha256,string eventSha256,Member[] members,int? auxiliaryEvent=null);
 public record Catalog(int schema,string scope,Encounter[] encounters);
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
 static string FileName(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static int N(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
 static EMEVD.Instruction I(int b,int id,params object[] v)=>new(b,id,v);
 static string Signature(PARAM.Row r)=>JsonSerializer.Serialize(r.Cells.Select(c=>c.Value),new JsonSerializerOptions{NumberHandling=System.Text.Json.Serialization.JsonNumberHandling.AllowNamedFloatingPointLiterals});
 static void Remove(object container,object entry){foreach(var p in container.GetType().GetProperties().Where(p=>p.GetIndexParameters().Length==0))if(p.GetValue(container) is IList list&&list.Contains(entry)){list.Remove(entry);return;}throw new InvalidDataException("Missing map entry collection");}
 static IEnumerable<uint> Values(EMEVD.Event e)=>e.Instructions.SelectMany(i=>Enumerable.Range(0,i.ArgData.Length/4).Select(n=>BitConverter.ToUInt32(i.ArgData,n*4)));
 static EMEVD.Event Relocate(EMEVD.Event e,uint id,Dictionary<uint,uint> remap){
  Check(e.Parameters.Count==0,"Parameterized cage event cannot be relocated");e.ID=id;
  foreach(var i in e.Instructions)for(int n=0;n+4<=i.ArgData.Length;n+=4)if(remap.TryGetValue(BitConverter.ToUInt32(i.ArgData,n),out uint target))BitConverter.GetBytes(target).CopyTo(i.ArgData,n);
  return e;
 }
 static EMEVD.Event Selector(uint p,uint draft,uint[] choices,uint[] cooling){
  var e=Cages.Selector(p,choices.Length);
  // Relocate by opcode/operand, not overlapping old contiguous flag ranges.
  foreach(var i in e.Instructions){
   if(i.Bank==2003&&i.ID is 22 or 17){BitConverter.GetBytes(draft).CopyTo(i.ArgData,0);BitConverter.GetBytes(draft+(uint)choices.Length-1).CopyTo(i.ArgData,4);}
   else if(i.Bank==1003&&i.ID==1){uint n=BitConverter.ToUInt32(i.ArgData,4)-(p+7100);Check(n<choices.Length,"Draft operand drift");BitConverter.GetBytes(draft+n).CopyTo(i.ArgData,4);}
   else if(i.Bank==1003&&i.ID==101&&i.ArgData[0]==1){uint n=BitConverter.ToUInt32(i.ArgData,4)-(p+7550);Check(n<choices.Length,"Cooling operand drift");BitConverter.GetBytes(cooling[n]).CopyTo(i.ArgData,4);}
   else if(i.Bank==2003&&i.ID==69){uint f=BitConverter.ToUInt32(i.ArgData,4);if(f>=p+7500&&f<p+7500+choices.Length)BitConverter.GetBytes(choices[f-p-7500]).CopyTo(i.ArgData,4);}
  }
  return e;
 }
 public static void Audit(string root,string mmv){
  var catalog=JsonSerializer.Deserialize<Catalog>(File.ReadAllText(Path.Combine(root,"compat/mmv/cage-encounters.json")))!;
  var reports=catalog.encounters.Select(e=>{
   var map=MSB_NR.Read(Path.Combine(mmv,"mod/map/mapstudio",e.map+".msb.dcx"));var names=e.members.Select(m=>m.sourceName).ToHashSet();
   var riding=map.Events.GetEntries().OfType<MSB_NR.Event.Riding>().Where(r=>names.Contains(r.RiderPartName??"")||names.Contains(r.MountPartName??"")).ToArray();
   return new{e.index,e.map,riding=riding.Select(r=>new{r.Name,r.RiderPartName,r.MountPartName}),missing=map.Parts.Enemies.Where(p=>!names.Contains(p.Name)&&riding.Any(r=>r.RiderPartName==p.Name||r.MountPartName==p.Name)).Select(p=>new{sourceName=p.Name,entity=p.EntityData.EntityID,npc=p.NpcParamId,think=p.NpcThinkParamId,model=p.ModelName,combat=false})};
  }).ToArray();
  File.WriteAllText(Path.Combine(root,"work/compat/mmv/cage-riding.json"),JsonSerializer.Serialize(reports,new JsonSerializerOptions{WriteIndented=true}));
 }
 public static void Stage(string root,string mmv,Func<string,Memory<byte>,PARAM> parse){
  string output=Path.Combine(root,"work/compat/mmv/staged"),mod=Path.Combine(output,"mod");
  var catalog=JsonSerializer.Deserialize<Catalog>(File.ReadAllText(Path.Combine(root,"compat/mmv/cage-encounters.json")))!;
  var encounters=catalog.encounters;var members=encounters.SelectMany(e=>e.members).ToArray();
  Check(catalog.schema==1&&encounters.Select(e=>e.index).SequenceEqual(Enumerable.Range(0,encounters.Length))&&members.Select(m=>m.index).SequenceEqual(Enumerable.Range(0,members.Length)),"Unstable MMV cage catalog indices");
  Check(members.Length<=200&&encounters.Length<=100,"MMV cage identity space exhausted");
  var inputs=new Dictionary<string,string>();var sources=new Dictionary<string,MSB_NR>();
  foreach(var e in encounters){
   Check(e.map.StartsWith("m45_")||e.map.StartsWith("m46_")||new[]{("m60_43_38_10",1038100890u),("m60_43_39_20",1039200890u),("m60_45_36_30",1056300820u)}.Contains((e.map,e.target)),"Only reviewed field/gaol encounters; night bosses are excluded");
   string path=Path.Combine(mmv,"mod/map/mapstudio",e.map+".msb.dcx"),ev=Path.Combine(mmv,"mod/event",e.map+".emevd.dcx");
   Check(Hash(path)==e.mapSha256&&Hash(ev)==e.eventSha256,"MMV cage source changed: "+e.map);inputs[path]=e.mapSha256;inputs[ev]=e.eventSha256;
   if(!sources.ContainsKey(e.map))sources[e.map]=MSB_NR.Read(path);
   var calls=EMEVD.Read(ev).Events.Single(x=>x.ID==0).Instructions.Where(i=>i.Bank==2000&&i.ID==6).ToArray();
   Check(calls.Any(i=>i.ArgData.Length>=20&&BitConverter.ToInt32(i.ArgData,4)==90015000&&BitConverter.ToUInt32(i.ArgData,12)==e.target&&BitConverter.ToInt32(i.ArgData,16)==e.healthBarName),"Missing native HUD evidence");
   Check(calls.Any(i=>i.ArgData.Length==28&&BitConverter.ToInt32(i.ArgData,4)==90015030&&BitConverter.ToUInt32(i.ArgData,12)==e.target&&BitConverter.ToInt32(i.ArgData,20)==e.bgm),"Missing native BGM evidence");
   Check(e.members.Count(m=>m.combat) is >0 and <=9,"Too many combat condition groups");
   Check(e.members.Select(m=>m.entity).Distinct().Count()==e.members.Length,"Alternative actors sharing an entity must be separate encounters");
   foreach(var m in e.members){var part=sources[e.map].Parts.Enemies.Single(p=>p.Name==m.sourceName);Check(part.EntityData.EntityID==m.entity&&part.NpcParamId==m.npc&&part.NpcThinkParamId==m.think&&part.ModelName==m.model,"MMV cage actor identity drift");}
  }
  string regulation=Path.Combine(mod,"regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(regulation);
  string[] names=["NpcParam","NpcThinkParam","MultiPlayCorrectionParam","SpEffectParam","ChaosMatchingCorrectParam","ChaosMatchingMutationEnemyTableParam","ChaosMatchingMutationCategoryParam","LotBaseMapPatternFlag","ItemLotParam_enemy"];
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>FileName(f)==n+".param").Bytes));
  var original=tables.ToDictionary(x=>x.Key,x=>x.Value.Rows.ToDictionary(r=>r.ID,Signature));
  PARAM.Row Clone(string n,int source,int id){var old=tables[n].Rows.SingleOrDefault(r=>r.ID==id);if(old!=null)return old;var row=new PARAM.Row(tables[n].Rows.Single(r=>r.ID==source)){ID=id};tables[n].Rows.Add(row);return row;}
  var newNpcs=new List<PARAM.Row>();
  foreach(var m in members){
   int id=95300200+m.index;Check(!tables["NpcParam"].Rows.Any(r=>r.ID==id)&&!tables["NpcThinkParam"].Rows.Any(r=>r.ID==id),"MMV cage parameter collision");
   var npc=Clone("NpcParam",m.npc,id);Set(npc,"disableRespawn",0);Set(npc,"excludeGroupRewardCheck",m.combat?0:1);
   // TeamFight owns the one shared weapon/body reward. Individual rune awards
   // stay native; no individual item-ball or depth override may duplicate it.
   foreach(string f in new[]{"itemLotId_enemy","itemLotId_map","rewardItemLot_1","rewardItemLot_2","chaosMatchingRewardLotId","chaosMatchingItemLotId","sleepCollectorItemLotId_enemy","sleepCollectorItemLotId_map"})Set(npc,f,-1);
   newNpcs.Add(npc);var ai=Clone("NpcThinkParam",m.think,id);
   foreach(var (field,limit) in CageAI.Limits){double value=Math.Min(limit,Convert.ToDouble(ai[field].Value));Check(value>=0&&double.IsFinite(value),"Invalid MMV cage AI range");Set(ai,field,value);}
  }
  CageCoop.Parameters(tables,Clone,newNpcs.ToArray());
  var reports=new List<object>();var changedFiles=new Dictionary<string,string>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block}_00_00",mapPath=Path.Combine(mod,"map/mapstudio",name+".msb.dcx"),eventPath=Path.Combine(mod,"event",name+".emevd.dcx");uint p=(uint)(4600+block)*10000;
   var map=MSB_NR.Read(mapPath);var beforeMap=map.Write();var modelNames=map.Models.GetEntries().Select(m=>m.Name).ToHashSet();
   var events=EMEVD.Read(eventPath);var beforeEvents=EMEVD.Read(eventPath);var ctor=events.Events.Single(e=>e.ID==0);
   Check(Cages.Same(events.Events.Single(e=>e.ID==p+9700),Cages.Selector(p,50)),"MMV base selector changed");
   var occupied=events.Events.Where(e=>e.ID!=p+9700).SelectMany(Values).Concat(events.Events.Select(e=>(uint)e.ID)).ToHashSet();
   foreach(var team in map.Events.GetEntries().OfType<MSB_NR.Event.TeamFight>())occupied.Add((uint)team.LeaderEntityID);
   // Flag storage only exists in the map's 7xxx bank. Pick unused words after
   // inspecting BOTH mods' events. Never continue the 50-entry old arrays.
   uint draft=0;int count=50+encounters.Length;
   for(uint start=p+7000;start+count<=p+8000;start++)if(Enumerable.Range(0,count).All(i=>!occupied.Contains(start+(uint)i))){draft=start;break;}
   Check(draft!=0,"No contiguous MMV cage lottery storage");foreach(uint f in Enumerable.Range(0,count).Select(i=>draft+(uint)i))occupied.Add(f);
   uint Allocate(){for(uint f=p+7000;f<p+8000;f++)if(occupied.Add(f))return f;throw new InvalidDataException("MMV cage flag bank exhausted");}
   // There is exactly one active encounter per gaol. Mutation decisions are
   // cleared before the next choice; delayed cleanup never touches them.
   uint red=Allocate(),decided=Allocate();
   var choices=Enumerable.Range(0,50).Select(i=>p+7500+(uint)i).ToList();var cooling=Enumerable.Range(0,50).Select(i=>p+7550+(uint)i).ToList();
   var addedEvents=new List<EMEVD.Event>();var addedEntries=new List<(object Container,object Entry)>();var actorReports=new List<object>();var encounterReports=new List<object>();
   var regionTemplate=map.Regions.GetEntries().Single(r=>r.Name=="NRHex_cage_spawn_0");
   var generatorTemplate=map.Events.Generators.Single(g=>g.EntityID==p+3800);
   var rewardTemplate=map.Events.GetEntries().OfType<MSB_NR.Event.TeamFight>().Single(t=>t.Name=="NRHex_cage_reward_47");
   var eventIds=events.Events.Select(e=>e.ID).ToHashSet();uint nextEvent=p+8000;
   void Event(EMEVD.Event e,Dictionary<uint,uint> remap){while(eventIds.Contains(nextEvent))nextEvent++;Check(nextEvent<p+9000,"MMV cage event space exhausted");var next=Relocate(e,nextEvent++,remap);eventIds.Add(next.ID);events.Events.Add(next);addedEvents.Add(next);ctor.Instructions.Insert(0,I(2000,0,0,(uint)next.ID,0));}
   foreach(var e in encounters){
    uint choice=Allocate(),cool=Allocate(),reward=Allocate(),target=p+6000+(uint)e.index;choices.Add(choice);cooling.Add(cool);
    var fakeMembers=e.members.Select((m,i)=>new CageEncounters.Member(37+i,m.sourceName,m.entity,m.npc,m.think,-1,m.model,m.combat,[0,0,0])).ToArray();
    var fake=new CageEncounters.Group(37,e.map,e.target,e.name,e.mapSha256,e.eventSha256,"",fakeMembers,[new(-1,e.healthBarName)],[],default,null);
    var remap=new Dictionary<uint,uint>{{p+7537,choice},{p+7587,cool},{p+7687,red},{p+7737,decided},{p+7887,reward},{p+5937,target}};
    var parts=new Dictionary<string,MSB_NR.Part.Enemy>();
    foreach(var (m,i) in e.members.Select((m,i)=>(m,i))){
     uint actor=p+9200+(uint)m.index,generator=actor-5200,regionId=p+4200+(uint)m.index,shown=Allocate(),dead=m.combat?Allocate():0;
     remap[p+9037+(uint)i]=actor;remap[p+3837+(uint)i]=generator;remap[p+7787+(uint)i]=shown;if(m.combat)remap[p+7937+(uint)i]=dead;
     var enemy=(MSB_NR.Part.Enemy)sources[e.map].Parts.Enemies.Single(v=>v.Name==m.sourceName).DeepCopy();
     // Keep source relative placement for group members, centered and rotated
     // into the gaol. No source patrol/map collision references may survive.
     var anchor=sources[e.map].Parts.Enemies.Single(v=>v.Name==e.members[0].sourceName);
     var offset=enemy.Position-anchor.Position;float yaw=regionTemplate.Rotation.Y-anchor.Rotation.Y;
     offset=Vector3.Transform(offset,Matrix4x4.CreateRotationY(yaw*MathF.PI/180));
     Check(offset.Length()<50,"Source formation too wide for a cage");
     enemy.Name=$"NRHex_mmv_cage_{m.index}_{m.model}";enemy.InstanceID=9700+m.index;enemy.EntityData.EntityID=actor;enemy.EntityData.EntityGroupIDs=new uint[8];if(m.combat)enemy.EntityData.EntityGroupIDs[0]=target;
     enemy.EntityData.Variation=-1;enemy.EntityData.RandomAppearParamID=0;enemy.NpcParamId=95300200+m.index;enemy.NpcThinkParamId=95300200+m.index;
     enemy.Position=offset;enemy.Rotation=new Vector3(0,enemy.Rotation.Y+yaw,0);enemy.HitPartName=null;enemy.PatrolRouteName=null;
     Check(!map.Parts.Enemies.Any(v=>v.EntityData.EntityID==actor||v.Name==enemy.Name),"MMV actor collision");map.Parts.Enemies.Add(enemy);addedEntries.Add((map.Parts,enemy));parts[m.sourceName]=enemy;
     if(!map.Models.GetEntries().Any(v=>v.Name==m.model))map.Models.Add(sources[e.map].Models.GetEntries().Single(v=>v.Name==m.model).DeepCopy());
     var region=regionTemplate.DeepCopy();region.Name=$"NRHex_mmv_cage_spawn_{m.index}";region.RegionID=1000+m.index;region.Common_EntityID=regionId;region.Position=enemy.Position;region.Rotation=enemy.Rotation;
     Check(!map.Regions.GetEntries().Any(v=>v.Common_EntityID==regionId),"MMV region collision");map.Regions.Add(region);addedEntries.Add((map.Regions,region));
     var gen=(MSB_NR.Event.Generator)generatorTemplate.DeepCopy();gen.Name=$"NRHex_mmv_cage_generator_{m.index}";gen.EventID=1000+m.index;gen.EntityID=generator;gen.SpawnRegionNames=new string[8];gen.SpawnRegionNames[0]=region.Name;gen.SpawnPartNames=new string[32];gen.SpawnPartNames[0]=enemy.Name;
     Check(!map.Events.Generators.Any(v=>v.EntityID==generator),"MMV generator collision");map.Events.Generators.Add(gen);addedEntries.Add((map.Events,gen));
     actorReports.Add(new{m.index,m.npc,clone=enemy.NpcParamId,actor,generator,shown,dead,m.combat});
    }
    foreach(var ride in sources[e.map].Events.GetEntries().OfType<MSB_NR.Event.Riding>().Where(r=>parts.ContainsKey(r.RiderPartName??"")||parts.ContainsKey(r.MountPartName??""))){
     Check(parts.ContainsKey(ride.RiderPartName)&&parts.ContainsKey(ride.MountPartName),"Incomplete MMV riding encounter");
     var next=(MSB_NR.Event.Riding)ride.DeepCopy();next.Name=$"NRHex_mmv_cage_riding_{e.index}_{ride.Name}";next.EventID=2000+addedEntries.Count;next.RiderPartName=parts[ride.RiderPartName].Name;next.MountPartName=parts[ride.MountPartName].Name;map.Events.Add(next);addedEntries.Add((map.Events,next));
    }
    var team=(MSB_NR.Event.TeamFight)rewardTemplate.DeepCopy();team.Name=$"NRHex_mmv_cage_reward_{e.index}";team.EventID=1000+e.index;team.TargetEntityGroupID=(int)target;team.LeaderEntityID=(int)reward;
    Check(!map.Events.GetEntries().OfType<MSB_NR.Event.TeamFight>().Any(v=>v.TargetEntityGroupID==target),"MMV reward group collision");
    foreach(int lot in new[]{team.FirstItemLot,team.SecondItemLot,team.UnkItemLot}.Where(x=>x>0)){var loot=tables["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=lot&&r.ID<lot+10).ToArray();Check(loot.Length==3&&loot.All(r=>N(r,"getItemFlagId")==0),"Cage reward must be repeatable two-weapon/body ball");}
    map.Events.Add(team);addedEntries.Add((map.Events,team));
    foreach(var m in fakeMembers){
     var birth=CageEncounters.Birth(p,fake,m);
     // Seath's detached tail appears only after the native part-break test.
     if(e.auxiliaryEvent.HasValue&&!m.combat)foreach(var ins in birth.Instructions.Where(i=>i.Bank==2004&&i.ID==5&&BitConverter.ToUInt32(i.ArgData)==p+9000+(uint)m.index))ins.ArgData[4]=0;
     Event(birth,remap);if(m.combat)Event(CageEncounters.Death(p,fake,m),remap);
    }
    if(e.auxiliaryEvent.HasValue){
     Check(e.auxiliaryEvent==1038102220&&e.members.Length==2,"Unreviewed auxiliary boss lifecycle");
     var native=EMEVD.Read(Path.Combine(mmv,"mod/event",e.map+".emevd.dcx")).Events.Single(x=>x.ID==e.auxiliaryEvent);
     Check(native.Parameters.Count==0&&native.Instructions.Count==8&&native.Instructions[3].Bank==4&&native.Instructions[3].ID==6,"Seath part-break event changed");
     var auxiliary=new EMEVD.Event(0,EMEVD.Event.RestBehaviorType.Restart);auxiliary.Instructions.Add(I(2000,2,(byte)0));
     // Fresh generated actors must exist before the native part HP is set.
     foreach(var m in fakeMembers)auxiliary.Instructions.Add(I(3,0,(sbyte)1,(byte)1,(byte)0,p+7750+(uint)m.index));
     auxiliary.Instructions.AddRange([I(3,0,(sbyte)1,(byte)1,(byte)0,p+7537),I(0,0,(sbyte)0,(byte)1,(sbyte)1)]);
     var sourceRemap=new Dictionary<uint,uint>(remap);
     foreach(var (m,i) in e.members.Select((m,i)=>(m,i)))sourceRemap[m.entity]=p+9200+(uint)m.index;
     foreach(var (ins,index) in native.Instructions.Select((ins,index)=>(ins,index))){
      var copy=new EMEVD.Instruction(ins.Bank,ins.ID,ins.ArgData.ToArray()){Layer=ins.Layer};
      if(index==3){copy.ArgData[0]=unchecked((byte)-2);auxiliary.Instructions.Add(copy);auxiliary.Instructions.AddRange([I(3,0,(sbyte)-2,(byte)0,(byte)0,p+7537),I(0,0,(sbyte)0,(byte)1,(sbyte)-2),I(1003,101,(byte)0,(byte)0,(byte)0,p+7537)]);}
      else auxiliary.Instructions.Add(copy);
     }
     auxiliary.Instructions.AddRange([I(3,0,(sbyte)0,(byte)0,(byte)0,p+7537),I(1014,0),I(1001,1,1),I(1000,4,(byte)1)]);Event(auxiliary,sourceRemap);
    }
    Event(CageEncounters.Battle(p,fake,tables,tables["NpcParam"].Rows.Single(r=>r.ID==95300200+e.members.First(m=>m.combat).index)),remap);
    Event(CageEncounters.Hud(p,fake),remap);
    Event(CageMusic.Event(p,new(37,e.name,e.map,e.target,e.bgm,e.eventSha256),new(50,84,[fake])),remap);
    encounterReports.Add(new{e.index,e.name,choice,cooling=cool,reward,target,e.bgm,e.healthBarName});
   }
   var selector=Selector(p,draft,choices.ToArray(),cooling.ToArray());events.Events[events.Events.FindIndex(e=>e.ID==p+9700)]=selector;
   map.Write(mapPath);events.Write(eventPath);var checkMap=MSB_NR.Read(mapPath);var checkEvents=EMEVD.Read(eventPath);
   Check(DCX.Decompress(checkMap.Write()).Span.SequenceEqual(DCX.Decompress(mapPath).Span),"MMV cage map readback");
   Check(checkEvents.Events.Count==events.Events.Count&&checkEvents.Events.Zip(events.Events).All(x=>Cages.Same(x.First,x.Second)),"MMV cage event readback");
   Check(beforeEvents.Events.Where(e=>e.ID!=0&&e.ID!=p+9700).All(e=>Cages.Same(e,checkEvents.Events.Single(v=>v.ID==e.ID))),"Existing encounter event changed");
   var checkCtor=checkEvents.Events.Single(e=>e.ID==0);checkCtor.Instructions.RemoveRange(0,addedEvents.Count);Check(Cages.Same(checkCtor,beforeEvents.Events.Single(e=>e.ID==0)),"Existing constructor changed");
   foreach(var (container,entry) in addedEntries)Remove(container,entry);
   foreach(var model in map.Models.GetEntries().Where(m=>!modelNames.Contains(m.Name)).ToArray())Remove(map.Models,model);
   Check(map.Write().SequenceEqual(beforeMap),"Removing cage additions must restore merged MMV map exactly");
   changedFiles["map/mapstudio/"+name+".msb.dcx"]=Hash(mapPath);changedFiles["event/"+name+".emevd.dcx"]=Hash(eventPath);
   reports.Add(new{name,encounterCount=count,actorCount=84+members.Length,draft,red,decided,actors=actorReports,encounters=encounterReports,addedEvents=addedEvents.Count,events=addedEvents.Select(Cages.Dump),selector=Cages.Dump(selector),inverseRestoresMap=true,existingEventsPreserved=true});
  }
  foreach(var (name,table) in tables){
   foreach(var r in table.Rows.Where(r=>original[name].ContainsKey(r.ID)))Check(Signature(r)==original[name][r.ID],"Existing parameter changed: "+name+"/"+r.ID);
   if(table.Rows.Count!=original[name].Count){table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>FileName(f)==name+".param").Bytes=table.Write();}
  }
  SFUtil.EncryptNightreignRegulation(regulation,binder,binder.Compression);var checkBinder=SFUtil.DecryptNightreignRegulation(regulation);
  Check(checkBinder.Files.Count==binder.Files.Count&&checkBinder.Files.Zip(binder.Files).All(x=>x.First.Name==x.Second.Name&&x.First.Bytes.Span.SequenceEqual(x.Second.Bytes.Span)),"MMV cage regulation readback");changedFiles["regulation.bin"]=Hash(regulation);
  Check(inputs.All(x=>Hash(x.Key)==x.Value),"MMV source changed during cage stage");
  string rust="// Generated MMV cage identities; never precreate these actors.\n"+"pub const EXTRA_NPCS: &[i32] = &["+string.Join(',',members.Select(m=>95300200+m.index))+"];\n"+"pub const EXTRA_REWARD_GROUPS: &[u32] = &["+string.Join(',',encounters.Select(e=>6000+e.index))+"];\n";
  File.WriteAllText(Path.Combine(output,"cage_data.rs"),rust);
  File.WriteAllText(Path.Combine(output,"cages-validation.json"),JsonSerializer.Serialize(new{addedEncounters=encounters.Length,addedActors=members.Length,inputs,files=changedFiles,reports,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"MMV cages: {encounters.Length} new encounters / {members.Length} actors; 4 gaols; source maps and existing events preserved");
 }
}
