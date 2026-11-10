using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// The native right-foot contact ends the movement boost. Append an EzState
// flag, never rewrite the shared greatsword motion or any existing TAE event.
static class OnyxDash {
 public const int Flag=28;
 static void Check(bool ok,string message){if(!ok)throw new InvalidDataException(message);}
 static bool Marker(TAE.Event e)=>e.Type==227&&BitConverter.ToInt32(e.GetParameterBytes(false))==Flag;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 public static float FootContact(TAE.Animation a){
  var feet=a.Events.Where(e=>e.Type==792&&BitConverter.ToInt32(e.GetParameterBytes(false),4)==5).ToArray();
  // Current character FLVER: dummy 5 attaches to R_Foot; dummy 6 to L_Foot.
  Check(feet.Length==1&&Math.Abs(feet[0].StartTime-14f/30f)<0.00001f,"Review changed right-foot landing timing");
  Check(a.Events.Any(e=>e.Type==1&&BitConverter.ToInt32(e.GetParameterBytes(false),8)==125&&e.StartTime>feet[0].StartTime),"Running-heavy attack changed");
  return feet[0].StartTime;
 }
 public static void Build(string input,string output){
  var b=BND4.Read(input);var original=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var f=b.Files.Single(f=>Name(f)=="a40.tae");var t=TAE.Read(f.Bytes);
  var targets=t.Animations.Select((a,i)=>(a,i)).Where(p=>p.a.ID==30210&&p.a.Events.Count>0).ToArray();
  Check(targets.Length==1,"Ambiguous one-hand greatsword running-heavy animation");
  var (a,index)=targets[0];float end=FootContact(a);
  foreach(var file in b.Files.Where(f=>Name(f).EndsWith(".tae")))foreach(var clip in TAE.Read(file.Bytes).Animations)
   Check(!clip.Events.Any(Marker)||(file.ID==f.ID&&clip.ID==a.ID),"EzState flag 28 collision");
  if(a.Events.Any(Marker)){
   var old=a.Events.Single(Marker);Check(old.StartTime==0&&old.EndTime==end,"Existing Onyx dash window differs");
  }else{
   var raw=new byte[16];BitConverter.GetBytes(Flag).CopyTo(raw,0);
   var e=new TAE.Event(0,end,227,0,raw,false){Group=new TAE.EventGroup(227)};
   f.Bytes=TaeOverlay.Add(f.Bytes.ToArray(),new Dictionary<int,List<TAE.Event>>{{index,[e]}});
  }
  Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(output))!);b.Write(output);
  var check=BND4.Read(output);Check(check.Files.Count==b.Files.Count,"Animation binder member count changed");
  foreach(var (expected,actual)in b.Files.Zip(check.Files)){
   Check(expected.ID==actual.ID&&expected.Name==actual.Name&&expected.Flags==actual.Flags&&expected.Bytes.Span.SequenceEqual(actual.Bytes.Span),"Animation binder readback mismatch");
   if(actual.ID!=f.ID)Check(actual.Bytes.Span.SequenceEqual(original[actual.ID]),"Unrelated animation member changed");
  }
  var actualClip=TAE.Read(check.Files.Single(v=>v.ID==f.ID).Bytes).Animations.Single(v=>v.ID==30210&&v.Events.Count>0);
  Check(actualClip.Events.Count(Marker)==1&&actualClip.Events.Single(Marker).EndTime==FootContact(actualClip),"Startup window did not end at the right-foot contact");
  File.WriteAllText(output+".onyx-dash.json",JsonSerializer.Serialize(new{animation="a040_030210",flag=Flag,endSeconds=end,nativeRootDistance=2.3905556f,movementScale=2.5f,normalScale=1.25f,estimatedExtraMetres=2.9881945f,originalEventBytesPreserved=true,outputSha256=Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(output))),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Onyx dash flag ends at right-foot contact; all existing event bytes and other binder members retained");
 }
}
