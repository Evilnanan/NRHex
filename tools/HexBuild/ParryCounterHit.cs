using SoulsFormats;
using System.Text.Json;

// Preserve the live guard's attack condition, including MMV-only behaviors.
static class ParryCounterHit {
 public const int TagBase=96000000;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 public static bool IsTag(int judge)=>judge is >=TagBase+14000 and <TagBase+24000;
 public static string Catalog(IEnumerable<PARAM.Row> behaviors,Dictionary<int,PARAM.Row> attacks){
  bool Counter(PARAM.Row row)=>Enumerable.Range(1,5).Any(i=>Convert.ToInt32(row["subCategory"+i].Value)==103);
  var ids=behaviors.Where(r=>Convert.ToInt32(r["refType"].Value)==0&&attacks.TryGetValue(Convert.ToInt32(r["refId"].Value),out var a)&&Counter(a)).Select(r=>r.ID).Distinct().Order().ToArray();
  Check(ids.Length>0,"Missing guard-counter catalog");
  return "pub const COUNTER_BEHAVIORS:&[i32]=&[\n"+string.Join("\n",ids.Select(id=>$" {id},"))+"\n];\n";
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,bool mmv){
  string label=mmv?"mmv":"standalone",dist=Path.Combine(root,mmv?"dist/mmv":"dist"),dir=Path.Combine(root,".work/parry-counter-hit",label);Directory.CreateDirectory(dir);
  var regulation=SFUtil.DecryptNightreignRegulation(Path.Combine(dist,"mod/regulation.bin"));
  PARAM Table(string n)=>parse(n,regulation.Files.Single(f=>Path.GetFileName(f.Name.Replace('\\','/'))==n+".param").Bytes);
  var attacks=Table("AtkParam_Pc").Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>{
   Check(Enumerable.Range(1,5).All(i=>g.Select(r=>Convert.ToInt32(r["subCategory"+i].Value)).Distinct().Count()==1),"Ambiguous counter categories: "+g.Key);return g.First();
  });
  var behaviors=Table("BehaviorParam_PC").Rows.GroupBy(r=>r.ID).Select(g=>{
   Check(new[]{"refType","refId"}.All(k=>g.Select(r=>Convert.ToInt32(r[k].Value)).Distinct().Count()==1),"Ambiguous counter behavior: "+g.Key);return g.First();
  }).ToArray();
  string source=Path.Combine(root,mmv?"work/compat/mmv/staged/parry_data.rs":"src/parry_data.rs");
  var data=File.ReadAllText(source);
  int conditionCount=data.Split("pub const CONDITIONS:")[1].Split("];",2)[0].Count(c=>c=='\n')-1;
  Check(conditionCount>0,"Missing guard conditions");int end=data.IndexOf("pub const COUNTER_BEHAVIORS:");if(end>=0)data=data[..end];
  File.WriteAllText(Path.Combine(dir,"parry_data.rs"),data+Catalog(behaviors,attacks));
  string input=Path.Combine(dist,"mod/chr/c0000.anibnd.dcx");var b=BND4.Read(input);int count=0;
  foreach(var f in b.Files.Where(f=>f.Name.EndsWith(".tae"))){
   byte[] raw=f.Bytes.ToArray(),original=raw.ToArray();var t=TAE.Read(raw);var allowed=new HashSet<int>();
   long L(int at)=>BitConverter.ToInt64(raw,at);int I(int at)=>BitConverter.ToInt32(raw,at);
   int headers=checked((int)L(0x58));
   for(int i=0;i<t.Animations.Count;i++){
    var a=t.Animations[i];int body=checked((int)L(headers+i*16+8)),events=checked((int)L(body));
    Check(L(headers+i*16)==a.ID&&I(body+32)==a.Events.Count,"TAE topology changed");
    for(int j=0;j<a.Events.Count;j++){
     var e=a.Events[j];var bytes=e.GetParameterBytes(false);
     if(e.Type!=0||bytes.Length<16||BitConverter.ToInt32(bytes)!=3)continue;
     int judge=BitConverter.ToInt32(bytes,8);if(judge!=Parry.GuardJudge&&!IsTag(judge))continue;
     int state=BitConverter.ToUInt16(bytes,14);Check(state>=14000&&state<14000+conditionCount*7,"Invalid private guard state");
     int header=checked((int)L(events+j*24+16)),payload=checked((int)L(header+8));
     Check(I(header)==0&&raw.AsSpan(payload,bytes.Length).SequenceEqual(bytes),"TAE payload moved");
     BitConverter.GetBytes(TagBase+state).CopyTo(raw,payload+8);for(int k=0;k<4;k++)allowed.Add(payload+8+k);count++;
    }
   }
   for(int i=0;i<raw.Length;i++)Check(allowed.Contains(i)||raw[i]==original[i],"Unrelated TAE byte changed");
   var read=TAE.Read(raw);Check(read.Animations.Count==t.Animations.Count,"Animation count changed");
   foreach(var pair in t.Animations.Zip(read.Animations))foreach(var ev in pair.First.Events.Zip(pair.Second.Events)){
    var expected=ev.First.GetParameterBytes(false).ToArray();
    if(ev.First.Type==0&&expected.Length>=16&&(BitConverter.ToInt32(expected,8)==Parry.GuardJudge||IsTag(BitConverter.ToInt32(expected,8))))BitConverter.GetBytes(TagBase+BitConverter.ToUInt16(expected,14)).CopyTo(expected,8);
    Check(ev.First.Type==ev.Second.Type&&ev.First.StartTime==ev.Second.StartTime&&ev.First.EndTime==ev.Second.EndTime&&expected.SequenceEqual(ev.Second.GetParameterBytes(false)),"Unexpected event change");
   }
   f.Bytes=raw;
  }
  Check(count>0,"No Clash guard events found");string output=Path.Combine(dir,"c0000.anibnd.dcx");b.Write(output);var final=BND4.Read(output);
  Check(final.Files.Count==b.Files.Count&&final.Compression==b.Compression,"Binder metadata changed");
  foreach(var pair in final.Files.Zip(b.Files))Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),"Binder readback mismatch");
  string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{label,inputSha256=Hash(input),outputSha256=Hash(output),guards=count,onlyGuardJudgeBytesChanged=true,originalTimingAndConditionsPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {label} {count} guard tags and native counter catalog staged/read back");
 }
}
