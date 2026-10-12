// Compare decrypted regulation members; encryption output bytes may differ.
using System.Reflection;
using System.Runtime.InteropServices;
using SoulsFormats;
if(args.Length!=2)throw new ArgumentException("Usage: AssetCompare <before regulation.bin> <after regulation.bin>");
var inputs=args.Select(Path.GetFullPath).ToArray();
const string smith=@"D:\Program Files\Smithbox";
const string game=@"D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game";
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,_,_)=>n.StartsWith("oo2core_")?NativeLibrary.Load(Path.Combine(game,n.EndsWith(".dll")?n:n+".dll")):IntPtr.Zero);
Directory.SetCurrentDirectory(game);
var before=SFUtil.DecryptNightreignRegulation(inputs[0]);
var after=SFUtil.DecryptNightreignRegulation(inputs[1]);
if(before.Version!=after.Version||before.Files.Count!=after.Files.Count)throw new InvalidDataException("Regulation binder version/count differ");
foreach(var (a,b) in before.Files.Zip(after.Files))
 if(a.ID!=b.ID||a.Name!=b.Name||a.Flags!=b.Flags||!a.Bytes.Span.SequenceEqual(b.Bytes.Span))throw new InvalidDataException("Changed regulation member: "+a.Name);
Console.WriteLine($"PASS: all {before.Files.Count} decrypted regulation members and identities are byte-identical.");
