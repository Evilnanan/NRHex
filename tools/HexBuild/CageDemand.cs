using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class CageDemand {
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static bool Is(EMEVD.Instruction instruction,int bank,int id,uint entity)=>instruction.Bank==bank&&instruction.ID==id&&instruction.ArgData.Length>=4&&BitConverter.ToUInt32(instruction.ArgData)==entity;
 public static void Birth(EMEVD.Event entry,uint actor){
  Check(entry.Parameters.Count==0,"Parameterized demand birth");
  uint generator=actor-5200;int branches=0;
  for(int index=0;index<entry.Instructions.Count;index++){
   if(!Is(entry.Instructions[index],2003,54,generator))continue;
   Check(index>=2,"Missing generator load branch");
   var network=entry.Instructions[index-2];var load=entry.Instructions[index-1];
   Check(Is(network,2004,34,actor)&&network.ArgData[4]==1&&Is(load,2004,29,actor)&&load.ArgData[4]==0,"Demand load order drift");
   var invoke=entry.Instructions[index];entry.Instructions[index-2]=invoke;entry.Instructions[index-1]=network;entry.Instructions[index]=load;branches++;
  }
  Check(branches==2,"Expected fresh and ongoing demand branches");
 }
 public static void Stage(string root){
  string directory=Path.Combine(root,".work/cage-demand/staged/mod/event");Directory.CreateDirectory(directory);var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),output=Path.Combine(directory,name+".emevd.dcx");uint prefix=(uint)(4600+block)*10000;
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);var changed=new HashSet<long>();
   foreach(var entry in next.Events.Where(entry=>entry.ID>=prefix+9900&&entry.ID<prefix+9984)){
    Birth(entry,prefix+9000+(uint)(entry.ID-prefix-9900));changed.Add(entry.ID);
   }
   Check(changed.Count==84,"Incomplete demand actor catalog");next.Write(output);var actual=EMEVD.Read(output);
   Check(actual.Events.Select(entry=>entry.ID).SequenceEqual(before.Events.Select(entry=>entry.ID)),"Event order changed");
   Check(actual.Format==before.Format&&actual.Compression==before.Compression&&actual.StringData.SequenceEqual(before.StringData)&&actual.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Event metadata changed");
   Check(actual.Events.All(entry=>Cages.Same(entry,next.Events.Single(value=>value.ID==entry.ID))),"Demand readback mismatch");
   Check(before.Events.Where(entry=>!changed.Contains(entry.ID)).All(entry=>Cages.Same(entry,actual.Events.Single(value=>value.ID==entry.ID))),"Unrelated event changed");
   foreach(long id in changed){
    var original=before.Events.Single(entry=>entry.ID==id);var patched=actual.Events.Single(entry=>entry.ID==id);uint generator=prefix+3800+(uint)(id-prefix-9900);int branches=0;
    for(int index=0;index<patched.Instructions.Count-2;index++){
     if(!Is(patched.Instructions[index],2003,54,generator))continue;
     var invoke=patched.Instructions[index];patched.Instructions[index]=patched.Instructions[index+1];patched.Instructions[index+1]=patched.Instructions[index+2];patched.Instructions[index+2]=invoke;branches++;index+=2;
    }
    Check(branches==2&&Cages.Same(original,patched),"Change extends beyond two load-order permutations");
   }
   reports.Add(new{name,inputSha256=Hash(input),outputSha256=Hash(output),birthEvents=changed.Count,branches=changed.Count*2});
  }
  File.WriteAllText(Path.Combine(root,".work/cage-demand/events-validation.json"),JsonSerializer.Serialize(new{reports,onlyLoadOrderChanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: 336 birth events, 672 load-order permutations; unrelated events and metadata preserved; dist untouched");
 }
}
