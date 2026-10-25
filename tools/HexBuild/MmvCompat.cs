using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

// Inspect three inputs before deciding compatibility. Never infer mergeability
// from matching filenames, and never mutate the independent distribution.
static class MmvCompat {
 public static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Value(object? v)=>v switch{
  float number=>"f32:"+BitConverter.SingleToInt32Bits(number).ToString("X8"),
  double number=>"f64:"+BitConverter.DoubleToInt64Bits(number).ToString("X16"),
  _=>JsonSerializer.Serialize(v)
 };
 static bool Same(PARAM.Row a,PARAM.Row b)=>a.Cells.Select(c=>Value(c.Value)).SequenceEqual(b.Cells.Select(c=>Value(c.Value)));
 static object Event(EMEVD.Event e)=>new{e.ID,e.Name,e.RestBehavior,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,i.Layer,data=Convert.ToHexString(i.ArgData)}),parameters=e.Parameters};
 static bool Same(EMEVD.Event a,EMEVD.Event b)=>Value(Event(a))==Value(Event(b));
 public static void Audit(string root,string mmv,BND4 original,Func<string,Memory<byte>,PARAM> parse,string? originalSfx=null){
  mmv=Path.GetFullPath(mmv);var dir=Path.Combine(root,"work/compat/mmv");Directory.CreateDirectory(dir);
  var records=new List<object>();var conflicts=new List<object>();
  var ours=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var theirs=SFUtil.DecryptNightreignRegulation(Path.Combine(mmv,"mod/regulation.bin"));
  if(ours.Version!=original.Version||theirs.Version!=original.Version)throw new InvalidDataException("MMV / NRHex regulation version does not match the original parameter definitions");
  var rowsAdded=0;var cellsChanged=0;
  foreach(var f in ours.Files){
   var bf=original.Files.SingleOrDefault(x=>Name(x)==Name(f));var tf=theirs.Files.SingleOrDefault(x=>Name(x)==Name(f));
   if(bf!=null&&f.Bytes.Span.SequenceEqual(bf.Bytes.Span))continue;
   var name=Name(f).Replace(".param","");
   if(bf==null||tf==null){conflicts.Add(new{file="regulation.bin",table=name,reason="table-added-or-removed"});continue;}
   try{
    var bg=parse(name,bf.Bytes).Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>g.ToArray());var ng=parse(name,f.Bytes).Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>g.ToArray());var mg=parse(name,tf.Bytes).Rows.GroupBy(r=>r.ID).ToDictionary(g=>g.Key,g=>g.ToArray());
    var b=bg.ToDictionary(x=>x.Key,x=>x.Value[0]);var n=ng.ToDictionary(x=>x.Key,x=>x.Value[0]);var m=mg.ToDictionary(x=>x.Key,x=>x.Value[0]);
    foreach(var pair in n){var id=pair.Key;var row=pair.Value;
     if(!b.TryGetValue(id,out var old)){rowsAdded+=ng[id].Length;if(mg.TryGetValue(id,out var other)&&(other.Length!=ng[id].Length||!other.Zip(ng[id]).All(p=>Same(p.First,p.Second))))conflicts.Add(new{file="regulation.bin",table=name,row=id,reason="new-row-id-collision"});continue;}
     if(bg[id].Length==ng[id].Length&&bg[id].Zip(ng[id]).All(p=>Same(p.First,p.Second)))continue;
     if(bg[id].Length!=1||ng[id].Length!=1||mg.TryGetValue(id,out var duplicates)&&duplicates.Length!=1){conflicts.Add(new{file="regulation.bin",table=name,row=id,reason="ambiguous-modified-duplicate"});continue;}
     if(!m.TryGetValue(id,out var target)){conflicts.Add(new{file="regulation.bin",table=name,row=id,reason="modified-row-missing"});continue;}
     foreach(var c in row.Cells){var key=c.Def.InternalName;var before=Value(old[key].Value);var wanted=Value(c.Value);if(before==wanted)continue;cellsChanged++;var current=Value(target[key].Value);if(current!=before&&current!=wanted)conflicts.Add(new{file="regulation.bin",table=name,row=id,field=key,before,wanted,current,reason="field-conflict"});}
    }
    foreach(var id in b.Keys.Except(n.Keys))if(m.TryGetValue(id,out var target)&&!Same(b[id],target))conflicts.Add(new{file="regulation.bin",table=name,row=id,reason="delete-versus-modify"});
   }catch(Exception ex){conflicts.Add(new{file="regulation.bin",table=name,reason="parse-failed",message=ex.Message});}
  }
  foreach(var path in Directory.GetFiles(Path.Combine(root,"dist/mod"),"*",SearchOption.AllDirectories)){
   var rel=Path.GetRelativePath(Path.Combine(root,"dist/mod"),path).Replace('\\','/');if(rel.StartsWith("natives/")||rel.EndsWith(".log"))continue;
   var other=Path.Combine(mmv,"mod",rel);var baseline=Path.Combine(root,"assets/original",rel);
   if(rel=="regulation.bin"){records.Add(new{path=rel,kind="param-fields",nrhexHash=Hash(path),mmvHash=Hash(other),rowsAdded,cellsChanged});continue;}
   if(!File.Exists(other)){records.Add(new{path=rel,kind="nrhex-only",nrhexHash=Hash(path)});continue;}
   // Extra original assets must be supplied explicitly; never use a stale research cache.
   if(!File.Exists(baseline)&&originalSfx!=null&&rel=="sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx")baseline=originalSfx;
   records.Add(new{path=rel,kind="overlap",nrhexHash=Hash(path),mmvHash=Hash(other),baseline=File.Exists(baseline)?baseline:null});
   if(Hash(path)==Hash(other))continue;
   if(!File.Exists(baseline)){conflicts.Add(new{file=rel,reason="baseline-missing"});continue;}
   try{
    if(rel.EndsWith(".emevd.dcx")){
     var b=EMEVD.Read(baseline).Events.ToDictionary(e=>e.ID);var n=EMEVD.Read(path).Events.ToDictionary(e=>e.ID);var m=EMEVD.Read(other).Events.ToDictionary(e=>e.ID);
     foreach(var (id,e) in n){if(b.TryGetValue(id,out var old)&&Same(old,e))continue;if(m.TryGetValue(id,out var cur)&&(!b.ContainsKey(id)||!Same(old!,cur))&&!Same(e,cur))conflicts.Add(new{file=rel,eventId=id,reason="event-conflict",baseline=b.ContainsKey(id)?Event(old!):null,nrhex=Event(e),mmv=Event(cur)});}
    }else if(rel.EndsWith("bnd.dcx")){
     var b=BND4.Read(baseline).Files.ToDictionary(Name);var n=BND4.Read(path).Files.ToDictionary(Name);var m=BND4.Read(other).Files.ToDictionary(Name);
     foreach(var (name,f) in n){if(b.TryGetValue(name,out var old)&&f.Bytes.Span.SequenceEqual(old.Bytes.Span))continue;if(!m.TryGetValue(name,out var cur)||cur.Bytes.Span.SequenceEqual(f.Bytes.Span))continue;if(old!=null&&cur.Bytes.Span.SequenceEqual(old.Bytes.Span))continue;
      if(name.EndsWith(".fmg")&&old!=null){var bf=FMG.Read(old.Bytes).Entries.ToDictionary(e=>e.ID,e=>e.Text);var nf=FMG.Read(f.Bytes).Entries.ToDictionary(e=>e.ID,e=>e.Text);var mf=FMG.Read(cur.Bytes).Entries.ToDictionary(e=>e.ID,e=>e.Text);foreach(var (id,text)in nf){bf.TryGetValue(id,out var bt);mf.TryGetValue(id,out var mt);if(text!=bt&&mt!=bt&&mt!=text)conflicts.Add(new{file=rel,member=name,id,reason="text-conflict",before=bt,wanted=text,current=mt});}}
      else if(name.EndsWith(".tae")&&old!=null){
       var bg=TAE.Read(old.Bytes).Animations.GroupBy(a=>a.ID).ToDictionary(g=>g.Key,g=>g.ToArray());var ng=TAE.Read(f.Bytes).Animations.GroupBy(a=>a.ID).ToDictionary(g=>g.Key,g=>g.ToArray());var mg=TAE.Read(cur.Bytes).Animations.GroupBy(a=>a.ID).ToDictionary(g=>g.Key,g=>g.ToArray());
       string Key(TAE.Event e)=>Value(new{e.Type,e.Unk04,e.StartTime,e.EndTime,group=e.Group?.GroupType,data=Convert.ToHexString(e.GetParameterBytes(false))});
       string Anim(TAE.Animation a)=>Value(new{a.ID,a.AnimFileName,events=a.Events.Select(Key)});
       foreach(var (id,animations) in ng){
        if(!bg.TryGetValue(id,out var originals)){if(mg.ContainsKey(id))conflicts.Add(new{file=rel,member=name,animation=id,reason="animation-id-collision"});continue;}
        if(originals.Select(Anim).SequenceEqual(animations.Select(Anim)))continue;
        if(originals.Length!=1||animations.Length!=1||mg.TryGetValue(id,out var duplicates)&&duplicates.Length!=1){conflicts.Add(new{file=rel,member=name,animation=id,reason="ambiguous-modified-animation"});continue;}
        var ba=originals[0];var a=animations[0];
        if(!mg.ContainsKey(id)){conflicts.Add(new{file=rel,member=name,animation=id,reason="modified-animation-missing"});continue;}
        // Match one-to-one, allowing only the known first-record alignment padding.
        var remaining=a.Events.ToList();var removed=new List<string>();
        for(int index=0;index<ba.Events.Count;index++){
         var e=ba.Events[index];var found=remaining.FindIndex(candidate=>Key(e)==Key(candidate));
         if(found<0&&index==0)found=remaining.FindIndex(candidate=>SameWithAlignment(e,candidate));
         if(found>=0)remaining.RemoveAt(found);else removed.Add(Key(e));
        }
        if(removed.Count>0)conflicts.Add(new{file=rel,member=name,animation=id,reason="nrhex-existing-animation-events-modified",removed});
       }
       // Additive TAE changes still need an adapter preserving MMV timings and raw data.
       conflicts.Add(new{file=rel,member=name,reason="tae-requires-timing-aware-adapter"});
      }else conflicts.Add(new{file=rel,member=name,reason="binder-member-conflict",baseSize=old?.Bytes.Length,nrhexSize=f.Bytes.Length,mmvSize=cur.Bytes.Length});
     }
    }else conflicts.Add(new{file=rel,reason=rel.EndsWith(".hks")?"hks-requires-source-adapter":"structured-file-requires-adapter"});
   }catch(Exception ex){conflicts.Add(new{file=rel,reason="parse-failed",message=ex.Message});}
  }
  var options=new JsonSerializerOptions{WriteIndented=true};File.WriteAllText(Path.Combine(dir,"audit.json"),JsonSerializer.Serialize(new{schemaVersion=1,stage="inspection-only",mmvRoot=mmv,parameterVersions=new{original=original.Version,nrhex=ours.Version,mmv=theirs.Version},records,conflicts,releaseReady=false,gameplayVerified=false},options));
  Console.WriteLine($"MMV audit: {records.Count} NRHex resources; {rowsAdded} added parameter rows; {cellsChanged} changed existing cells; {conflicts.Count} conflict records. Report: {dir}/audit.json");
 }
 static bool SameWithAlignment(TAE.Event a,TAE.Event b){
  if(a.Type!=b.Type||a.Unk04!=b.Unk04||a.StartTime!=b.StartTime||a.EndTime!=b.EndTime||a.Group?.GroupType!=b.Group?.GroupType)return false;
  var x=a.GetParameterBytes(false);var y=b.GetParameterBytes(false);
  return y.Length==x.Length+8&&x.SequenceEqual(y.Take(x.Length))&&y.Skip(x.Length).All(v=>v==0);
 }
}
