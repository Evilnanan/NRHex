using System.Numerics;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class CagePresentation {
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static bool HasTowerIntro(CageEncounters.Group group,CageEncounters.Member member)=>group.birthAnimation==null&&(group.map.StartsWith("m48_")||group.map.StartsWith("m49_"))&&member.model is "c3160" or "c3250" or "c3251" or "c4130" or "c5810";
 public static void StageBodyFog(string root){
  var catalog=CageEncounters.Load(root);var reports=new List<object>();
  string directory=Path.Combine(root,".work/cage-demand/body-fog-stage/mod/event");Directory.CreateDirectory(directory);
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),output=Path.Combine(directory,name+".emevd.dcx");uint prefix=(uint)(4600+block)*10000;
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changed=new HashSet<long>();
   foreach(var group in catalog.groups)foreach(var member in group.members.Where(member=>HasTowerIntro(group,member))){
    var old=next.Events.Single(entry=>entry.ID==prefix+9900+member.index);
    Check(Cages.Same(old,CageEncounters.Birth(prefix,group,member,nativeTowerIntro:false)),"Group intro input drift");
    var replacement=CageEncounters.Birth(prefix,group,member);
    var animation=replacement.Instructions.Single(instruction=>instruction.Bank==2003&&instruction.ID==18);
    Check(BitConverter.ToInt32(animation.ArgData,4)==20026&&animation.ArgData[8]==0,"Expected native nonlooped tower intro");
    int at=replacement.Instructions.IndexOf(animation);replacement.Instructions.RemoveAt(at);
    Check(Cages.Same(old,replacement),"Group intro changed beyond animation request");replacement.Instructions.Insert(at,animation);
    next.Events[next.Events.IndexOf(old)]=replacement;changed.Add(old.ID);
   }
   Check(changed.Count==6,"Unexpected native tower intro member count");next.Write(output);var actual=EMEVD.Read(output);
   Check(actual.Events.Select(entry=>entry.ID).SequenceEqual(before.Events.Select(entry=>entry.ID)),"Event order changed");
   Check(actual.Format==before.Format&&actual.Compression==before.Compression&&actual.StringData.SequenceEqual(before.StringData)&&actual.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Event metadata changed");
   Check(actual.Events.All(entry=>Cages.Same(entry,next.Events.Single(value=>value.ID==entry.ID))),"Intro readback mismatch");
   Check(before.Events.Where(entry=>!changed.Contains(entry.ID)).All(entry=>Cages.Same(entry,actual.Events.Single(value=>value.ID==entry.ID))),"Unrelated event changed");
   reports.Add(new{name,inputSha256=Hash(input),outputSha256=Hash(output),changed=changed.Order().ToArray(),events=actual.Events.Where(entry=>changed.Contains(entry.ID)).Select(Cages.Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/cage-demand/body-fog-validation.json"),JsonSerializer.Serialize(new{reports,onlyNativeIntroAdded=true,centralFogUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: 24 native group intro requests; central fog, single bosses, reconnect branches and unrelated events preserved; dist untouched");
 }
 public static void CentralFog(EMEVD.Event entry,uint prefix,CageEncounters.Group group,CageEncounters.Member member){
  Check(entry.Parameters.Count==0,"Expected nonparameterized cage presentation");
  var fog=entry.Instructions.Single(i=>i.Bank==2006&&i.ID==3);
  var expected=new EMEVD.Instruction(2006,3,new object[]{2,prefix+9000+(uint)member.index,900,690047});
  Check(fog.Layer==null&&fog.ArgData.SequenceEqual(expected.ArgData),"Unexpected group fog attachment");
  entry.Instructions[entry.Instructions.IndexOf(fog)]=member.index==group.members[0].index
   ?new EMEVD.Instruction(2006,3,new object[]{1,prefix+3900,-1,690047})
   :new EMEVD.Instruction(1000,3,new object[]{(byte)0});
 }
 public static void Stage(string root){
  var catalog=CageEncounters.Load(root);Check(catalog.groups.Length==13&&catalog.groups.Sum(g=>g.members.Length)==47,"Group catalog changed");
  string directory=Path.Combine(root,".work/cage-presentation/staged/mod/event");Directory.CreateDirectory(directory);
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),output=Path.Combine(directory,name+".emevd.dcx");uint prefix=(uint)(4600+block)*10000;
   string mapPath=Path.Combine(root,"dist/mod/map/mapstudio",name+".msb.dcx");var map=MSB_NR.Read(mapPath);
   var center=map.Regions.GetEntries().Single(r=>r.Common_EntityID==prefix+3900);
   Check(center.Name=="NRHex_cage_spawn_0"&&center.Position==Vector3.Zero,"Expected existing central spawn region");
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changed=new HashSet<long>();
   foreach(var group in catalog.groups)foreach(var member in group.members){
    var entry=next.Events.Single(e=>e.ID==prefix+9900+member.index);
    var legacy=CageEncounters.Birth(prefix,group,member,centralFog:false);
    Check(Cages.Same(entry,legacy),$"Installed group presentation drift: {name}/{entry.ID}");
    CentralFog(entry,prefix,group,member);
    Check(Cages.Same(entry,CageEncounters.Birth(prefix,group,member)),"Incremental/full-build mismatch");changed.Add(entry.ID);
   }
   next.Write(output);var actual=EMEVD.Read(output);
   Check(actual.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID)),"Event order changed");
   Check(actual.Format==before.Format&&actual.Compression==before.Compression&&actual.StringData.SequenceEqual(before.StringData)&&actual.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Event metadata changed");
   Check(actual.Events.All(e=>Cages.Same(e,next.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
   Check(before.Events.Where(e=>!changed.Contains(e.ID)).All(e=>Cages.Same(e,actual.Events.Single(v=>v.ID==e.ID))),"Unrelated event changed");
   foreach(long id in changed){
    var old=before.Events.Single(e=>e.ID==id);var current=actual.Events.Single(e=>e.ID==id);
    int at=old.Instructions.FindIndex(i=>i.Bank==2006&&i.ID==3);var saved=current.Instructions[at];current.Instructions[at]=old.Instructions[at];
    Check(Cages.Same(old,current),"Non-fog presentation logic changed");current.Instructions[at]=saved;
   }
   reports.Add(new{name,prefix,inputSha256=Hash(input),outputSha256=Hash(output),mapSha256=Hash(mapPath),center=prefix+3900,changedEvents=changed.Count,events=actual.Events.Where(e=>e.ID>=prefix+9900&&e.ID<prefix+9984).Select(Cages.Dump)});
  }
  File.WriteAllText(Path.Combine(root,".work/cage-presentation/validation.json"),JsonSerializer.Serialize(new{reports,onlyFogInstructionChanged=true,mapsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: 188 group presentations; one central fog per encounter; single bosses, maps, battle/reward logic and metadata preserved");
 }
}
