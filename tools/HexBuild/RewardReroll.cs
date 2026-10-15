using SoulsFormats;
using System.Text.Json;

static class RewardReroll{
 public const int First=95002826,Label=95002850;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 public static void Parameters(Func<string,int,int,PARAM.Row> clone){
  for(int i=0;i<3;i++){
   var r=clone("SpEffectParam",8000000,First+i);
   foreach(var (f,v) in new (string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);
  }
 }
 public static void Events(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   Check(!e.Instructions.Any(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)>=First&&BitConverter.ToInt32(i.ArgData,4)<First+3),"Reroll reset already present");
   var anchors=e.Instructions.Select((i,n)=>(i,n)).Where(v=>v.i.Bank==2004&&v.i.ID==8&&BitConverter.ToInt32(v.i.ArgData,4)==RewardCycle.Reset).ToArray();Check(anchors.Length==1,"Guarded exposure reset notice missing");int at=anchors[0].n;
   e.Instructions.InsertRange(at,Enumerable.Range(First,3).Select(v=>new EMEVD.Instruction(2004,21,new object[]{20000u,v})));
   foreach(var p in e.Parameters)if(p.InstructionIndex>=at)p.InstructionIndex+=3;
  }
 }
 public static void Text(string root,BND4 binder,string language){
  var catalog=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"data/text/menu.json"))).RootElement.GetProperty("reroll");
  Check(catalog.GetProperty("id").GetInt32()==Label,"Runtime/menu label mismatch");
  var value=catalog.GetProperty(language=="zhocn"?"zh":"en").GetString();Check(!string.IsNullOrWhiteSpace(value),"Missing menu translation");
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="CL_KeyGuide.fmg");var fmg=FMG.Read(member.Bytes);
  Check(fmg.Entries.Single(e=>e.ID==120600).Text==(language=="zhocn"?"放置":"Leave"),"Original Leave hint mismatch");
  Check(!fmg.Entries.Any(e=>e.ID==Label),"Reroll label collision");
  fmg.Entries.Add(new FMG.Entry(fmg,Label,value));fmg.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));member.Bytes=fmg.Write();
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/hex-reroll/staged");Directory.CreateDirectory(dir);var reports=new List<object>();
  string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(dir,"regulation.bin");var b=SFUtil.DecryptNightreignRegulation(input);var original=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var member=b.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var sp=parse("SpEffectParam",member.Bytes);var old=parse("SpEffectParam",member.Bytes);
  PARAM.Row Add(string table,int source,int id){Check(table=="SpEffectParam"&&!sp.Rows.Any(r=>r.ID==id),"Reroll effect collision");var r=new PARAM.Row(sp.Rows.Single(r=>r.ID==source)){ID=id};sp.Rows.Add(r);return r;}
  Parameters(Add);sp.Rows=sp.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=sp.Write();SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Files.Count==b.Files.Count&&actual.Compression==b.Compression,"Regulation metadata changed");
  foreach(var f in actual.Files){var wanted=b.Files.Single(v=>v.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(original[f.ID]),"Unrelated parameter table changed");}
  var final=parse("SpEffectParam",actual.Files.Single(f=>f.ID==member.ID).Bytes);Check(final.Rows.Count==old.Rows.Count+3,"Unexpected marker count");foreach(var r in old.Rows)Check(CageAI.Same(r,final.Rows.Single(v=>v.ID==r.ID)),"Existing effect changed");
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var ev=EMEVD.Read(eventInput);var oldEv=EMEVD.Read(eventInput);Events(ev);ev.Write(eventOutput);var current=EMEVD.Read(eventOutput);
  Check(current.Events.Select(e=>e.ID).SequenceEqual(oldEv.Events.Select(e=>e.ID))&&current.Format==oldEv.Format&&current.Compression==oldEv.Compression&&current.StringData.SequenceEqual(oldEv.StringData)&&current.LinkedFileOffsets.SequenceEqual(oldEv.LinkedFileOffsets),"Event metadata changed");Check(ev.Events.All(e=>Cages.Same(e,current.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
  var changed=current.Events.Where(e=>e.ID is 1111 or 9100941).Select(e=>JsonSerializer.SerializeToElement(Cages.Dump(e))).ToArray();
  foreach(var e in oldEv.Events){var n=current.Events.Single(v=>v.ID==e.ID);if(e.ID is 1111 or 9100941){int at=n.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==First);Check(at>=0,"Reroll cleanup missing");n.Instructions.RemoveRange(at,3);foreach(var p in n.Parameters)if(p.InstructionIndex>=at+3)p.InstructionIndex-=3;}Check(Cages.Same(e,n),"Unrelated event instructions/parameters changed");}
  foreach(string lang in new[]{"zhocn","engus"}){
   string source=Path.Combine(root,"dist/mod/msg",lang,"menu_dlc01.msgbnd.dcx"),target=Path.Combine(dir,lang,"menu_dlc01.msgbnd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(target)!);var text=BND4.Read(source);var before=BND4.Read(source);Text(root,text,lang);text.Write(target);var read=BND4.Read(target);
   Check(read.Format==before.Format&&read.Version==before.Version&&read.Compression==before.Compression&&read.Files.Count==before.Files.Count,"Menu metadata changed");
   foreach(var f in read.Files){var prev=before.Files.Single(v=>v.ID==f.ID);Check(f.Name==prev.Name&&f.Flags==prev.Flags&&f.Bytes.Span.SequenceEqual(text.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Menu readback mismatch");if(Path.GetFileName(f.Name)=="CL_KeyGuide.fmg"){
    var a=FMG.Read(f.Bytes);var oldFmg=FMG.Read(prev.Bytes);Check(a.Version==oldFmg.Version&&a.BigEndian==oldFmg.BigEndian&&a.Entries.Count==oldFmg.Entries.Count+1&&oldFmg.Entries.All(e=>a.Entries.Single(v=>v.ID==e.ID).Text==e.Text),"Original key hints changed");
   }else Check(f.Bytes.Span.SequenceEqual(prev.Bytes.Span),"Unrelated menu text changed");}
   reports.Add(new{language=lang,inputSha256=Hash(source),outputSha256=Hash(target),originalLeavePreserved=true,onlyOneKeyGuideEntryAdded=true});
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),effects=final.Rows.Where(r=>r.ID>=First&&r.ID<First+3).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),events=changed,previousEvents=oldEv.Events.Where(e=>e.ID is 1111 or 9100941).Select(Cages.Dump),texts=reports,onlyThreeLocalSavedMarkersAndGuardedResetsAdded=true,originalRowsTablesEventsAndTextPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: three saved local reroll uses, guarded resets and Chinese/English key hints staged; all existing content preserved");
 }
}
