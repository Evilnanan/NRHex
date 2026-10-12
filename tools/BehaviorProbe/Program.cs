using System.Reflection;
using System.Runtime.InteropServices;
using SoulsFormats;
var smithbox=@"D:\Program Files\Smithbox";
var game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>Assembly.LoadFrom(Path.Combine(smithbox,new AssemblyName(a.Name).Name+".dll"));
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(name,_,_)=>name.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,name.EndsWith(".dll")?name:name+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
var bnd=BND4.Read(@"D:\Projects\NRMod\NRHex\assets\original\chr\c0000.behbnd.dcx");
var dir=@"D:\Projects\NRMod\NRHex\.work\behavior";Directory.CreateDirectory(dir);
foreach(var f in bnd.Files){var path=Path.Combine(dir,Path.GetFileName(f.Name));File.WriteAllBytes(path,f.Bytes.ToArray());Console.WriteLine($"{f.ID}: {f.Name} ({f.Bytes.Length})");}
var serializer=new HKLib.Serialization.hk2018.Binary.HavokBinarySerializer();
if(args.Contains("api")){foreach(var m in serializer.GetType().GetMethods())Console.WriteLine(m);foreach(var p in serializer.GetType().GetProperties())Console.WriteLine(p);return;}
var behavior=bnd.Files.Single(f=>f.ID==7000500).Bytes.ToArray();
// Read-only exploratory parsing: the installed registry predates NR's custom
// selector hash. Skip THSH in this memory copy; never write/load this copy.
var thsh=behavior.AsSpan().IndexOf("THSH"u8);if(thsh<0)throw new Exception("Missing hashes");
var hashLength=(int)(System.Buffers.Binary.BinaryPrimitives.ReadUInt32BigEndian(behavior.AsSpan(thsh-4,4))&0x3fffffff);
behavior.AsSpan(thsh+4,hashLength-8).Clear();
var reader=new HKLib.Serialization.hk2018.Binary.Util.HavokBinaryReader(behavior);
reader.Position=behavior.AsSpan().IndexOf("TYPE"u8)-4;
var readType=serializer.GetType().GetMethod("ReadTYPE",BindingFlags.NonPublic|BindingFlags.Instance)!;
var types=(IReadOnlyList<HKLib.Reflection.hk2018.HavokType>)readType.Invoke(serializer,new object[]{reader})!;
var originalReader=new HKLib.Serialization.hk2018.Binary.Util.HavokBinaryReader(bnd.Files.Single(f=>f.ID==7000500).Bytes.ToArray());originalReader.Position=thsh+4;
var hashes=new Dictionary<int,uint>();var numHashes=originalReader.ReadHavokVarUInt();
for(ulong i=0;i<numHashes;i++){var idx=(int)originalReader.ReadHavokVarUInt();hashes[idx]=originalReader.ReadUInt32();}
File.WriteAllText(Path.Combine(dir,"original-hashes.json"),System.Text.Json.JsonSerializer.Serialize(hashes));
foreach(var kv in hashes.Where(kv=>kv.Value!=types[kv.Key].Hash))Console.WriteLine("NR registry mismatch (no object traversal): "+types[kv.Key].Identity);
var report=types.Select(t=>t is null?null:new {t.Identity,t.Size,t.Alignment,t.Hash,Fields=t.Fields.Select(f=>new {f.Name,f.Offset,Type=f.Type.Identity})});
File.WriteAllText(Path.Combine(dir,"types.json"),System.Text.Json.JsonSerializer.Serialize(report,new System.Text.Json.JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine("Type metadata exported without traversing incompatible NR selector objects: "+types.Count);
