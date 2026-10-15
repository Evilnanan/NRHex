using SoulsFormats;
using System.Text.Json;

static partial class NightBlessing{
 public static void ResetReceipt(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);
   Check(!e.Instructions.Any(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==Owner+5),"Night blessing reset already installed");
   var anchors=e.Instructions.Select((i,n)=>(i,n)).Where(v=>v.i.Bank==2004&&v.i.ID==21&&BitConverter.ToInt32(v.i.ArgData,4)==ContractHeart.Second).ToArray();
   Check(anchors.Length==1,"Guarded new-expedition/hub receipt reset anchor missing");int at=anchors[0].n+1;
   e.Instructions.Insert(at,new EMEVD.Instruction(2004,21,new object[]{20000u,Owner+5}));foreach(var p in e.Parameters)if(p.InstructionIndex>=at)p.InstructionIndex++;
  }
 }
 public static void Rework(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  ValidateNative(original,parse);
  string dir=Path.Combine(root,".work/night-rework/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");var old=parse("SpEffectParam",member.Bytes);var sp=parse("SpEffectParam",member.Bytes);
  PARAM.Row Replace(string name,int source,int id){Check(name=="SpEffectParam","Unexpected table");var r=new PARAM.Row(sp.Rows.Single(r=>r.ID==source)){ID=id};var existing=sp.Rows.SingleOrDefault(r=>r.ID==id);if(id<=Owner+7)Check(existing!=null,"Existing support effect missing");else Check(existing==null,"New support effect collision");if(existing==null)sp.Rows.Add(r);else sp.Rows[sp.Rows.IndexOf(existing)]=r;return r;}
  Mechanics(Replace);sp.Rows=sp.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=sp.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var read=SFUtil.DecryptNightreignRegulation(output);
  Check(read.Version==binder.Version&&read.Compression==binder.Compression&&read.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var f in read.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter readback mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter table changed");}
  var actual=parse("SpEffectParam",read.Files.Single(f=>f.ID==member.ID).Bytes);Check(actual.Rows.Count==old.Rows.Count+4,"Unexpected row count");foreach(var r in old.Rows.Where(r=>r.ID<=Owner||r.ID>Owner+7))Check(CageAI.Same(r,actual.Rows.Single(v=>v.ID==r.ID)),"Unrelated effect changed");
  foreach(var r in sp.Rows.Where(r=>r.ID>Owner&&r.ID<=Owner+11))Check(CageAI.Same(r,actual.Rows.Single(v=>v.ID==r.ID)),"Support effect readback mismatch");
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx"),eventOutput=Path.Combine(dir,"common.emevd.dcx");var events=EMEVD.Read(eventInput);var oldEvents=EMEVD.Read(eventInput);ResetReceipt(events);events.Write(eventOutput);var current=EMEVD.Read(eventOutput);
  Check(current.Events.Select(e=>e.ID).SequenceEqual(oldEvents.Events.Select(e=>e.ID))&&current.Format==oldEvents.Format&&current.Compression==oldEvents.Compression&&current.StringData.SequenceEqual(oldEvents.StringData)&&current.LinkedFileOffsets.SequenceEqual(oldEvents.LinkedFileOffsets),"Event metadata changed");Check(events.Events.All(e=>Cages.Same(e,current.Events.Single(v=>v.ID==e.ID))),"Event readback mismatch");
  foreach(var e in oldEvents.Events){var n=current.Events.Single(v=>v.ID==e.ID);if(e.ID is 1111 or 9100941){int at=n.Instructions.FindIndex(i=>i.Bank==2004&&i.ID==21&&BitConverter.ToInt32(i.ArgData,4)==Owner+5);Check(at>=0,"Receipt cleanup missing");n.Instructions.RemoveAt(at);foreach(var p in n.Parameters)if(p.InstructionIndex>at)p.InstructionIndex--; }Check(Cages.Same(e,n),"Original event changed");}
  Assets(root);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:[Owner]);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),changedEffects=Enumerable.Range(Owner+1,11),effects=actual.Rows.Where(r=>r.ID>=Owner&&r.ID<=Owner+11).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),onlySupportEffectsAndReceiptResetsChanged=true,originalRowsAndOtherTablesPreserved=true,originalEventsAndMetadataPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Night blessing rework staged; native rates, effect chains, saved receipt resets and bilingual text verified");
 }
}
