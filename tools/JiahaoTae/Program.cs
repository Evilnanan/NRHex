// Copy the actual Duchess collision-invincibility event onto ordinary backsteps.
// A private SpEffect state gates it; animation timing owns the entire lifetime.
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;
var root=Path.GetFullPath(args.FirstOrDefault()??@"D:\Projects\NRMod\NRHex");
var smith=@"D:\Program Files\Smithbox";
var game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>Assembly.LoadFrom(Path.Combine(smith,new AssemblyName(a.Name).Name+".dll"));
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
void Require(bool v,string m){if(!v)throw new InvalidDataException(m);}
string Hash(byte[] b)=>Convert.ToHexString(SHA256.HashData(b));
var source=Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx");
Require(Hash(File.ReadAllBytes(source))=="610F553520E5127DA7366FE6C619683ACAC11B7AEE6F6B2450B8186AF0B573E1","Original animation bundle changed");
var original=BND4.Read(source);var patched=BND4.Read(source);
var a00=TAE.Read(original.Files.Single(f=>Path.GetFileName(f.Name)=="a00.tae").Bytes);
var duchess=a00.Animations.Single(a=>a.ID==27060).Events.Single(e=>e.Type==0&&e.StartTime==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==8);
Require(duchess.EndTime==.2f&&BitConverter.ToInt16(duchess.GetParameterBytes(false),14)==0,"Duchess iframe window changed");
Require(duchess.Group!=null&&duchess.Group.GroupType==0,"Native Duchess iframe event group changed");
const short ownerState=25013;
const int iframeFrames=11;
const float iframeDuration=iframeFrames/30f;
var flag=duchess.GetParameterBytes(false);BitConverter.GetBytes(ownerState).CopyTo(flag,14);
var ids=new long[]{27000,27010,27020,27030};var changes=new List<object>();
var options=new JsonSerializerOptions{IncludeFields=true};
string Header(TAE.Animation a)=>JsonSerializer.Serialize(a.MiniHeader,a.MiniHeader.GetType(),options)+a.AnimFileName;
bool SameParameters(byte[] a,byte[] b){var n=Math.Min(a.Length,b.Length);return Math.Abs(a.Length-b.Length)<=15&&a.AsSpan(0,n).SequenceEqual(b.AsSpan(0,n))&&a.Skip(n).Concat(b.Skip(n)).All(v=>v==0);}
string Group(TAE.EventGroup g)=>JsonSerializer.Serialize(g,options);
bool SameEvent(TAE.Event a,TAE.Event b)=>a.Type==b.Type&&a.Unk04==b.Unk04&&a.StartTime==b.StartTime&&a.EndTime==b.EndTime&&Group(a.Group)==Group(b.Group)&&SameParameters(a.GetParameterBytes(false),b.GetParameterBytes(false));
foreach(var file in patched.Files.Where(f=>f.Name.EndsWith(".tae"))){
 var tae=TAE.Read(file.Bytes);var clips=tae.Animations.Where(a=>ids.Contains(a.ID)).ToArray();if(clips.Length==0)continue;
 foreach(var a in clips){
  Require(!a.Events.Any(e=>e.Type==0&&BitConverter.ToInt32(e.GetParameterBytes(false))==8),"Unexpected ordinary backstep iframe");
  var group=new TAE.EventGroup(duchess.Group.GroupType){GroupData=duchess.Group.GroupData};
  a.EventGroups.Add(group);
  a.Events.Add(new TAE.Event(duchess.StartTime,iframeDuration,duchess.Type,duchess.Unk04,flag.ToArray(),false){Group=group});
  changes.Add(new{file=Path.GetFileName(file.Name),a.ID,start=duchess.StartTime,end=iframeDuration,flag=8,state=ownerState});
 }
 file.Bytes=tae.Write();
}
Require(changes.Count==36,"Unexpected normal backstep clip count");
var output=Path.Combine(root,"dist/mod/chr/c0000.anibnd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(output)!);patched.Write(output);
var reopened=BND4.Read(output);Require(reopened.Files.Count==original.Files.Count,"Binder count changed");
for(int i=0;i<original.Files.Count;i++){
 var old=original.Files[i];var current=reopened.Files[i];Require(old.ID==current.ID&&old.Name==current.Name&&old.Flags==current.Flags,"Binder member identity changed");
 if(old.Bytes.Span.SequenceEqual(current.Bytes.Span))continue;
 Require(old.Name.EndsWith(".tae"),"Non-TAE asset changed");var before=TAE.Read(old.Bytes);var after=TAE.Read(current.Bytes);
 Require(before.ID==after.ID&&before.Format==after.Format&&before.BigEndian==after.BigEndian&&before.EventBank==after.EventBank&&before.Flags.SequenceEqual(after.Flags)&&before.SkeletonName==after.SkeletonName&&before.SibName==after.SibName&&before.Animations.Count==after.Animations.Count,"TAE metadata changed");
 for(int j=0;j<before.Animations.Count;j++){
  var a=before.Animations[j];var b=after.Animations[j];Require(a.ID==b.ID&&Header(a)==Header(b),"Animation header changed");
  bool modified=ids.Contains(a.ID);
  Require(b.EventGroups.Count==a.EventGroups.Count+(modified?1:0)&&a.EventGroups.Select(Group).SequenceEqual(b.EventGroups.Take(a.EventGroups.Count).Select(Group)),"Original event groups changed "+a.ID);
  Require(b.Events.Count==a.Events.Count+(modified?1:0),"Unexpected event count");
  Require(a.Events.Zip(b.Events).All(p=>SameEvent(p.First,p.Second)),"Original event changed "+old.Name+" "+a.ID);
  if(modified){var e=b.Events.Last();Require(e.Type==0&&e.StartTime==0&&e.EndTime==iframeDuration&&SameParameters(e.GetParameterBytes(false),flag),"New native iframe event failed readback");Require(e.Group!=null&&ReferenceEquals(e.Group,b.EventGroups.Last())&&Group(e.Group)==Group(duchess.Group),"New iframe event missing native group membership");}
 }
}
var report=new{version="0.14.8",sourceSha256=Hash(File.ReadAllBytes(source)),sha256=Hash(File.ReadAllBytes(output)),ownerMarker=6999700,ownerState,sourceDuchessClip=27060,nativeJumpTableFlag=8,nativeGroupType=0,allNewEventsHaveSerializedGroupMembership=true,originalGroupAssociationsUnchanged=true,motionSeconds=iframeDuration,frames=iframeFrames,referenceFps=30,originalEventsAndAnimationHeadersUnchanged=true,unrelatedBinderMembersUnchanged=true,duchessAnimationsUnchanged=true,changes,gameplayVerified=false};
Directory.CreateDirectory(Path.Combine(root,"dist/reports"));
File.WriteAllText(Path.Combine(root,"dist/reports/jiahao-tae-validation.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine("PASS: 36 owner-gated native backstep iframe events and their native event groups reopened; original animations preserved.");
