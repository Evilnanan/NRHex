using SoulsFormats;
using System.Text.Json;

static class StormVfx {
 static readonly int[] StormEffects=[450940,450941,450945,450946,450950,450951,450954,450955,450956,450957,450958,450959,450960,450961,450962,450963,450965];
 static string Digest(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Stage(string root){
  void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
  string dir=Path.Combine(root,".work/storm-vfx-small"),source=Path.Combine(root,".work/storm-vfx-01757/extracted/sfx");
  string commonPath=Path.Combine(source,"sfxbnd_commoneffects_dlc01.ffxbnd.dcx"),bossPath=Path.Combine(source,"sfxbnd_c7540.ffxbnd.dcx");
  string sharedPath=Path.Combine(source,"sfxbnd_commoneffects.ffxbnd.dcx");
  // Refuse to overwrite another mod's common-resource changes without merging.
  string installed=Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");
  var common=BND4.Read(commonPath);var boss=BND4.Read(bossPath);var baseline=common.Files.ToArray();
  // Both common binders are requested by the same native initializer (C24BF9/C24C56).
  // Keep base-common textures/models in the original game archive instead of shipping it.
  var shared=BND4.Read(sharedPath).Files.Select(f=>Path.GetFileName(f.Name)).ToHashSet(StringComparer.OrdinalIgnoreCase);
  var byName=common.Files.ToDictionary(f=>Path.GetFileName(f.Name),StringComparer.OrdinalIgnoreCase);
  var originals=boss.Files.ToDictionary(f=>Path.GetFileName(f.Name),StringComparer.OrdinalIgnoreCase);
  var added=new List<BinderFile>();var refs=new HashSet<string>(StringComparer.OrdinalIgnoreCase);
  int NextId(int group)=>common.Files.Where(f=>f.ID>=group&&f.ID<group+100000).Max(f=>f.ID)+1;
  void Add(string name,int group){
   var input=originals[name];
   if(byName.TryGetValue(name,out var existing)){Check(existing.Bytes.Span.SequenceEqual(input.Bytes.Span),"Conflicting native resource: "+name);return;}
   int id=NextId(group);Check(id<group+100000,"Binder ID group overflow");
   var f=new BinderFile(input.Flags,id,input.Name,input.Bytes.ToArray());
   common.Files.Add(f);byName.Add(name,f);added.Add(f);
  }
  foreach(int effect in StormEffects){
   string stem=$"f{effect:000000000}";Add(stem+".fxr",0);Add(stem+".ffxreslist",400000);
   foreach(string r in System.Text.Encoding.UTF8.GetString(originals[stem+".ffxreslist"].Bytes.Span).Split(['\r','\n'],StringSplitOptions.RemoveEmptyEntries))refs.Add(r.Trim());
  }
  foreach(string r in refs){
   string ext=Path.GetExtension(r).ToLowerInvariant();Check(ext is ".tif" or ".sib","Unknown effect dependency: "+r);
   string name=Path.GetFileNameWithoutExtension(r)+(ext==".tif"?".tpf":".flver");
   if(!byName.ContainsKey(name)&&!shared.Contains(name))Add(name,ext==".tif"?100000:200000);
   // Native common resources are reused; only resources absent there are copied.
   Check(byName.ContainsKey(name)||shared.Contains(name),"Unresolved effect dependency: "+r);
  }
  common.Files.Sort((a,b)=>a.ID.CompareTo(b.ID));
  Check(common.Files.Select(f=>f.ID).Distinct().Count()==common.Files.Count,"Duplicate binder file ID");
  string output=Path.Combine(dir,"staged/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(output)!);common.Write(output);
  var final=BND4.Read(output);
  Check(!File.Exists(installed)||Digest(installed)==Digest(commonPath)||Digest(installed)==Digest(output),"Existing custom common effects require an explicit merge");
  Check(final.Files.Count==baseline.Length+added.Count&&final.Version==common.Version&&final.Format==common.Format&&final.Compression==common.Compression&&final.BigEndian==common.BigEndian&&final.Unicode==common.Unicode,"Common binder metadata/count changed");
  foreach(var old in baseline){var f=final.Files.Single(f=>f.ID==old.ID);Check(f.Name==old.Name&&f.Flags==old.Flags&&f.Bytes.Span.SequenceEqual(old.Bytes.Span),"Original common member changed");}
  foreach(var expected in added){var f=final.Files.Single(f=>f.ID==expected.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Added member differs from native boss resource");}
  Check(new FileInfo(output).Length<4*1024*1024,"Compact Storm VFX package unexpectedly exceeds 4 MiB");
  File.WriteAllText(Path.Combine(dir,"resource-validation.json"),JsonSerializer.Serialize(new{version="0.17.58",resourceRevision="compact-common-dlc01",resourcePath="mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx",commonInputSha256=Digest(commonPath),sharedInputSha256=Digest(sharedPath),bossInputSha256=Digest(bossPath),outputSha256=Digest(output),outputBytes=new FileInfo(output).Length,replacedLargePackageBytes=80602304,effects=StormEffects,dependencies=refs.Order(),added=added.Select(f=>new{f.ID,f.Name,bytes=f.Bytes.Length}),originalCommonMembers=baseline.Length,originalCommonMembersUnchanged=true,originalResourceBytesPreserved=true,attacksAnimationsAndStateConditionsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {StormEffects.Length} Storm Ruler effects and dependencies added ({added.Count} members); all {baseline.Length} native common members preserved; readback verified");
 }
 public static void Resources(string root){
  string dir=Path.Combine(root,".work/storm-vfx-01757/extracted/sfx");
  var boss=BND4.Read(Path.Combine(dir,"sfxbnd_c7540.ffxbnd.dcx"));
  var lists=boss.Files.Where(f=>f.Name.EndsWith(".ffxreslist")&&f.Name.Contains("4509"));
  foreach(var f in lists)Console.WriteLine(Path.GetFileName(f.Name)+": "+System.Text.Encoding.UTF8.GetString(f.Bytes.Span).Replace("\n"," | "));
  foreach(var f in boss.Files.Where(f=>f.Name.EndsWith(".tpf")||f.Name.EndsWith(".flver")))Console.WriteLine(Path.GetFileName(f.Name)+" "+f.Bytes.Length);
 }
 public static void Extract(string root,string smith,string game){
  var assembly=System.Reflection.Assembly.LoadFrom(Path.Combine(smith,"Andre.Formats.dll"));
  System.Runtime.InteropServices.NativeLibrary.SetDllImportResolver(assembly,(n,a,p)=>n.Contains("bhd5_decrypt_rust")?System.Runtime.InteropServices.NativeLibrary.Load(Path.Combine(smith,"bhd5_decrypt_rust.dll")):IntPtr.Zero);
  var keys=(Dictionary<string,string>)assembly.GetType("Andre.Formats.Util.ArchiveKeys")!.GetField("NightreignKeys",System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic|System.Reflection.BindingFlags.Static)!.GetValue(null)!;
  var key=keys.Single(k=>Path.GetFileNameWithoutExtension(k.Key).Equals("data0",StringComparison.OrdinalIgnoreCase)).Value;
  var rsa=assembly.GetType("Andre.Formats.Util.NativeRsa")!;
  var decrypt=rsa.GetMethods(System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.Static).Single(m=>m.Name=="Decrypt"&&m.GetParameters().Length==3);
  var encrypted=File.ReadAllBytes(Path.Combine(game,"data0.bhd"));
  var bytes=(byte[])decrypt.Invoke(null,new object[]{new Memory<byte>(encrypted),key!,2})!;
  var headers=BHD5.Read(bytes,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).ToDictionary(h=>h.FileNameHash);
  ulong Hash(string name){ulong h=0;foreach(char c in name.ToLowerInvariant())h=unchecked(h*0x85+c);return h;}
  var dir=Path.Combine(root,".work/storm-vfx-01757/extracted/sfx");Directory.CreateDirectory(dir);
  using var bdt=File.OpenRead(Path.Combine(game,"data0.bdt"));
  foreach(var name in new[]{"sfxbnd_commoneffects.ffxbnd.dcx","sfxbnd_commoneffects_dlc01.ffxbnd.dcx","sfxbnd_c7540.ffxbnd.dcx"}){
   var path=Path.Combine(dir,name);var data=headers[Hash("/sfx/"+name)].ReadFile(bdt);
   if(File.Exists(path)){if(!File.ReadAllBytes(path).AsSpan().SequenceEqual(data))throw new IOException("Extracted input differs: "+path);}else File.WriteAllBytes(path,data);
   var binder=BND4.Read(path);Console.WriteLine($"{name}: {binder.Files.Count} members; "+string.Join(",",binder.Files.Where(f=>f.Name.Contains("45095")).Select(f=>f.Name)));
   File.WriteAllText(path+".members.json",JsonSerializer.Serialize(binder.Files.Select(f=>new{f.ID,f.Name,size=f.Bytes.Length})));
  }
  File.WriteAllBytes(Path.Combine(root,".work/storm-vfx-01757/data0.bhd"),bytes);
  Console.WriteLine("PASS: fresh original BHD decrypted; selected resources extracted without using stale cache");
 }
 public static void ArchiveProbe(string smith){
  foreach(string file in new[]{"Andre.Formats.dll","Andre.IO.dll"}){
   var assembly=System.Reflection.Assembly.LoadFrom(Path.Combine(smith,file));
   foreach(var t in assembly.GetTypes().Where(t=>t.FullName!.Contains("Rsa",StringComparison.OrdinalIgnoreCase)||t.FullName!.Contains("Key",StringComparison.OrdinalIgnoreCase)||t.FullName!.Contains("Bhd",StringComparison.OrdinalIgnoreCase)||t.FullName!.Contains("Crypt",StringComparison.OrdinalIgnoreCase))){
    Console.WriteLine(t.FullName);
    foreach(var m in t.GetMethods(System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic|System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.Instance|System.Reflection.BindingFlags.DeclaredOnly))Console.WriteLine(m+" "+string.Join(",",m.GetParameters().Select(p=>$"{p.Name}={p.DefaultValue}")));
    foreach(var f in t.GetFields(System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic|System.Reflection.BindingFlags.Static))Console.WriteLine(f);
   }
  }
 }
 public static void Probe(string root,BND4 regulation,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/storm-vfx-01757");Directory.CreateDirectory(dir);
  PARAM Table(string n)=>parse(n,regulation.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes);
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  var result=new Dictionary<string,object>();
  result["arts"]=Table("SwordArtsParam").Rows.Where(r=>r.ID==1200).Select(Dump).ToArray();
  var weapons=Table("EquipParamWeapon").Rows.Where(r=>Convert.ToInt32(r["swordArtsParamId"].Value)==1200).ToArray();
  result["weapons"]=weapons.Select(Dump).ToArray();
  var behaviors=Table("BehaviorParam_PC").Rows.Where(r=>Convert.ToInt32(r["variationId"].Value)==0&&Convert.ToInt32(r["behaviorJudgeId"].Value) is >=4200 and <=4250).ToArray();
  result["behaviors"]=behaviors.Select(Dump).ToArray();
  // Export attack/visual linkage only; large tables are never printed to the console.
  foreach(string n in new[]{"Bullet","SpEffectParam","SpEffectVfxParam"})result[n]=Table(n).Rows.Select(Dump).ToArray();
  var anim=BND4.Read(Path.Combine(root,"assets/original/chr/c0000.anibnd.dcx"));
  result["taeFiles"]=anim.Files.Where(f=>f.Name.EndsWith(".tae")).Select(f=>new{f.ID,f.Name}).ToArray();
  result["animation"]=anim.Files.Where(f=>f.Name.EndsWith("a860.tae")).Select(tae=>new{tae.Name,animations=TAE.Read(tae.Bytes).Animations.Select(a=>new{a.ID,events=a.Events.Select(e=>new{e.Type,e.StartTime,e.EndTime,hex=Convert.ToHexString(e.GetParameterBytes(false))})})});
  File.WriteAllText(Path.Combine(dir,"probe.json"),JsonSerializer.Serialize(result));
  Console.WriteLine("PASS: original Storm Ruler parameter and animation references exported read-only");
 }
}
