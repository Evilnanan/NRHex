using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Xml.Linq;
using SoulsFormats;

if(args.Length!=4)throw new ArgumentException("MmvFixes <repository> <MMV> <original-menu-extract> <output>");
var root=Path.GetFullPath(args[0]);var mmv=Path.GetFullPath(args[1]);var original=Path.GetFullPath(args[2]);var output=Path.GetFullPath(args[3]);
if(output.StartsWith(mmv+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase)||output==mmv)throw new InvalidDataException("Fix package must be outside MMV");
var drive=Path.GetPathRoot(root)!;
var smith=Environment.GetEnvironmentVariable("NRHEX_SMITHBOX")??Path.Combine(drive,"Program Files/Smithbox");
var game=Environment.GetEnvironmentVariable("NRHEX_GAME")??Path.Combine(drive,"Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game");
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
string Sha(byte[] b)=>Convert.ToHexString(SHA256.HashData(b)).ToLowerInvariant();
string Hash(string p)=>Sha(File.ReadAllBytes(p));
void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
void BinderCheck(BND4 a,BND4 b){Check(a.Version==b.Version&&a.Compression==b.Compression&&a.Files.Count==b.Files.Count,"Binder metadata mismatch");foreach(var (x,y) in a.Files.Zip(b.Files))Check(x.ID==y.ID&&x.Name==y.Name&&x.Flags==y.Flags&&x.Bytes.Span.SequenceEqual(y.Bytes.Span),"Binder member mismatch: "+x.Name);}
using var boneSpec=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"compat/mmv-fixes/c6220-twist-bones.json")));
using var iconSpec=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"compat/mmv-fixes/dlc-missing-icons.json")));
var inputs=new Dictionary<string,string>();
foreach(var spec in new[]{boneSpec.RootElement,iconSpec.RootElement})foreach(var field in spec.GetProperty("sourceFiles").EnumerateObject()){
 var path=Path.Combine(mmv,"mod",field.Name);Check(Hash(path)==field.Value.GetString(),"MMV changed; review/remove this fix before rebuilding: "+field.Name);inputs[path]=Hash(path);
}
foreach(var field in iconSpec.RootElement.GetProperty("originalFiles").EnumerateObject()){
 var path=Path.Combine(original,field.Name);Check(Hash(path)==field.Value.GetString(),"Original icon input changed: "+field.Name);inputs[path]=Hash(path);
}
Directory.CreateDirectory(Path.Combine(output,"mod/chr"));Directory.CreateDirectory(Path.Combine(output,"mod/menu"));
var specBone=boneSpec.RootElement;
var behavior=BND4.Read(Path.Combine(mmv,"mod/chr/c6220.behbnd.dcx"));
var before=BND4.Read(Path.Combine(mmv,"mod/chr/c6220.behbnd.dcx"));
var member=behavior.Files.Single(f=>f.ID==specBone.GetProperty("behaviorMemberId").GetInt32());
Check(Sha(member.Bytes.ToArray())==specBone.GetProperty("behaviorSha256").GetString(),"Behavior member changed");
var skeleton=BND4.Read(Path.Combine(mmv,"mod/chr/c6220.anibnd.dcx")).Files.Single(f=>f.ID==specBone.GetProperty("skeletonMemberId").GetInt32());
Check(Sha(skeleton.Bytes.ToArray())==specBone.GetProperty("skeletonSha256").GetString(),"Skeleton member changed");
var bytes=member.Bytes.ToArray();var allowed=new HashSet<int>();
foreach(var patch in specBone.GetProperty("patches").EnumerateArray()){
 int offset=patch.GetProperty("offset").GetInt32();var old=Convert.FromHexString(patch.GetProperty("before").GetString()!);var next=Convert.FromHexString(patch.GetProperty("after").GetString()!);
 Check(old.Length==2&&next.Length==2&&bytes.AsSpan(offset,2).SequenceEqual(old),"Bone field mismatch");
 Check(BitConverter.ToInt16(next)>=0&&BitConverter.ToInt16(next)<specBone.GetProperty("boneCount").GetInt32(),"Bone index out of range");
 foreach(int at in Enumerable.Range(offset,2))Check(allowed.Add(at),"Overlapping bone patches");
 next.CopyTo(bytes,offset);
}
var previous=member.Bytes.ToArray();Check(previous.Where((value,index)=>value!=bytes[index]&&!allowed.Contains(index)).Count()==0,"Unrelated behavior changed");member.Bytes=bytes;
foreach(var (a,b) in before.Files.Zip(behavior.Files))if(a.ID!=member.ID)Check(a.Bytes.Span.SequenceEqual(b.Bytes.Span),"Unrelated behavior member changed");
var behaviorPath=Path.Combine(output,"mod/chr/c6220.behbnd.dcx");behavior.Write(behaviorPath);BinderCheck(behavior,BND4.Read(behaviorPath));

