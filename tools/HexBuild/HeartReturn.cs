using SoulsFormats;
using System.Text.Json;

static partial class ContractHeart {
 public static void StageTwoReturns(string root,Func<string,Memory<byte>,PARAM> parse){
  string dir=Path.Combine(root,".work/heart-two/staged"),input=Path.Combine(root,"dist/mod/regulation.bin"),eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var old=parse("SpEffectParam",member.Bytes);var sp=parse("SpEffectParam",member.Bytes);
  Check(!sp.Rows.Any(r=>r.ID==Second),"Second-return marker collision");
  var second=new PARAM.Row(sp.Rows.Single(r=>r.ID==Return)){ID=Second};Set(second,"isDisableNetSync",1);sp.Rows.Add(second);sp.Rows=sp.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=sp.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Files.Count==binder.Files.Count&&actual.Compression==binder.Compression,"Regulation metadata changed");
  foreach(var f in actual.Files){var wanted=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Parameter readback mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var final=parse("SpEffectParam",actual.Files.Single(f=>f.ID==member.ID).Bytes);Check(final.Rows.Count==old.Rows.Count+1,"Unexpected effect rows");
  foreach(var row in old.Rows)Check(CageAI.Same(row,final.Rows.Single(r=>r.ID==row.ID)),"Existing effect changed");Check(CageAI.Same(second,final.Rows.Single(r=>r.ID==Second)),"Second marker changed");

  var ev=EMEVD.Read(eventInput);var eventBefore=EMEVD.Read(eventInput);
  var oldUse=new EMEVD.Event(EventIds[1],EMEVD.Event.RestBehaviorType.Restart);
  oldUse.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Used,true),Wait(),I(2004,21,20000u,Used),I(1004,2,(byte)1,20000u,Heart,(byte)0,(sbyte)0,1),I(1004,2,(byte)1,20000u,Cooldown,(byte)1,(sbyte)0,1),I(1004,2,(byte)1,20000u,Return,(byte)1,(sbyte)0,1),I(2004,8,20000u,Cooldown),I(1000,4,(byte)1)]);
  var oldReturn=new EMEVD.Event(EventIds[2],EMEVD.Event.RestBehaviorType.Restart);
  oldReturn.Instructions.AddRange([I(2000,2,(byte)0),Has(1,Heart,true),Has(1,Return,true),Wait(),I(2004,21,20000u,Return),Give(ReturnLot),I(1000,4,(byte)1)]);
  foreach(var pair in new[]{(oldUse,UseEvent()),(oldReturn,ReturnEvent())}){
   var current=ev.Events.Single(e=>e.ID==pair.Item1.ID);Check(JsonSerializer.Serialize(current)==JsonSerializer.Serialize(pair.Item1),"Existing Heartstealer event differs from expected baseline");current.Instructions=pair.Item2.Instructions;
  }
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);var anchors=e.Instructions.Select((i,n)=>(i,n)).Where(v=>v.i.Bank==2004&&v.i.ID==21&&BitConverter.ToInt32(v.i.ArgData,4)==Return).ToArray();Check(anchors.Length==1,"Lifecycle anchor mismatch");int index=anchors[0].n+1;
   e.Instructions.Insert(index,I(2004,21,20000u,Second));foreach(var p in e.Parameters)if(p.InstructionIndex>=index)p.InstructionIndex++;
  }
  string eventOutput=Path.Combine(dir,"common.emevd.dcx");ev.Write(eventOutput);var read=EMEVD.Read(eventOutput);
  Check(read.Format==eventBefore.Format&&read.Compression==eventBefore.Compression&&read.StringData.SequenceEqual(eventBefore.StringData)&&read.LinkedFileOffsets.SequenceEqual(eventBefore.LinkedFileOffsets)&&read.Events.Count==eventBefore.Events.Count,"Event metadata changed");
  foreach(var e in ev.Events)Check(JsonSerializer.Serialize(e)==JsonSerializer.Serialize(read.Events.Single(v=>v.ID==e.ID)),"Event readback mismatch");
  foreach(var original in eventBefore.Events.Where(e=>e.ID!=EventIds[1]&&e.ID!=EventIds[2])){
   var current=read.Events.Single(e=>e.ID==original.ID);
   if(original.ID is 1111 or 9100941){int index=current.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==Second);Check(index>=0,"Second marker reset missing");current.Instructions.RemoveAt(index);foreach(var p in current.Parameters)if(p.InstructionIndex>index)p.InstructionIndex--;}
   Check(JsonSerializer.Serialize(original)==JsonSerializer.Serialize(current),"Unrelated event changed");
  }
  ExportChecks(root,parse,output,eventOutput);
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(dir,"texts"),allRewards:true,onlyIds:[Heart]);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),addedEffect=Second,changedEvents=new[]{9100971,9100972,1111,9100941},existingEffectsAndOtherTablesPreserved=true,unrelatedEventsAndMetadataPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: two-return Heartstealer staged; only hidden second marker and four event bodies changed; text readback passed");
 }
}
