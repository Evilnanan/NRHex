using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// The source MMV directory is read-only. Only the selected staged overlay is written.
static class MmvSpirit {
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p))).ToLowerInvariant();
 public static void Export(string root,string overlay,string mmv,Func<string,Memory<byte>,PARAM> parse){
  overlay=Path.GetFullPath(overlay);mmv=Path.GetFullPath(mmv);
  var regulation=Path.Combine(overlay,"mod/regulation.bin");var hash=Hash(regulation);
  var binder=SFUtil.DecryptNightreignRegulation(regulation);
  var npc=parse("NpcParam",binder.Files.Single(f=>Name(f)=="NpcParam.param").Bytes);
  var maps=new Dictionary<string,object>();var events=new Dictionary<string,object>();var inputs=new Dictionary<string,string>();
  // Native common functions remain available where MMV supplies only a map event.
  foreach(var directory in new[]{Path.Combine(root,"assets/original"),Path.Combine(mmv,"mod"),Path.Combine(overlay,"mod")}){
   var eventDir=Path.Combine(directory,"event");
   if(Directory.Exists(eventDir))foreach(var p in Directory.GetFiles(eventDir,"*.emevd.dcx").Order()){
    var name=Path.GetFileName(p).Replace(".emevd.dcx","");inputs[p]=Hash(p);
    events[name]=EMEVD.Read(p).Events.Select(e=>new{e.ID,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})}).ToArray();
   }
  }
  foreach(var directory in new[]{Path.Combine(mmv,"mod/map/mapstudio"),Path.Combine(overlay,"mod/map/mapstudio")})
  foreach(var p in Directory.Exists(directory)?Directory.GetFiles(directory,"*.msb.dcx").Order():Enumerable.Empty<string>()){
   inputs[p]=Hash(p);var name=Path.GetFileName(p).Replace(".msb.dcx","");
   maps[name]=MSB_NR.Read(p).Parts.Enemies.Select(e=>new{e.Name,e.ModelName,e.NpcParamId,e.NpcThinkParamId,entity=e.EntityData.EntityID,groups=e.EntityData.EntityGroupIDs}).ToArray();
  }
  if(hash!=Hash(regulation)||inputs.Any(p=>Hash(p.Key)!=p.Value))throw new InvalidDataException("Spirit inputs changed during export");
  var output=Path.Combine(root,"work/compat/mmv/spirit-parameters.json");
  File.WriteAllText(output,JsonSerializer.Serialize(new{overlay,mmv,regulationSha256=hash,inputs,maps,events,rows=npc.Rows.Select(r=>new{r.ID,r.Name,fields=r.Cells.Where(c=>c.Value is not byte[]).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})}));
  Console.WriteLine($"Exported {npc.Rows.Count} NPC rows, {maps.Count} MMV maps and {events.Count} event files");
 }
 public static void Apply(string overlay,string planPath,Func<string,Memory<byte>,PARAM> parse){
  using var plan=JsonDocument.Parse(File.ReadAllText(planPath));var p=plan.RootElement;
  var regulation=Path.Combine(Path.GetFullPath(overlay),"mod/regulation.bin");
  if(Hash(regulation)!=p.GetProperty("regulationSha256").GetString())throw new InvalidDataException("Stale spirit plan");
  var binder=SFUtil.DecryptNightreignRegulation(regulation);var file=binder.Files.Single(f=>Name(f)=="NpcParam.param");
  var table=parse("NpcParam",file.Bytes);var sources=table.Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>g.Single());
  var changed=new HashSet<int>();
  var zero=new[]{"getSoul","affectedByNecromancy","isSoulGetByBoss"};
  var one=new[]{"excludeGroupRewardCheck","isCalculatePvPDamage"};
  var absent=new[]{"itemLotId_enemy","itemLotId_map","rewardItemLot_1","rewardItemLot_2","sleepCollectorItemLotId_enemy","sleepCollectorItemLotId_map","chaosMatchingRewardLotId","chaosMatchingItemLotId"};
  foreach(var pair in p.GetProperty("pairs").EnumerateArray()){
   int source=pair[0].GetInt32(),id=pair[1].GetInt32();
   if(id<95400000||id>=95500000||source is >=95400000 and <95500000||!changed.Add(id))throw new InvalidDataException("Invalid spirit private ID");
   var row=new PARAM.Row(sources[source]){ID=id};
   foreach(var key in zero)row[key].Value=Convert.ChangeType(0,row[key].Value.GetType());
   foreach(var key in one)row[key].Value=Convert.ChangeType(1,row[key].Value.GetType());
   foreach(var key in absent)row[key].Value=Convert.ChangeType(-1,row[key].Value.GetType());
   int index=table.Rows.FindIndex(r=>r.ID==id);
   if(index>=0)table.Rows[index]=row;
   else {index=table.Rows.FindIndex(r=>r.ID>id);table.Rows.Insert(index<0?table.Rows.Count:index,row);}
  }
  file.Bytes=table.Write();var temporary=regulation+".spirit-stage";
  SFUtil.EncryptNightreignRegulation(temporary,binder,binder.Compression);
  var check=SFUtil.DecryptNightreignRegulation(temporary);
  if(binder.Files.Count!=check.Files.Count||!binder.Files.Zip(check.Files).All(x=>x.First.Name==x.Second.Name&&x.First.Bytes.Span.SequenceEqual(x.Second.Bytes.Span)))throw new InvalidDataException("Spirit binder readback mismatch");
  var readback=parse("NpcParam",check.Files.Single(f=>Name(f)=="NpcParam.param").Bytes);
  string Cells(PARAM.Row r)=>JsonSerializer.Serialize(r.Cells.Select(c=>c.Value));
  if(table.Rows.Count!=readback.Rows.Count||!table.Rows.Zip(readback.Rows).All(x=>x.First.ID==x.Second.ID&&Cells(x.First)==Cells(x.Second)))throw new InvalidDataException("Spirit row readback mismatch");
  foreach(var row in readback.Rows.Where(r=>!changed.Contains(r.ID)))if(Cells(row)!=Cells(sources[row.ID]))throw new InvalidDataException("Non-spirit NPC modified");
  File.Move(temporary,regulation,true);
  Console.WriteLine($"Rebased {changed.Count} private spirit rows; every other NPC and binder member preserved. SHA256 {Hash(regulation)}");
 }
}