const string bank="21_common_dlc01_h";
var layouts=BND4.Read(Path.Combine(mmv,"mod/menu",bank+".sblytbnd.dcx"));var textures=TPF.Read(Path.Combine(mmv,"mod/menu",bank+".tpf.dcx"));
var nativeLayouts=BND4.Read(Path.Combine(original,"menu",bank+".sblytbnd.dcx"));var nativeTextures=TPF.Read(Path.Combine(original,"menu",bank+".tpf.dcx"));
var oldLayouts=layouts.Files.Select(f=>(f.ID,f.Name,f.Flags,Data:f.Bytes.ToArray())).ToArray();
var oldTextures=textures.Textures.Select(t=>(t.Name,t.Format,t.Flags1,t.Type,Data:t.Bytes.ToArray())).ToArray();
XDocument Xml(BinderFile f){using var stream=new MemoryStream(f.Bytes.ToArray());return XDocument.Load(stream);}
var present=layouts.Files.SelectMany(f=>Xml(f).Root!.Elements().Select(e=>(string)e.Attribute("name")!)).ToHashSet();
var added=new List<string>();
foreach(var entry in iconSpec.RootElement.GetProperty("missing").EnumerateObject()){
 var wanted=entry.Value.EnumerateArray().Select(v=>v.GetString()!).ToHashSet();Check(!wanted.Any(present.Contains),"An icon is now provided by MMV; review/remove the old fix");
 var layout=nativeLayouts.Files.Single(f=>Name(f)==entry.Name+".layout");var doc=Xml(layout);var elements=doc.Root!.Elements().ToArray();
 Check(wanted.IsSubsetOf(elements.Select(e=>(string)e.Attribute("name")!)),"Missing original icon");
 foreach(var element in elements)if(!wanted.Contains((string)element.Attribute("name")!))element.Remove();
 string newName="MMVFixes_"+entry.Name;
 doc.Root.SetAttributeValue("imagePath",((string)doc.Root.Attribute("imagePath")!).Replace(entry.Name,newName));
 var data=Encoding.UTF8.GetBytes(doc.ToString());
 var f=new BinderFile(layout.Flags,layouts.Files.Max(x=>x.ID)+1,layout.Name.Replace(entry.Name,newName),data);layouts.Files.Add(f);
 var texture=nativeTextures.Textures.Single(t=>t.Name==entry.Name);Check(!textures.Textures.Any(t=>t.Name==newName),"Duplicate supplemental texture");texture.Name=newName;textures.Textures.Add(texture);
 added.AddRange(wanted);
}
var layoutPath=Path.Combine(output,"mod/menu",bank+".sblytbnd.dcx");var texturePath=Path.Combine(output,"mod/menu",bank+".tpf.dcx");
layouts.Write(layoutPath);textures.Write(texturePath);
var readLayouts=BND4.Read(layoutPath);BinderCheck(layouts,readLayouts);var readTextures=TPF.Read(texturePath);
Check(readTextures.Textures.Count==textures.Textures.Count&&textures.Textures.Zip(readTextures.Textures).All(p=>p.First.Name==p.Second.Name&&p.First.Bytes.SequenceEqual(p.Second.Bytes)),"Texture readback failed");
foreach(var x in oldLayouts){var y=readLayouts.Files.Single(f=>f.ID==x.ID);Check(x.Name==y.Name&&x.Flags==y.Flags&&x.Data.AsSpan().SequenceEqual(y.Bytes.Span),"Existing MMV layout changed");}
foreach(var x in oldTextures){var y=readTextures.Textures.Single(f=>f.Name==x.Name);Check(x.Format==y.Format&&x.Flags1==y.Flags1&&x.Type==y.Type&&x.Data.SequenceEqual(y.Bytes),"Existing MMV texture changed");}
Check(inputs.All(x=>Hash(x.Key)==x.Value),"Source modified during build");
var files=Directory.GetFiles(Path.Combine(output,"mod"),"*",SearchOption.AllDirectories).ToDictionary(p=>Path.GetRelativePath(output,p).Replace('\\','/'),Hash);
File.WriteAllText(Path.Combine(output,"manifest.json"),JsonSerializer.Serialize(new{schemaVersion=1,name="MMV-Fixes",sourceFiles=inputs.Where(x=>Path.GetFullPath(x.Key).StartsWith(mmv+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase)).ToDictionary(x=>x.Key,x=>x.Value),buildInputs=inputs,files,fixes=new[]{"c6220-twist-bones","dlc-missing-icons"},restoredIcons=added.Order(),behaviorPatchedFields=4,existingMmvAtlasEntriesUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine($"PASS: four bone fields corrected; {added.Count} missing icons restored in two supplemental atlases; MMV inputs unchanged. {output}");
