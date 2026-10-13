using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

static class CageNetwork {
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
 public static void CheckDraftFlags(EMEVD original,uint prefix,int count){
  var flags=Enumerable.Range((int)prefix+7100,count).Select(v=>(uint)v).ToHashSet();
  Check(count<=50&&flags.All(f=>f/1000%10==7),"Lottery needs allocated local flag storage");
  Check(!original.Events.Any(e=>flags.Contains((uint)e.ID)||e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>flags.Contains(BitConverter.ToUInt32(i.ArgData,n*4))))),"Lottery scratch flags collide with native events");
 }
 public static void Stage(string root){
  string output=Path.Combine(root,".work/cage-network-01791");Directory.CreateDirectory(Path.Combine(output,"staged/mod/event"));
  var reports=new List<object>();int count=CageEncounters.Load(root).encounterCount;
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block:00}_00_00",input=Path.Combine(root,"dist/mod/event",name+".emevd.dcx"),target=Path.Combine(output,"staged/mod/event",name+".emevd.dcx");uint p=(uint)(4600+block)*10000;
   var original=EMEVD.Read(Path.Combine(root,"assets/original/event",name+".emevd.dcx"));CheckDraftFlags(original,p,count);
   var before=EMEVD.Read(input);var next=EMEVD.Read(input);var old=next.Events.Single(e=>e.ID==p+9700);
   var batch=old.Instructions.Where(i=>i.Bank==2003&&i.ID==63).ToArray();var random=old.Instructions.Where(i=>i.Bank==2003&&i.ID==96).ToArray();
   Check(batch.Length==1&&random.Length==1&&old.Parameters.Count==0,"Expected pre-fix network lottery");
   Check(BitConverter.ToUInt32(batch[0].ArgData)==p+7500&&BitConverter.ToUInt32(batch[0].ArgData,4)==p+7500+count-1&&batch[0].ArgData[8]==0,"Unexpected shared clear range");
   Check(BitConverter.ToUInt32(random[0].ArgData)==p+7500&&BitConverter.ToUInt32(random[0].ArgData,4)==p+7500+count-1&&random[0].ArgData[8]==1,"Unexpected network draw range");
   Check(!before.Events.Any(e=>e.ID!=old.ID&&e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>{uint f=BitConverter.ToUInt32(i.ArgData,n*4);return f>=p+7100&&f<p+7150;}))),"Draft flags collide with installed events");
   next.Events[next.Events.IndexOf(old)]=Cages.Selector(p,count);next.Write(target);var read=EMEVD.Read(target);
   Check(read.Events.Select(e=>e.ID).SequenceEqual(before.Events.Select(e=>e.ID))&&read.Events.All(e=>Cages.Same(e,next.Events.Single(v=>v.ID==e.ID))),"Selector readback failed");
   Check(read.Events.Where(e=>e.ID!=old.ID).All(e=>Cages.Same(e,before.Events.Single(v=>v.ID==e.ID))),"Unrelated event changed");
   Check(read.Format==before.Format&&read.Compression==before.Compression&&read.StringData.SequenceEqual(before.StringData)&&read.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Event metadata changed");
   reports.Add(new{mapName=name,prefix=p,encounterCount=count,inputSha256=Hash(input),outputSha256=Hash(target),before=Cages.Dump(old),after=Cages.Dump(read.Events.Single(e=>e.ID==old.ID))});
  }
  File.WriteAllText(Path.Combine(output,"selectors.json"),JsonSerializer.Serialize(new{reports,onlyFourSelectorsChanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: four selectors use host-local drafts and publish only the accepted choice; all other events and metadata preserved");
 }
}
