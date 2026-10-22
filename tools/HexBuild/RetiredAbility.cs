using SoulsFormats;
using System.Text.Json;

static class RetiredAbility {
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static string Name(BinderFile file)=>file.Name.Replace('\\','/').Split('/').Last();
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,int owner,int[] privateGoods,Dictionary<string,int[]>? extraRows=null){
  var hexes=HexCatalog.Load(root);Check(!hexes.Any(hex=>hex.Id==owner),"Remove the ability from the active catalog first");
  Check(privateGoods.Distinct().Count()==privateGoods.Length&&privateGoods.All(id=>id>owner&&id<owner+100),"Private goods must belong to this ability");
  string output=Path.Combine(root,"work/retired-ability");Directory.CreateDirectory(output);
  var wanted=new Dictionary<string,int[]>{{"SpEffectParam",new[]{owner}},{"AttachEffectParam",new[]{owner}},{"PermanentBuffParam",new[]{owner}},{"EquipParamGoods",new[]{owner}.Concat(privateGoods).ToArray()},{"ItemTableParam",new[]{RewardCatalog.Table(owner)}}};
  foreach(var (table,ids) in extraRows??new Dictionary<string,int[]>()){
   Check(ids.Length>0&&ids.Distinct().Count()==ids.Length,"Invalid private row list");
   wanted[table]=wanted.TryGetValue(table,out var previous)?previous.Concat(ids).Distinct().ToArray():ids;
  }
  var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  foreach(var (table,ids) in wanted){
   var file=binder.Files.Single(file=>Name(file)==table+".param");var rows=parse(table,file.Bytes);
   Check(ids.All(id=>rows.Rows.Count(row=>row.ID==id)==1),"Missing/duplicate retired row: "+table);
   rows.Rows.RemoveAll(row=>ids.Contains(row.ID));file.Bytes=rows.Write();
  }
  string regulation=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(regulation,binder,binder.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(regulation);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Regulation metadata changed");
  foreach(var file in actual.Files){
   var expected=binder.Files.Single(item=>item.ID==file.ID);Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Regulation readback mismatch");
   string table=Name(file).Replace(".param","");
   if(!wanted.TryGetValue(table,out var ids)){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}
   var previous=parse(table,before[file.ID]).Rows.Where(row=>!ids.Contains(row.ID)).OrderBy(row=>row.ID).ToArray();var remaining=parse(table,file.Bytes).Rows.OrderBy(row=>row.ID).ToArray();
   Check(previous.Length==remaining.Length&&previous.Zip(remaining).All(pair=>CageAI.Same(pair.First,pair.Second)),"Unrelated parameter row changed");
  }
  var eventSource=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var events=EMEVD.Read(eventSource);var originalEvents=EMEVD.Read(eventSource);int removed=0;
  var removedEffects=wanted["SpEffectParam"].ToHashSet();
  foreach(var entry in events.Events.Where(entry=>entry.ID is 1111 or 9100941)){
   Check(entry.Parameters.Count==0,"Parameterized cleanup requires relocation");
   bool Removal(EMEVD.Instruction instruction,int effect)=>instruction.Bank==2004&&instruction.ID==21&&instruction.ArgData.Length==8&&BitConverter.ToUInt32(instruction.ArgData)==20000&&BitConverter.ToInt32(instruction.ArgData,4)==effect;
   int ownerRemovals=entry.Instructions.Count(instruction=>Removal(instruction,owner));
   Check(extraRows==null?ownerRemovals==1:ownerRemovals<=1,$"Unexpected retired-owner cleanup count: event {entry.ID}, {ownerRemovals}");
   Check(removedEffects.All(effect=>entry.Instructions.Count(instruction=>Removal(instruction,effect))<=1),"Duplicated private cleanup");
   removed+=entry.Instructions.RemoveAll(instruction=>removedEffects.Any(effect=>Removal(instruction,effect)));
  }
  Check(removed>0,"Missing cleanup event");string eventOutput=Path.Combine(output,"common.emevd.dcx");events.Write(eventOutput);var actualEvents=EMEVD.Read(eventOutput);
  Check(actualEvents.Format==originalEvents.Format&&actualEvents.Compression==originalEvents.Compression&&actualEvents.Events.Count==originalEvents.Events.Count&&actualEvents.StringData.SequenceEqual(originalEvents.StringData)&&actualEvents.LinkedFileOffsets.SequenceEqual(originalEvents.LinkedFileOffsets),"Event metadata changed");
  foreach(var entry in actualEvents.Events){Check(Cages.Same(events.Events.Single(previous=>previous.ID==entry.ID),entry),"Event readback mismatch");if(entry.ID is not (1111 or 9100941))Check(Cages.Same(originalEvents.Events.Single(previous=>previous.ID==entry.ID),entry),"Unrelated event changed");}
  var texts=new List<object>();
  foreach(string language in new[]{"zhocn","engus"})foreach(string category in new[]{"item","menu"}){
   string filename=category+"_dlc01.msgbnd.dcx";var messages=BND4.Read(Path.Combine(root,"dist/mod/msg",language,filename));var original=messages.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());int count=0;
   foreach(var file in messages.Files){var text=FMG.Read(file.Bytes);int changed=text.Entries.RemoveAll(entry=>entry.ID==owner);if(changed>0){file.Bytes=text.Write();count+=changed;}}
   Check(count>0,"Retired text missing");string destination=Path.Combine(output,"msg",language,filename);Directory.CreateDirectory(Path.GetDirectoryName(destination)!);messages.Write(destination);var reopened=BND4.Read(destination);
   Check(reopened.Files.Count==messages.Files.Count&&reopened.Compression==messages.Compression&&reopened.Format==messages.Format&&reopened.Version==messages.Version,"Text metadata changed");
   foreach(var file in reopened.Files){var expected=messages.Files.Single(entry=>entry.ID==file.ID);Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Text readback mismatch");var previous=FMG.Read(original[file.ID]);var current=FMG.Read(file.Bytes);Check(current.Version==previous.Version&&current.BigEndian==previous.BigEndian&&current.Entries.Count==previous.Entries.Count(entry=>entry.ID!=owner)&&current.Entries.All(entry=>entry.ID!=owner&&previous.Entries.Single(old=>old.ID==entry.ID).Text==entry.Text),"Unrelated text changed");}
   texts.Add(new{language,category,count});
  }
  RewardCatalog.Write(Path.Combine(output,"hex_pool.rs"),hexes);
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{owner,privateGoods,removedRows=wanted,removedCleanupInstructions=removed,texts,activeAbilities=hexes.Length,otherRowsEventsAndTextsUnchanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: retired ability {owner}; {hexes.Length} active abilities; unrelated resources preserved");
 }
}
