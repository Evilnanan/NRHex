using SoulsFormats;
using System.Text.Json;

static class RewardCycle{
 public const int First=95002800,Count=28,Reset=95002825;
 public static int Marker(int index)=>index<25?First+index:95002829+(index-25);
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v){r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Parameters(Func<string,int,int,PARAM.Row> clone,int[]? historyIds=null){
  foreach(int id in (historyIds??RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length)).Append(Reset)){
   var r=clone("SpEffectParam",8000000,id);
   foreach(var (f,v) in new (string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);
  }
 }
 public static void Events(EMEVD ev,int[]? historyIds=null){
  // Runtime consumes this notice and clears only the active packed records.
  var records=Array.Empty<int>();
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   Check(!e.Instructions.Any(i=>i.Bank==2004&&i.ID is 8 or 21&&BitConverter.ToInt32(i.ArgData,4)>=First&&BitConverter.ToInt32(i.ArgData,4)<=Reset),"Cycle reset already present");
   var anchors=e.Instructions.Select((i,n)=>(i,n)).Where(v=>v.i.Bank==2004&&v.i.ID==21&&BitConverter.ToInt32(v.i.ArgData,4)==95002505).ToArray();Check(anchors.Length==1,"Guarded expedition/hub reset anchor missing");int at=anchors[0].n+1;
   var added=records.Append(Reset).Select(v=>new EMEVD.Instruction(2004,21,new object[]{20000u,v})).Append(new EMEVD.Instruction(2004,8,new object[]{20000u,Reset}));
   e.Instructions.InsertRange(at,added);foreach(var p in e.Parameters)if(p.InstructionIndex>=at)p.InstructionIndex+=records.Length+2;
  }
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/reward-cycle/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var sp=parse("SpEffectParam",member.Bytes);var old=parse("SpEffectParam",member.Bytes);
  PARAM.Row Add(string table,int source,int id){Check(table=="SpEffectParam"&&!sp.Rows.Any(r=>r.ID==id),"Cycle effect collision");var r=new PARAM.Row(sp.Rows.Single(r=>r.ID==source)){ID=id};sp.Rows.Add(r);return r;}
  Parameters(Add);sp.Rows=sp.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=sp.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Files.Count==binder.Files.Count&&actual.Compression==binder.Compression,"Regulation metadata changed");
  foreach(var f in actual.Files){var wanted=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var final=parse("SpEffectParam",actual.Files.Single(f=>f.ID==member.ID).Bytes);Check(final.Rows.Count==old.Rows.Count+Count+1,"Unexpected effect count");foreach(var r in old.Rows)Check(CageAI.Same(r,final.Rows.Single(v=>v.ID==r.ID)),"Existing effect changed");
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);Events(ev);ev.Write(eventOutput);var current=EMEVD.Read(eventOutput);
  Check(current.Events.Select(e=>e.ID).SequenceEqual(oldEv.Events.Select(e=>e.ID))&&current.Format==oldEv.Format&&current.Compression==oldEv.Compression&&current.StringData.SequenceEqual(oldEv.StringData)&&current.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");Check(ev.Events.All(e=>Cages.Same(e,current.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
  var changed=current.Events.Where(e=>e.ID is 1111 or 9100941).Select(e=>JsonSerializer.SerializeToElement(Cages.Dump(e))).ToArray();
  foreach(var e in oldEv.Events){var n=current.Events.Single(v=>v.ID==e.ID);
   if(e.ID is 1111 or 9100941){int at=n.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==First);Check(at>=0,"Cycle cleanup missing");n.Instructions.RemoveRange(at,Count+2);foreach(var p in n.Parameters)if(p.InstructionIndex>=at+Count+2)p.InstructionIndex-=Count+2;}
   Check(Cages.Same(e,n),"Unrelated event instructions/parameters changed");
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),effects=final.Rows.Where(r=>Enumerable.Range(0,Count).Select(Marker).Append(Reset).Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),events=changed,previousEvents=oldEv.Events.Where(e=>e.ID is 1111 or 9100941).Select(Cages.Dump),onlyNeutralSavedEffectsAndTwoResetsAdded=true,originalRowsTablesEventsAndMetadataPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {Count} local saved exposure records and consumed run-reset notice staged; original tables/events preserved");
 }
}
