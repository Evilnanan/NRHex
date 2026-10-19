using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class CageMusic {
 public record Track(int choice,string name,string map,uint sourceTarget,int bgm,string sourceSha256);
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static EMEVD.Instruction I(int bank,int id,params object[] values)=>new(bank,id,values);
 static EMEVD.Instruction Flag(int group,uint flag,bool on)=>I(3,0,(sbyte)group,(byte)(on?1:0),(byte)0,flag);
 static EMEVD.Instruction Wait(int group)=>I(0,0,(sbyte)0,(byte)1,(sbyte)group);
 static uint Shown(uint prefix,int member)=>prefix+(member<37?7600u:7750u)+(uint)member;
 public static Track[] Catalog(string root){
  var singles=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
  var groups=CageEncounters.Load(root).groups;
  var encounters=singles.Select((boss,index)=>(choice:index,boss.name,boss.map,entities:new[]{boss.entity})).Concat(groups.Select(group=>(group.choice,group.name,group.map,entities:group.members.Where(member=>member.combat).Select(member=>member.entity).ToArray())));
  var tracks=new List<Track>();
  foreach(var encounter in encounters){
   string source=Path.Combine(root,"assets/original/event",encounter.map+".emevd.dcx");
   var map=MSB_NR.Read(Path.Combine(root,"assets/original/map/mapstudio",encounter.map+".msb.dcx"));
   var calls=EMEVD.Read(source).Events.Single(entry=>entry.ID==0).Instructions.Where(instruction=>instruction.Bank==2000&&instruction.ID==6&&instruction.ArgData.Length==28&&BitConverter.ToUInt32(instruction.ArgData,4)==90015030);
   bool Includes(uint target,uint entity){
    if(target==entity)return true;
    var variants=map.Parts.Enemies.Where(enemy=>enemy.EntityData.EntityID==entity).ToArray();
    return variants.Length>0&&variants.All(enemy=>enemy.EntityData.EntityGroupIDs.Contains(target));
   }
   var group=groups.SingleOrDefault(group=>group.choice==encounter.choice);
   bool Covers(uint target)=>group==null?encounter.entities.All(entity=>Includes(target,entity)):group.members.Where(member=>member.combat).All(member=>{
    var enemy=map.Parts.Enemies.Single(enemy=>enemy.Name==member.sourceName);
    Check(enemy.EntityData.EntityID==member.entity&&enemy.NpcParamId==member.npc,"Music source member drift");
    return target==enemy.EntityData.EntityID||enemy.EntityData.EntityGroupIDs.Contains(target);
   });
   var matches=calls.Select(instruction=>(target:BitConverter.ToUInt32(instruction.ArgData,12),bgm:BitConverter.ToInt32(instruction.ArgData,20))).Where(call=>Covers(call.target)).Distinct().ToArray();
   Check(matches.Length==1&&matches[0].bgm>0,"Ambiguous native cage music: "+encounter.name);
   tracks.Add(new(encounter.choice,encounter.name,encounter.map,matches[0].target,matches[0].bgm,Hash(source)));
  }
  Check(tracks.Count==50&&tracks.Select(track=>track.choice).SequenceEqual(Enumerable.Range(0,50)),"Incomplete music catalog");
  return tracks.ToArray();
 }
 public static EMEVD.Event Event(uint prefix,Track track,CageEncounters.Catalog catalog){
  int[] members=track.choice<37?[track.choice]:catalog.groups.Single(group=>group.choice==track.choice).members.Where(member=>member.combat).Select(member=>member.index).ToArray();
  Check(members.Length is >0 and <=9,"Too many music condition groups");
  uint target=track.choice<37?prefix+9000+(uint)track.choice:prefix+5900+(uint)track.choice;
  uint choice=prefix+7500+(uint)track.choice;
  var entry=new EMEVD.Event(prefix+9400+(uint)track.choice,EMEVD.Event.RestBehaviorType.Restart);
  entry.Instructions.AddRange([I(2000,2,(byte)0),Flag(1,choice,true),Flag(1,prefix+7404,true),I(4,0,(sbyte)1,20000u,(byte)0,(sbyte)0,1f),I(3,3,(sbyte)1,(byte)1,20000u,target,50f,1),I(10,3,(sbyte)1,(sbyte)5,track.bgm)]);
  foreach(var (member,index) in members.Select((member,index)=>(member,index))){
   int condition=index+2;uint actor=prefix+9000+(uint)member;
   entry.Instructions.AddRange([Flag(condition,Shown(prefix,member),true),I(4,7,(sbyte)condition,actor,(byte)1,(sbyte)0,1f),I(4,14,(sbyte)condition,actor,(sbyte)2,0,(sbyte)0,1f),I(0,0,(sbyte)-1,(byte)1,(sbyte)condition)]);
  }
  entry.Instructions.AddRange([I(0,0,(sbyte)1,(byte)1,(sbyte)-1),Wait(1),I(2010,10,track.bgm,0),I(1001,1,1)]);
  foreach(int member in members){
   uint actor=prefix+9000+(uint)member;
   entry.Instructions.Add(I(4,14,(sbyte)11,actor,(sbyte)5,0,(sbyte)0,1f));
  }
  entry.Instructions.AddRange([I(0,0,(sbyte)-12,(byte)1,(sbyte)11),Flag(-12,choice,false),Flag(-12,prefix+7404,false),I(4,0,(sbyte)-12,20000u,(byte)1,(sbyte)0,1f),I(3,3,(sbyte)-12,(byte)0,20000u,target,100f,1),I(10,2,(sbyte)-12,track.bgm,(byte)0)]);
  foreach(var (member,index) in members.Select((member,index)=>(member,index))){
   int condition=index+2;
   entry.Instructions.AddRange([I(4,14,(sbyte)condition,prefix+9000+(uint)member,(sbyte)2,0,(sbyte)0,1f),Flag(condition,Shown(prefix,member),false),I(0,0,(sbyte)-12,(byte)1,(sbyte)condition)]);
  }
  entry.Instructions.AddRange([Wait(-12),I(1000,7,(byte)1,(byte)0,(sbyte)11),I(1001,0,Cages.VictoryDelaySeconds),I(1010,4,(byte)1,track.bgm,(byte)0),I(2010,10,track.bgm,-1),I(1001,1,1),I(1000,4,(byte)1)]);
  return entry;
 }
 public static List<EMEVD.Event> Install(string root,EMEVD events,uint prefix){
  var catalog=CageEncounters.Load(root);var constructor=events.Events.Single(entry=>entry.ID==0);
  var added=new List<EMEVD.Event>();
  foreach(var track in Catalog(root)){
   var entry=Event(prefix,track,catalog);Check(!events.Events.Any(existing=>existing.ID==entry.ID),"Music event ID collision");
   events.Events.Add(entry);added.Add(entry);constructor.Instructions.Insert(0,I(2000,0,0,(uint)entry.ID,0));
  }
  return added;
 }
 public static void Stage(string root,bool repairShown=false){
  string directory=Path.Combine(root,".work/cage-music/staged/mod/event");Directory.CreateDirectory(directory);
  var tracks=Catalog(root);var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),output=Path.Combine(directory,name+".emevd.dcx");uint prefix=(uint)(4600+block)*10000;
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);
   if(repairShown){
    var catalog=CageEncounters.Load(root);
    foreach(var track in tracks){
     var replacement=Event(prefix,track,catalog);var current=next.Events.Single(entry=>entry.ID==replacement.ID);
     var legacy=Event(prefix,track,catalog);
     if(track.choice<37){
      var flags=legacy.Instructions.Where(instruction=>instruction.Bank==3&&instruction.ID==0&&BitConverter.ToUInt32(instruction.ArgData,4)==Shown(prefix,track.choice)).ToArray();
      Check(flags.Length==2,"Single music shown references");
      foreach(var instruction in flags)BitConverter.GetBytes(prefix+7750+(uint)track.choice).CopyTo(instruction.ArgData,4);
     }
     Check(Cages.Same(current,legacy),"Unexpected installed music event: "+current.ID);
     next.Events[next.Events.IndexOf(current)]=replacement;
    }
   }else Install(root,next,prefix);
   next.Write(output);var actual=EMEVD.Read(output);
   Check(actual.Events.Count==before.Events.Count+(repairShown?0:50),"Music event count");
   Check(actual.Events.All(entry=>Cages.Same(entry,next.Events.Single(expected=>expected.ID==entry.ID))),"Music readback mismatch");
   Check(actual.Format==before.Format&&actual.Compression==before.Compression&&actual.StringData.SequenceEqual(before.StringData)&&actual.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Music metadata drift");
   Check(before.Events.Where(entry=>entry.ID!=0&&(!repairShown||entry.ID<prefix+9400||entry.ID>=prefix+9437)).All(entry=>Cages.Same(entry,actual.Events.Single(value=>value.ID==entry.ID))),"Existing event changed");
   var constructor=actual.Events.Single(entry=>entry.ID==0);var initializers=constructor.Instructions.Take(50).ToArray();
   Check(initializers.Select(instruction=>BitConverter.ToUInt32(instruction.ArgData,4)).SequenceEqual(Enumerable.Range(0,50).Reverse().Select(index=>prefix+9400+(uint)index)),"Music constructor placement");
   if(!repairShown)constructor.Instructions.RemoveRange(0,50);Check(Cages.Same(constructor,before.Events.Single(entry=>entry.ID==0)),"Existing constructor changed");
   reports.Add(new{name,inputSha256=Hash(input),outputSha256=Hash(output),prefix,events=actual.Events.Where(entry=>entry.ID>=prefix+9400&&entry.ID<prefix+9450).Select(Cages.Dump),birthEvents=actual.Events.Where(entry=>entry.ID>=prefix+9900&&entry.ID<prefix+9984).Select(Cages.Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/cage-music/validation.json"),JsonSerializer.Serialize(new{tracks,reports,repairShown,unrelatedEventsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {tracks.Length} native encounter BGM bindings; 200 local music events; unrelated events preserved; dist untouched");
 }
 public static void Probe(string root){
  string output=Path.Combine(root,".work/cage-music");Directory.CreateDirectory(output);
  var singles=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
  var maps=singles.Select(boss=>boss.map).Concat(CageEncounters.Load(root).groups.Select(group=>group.map)).Append("common_func").Distinct().Order();
  foreach(string map in maps){
   string path=Path.Combine(root,"assets/original/event",map+".emevd.dcx");
   var events=EMEVD.Read(path);
   File.WriteAllText(Path.Combine(output,map+".json"),JsonSerializer.Serialize(new{sha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),events=events.Events.Select(Cages.Dump)},new JsonSerializerOptions{WriteIndented=true}));
  }
  Console.WriteLine("Exported original encounter events for music research; dist untouched");
 }
}
