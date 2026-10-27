using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class HexTexts {
 sealed record StatusText(int TextId,int OwnerId,string ZhInfo,string EnInfo,string? ZhName=null,string? EnName=null,string ZhSuffix="",string EnSuffix="");
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool value,string why){if(!value)throw new InvalidDataException(why);}
 public static void Verify(string root,Hex[] hexes){
  foreach(var language in new[]{"zhocn","engus"}){
   var current=BND4.Read(Path.Combine(root,"dist/mod/msg",language,"item_dlc01.msgbnd.dcx"));
   foreach(var filename in new[]{"AttachEffectName.fmg","AttachEffectInfo.fmg","PermanentBuffName.fmg","PermanentBuffInfo.fmg","PermanentBuffCaption.fmg"}){
    var fmg=FMG.Read(current.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()==filename).Bytes);
    foreach(var h in hexes){
     var expected=language=="zhocn"?(filename.Contains("Name")?h.Zh:h.ZhInfo):(filename.Contains("Name")?h.En:h.EnInfo);
     Check(fmg.Entries.SingleOrDefault(e=>e.ID==h.Id)?.Text==expected,$"Document/package text mismatch: {language}/{filename} {h.Id}");
    }
   }
   var menu=BND4.Read(Path.Combine(root,"dist/mod/msg",language,"menu_dlc01.msgbnd.dcx"));
   var statuses=Statuses(root,hexes,language);
   foreach(var filename in new[]{"SpEffectName.fmg","SpEffectInfo.fmg"}){
    var fmg=FMG.Read(menu.Files.Single(f=>f.Name.Replace('\\','/').Split('/').Last()==filename).Bytes);
    foreach(var (id,text) in statuses)Check(fmg.Entries.SingleOrDefault(e=>e.ID==id)?.Text==(filename.Contains("Name")?text.Name:text.Info),$"Status catalog/package text mismatch: {language}/{filename} {id}");
   }
  }
  Console.WriteLine($"PASS: {hexes.Length} abilities and all status texts match the Chinese and English packages (five reward and two status FMGs per language).");
 }
 public static Dictionary<int,(string Name,string Info)> Statuses(string root,Hex[] hexes,string language){
  var rows=JsonSerializer.Deserialize<StatusText[]>(File.ReadAllText(Path.Combine(root,"data/text/status-effects.json")),new JsonSerializerOptions{PropertyNameCaseInsensitive=true})??throw new InvalidDataException("Missing status text catalog");
  Check(language is "zhocn" or "engus","Unsupported status language");
  Check(rows.Select(r=>r.TextId).Distinct().Count()==rows.Length&&rows.All(r=>r.TextId>=95000000&&r.TextId<95006000&&!string.IsNullOrWhiteSpace(r.ZhInfo)&&!string.IsNullOrWhiteSpace(r.EnInfo)),"Invalid status text IDs/descriptions");
  return rows.ToDictionary(r=>r.TextId,r=>{
   var owner=hexes.Single(h=>h.Id==r.OwnerId);
   return language=="zhocn"?((r.ZhName??owner.Zh)+r.ZhSuffix,r.ZhInfo):((r.EnName??owner.En)+r.EnSuffix,r.EnInfo);
  });
 }
 public static void Stage(string root,Hex[] hexes,string? destination=null,bool statusOnly=false,bool allRewards=false,int[]? onlyIds=null,int[]? retiredIds=null,string? inputMod=null){
  var output=destination??Path.Combine(root,".work/crystal-rebound-01730/texts");Directory.CreateDirectory(output);
  var reports=new List<object>();
  foreach(var language in destination==null&&!statusOnly?new[]{"zhocn"}:new[]{"zhocn","engus"}){
  var selected=onlyIds!=null?hexes.Where(h=>onlyIds.Contains(h.Id)).ToArray():language=="zhocn"||allRewards?hexes:hexes.Where(h=>h.Id==95001300).ToArray();
  var names=selected.ToDictionary(h=>h.Id,h=>language=="zhocn"?h.Zh:h.En);var infos=selected.ToDictionary(h=>h.Id,h=>language=="zhocn"?h.ZhInfo:h.EnInfo);
  foreach(int retired in retiredIds??Array.Empty<int>()){names[retired]="";infos[retired]="";}
  foreach(var kind in statusOnly?new[]{"menu"}:new[]{"item","menu"}){
   if(language=="engus"&&kind=="menu"&&!statusOnly&&!allRewards)continue;
   var replacements=new Dictionary<string,Dictionary<int,string>>();
   if(kind=="item"){
    foreach(string f in new[]{"AttachEffectName.fmg","PermanentBuffName.fmg"})replacements[f]=names;
    foreach(string f in new[]{"AttachEffectInfo.fmg","PermanentBuffInfo.fmg","PermanentBuffCaption.fmg"})replacements[f]=infos;
   }else{
    var statuses=Statuses(root,hexes,language);
    if(allRewards)foreach(var h in selected)statuses.TryAdd(h.Id,(names[h.Id],infos[h.Id]));
    replacements["SpEffectName.fmg"]=statuses.ToDictionary(v=>v.Key,v=>v.Value.Name);
    replacements["SpEffectInfo.fmg"]=statuses.ToDictionary(v=>v.Key,v=>v.Value.Info);
   }
   string file=kind+"_dlc01.msgbnd.dcx",input=Path.Combine(inputMod??Path.Combine(root,"dist/mod"),"msg",language,file),target=Path.Combine(output,language,file);Directory.CreateDirectory(Path.GetDirectoryName(target)!);
   // Retain the historical Chinese-only staging location for older tools.
   if(destination==null)target=Path.Combine(output,file);
   var before=BND4.Read(input);var patched=BND4.Read(input);var changes=new List<object>();
   Check(replacements.Keys.All(n=>patched.Files.Count(f=>f.Name.Replace('\\','/').Split('/').Last()==n)==1),"Missing custom text FMG");
   foreach(var member in patched.Files.Where(f=>replacements.ContainsKey(f.Name.Replace('\\','/').Split('/').Last()))){
    var text=FMG.Read(member.Bytes);var wanted=replacements[member.Name.Replace('\\','/').Split('/').Last()];
    foreach(var (id,value) in wanted){
     var row=text.Entries.SingleOrDefault(e=>e.ID==id);
     Check(row!=null||kind=="menu"||allRewards,"Missing reward text entry");
     if(row?.Text!=value)changes.Add(new{fmg=member.Name.Replace('\\','/').Split('/').Last(),id,before=row?.Text,after=value});
     if(row==null){row=new FMG.Entry(text,id,value);text.Entries.Add(row);}
     row.Text=value;
    }
    member.Bytes=text.Write();
   }
   patched.Write(target);var reopened=BND4.Read(target);
   Check(reopened.Files.Count==before.Files.Count&&reopened.Format==before.Format&&reopened.Version==before.Version&&reopened.Compression==before.Compression,"Text binder metadata changed");
   foreach(var member in reopened.Files){
    var old=before.Files.Single(f=>f.ID==member.ID);var intended=patched.Files.Single(f=>f.ID==member.ID);
    Check(member.Name==old.Name&&member.Flags==old.Flags&&member.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Text readback mismatch");
    if(replacements.TryGetValue(member.Name.Replace('\\','/').Split('/').Last(),out var wanted)){
     var a=FMG.Read(member.Bytes);var b=FMG.Read(old.Bytes);
     Check(a.Version==b.Version&&a.BigEndian==b.BigEndian&&a.Entries.Count==b.Entries.Count+wanted.Keys.Count(id=>!b.Entries.Any(e=>e.ID==id))&&b.Entries.All(old=>a.Entries.Single(e=>e.ID==old.ID).Text==(wanted.TryGetValue(old.ID,out var value)?value:old.Text))&&wanted.All(v=>a.Entries.Single(e=>e.ID==v.Key).Text==v.Value),"Unrelated text changed");
    }else Check(member.Bytes.Span.SequenceEqual(old.Bytes.Span),"Unrelated text member changed");
   }
   reports.Add(new{language,file,inputSha256=Hash(input),outputSha256=Hash(target),changes,unrelatedEntriesAndMembersUnchanged=true});
  }
  }
  File.WriteAllText(Path.Combine(output,"validation.json"),JsonSerializer.Serialize(new{source="docs/hexes.md",sourceSha256=Hash(Path.Combine(root,"docs/hexes.md")),englishSource="docs/hexes.en.md",englishSourceSha256=Hash(Path.Combine(root,"docs/hexes.en.md")),statusSource="data/text/status-effects.json",statusSourceSha256=Hash(Path.Combine(root,"data/text/status-effects.json")),statusOnly,reports},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: staged names/descriptions from the Markdown and status catalog; unrelated text preserved");
 }
}
