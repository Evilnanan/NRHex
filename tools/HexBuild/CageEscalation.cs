using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Final event pass, run after the standalone or MMV encounter pool is complete.
// Category 5 counters are peer/map-instance local; category 7 publishes only
// the opener's previous count. Each peer reconstructs native local effects.
static class CageEscalation {
 public const int Effect=95001604, Icon=20406, DefenseIcon=20408, Bits=32;
 const int LegacyEventBase=9102000;
 static bool Owned(EMEVD.Event e,uint prefix)=>
  (e.ID>=LegacyEventBase&&e.ID<LegacyEventBase+256||e.ID>=prefix+8500&&e.ID<prefix+9000)
  &&e.Instructions.Any(i=>Op(i,2004,8)&&U(i,4)==Effect);
 static EMEVD.Instruction I(int b,int id,params object[] a)=>new(b,id,a);
 static uint U(EMEVD.Instruction i,int n=0)=>BitConverter.ToUInt32(i.ArgData,n);
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static bool Op(EMEVD.Instruction i,int b,int id)=>i.Bank==b&&i.ID==id;
 static EMEVD.Instruction Flag(int group,uint flag,bool on)=>I(3,0,(sbyte)group,(byte)(on?1:0),(byte)0,flag);
 static EMEVD.Instruction Set(uint flag,bool on)=>I(2003,66,(byte)0,flag,(byte)(on?1:0));
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static void Field(PARAM.Row row,string name,object value)=>row[name].Value=Convert.ChangeType(value,row[name].Value.GetType());
 public static void StripMap(EMEVD map,uint prefix){
  var owned=map.Events.Where(e=>Owned(e,prefix)).Select(e=>e.ID).ToHashSet();
  if(owned.Count==0)return;
  uint scratch=U(map.Events.First(e=>owned.Contains(e.ID)).Instructions.Single(i=>Op(i,2003,32)));
  map.Events.RemoveAll(e=>owned.Contains(e.ID));
  foreach(var e in map.Events){
   Check(e.Parameters.Count==0||!e.Instructions.Any(i=>Op(i,2004,8)&&U(i,4)==CageCoop.Ready),"Parameterized cage birth");
   e.Instructions.RemoveAll(i=>Op(i,2000,0)&&owned.Contains(U(i,4))||Op(i,2003,66)&&U(i,4) is var f&&(f==scratch+32||f==scratch+33)||Op(i,3,0)&&U(i,4)==scratch+33);
   foreach(var i in e.Instructions.Where(i=>Op(i,2000,6)&&U(i,4)==Cages.OpenEvent)){
    Check(i.ArgData.Length==40+Bits*8,"Unexpected escalation opener arguments");i.ArgData=i.ArgData.Take(40).ToArray();
   }
  }
 }
 public static EMEVD.Event Open(EMEVD.Event old){
  if(old.Parameters.Any(p=>p.SourceStartByte>=32))return old;
  var next=new EMEVD.Event(old.ID,old.RestBehavior){Name=old.Name};
  int removed=0,increments=0;
  void Add(EMEVD.Instruction i)=>next.Instructions.Add(i);
  void Param(int target,int source)=>next.Parameters.Add(new EMEVD.Parameter{InstructionIndex=next.Instructions.Count-1,TargetStartByte=target,SourceStartByte=source,ByteCount=4,UnkID=source/4+1});
  for(int n=0;n<old.Instructions.Count;n++){
   var ins=old.Instructions[n];
   if(Op(ins,3,12)&&old.Parameters.Any(p=>p.InstructionIndex==n&&p.SourceStartByte==28)){
    Check(ins.ArgData[8]==3&&U(ins,12)==5&&Op(old.Instructions[n+1],1000,101),"Old cage limit changed");removed++;n++;continue;
   }
   if(Op(ins,2003,127)){
    Check(old.Parameters.Any(p=>p.InstructionIndex==n&&p.SourceStartByte==28),"Unexpected increment");
    // Write every shared bit exactly once. Native packets reorder ON before
    // OFF, so clear-then-set would corrupt counts at the receiving peer.
    for(int bit=0;bit<Bits;bit++){
     Add(I(1003,1,(byte)2,(byte)1,(byte)0,0u));Param(4,32+bit*8);
     Add(I(2003,69,(byte)0,0u,(byte)0));Param(4,36+bit*8);
     Add(I(1000,3,(byte)1));
     Add(I(2003,69,(byte)0,0u,(byte)1));Param(4,36+bit*8);
    }
    Add(I(2003,127,0u,(uint)Bits,uint.MaxValue));Param(0,28);increments++;continue;
   }
   int at=next.Instructions.Count;Add(new(ins.Bank,ins.ID,ins.ArgData.ToArray()){Layer=ins.Layer});
   foreach(var p in old.Parameters.Where(p=>p.InstructionIndex==n))next.Parameters.Add(new EMEVD.Parameter{InstructionIndex=at,TargetStartByte=p.TargetStartByte,SourceStartByte=p.SourceStartByte,ByteCount=p.ByteCount,UnkID=p.UnkID});
  }
  Check(removed==1&&increments==1,"Expected one personal limit and increment");return next;
 }
 static object Map(EMEVD map,uint prefix){
  StripMap(map,prefix);
  uint counter=prefix+5700;
  var occupied=new HashSet<uint>();
  foreach(var e in map.Events){occupied.Add((uint)e.ID);foreach(var i in e.Instructions){
   for(int j=0;j+4<=i.ArgData.Length;j+=4)occupied.Add(U(i,j));
   if(Op(i,2003,22)||Op(i,2003,63)||Op(i,2003,17)){uint a=U(i),b=U(i,4);if(b>=a&&b-a<1000)for(uint f=a;f<=b;f++)occupied.Add(f);}
  }}
  for(uint f=counter+3;f<counter+Bits;f++)Check(!occupied.Contains(f),"Expanded personal counter collision");
  for(uint f=counter;f<counter+Bits;f++)occupied.Add(f);
  // Reserve the low category-5 range used by native per-map runtime state,
  // including fields without literal references in this EMEVD.
  uint scratch=Enumerable.Range(5800,200-Bits-1).Select(n=>prefix+(uint)n).First(n=>Enumerable.Range(0,Bits+2).All(b=>!occupied.Contains(n+(uint)b)));
  uint initialized=scratch+32,done=scratch+33;
  uint shared=Enumerable.Range(7000,1000-Bits).Select(n=>prefix+(uint)n).First(n=>Enumerable.Range(0,Bits).All(b=>!occupied.Contains(n+(uint)b)));
  var ctor=map.Events.Single(e=>e.ID==0);
  var openers=ctor.Instructions.Where(i=>Op(i,2000,6)&&U(i,4)==Cages.OpenEvent).ToArray();Check(openers.Length>0,"No custom gaol openers");
  foreach(var i in openers){Check(i.ArgData.Length==40&&U(i,36)==counter,"Personal counter binding changed");var data=new byte[40+Bits*8];i.ArgData.CopyTo(data,0);for(int bit=0;bit<Bits;bit++){BitConverter.GetBytes(counter+(uint)bit).CopyTo(data,40+bit*8);BitConverter.GetBytes(shared+(uint)bit).CopyTo(data,44+bit*8);}i.ArgData=data;}
  var births=map.Events.Where(e=>e.Instructions.Any(i=>Op(i,2004,8)&&U(i,4)==CageCoop.Ready)).ToArray();
  var actors=new Dictionary<uint,(uint choice,uint shown)>();
  foreach(var birth in births){
   Check(birth.Parameters.Count==0,"Parameterized birth");int ready=birth.Instructions.FindIndex(i=>Op(i,2004,8)&&U(i,4)==CageCoop.Ready);uint actor=U(birth.Instructions[ready]);
   uint choice=birth.Instructions.Where(i=>Op(i,3,0)&&i.ArgData[0]==1&&i.ArgData[1]==1).Select(i=>U(i,4)).Where(f=>f!=prefix+7403).Distinct().Single();
   var show=birth.Instructions.Skip(ready+1).First(i=>Op(i,2003,66)&&i.ArgData[8]==1);uint shown=U(show,4);
   actors.Add(actor,(choice,shown));birth.Instructions.InsertRange(ready+1,[Set(initialized,false),Set(done,false)]);
  }
  var battles=map.Events.Where(e=>e.Instructions.Any(i=>Op(i,2003,69)&&U(i,4)==prefix+7404&&i.ArgData[8]==1)).ToArray();
  Check(battles.Length is 50 or 109&&births.Length is 84 or 179,"Unexpected encounter pool");
  // Map-prefixed event identities follow native instance relocation. A shared
  // 9xxxxxx ID does not identify which loaded gaol owns the local worker.
  uint eventBase=Enumerable.Range(8500,500-battles.Length+1).Select(n=>prefix+(uint)n).First(n=>Enumerable.Range(0,battles.Length).All(b=>!occupied.Contains(n+(uint)b)));
  var report=new List<object>();
  foreach(var battle in battles){
   Check(battle.Parameters.Count==0,"Parameterized battle");
   var members=battle.Instructions.Where(i=>Op(i,2004,1)&&i.ArgData[4]==1&&actors.ContainsKey(U(i))).Select(i=>U(i)).Distinct().ToArray();Check(members.Length>0,"No battle actors");
   uint choice=members.Select(a=>actors[a].choice).Distinct().Single();
   // Include mounts/helper actors sharing this encounter as well.
   members=actors.Where(a=>a.Value.choice==choice).Select(a=>a.Key).Order().ToArray();
   int id=(int)eventBase+report.Count;Check(map.Events.All(e=>e.ID!=id),"Escalation event ID collision");
   var e=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
   e.Instructions.AddRange([I(2000,2,(byte)0),Flag(1,prefix+7403,true),Flag(1,choice,true)]);
   foreach(uint a in members)e.Instructions.Add(Flag(1,actors[a].shown,true));
   e.Instructions.AddRange([I(0,0,(sbyte)0,(byte)1,(sbyte)1),I(1003,101,(byte)0,(byte)1,(byte)0,initialized)]);
   // Native removal by ID can remove just the first independent node. Drain
   // all stale instances before rebuilding, including after a local reload.
   foreach(uint a in members)e.Instructions.Add(I(4,5,(sbyte)-2,a,Effect,(byte)1,(sbyte)0,1f));
   e.Instructions.Add(I(1000,101,(byte)2,(byte)0,(sbyte)-2));
   foreach(uint a in members)e.Instructions.Add(I(2004,21,a,Effect));
   e.Instructions.AddRange([I(1001,1,1),I(1000,4,(byte)1),I(1014,2),I(2003,32,scratch,(uint)Bits),Set(initialized,true),I(1014,0),I(3,20,(sbyte)-1,shared,(byte)Bits,(byte)5,scratch,(byte)Bits),I(1000,101,(byte)1,(byte)1,(sbyte)-1)]);
   foreach(uint a in members)e.Instructions.Add(I(2004,8,a,Effect));
   e.Instructions.AddRange([I(2003,127,scratch,(uint)Bits,uint.MaxValue),I(1000,4,(byte)1),I(1014,1),Set(done,true),Flag(0,choice,false),Set(initialized,false),Set(done,false),I(1000,4,(byte)1)]);
   int ai=battle.Instructions.FindIndex(i=>Op(i,2004,1)&&i.ArgData[4]==1);Check(ai>=0,"Missing AI enable");battle.Instructions.Insert(ai,Flag(0,done,true));
   map.Events.Add(e);ctor.Instructions.Add(I(2000,0,0,id,0));
   report.Add(new{id,battle=battle.ID,choice,members,shown=members.Select(a=>actors[a].shown).ToArray()});
  }
  return new{prefix,counter,shared,scratch,initialized,done,births=births.Length,encounters=report};
 }
 public static void Stage(string root,string input,string output,Func<string,Memory<byte>,PARAM> parse){
  Check(Path.GetFullPath(input)!=Path.GetFullPath(output),"Use a separate staging directory");Directory.CreateDirectory(Path.Combine(output,"event"));
  var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(input,"regulation.bin"));var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var file=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var rows=parse("SpEffectParam",file.Bytes);var old=parse("SpEffectParam",file.Bytes);
  var fx=new PARAM.Row(rows.Rows.Single(r=>r.ID==Cages.Mutation)){ID=Effect};
  foreach(var (f,v) in new (string,object)[]{("spCategory",10),("stateInfo",0),("saveCategory",-1),("effectEndurance",-1f),("isDisableNetSync",1),("iconId",Icon),("displayIconForPermamentEffect",1),("dispDartDebuffStacks",0),("spEffectTextId_1",-1),("maxHpRate",1f)})Field(fx,f,v);
  foreach(string f in Erosion.AttackRates)Field(fx,f,1.1f);
  Field(fx,"secondaryIconId",DefenseIcon);
  // wepParamChange=0 is unrestricted, not disabled. Likewise the throw flag
  // is an ONLY-throw filter. Only the spell applicability bits need enabling.
  foreach(string f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Field(fx,f,1);
  foreach(string f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Field(fx,f,.9f);
  var existing=old.Rows.SingleOrDefault(r=>r.ID==Effect);
  Check(existing==null||existing.Cells.Zip(fx.Cells).All(p=>Equals(p.First.Value,p.Second.Value)||p.First.Def.InternalName=="secondaryIconId"&&Convert.ToInt32(p.First.Value)==-1),"Existing escalation effect differs; review before replacing it");
  rows.Rows.RemoveAll(r=>r.ID==Effect);rows.Rows.Add(fx);rows.Rows=rows.Rows.OrderBy(r=>r.ID).ToList();file.Bytes=rows.Write();
  string regulation=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(regulation,binder,binder.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(regulation);foreach(var f in actual.Files){Check(f.Bytes.Span.SequenceEqual(binder.Files.Single(b=>b.ID==f.ID).Bytes.Span),"Regulation readback");if(f.ID!=file.ID)Check(f.Bytes.Span.SequenceEqual(original[f.ID]),"Unrelated parameter table changed");}
  var read=parse("SpEffectParam",actual.Files.Single(f=>f.ID==file.ID).Bytes);foreach(var r in old.Rows.Where(r=>r.ID!=Effect))Check(r.Cells.Zip(read.Rows.Single(a=>a.ID==r.ID).Cells).All(p=>Equals(p.First.Value,p.Second.Value)),"Unrelated effect changed");
  File.WriteAllText(Path.Combine(output,"effect.json"),JsonSerializer.Serialize(fx.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value),new JsonSerializerOptions{WriteIndented=true}));
  var fixture=parse("SpEffectParam",file.Bytes);fixture.Rows=[new PARAM.Row(fx)];File.WriteAllBytes(Path.Combine(output,"effect.param"),fixture.Write());
  var reports=new List<object>();
  foreach(string name in new[]{"common_func","m46_50_00_00","m46_60_00_00","m46_70_00_00","m46_80_00_00"}){
   var before=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));var events=EMEVD.Read(Path.Combine(input,"event",name+".emevd.dcx"));object? layout=null;
   if(name=="common_func"){var opener=events.Events.Single(e=>e.ID==Cages.OpenEvent);events.Events[events.Events.IndexOf(opener)]=Open(opener);}
   else layout=Map(events,(uint)(4600+int.Parse(name.Substring(4,2)))*10000);
   string target=Path.Combine(output,"event",name+".emevd.dcx");events.Write(target);var after=EMEVD.Read(target);
   Check(events.Events.Count==after.Events.Count&&events.Events.Zip(after.Events).All(p=>Cages.Same(p.First,p.Second)),"Event readback mismatch");
   Check(before.Format==after.Format&&before.Compression==after.Compression&&before.StringData.SequenceEqual(after.StringData)&&before.LinkedFileOffsets.SequenceEqual(after.LinkedFileOffsets),"Event metadata changed");
   var changed=after.Events.Where(e=>!before.Events.Any(b=>Cages.Same(e,b))).Select(e=>e.ID).ToArray();
   if(name=="common_func")Check(before.Events.Where(e=>e.ID!=Cages.OpenEvent).All(e=>Cages.Same(e,after.Events.Single(a=>a.ID==e.ID))),"Unrelated common event changed");
   else{
    uint prefix=(uint)(4600+int.Parse(name.Substring(4,2)))*10000;
    var undo=EMEVD.Read(target);StripMap(undo,prefix);StripMap(before,prefix);
    Check(undo.Events.Count==before.Events.Count&&undo.Events.Zip(before.Events).All(pair=>Cages.Same(pair.First,pair.Second)),"Removing escalation must restore every previous event exactly");
   }
   reports.Add(new{name,layout,inputSha256=Hash(Path.Combine(input,"event",name+".emevd.dcx")),sha256=Hash(target),changed,events=after.Events.Where(e=>e.ID==0||e.ID==Cages.OpenEvent||layout!=null&&(Owned(e,(uint)(4600+int.Parse(name.Substring(4,2)))*10000)||e.Instructions.Any(i=>Op(i,2004,8)&&U(i,4)==CageCoop.Ready)||e.Instructions.Any(i=>Op(i,2003,69)&&U(i,4)==(uint)(4600+int.Parse(name.Substring(4,2)))*10000+7404&&i.ArgData[8]==1))).Select(Cages.Dump)});
  }
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(output,"texts"),allRewards:true,onlyIds:new[]{Cages.Owner},inputMod:input);
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{effect=Effect,icon=Icon,bits=Bits,attackPerStack=1.1,damageTakenPerStack=.9,firstOpeningStacks=0,reports,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: unlimited cage openings, personal gaol counts, native multiplicative effects and HUD stack aggregation staged");
 }
}
