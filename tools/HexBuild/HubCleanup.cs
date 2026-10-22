using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class HubCleanup {
 public const int EventId=9100941;
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static bool Removal(EMEVD.Instruction instruction,int effect)=>instruction.Bank==2004&&instruction.ID==21&&instruction.ArgData.Length==8&&BitConverter.ToUInt32(instruction.ArgData)==20000&&BitConverter.ToInt32(instruction.ArgData,4)==effect;
 static void Guard(EMEVD.Event entry){
  Check(entry.Parameters.Count==0&&entry.RestBehavior==EMEVD.Event.RestBehaviorType.Restart&&entry.Instructions.Count>=8,"Unexpected hub reset event layout");
  var boundary=new[]{(0,2000,2,"00000000"),(1,3,8,"FF010A0000000000"),(2,3,8,"FF010B0000000000"),(3,0,0,"0001FF00"),(entry.Instructions.Count-4,3,8,"01000A0000000000"),(entry.Instructions.Count-3,3,8,"01000B0000000000"),(entry.Instructions.Count-2,0,0,"00010100"),(entry.Instructions.Count-1,1000,4,"01000000")};
  foreach(var (index,bank,id,hex)in boundary){var instruction=entry.Instructions[index];Check(instruction.Bank==bank&&instruction.ID==id&&Convert.ToHexString(instruction.ArgData)==hex,"Hub map guard/restart boundary changed");}
  Check(entry.Instructions.Skip(4).Take(entry.Instructions.Count-8).All(instruction=>instruction.Bank==2004&&instruction.ID is 8 or 21),"Unexpected branch or operation inside hub cleanup");
 }
 static int[] Effects(Hex[] hexes,PARAM effects){
  var owners=hexes.Where(hex=>hex.Id<95003800).Select(hex=>hex.Id).ToHashSet();
  Check(owners.Count>0&&owners.All(owner=>RewardCatalog.HistoricalOwners.Contains(owner)&&effects.Rows.Count(row=>row.ID==owner)==1),"Missing legacy owner effect or catalog registration");
  return owners.Order().Concat(effects.Rows.Where(row=>owners.Any(owner=>row.ID>owner&&row.ID<owner+100)||owners.Contains(95000500)&&row.ID>95005000&&row.ID<95005080).Select(row=>row.ID).Order()).Distinct().ToArray();
 }
 public static int[] Events(EMEVD events,Hex[] hexes,PARAM effects){
  var entry=events.Events.Single(item=>item.ID==EventId);Guard(entry);
  var wanted=Effects(hexes,effects);
  Check(wanted.All(effect=>entry.Instructions.Count(instruction=>Removal(instruction,effect))<=1),"Duplicate legacy cleanup instructions");
  var added=wanted.Where(effect=>!entry.Instructions.Any(instruction=>Removal(instruction,effect))).ToArray();
  entry.Instructions.InsertRange(4,added.Select(effect=>new EMEVD.Instruction(2004,21,new object[]{20000u,effect})));
  Guard(entry);
  Check(wanted.All(effect=>entry.Instructions.Count(instruction=>Removal(instruction,effect))==1),"Legacy hub cleanup incomplete");
  return added;
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string input=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),directory=Path.Combine(root,"work/hub-cleanup/staged");Directory.CreateDirectory(directory);
  var regulation=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var effects=parse("SpEffectParam",regulation.Files.Single(file=>file.Name.Replace('\\','/').Split('/').Last()=="SpEffectParam.param").Bytes);
  var original=EMEVD.Read(input);var events=EMEVD.Read(input);var added=Events(events,hexes,effects);
  string output=Path.Combine(directory,"common.emevd.dcx");events.Write(output);var actual=EMEVD.Read(output);
  Check(actual.Format==original.Format&&actual.Compression==original.Compression&&actual.Events.Count==original.Events.Count&&actual.StringData.SequenceEqual(original.StringData)&&actual.LinkedFileOffsets.SequenceEqual(original.LinkedFileOffsets),"EMEVD metadata changed");
  foreach(var entry in actual.Events){Check(Cages.Same(events.Events.Single(previous=>previous.ID==entry.ID),entry),"Event readback mismatch");if(entry.ID!=EventId)Check(Cages.Same(original.Events.Single(previous=>previous.ID==entry.ID),entry),"Unrelated event changed");}
  Check(Events(actual,hexes,effects).Length==0,"Hub cleanup must be idempotent");
  var target=actual.Events.Single(entry=>entry.ID==EventId);target.Instructions.RemoveRange(4,added.Length);
  Check(Cages.Same(original.Events.Single(entry=>entry.ID==EventId),target),"Existing hub operations or map guards changed");
  var invalid=EMEVD.Read(input);invalid.Events.Single(entry=>entry.ID==EventId).Instructions[1]=new(3,8,new object[]{(sbyte)-1,(byte)1,(sbyte)12,(sbyte)0,(sbyte)0,(sbyte)0});
  bool rejected=false;try{Events(invalid,hexes,effects);}catch(InvalidDataException){rejected=true;}Check(rejected,"Changed map guard must be rejected");
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventId=EventId,owners=hexes.Where(hex=>hex.Id<95003800).Select(hex=>new{hex.Id,hex.Zh}),cleanupEffects=Effects(hexes,effects),addedEffects=added,onlyHubEventChanged=true,originalInstructionsAndMapGuardsPreserved=true,idempotencePassed=true,changedMapGuardRejected=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: hub cleanup adds {added.Length} removals; all legacy owners and private effects covered; other events and map guards preserved");
 }
}
