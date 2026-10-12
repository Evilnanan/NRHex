using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

// Read-only evidence export. No runtime or package assets are changed.
static class CageGroups {
 public static void ExportMetadata(string root){
  var paths=Directory.GetFiles(Path.Combine(root,"assets/original/map/mapstudio"),"m46_*.msb.dcx").Concat(new[]{"extracted","field-extracted"}.SelectMany(d=>Directory.GetFiles(Path.Combine(root,".work/cage-groups",d,"map/mapstudio"),"*.msb.dcx")));
  var rows=paths.Select(p=>{var m=MSB_NR.Read(p);return new{map=Path.GetFileName(p),events=m.Events.GetEntries().Select(e=>new{type=e.GetType().Name,json=JsonSerializer.Serialize(e,e.GetType(),new JsonSerializerOptions{IncludeFields=true})}),regions=m.Regions.GetEntries().Select(e=>new{type=e.GetType().Name,json=JsonSerializer.Serialize(e,e.GetType(),new JsonSerializerOptions{IncludeFields=true})})};});
  File.WriteAllText(Path.Combine(root,".work/cage-groups/metadata.json"),JsonSerializer.Serialize(rows));
 }
 public static void ExportNpcs(string root,PARAM npc,string regulationHash){
  string output=Path.Combine(root,".work/cage-groups");
  using var catalog=JsonDocument.Parse(File.ReadAllText(Path.Combine(output,"encounters.json")));
  var ids=catalog.RootElement.GetProperty("encounters").EnumerateArray().SelectMany(e=>e.GetProperty("members").EnumerateArray()).Select(e=>e.GetProperty("NpcParamId").GetInt32()).Distinct().Order().ToArray();
  var keys=new[]{"hp","getSoul","rewardItemLot_1","rewardItemLot_2","itemLotId_enemy","itemLotId_map","chaosMatchingRewardLotId","chaosMatchingItemLotId","chaosMatchingCorrectParamId","chaosMatchingSpEffectSetParamId","multiPlayCorrectionParamId","disableRespawn","excludeGroupRewardCheck","isSoulGetByBoss"};
  var rows=ids.Select(id=>npc.Rows.Single(r=>r.ID==id)).Select(r=>new{r.ID,fields=keys.ToDictionary(k=>k,k=>r[k].Value)});
  File.WriteAllText(Path.Combine(output,"npcs.json"),JsonSerializer.Serialize(new{sourceRegulationSha256=regulationHash,rows},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"Exported {ids.Length} original group NPC health/scaling/reward rows; no package changed");
 }
 public static void Export(string root){
  var output=Path.Combine(root,".work/cage-groups");Directory.CreateDirectory(Path.Combine(output,"events"));Directory.CreateDirectory(Path.Combine(output,"maps"));
  var manifest=new List<object>();
  string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p))).ToLowerInvariant();
  foreach(var p in Directory.GetFiles(Path.Combine(root,"assets/original/event"),"*.emevd.dcx").Order()){
   string name=Path.GetFileName(p).Replace(".emevd.dcx","");var emevd=EMEVD.Read(p);
   File.WriteAllText(Path.Combine(output,"events",name+".json"),JsonSerializer.Serialize(emevd.Events.Select(e=>new{e.ID,e.RestBehavior,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,i.Layer,hex=Convert.ToHexString(i.ArgData)})})));
   manifest.Add(new{kind="event",name,sha256=Hash(p),count=emevd.Events.Count});
  }
  var mapInputs=Directory.GetFiles(Path.Combine(root,"assets/original/map/mapstudio"),"*.msb.dcx").Concat(new[]{"extracted","field-extracted"}.SelectMany(d=>Directory.Exists(Path.Combine(output,d))?Directory.GetFiles(Path.Combine(output,d),"*.msb.dcx",SearchOption.AllDirectories):[])).ToArray();
  foreach(var p in mapInputs.Order()){
   string name=Path.GetFileName(p).Replace(".msb.dcx","");var msb=MSB_NR.Read(p);
   var bytes=DCX.Decompress(p).ToArray();
   object? RewardBinding(MSB_NR.Part.Enemy e){
    if(e.EntityData.EntityID==0)return null;
    var at=Enumerable.Range(0,bytes.Length-0xc0).Where(i=>i%4==0&&BitConverter.ToUInt32(bytes,i)==e.EntityData.EntityID&&BitConverter.ToInt32(bytes,i+0x58)==e.NpcThinkParamId&&BitConverter.ToInt32(bytes,i+0x5c)==e.NpcParamId).ToArray();
    if(at.Length!=1)throw new InvalidDataException("Ambiguous enemy type-data reward binding");
    int off=at[0];return new{entityOffset=off,firstFlag=BitConverter.ToUInt32(bytes,off+0xa0),firstLot=BitConverter.ToInt32(bytes,off+0xa4),secondFlag=BitConverter.ToUInt32(bytes,off+0xa8),secondLot=BitConverter.ToInt32(bytes,off+0xac),alternateThink=BitConverter.ToInt32(bytes,off+0xb0)};
   }
   File.WriteAllText(Path.Combine(output,"maps",name+".json"),JsonSerializer.Serialize(new{enemies=msb.Parts.Enemies.Select(e=>new{e.Name,e.ModelName,e.NpcParamId,e.NpcThinkParamId,entity=e.EntityData.EntityID,groups=e.EntityData.EntityGroupIDs,e.EntityData.Variation,position=new{e.Position.X,e.Position.Y,e.Position.Z},rewardBinding=RewardBinding(e)}),generators=msb.Events.Generators,treasures=msb.Events.Treasures}));
   manifest.Add(new{kind="map",name,sha256=Hash(p),count=msb.Parts.Enemies.Count});
  }
  File.WriteAllText(Path.Combine(output,"manifest.json"),JsonSerializer.Serialize(manifest,new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"Exported {manifest.Count} original event/map files for group encounter research; dist untouched");
 }
}
