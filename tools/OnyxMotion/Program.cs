using System.Reflection;
using System.Runtime.InteropServices;
using System.Numerics;
using System.Text.Json;
using System.Security.Cryptography;
using SoulsFormats;
using HKLib.hk2018;
using HKLib.Serialization.hk2018.Binary;

if(args.Length!=3)throw new ArgumentException("OnyxMotion <repository> <target a9x binder> <output binder>");
var root=Path.GetFullPath(args[0]);var input=Path.GetFullPath(args[1]);var output=Path.GetFullPath(args[2]);
var drive=Path.GetPathRoot(root)!;
var smith=Environment.GetEnvironmentVariable("NRHEX_SMITHBOX")??Path.Combine(drive,"Program Files/Smithbox");
var dsas=Environment.GetEnvironmentVariable("NRHEX_DSAS")??Path.Combine(drive,"Program Files/DSAS");
var game=Environment.GetEnvironmentVariable("NRHEX_GAME")??Path.Combine(drive,"Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game");
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{foreach(var dir in new[]{smith,dsas}){var p=Path.Combine(dir,new AssemblyName(a.Name).Name+".dll");if(File.Exists(p))return Assembly.LoadFrom(p);}return null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
void Check(bool v,string why){if(!v)throw new InvalidDataException(why);}
var binder=BND4.Read(input);var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
var source=binder.Files.Single(f=>f.Name.EndsWith("a840_040000.hkx"));
var reader=new HavokBinarySerializer();reader.LoadCompendium(new MemoryStream(binder.Files.Single(f=>f.Name.EndsWith(".compendium")).Bytes.ToArray()));
var sourceRoot=(hkRootLevelContainer)reader.Read(new MemoryStream(source.Bytes.ToArray()));
var container=(hkaAnimationContainer)sourceRoot.m_namedVariants.Single().m_variant;
var native=(hkaSplineCompressedAnimation)container.m_animations.Single();
Check(native.m_numFrames==151&&native.m_duration==5&&native.m_numberOfTransformTracks==72&&native.m_numberOfFloatTracks==0,"Review changed target Onyx motion before rebuilding");
var decoder=Assembly.LoadFrom(Path.Combine(dsas,"SoulsAssetPipeline.dll")).GetType("SoulsAssetPipeline.Animation.SplineCompressedAnimation")!.GetMethod("ReadSplCmpAnimBytesAndSampleToUncomp")!;
var decoded=(System.Collections.IList)decoder.Invoke(null,new object[]{native.m_data.ToArray(),native.m_numberOfTransformTracks,native.m_numBlocks,native.m_numFrames,native.m_maxFramesPerBlock})!;
Check(decoded.Count==151*72,"Incomplete motion sampling");
var poses=decoded.Cast<object>().Select(obj=>{dynamic p=obj;return new hkQsTransform{m_translation=new Vector4((Vector3)p.Translation,0),m_rotation=Quaternion.Normalize((Quaternion)p.Rotation),m_scale=new Vector4((Vector3)p.Scale,0)};}).ToArray();
Check(poses.All(p=>float.IsFinite(p.m_rotation.LengthSquared())&&Math.Abs(p.m_rotation.LengthSquared()-1)<0.01f),"Invalid decoded rotation");
var originalMotion=(hkaDefaultAnimatedReferenceFrame)native.m_extractedMotion;
Vector4 RootAt(float seconds){float f=seconds/5*(originalMotion.m_referenceFrameSamples.Count-1);int a=(int)f,b=Math.Min(a+1,originalMotion.m_referenceFrameSamples.Count-1);return Vector4.Lerp(originalMotion.m_referenceFrameSamples[a],originalMotion.m_referenceFrameSamples[b],f-a);}
const int HoldFrame=81; // Preserve the native impact recoil at entry, not at exit.
const int RestFrame=99; // Settled planted pose, before the native sword withdrawal.
// Play the 0.6s impact once, then restore the earlier two eased pose excursions.
// Return to RestFrame before the finish marker so the end clip is continuous.
float HoldSource(float t){
 if(t<=0.6f)return HoldFrame+t*30;
 if(t>=3f)return RestFrame;
 float u=(t-0.6f)/2.4f;
 float envelope=MathF.Sin(MathF.PI*u);envelope*=envelope;
 float movement=MathF.Sin(2*MathF.PI*u);movement*=movement;
 return RestFrame-8f*envelope*movement;
}
hkQsTransform PoseAt(float frame,int track){
 int left=Math.Clamp((int)frame,0,150),right=Math.Min(left+1,150);float t=frame-left;
 var a=poses[left*72+track];var b=poses[right*72+track];
 return new hkQsTransform{m_translation=Vector4.Lerp(a.m_translation,b.m_translation,t),m_rotation=Quaternion.Normalize(Quaternion.Slerp(a.m_rotation,b.m_rotation,t)),m_scale=Vector4.Lerp(a.m_scale,b.m_scale,t)};
}
Check(HoldSource(0)==HoldFrame&&HoldSource(3.1f)==RestFrame&&HoldSource(3.2f)==RestFrame,"Channel boundaries differ");
Check(Enumerable.Range(19,71).Any(i=>HoldSource(i/30f)<98.5f),"Channel must contain live pose movement");
var holdSamples=Enumerable.Range(18,73).Select(i=>HoldSource(i/30f)).ToArray();
int reversals=0,previousSign=0;
for(int i=1;i<holdSamples.Length;i++){
 int sign=Math.Sign(holdSamples[i]-holdSamples[i-1]);if(sign==0)continue;
 if(previousSign!=0&&sign!=previousSign)reversals++;previousSign=sign;
}
Check(reversals==3&&holdSamples.Max()-holdSamples.Min()<=4.75f,"Hold must contain the earlier two eased pose excursions");
var reports=new List<object>();
var boundaries=new Dictionary<int,hkaInterleavedUncompressedAnimation>();
foreach(int id in new[]{40003,40004}){
 int count=id==40003?97:151-RestFrame;float duration=(count-1)/30f;
 var animation=new hkaInterleavedUncompressedAnimation{m_type=(hkaAnimation.AnimationType)1,m_duration=duration,m_numberOfTransformTracks=72,m_numberOfFloatTracks=0,m_floats=new(),m_transforms=new(),m_annotationTracks=new()};
 var motion=new hkaDefaultAnimatedReferenceFrame{m_up=originalMotion.m_up,m_forward=originalMotion.m_forward,m_duration=duration,m_referenceFrameSamples=new()};
 for(int frame=0;frame<count;frame++){
  float from=id==40003?HoldSource(frame/30f):RestFrame+frame;
  for(int track=0;track<72;track++)animation.m_transforms.Add(PoseAt(from,track));
  motion.m_referenceFrameSamples.Add(RootAt(from/30f)-RootAt((id==40003?HoldFrame:RestFrame)/30f));
 }
 animation.m_extractedMotion=motion;
 var parsed=(hkRootLevelContainer)reader.Read(new MemoryStream(source.Bytes.ToArray()));
 var target=(hkaAnimationContainer)parsed.m_namedVariants.Single().m_variant;
 target.m_animations[0]=animation;target.m_bindings.Single().m_animation=animation;
 using var stream=new MemoryStream();new HavokBinarySerializer().Write(parsed,stream);var bytes=stream.ToArray();
 var readback=(hkRootLevelContainer)new HavokBinarySerializer().Read(new MemoryStream(bytes));
 var actual=(hkaInterleavedUncompressedAnimation)((hkaAnimationContainer)readback.m_namedVariants.Single().m_variant).m_animations.Single();
 Check(actual.m_duration==duration&&actual.m_transforms.Count==count*72,"Motion readback length differs");
 foreach(var (a,b) in animation.m_transforms.Zip(actual.m_transforms))Check(a.m_translation==b.m_translation&&a.m_rotation==b.m_rotation&&a.m_scale==b.m_scale,"Motion readback pose differs");
 boundaries.Add(id,actual);
 int member=1841000000+id;Check(!binder.Files.Any(f=>f.ID==member),"Onyx custom motion ID already present");
 binder.Files.Add(new BinderFile(source.Flags,member,source.Name.Replace("a840_040000.hkx",$"a841_{id:D6}.hkx"),bytes));
 reports.Add(new{id,frames=count,duration,sourceFrame=id==40003?HoldFrame:RestFrame,tracks=72,continuousBoundaryFrame=RestFrame,impactRecoilOnlyAtEntry=true,uncompressed=true});
}
// Compare decoded output at the actual finish-marker frame against recovery.
for(int track=0;track<72;track++){
 var a=boundaries[40003].m_transforms[93*72+track];var b=boundaries[40004].m_transforms[track];
 Check(a.m_translation==b.m_translation&&a.m_rotation==b.m_rotation&&a.m_scale==b.m_scale,"Channel-to-recovery pose seam");
}
var settled=boundaries[40004];
float exitStep=Enumerable.Range(0,72).Max(t=>2*MathF.Acos(MathF.Min(1,MathF.Abs(Quaternion.Dot(settled.m_transforms[t].m_rotation,settled.m_transforms[72+t].m_rotation)))));
Check(exitStep<0.06f,"Recovery repeats the impact jerk");
binder.Files.Sort((a,b)=>a.ID.CompareTo(b.ID));Directory.CreateDirectory(Path.GetDirectoryName(output)!);binder.Write(output);
var check=BND4.Read(output);Check(check.Files.Count==original.Count+2,"Unexpected motion binder entries");
foreach(var f in check.Files)if(original.TryGetValue(f.ID,out var before))Check(f.Bytes.Span.SequenceEqual(before),"Existing animation modified");
File.WriteAllText(output+".validation.json",JsonSerializer.Serialize(new{inputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(input))),outputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(output))),motions=reports,originalEntriesUnchanged=true,source="target a840_040000",gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine("PASS: native impact and moving planted poses for 3.2s with continuous 1.7s withdrawal; both clips roundtrip exactly; all target a9x entries preserved");
