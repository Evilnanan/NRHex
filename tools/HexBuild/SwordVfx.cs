using SoulsFormats;
using System.Text.Json;

static class SwordVfx {
 public static void StormProbe(string root){
  var binder=BND4.Read(Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx"));
  var dir=Path.Combine(root,".work/sword-vfx/storm");Directory.CreateDirectory(dir);
  foreach(var f in binder.Files.Where(f=>f.Name.Contains("f0004509")&&(f.Name.EndsWith(".fxr")||f.Name.EndsWith(".ffxreslist")))){
   File.WriteAllBytes(Path.Combine(dir,Path.GetFileName(f.Name)),f.Bytes.ToArray());
   if(f.Name.EndsWith(".ffxreslist"))Console.WriteLine(Path.GetFileName(f.Name)+": "+System.Text.Encoding.UTF8.GetString(f.Bytes.Span).Replace("\n"," | "));
  }
 }
 public static void StageFxr(string root){
  string input=Path.Combine(root,"dist/mod/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");
  var binder=BND4.Read(input);var baseline=binder.Files.ToArray();
  // Native a860 40050/40051/40052 keep 450955 at dummy 20 throughout stance.
  // It contains the conjured sword model 88350. 450954 is its separate wind layer.
  var fxr=binder.Files.Single(f=>Path.GetFileName(f.Name)=="f000450955.fxr");
  Check(BitConverter.ToInt32(fxr.Bytes.Span.Slice(12,4))==450955,"Storm stance FXR identity changed");
  Check(Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(fxr.Bytes.Span))=="2AB5BE0FF8CD2C5A1B3B4DA9D630974BE1B34707148A26720490C8F8CEB7B1F1","Native Storm stance FXR changed");
  var resources=binder.Files.Single(f=>Path.GetFileName(f.Name)=="f000450955.ffxreslist");
  Check(Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(resources.Bytes.Span))=="498A4A09561AB8193E5001176ED971547105EBA2F0BF8BBAAC03340F9715118C","Native Storm stance resources changed");
  Check(System.Text.Encoding.UTF8.GetString(resources.Bytes.Span).Contains("s88350.sib"),"Storm stance sword model missing");
  Check(binder.Files.Any(f=>Path.GetFileName(f.Name).Equals("s88350.flver",StringComparison.OrdinalIgnoreCase)),"Storm sword model not packaged");
  var retired=binder.Files.Where(f=>Path.GetFileName(f.Name) is "f000950940.fxr" or "f000950940.ffxreslist").ToArray();
  foreach(var f in retired){
   if(f.Name.EndsWith(".fxr"))Check(Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(f.Bytes.Span))=="1D5481D11BFB316F1B1EE3F9A95F1D910C992D23CE7C66707AE128EDC2DF5AAB","Unknown private visual; refusing removal");
   else Check(System.Text.Encoding.UTF8.GetString(f.Bytes.Span).Replace("\r","")=="s31211_a.tif\ns34030_a.tif\ns34050_a.tif\n","Unknown private visual dependencies");
   binder.Files.Remove(f);
  }
  binder.Files.Sort((a,b)=>a.ID.CompareTo(b.ID));
  string output=Path.Combine(root,".work/sword-vfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx");binder.Write(output);
  var final=BND4.Read(output);
  Check(final.Files.Count==binder.Files.Count&&final.Version==binder.Version&&final.Format==binder.Format&&final.Compression==binder.Compression&&final.BigEndian==binder.BigEndian&&final.Unicode==binder.Unicode,"FXR binder metadata changed");
  foreach(var f in binder.Files){var actual=final.Files.Single(x=>x.ID==f.ID);Check(actual.Name==f.Name&&actual.Flags==f.Flags&&actual.Bytes.Span.SequenceEqual(f.Bytes.Span),"FXR member readback failed");}
  foreach(var f in baseline.Except(retired))Check(final.Files.Single(x=>x.ID==f.ID).Bytes.Span.SequenceEqual(f.Bytes.Span),"Existing FXR member changed");
  string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
  File.WriteAllText(Path.Combine(root,".work/sword-vfx/resource-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),scale=1.0,sfx=450955,swordModel=88350,retired=retired.Select(f=>f.Name),originalMembersPreserved=true,addedBytes=new FileInfo(output).Length-new FileInfo(input).Length},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native Storm Ruler stance sword reused; retired Loretta copy removed; all other members preserved");
 }
 public static void ModelProbe(string root,string smith,string game){
  var assembly=System.Reflection.Assembly.LoadFrom(Path.Combine(smith,"Andre.Formats.dll"));
  System.Runtime.InteropServices.NativeLibrary.SetDllImportResolver(assembly,(n,a,p)=>n.Contains("bhd5_decrypt_rust")?System.Runtime.InteropServices.NativeLibrary.Load(Path.Combine(smith,"bhd5_decrypt_rust.dll")):IntPtr.Zero);
  var keys=(Dictionary<string,string>)assembly.GetType("Andre.Formats.Util.ArchiveKeys")!.GetField("NightreignKeys",System.Reflection.BindingFlags.Public|System.Reflection.BindingFlags.NonPublic|System.Reflection.BindingFlags.Static)!.GetValue(null)!;
  var decrypt=assembly.GetType("Andre.Formats.Util.NativeRsa")!.GetMethods().Single(m=>m.Name=="Decrypt"&&m.GetParameters().Length==3);
  ulong Hash(string name){ulong h=0;foreach(char c in name.ToLowerInvariant())h=unchecked(h*0x85+c);return h;}
  string dir=Path.Combine(root,".work/sword-vfx/models");Directory.CreateDirectory(dir);
  foreach(var group in new[]{("data0",new[]{"/parts/wp_a_1310.partsbnd.dcx","/parts/wp_a_1330.partsbnd.dcx"}),("data3",new[]{"/chr/c0000.chrbnd.dcx"})}){
   var key=keys.Single(k=>Path.GetFileNameWithoutExtension(k.Key)==group.Item1).Value;
   var bytes=(byte[])decrypt.Invoke(null,new object[]{new Memory<byte>(File.ReadAllBytes(Path.Combine(game,group.Item1+".bhd"))),key,2})!;
   var headers=BHD5.Read(bytes,BHD5.Game.EldenRing).Buckets.SelectMany(b=>b).ToDictionary(h=>h.FileNameHash);
   using var bdt=File.OpenRead(Path.Combine(game,group.Item1+".bdt"));
   foreach(string name in group.Item2){
    var data=headers[Hash(name)].ReadFile(bdt);var binder=BND4.Read(data);
    foreach(var f in binder.Files.Where(f=>f.Name.EndsWith(".flver")||f.Name.EndsWith(".flv"))){
     var model=FLVER2.Read(f.Bytes);var nodes=model.GetType().GetProperty("Nodes")??model.GetType().GetProperty("Bones");
     File.WriteAllText(Path.Combine(dir,Path.GetFileName(f.Name)+".json"),JsonSerializer.Serialize(new{f.Name,model.Dummies,nodes=nodes?.GetValue(model)},new JsonSerializerOptions{WriteIndented=true,IncludeFields=true}));
     Console.WriteLine($"{f.Name}: {model.Dummies.Count} dummy points");
    }
   }
  }
 }
 static readonly int[] Ids=[95000940,95000941,95000944,95000952,95000960,95000968];
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string field,object v){r[field].Value=Convert.ChangeType(v,r[field].Value.GetType());}
 static bool Same(PARAM.Row a,PARAM.Row b)=>a.Cells.Where(c=>c!=null).All(c=>JsonSerializer.Serialize(c.Value)==JsonSerializer.Serialize(b[c.Def.InternalName].Value));
 public static void Build(Func<string,int,int,PARAM.Row> clone){
  foreach(var (source,id,vfx) in new[]{(1646,95000940,8555),(1648,95000941,8556)}){
   var row=clone("SpEffectParam",source,id);Check(Convert.ToInt32(row["vfxId"].Value)==vfx,"Native Loretta visual changed");
   Check(Convert.ToSingle(row["effectEndurance"].Value)==.1f,"Native Loretta lifetime changed");
   Set(row,"effectEndurance",-1f);Set(row,"vfxId",id);
   var visual=clone("SpEffectVfxParam",vfx,id);
   Check(Convert.ToInt32(visual["midstSfxId"].Value)==303301,"Native Loretta resource changed");
   Set(visual,"midstSfxId",450955);Set(visual,"midstSeId",-1);
   // Staff/seal FLVERs lack the 300-series enchant dummies used by 8555/8556.
   // Native Storm stance uses c0000 dummy 20 (R_Weapon). Its left counterpart
   // is 21 (L_Weapon); both belong to the actor and follow the casting weapon.
   Set(visual,"effectType",0);Set(visual,"midstDmyId",source==1646?20:21);
   Set(visual,"isUseOffsetEnchantSfxSize",0);Set(visual,"traceSfxIdOffsetType",0);
   for(int i=0;i<8;i++){Set(visual,$"enchantStartDmyId_{i}",-1);Set(visual,$"enchantEndDmyId_{i}",-1);}
  }
  foreach(int id in new[]{95000944,95000952,95000960,95000968})clone("SpEffectParam",95000904,id);
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/sword-vfx");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);
  string[] names=["SpEffectParam","SpEffectVfxParam"];
  var files=names.ToDictionary(n=>n,n=>binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param"));
  var tables=files.ToDictionary(k=>k.Key,k=>parse(k.Key,k.Value.Bytes));
  var originals=files.ToDictionary(k=>k.Key,k=>parse(k.Key,k.Value.Bytes));
  Build((name,source,id)=>{
   var table=tables[name];var row=new PARAM.Row(table.Rows.Single(r=>r.ID==source)){ID=id};
   table.Rows.RemoveAll(r=>r.ID==id);table.Rows.Add(row);return row;
  });
  foreach(string name in names){
   var table=tables[name];var intended=table.Rows.ToDictionary(r=>r.ID);
   foreach(var row in originals[name].Rows){
    var expected=intended[row.ID];if(Same(row,expected))continue;
    if(name=="SpEffectVfxParam"&&row.ID is 95000940 or 95000941){
     var previous=new PARAM.Row(expected);Set(previous,"midstSfxId",Convert.ToInt32(row["midstSfxId"].Value));
     Check(Convert.ToInt32(previous["midstSfxId"].Value) is 303301 or 950940,"Unsupported previous Sword FXR");
     Set(previous,"midstSeId",303071);Set(previous,"midstDmyId",row.ID==95000940?1:2);
     Check(Same(row,previous),"Unsupported previous Sword FXR row");continue;
    }
    Check(name=="SpEffectParam"&&row.ID is 95000940 or 95000941,"Unexpected existing row change");
    var legacy=new PARAM.Row(expected);Set(legacy,"vfxId",row.ID==95000940?8555:8556);
    Check(Same(row,legacy),"Unsupported previous Sword visual row");
   }
   table.Rows.Sort((a,b)=>a.ID.CompareTo(b.ID));files[name].Bytes=table.Write();
  }
  string path=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(path,binder,binder.Compression);
  var output=SFUtil.DecryptNightreignRegulation(path);
  Check(output.Files.Count==binder.Files.Count&&output.Version==binder.Version&&output.Compression==binder.Compression,"Binder metadata changed");
  foreach(var f in output.Files){var expected=binder.Files.Single(x=>x.ID==f.ID);Check(f.Name==expected.Name&&f.Flags==expected.Flags&&f.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder readback differs");}
  foreach(string name in names){
   var final=parse(name,output.Files.Single(f=>f.ID==files[name].ID).Bytes);var intended=tables[name].Rows.ToDictionary(r=>r.ID);
   Check(final.Rows.Count==intended.Count&&final.Rows.All(row=>Same(row,intended[row.ID])),"Parameter readback differs");
  }
  string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
  File.WriteAllText(Path.Combine(dir,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(path),privateEffects=Ids,nativeVisualEffects=new[]{1646,1648},vfx=new[]{95000940,95000941},sfx=450955,rightHandDummy=20,leftHandDummy=21,weaponOffsetsDisabled=true,originalRowsAndOtherTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Storm stance sword attached to verified right/left weapon bones; only private Sword rows changed");
 }
 public static void Probe(string root,BND4 binder,Func<string,Memory<byte>,PARAM> parse){
  var effects=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var vfx=parse("SpEffectVfxParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
  var weapons=parse("EquipParamWeapon",binder.Files.Single(f=>Path.GetFileName(f.Name)=="EquipParamWeapon.param").Bytes);
  var rows=effects.Rows.Where(r=>new[]{1645,1646,1647,1648,8000000}.Contains(r.ID)).ToArray();
  var ids=rows.Select(r=>Convert.ToInt32(r["vfxId"].Value)).ToHashSet();
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  string dir=Path.Combine(root,".work/sword-vfx");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"native-weapons.json"),JsonSerializer.Serialize(weapons.Rows.Where(r=>r.ID%10000==0).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null&&(c.Def.InternalName.Contains("Sfx")||c.Def.InternalName.Contains("Dmy")||c.Def.InternalName.Contains("Model")||c.Def.InternalName=="wepType")).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),new JsonSerializerOptions{WriteIndented=true}));
  var handRows=vfx.Rows.Where(r=>new[]{100,101,110,111,120,121,200,201}.Contains(Convert.ToInt32(r["midstDmyId"].Value))&&Convert.ToInt32(r["effectType"].Value)==0).ToArray();
  var handIds=handRows.Select(r=>r.ID).ToHashSet();
  File.WriteAllText(Path.Combine(dir,"native-hands.json"),JsonSerializer.Serialize(new{vfx=handRows.Select(Dump),effects=effects.Rows.Where(r=>handIds.Contains(Convert.ToInt32(r["vfxId"].Value))).Select(r=>new{r.ID,vfxId=r["vfxId"].Value})},new JsonSerializerOptions{WriteIndented=true}));
  File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(new{effects=rows.Select(Dump),vfx=vfx.Rows.Where(r=>ids.Contains(r.ID)).Select(Dump)},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Exported native Loretta weapon effects.");
 }
}
