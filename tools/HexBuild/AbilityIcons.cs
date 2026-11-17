using SoulsFormats;
using System.Text.Json;

static class AbilityIcons {
 static readonly JsonSerializerOptions CellJson=new(){NumberHandling=System.Text.Json.Serialization.JsonNumberHandling.AllowNamedFloatingPointLiterals};
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static string Name(BinderFile file)=>file.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes,int owner=Erosion.Owner,int previousLarge=21207,int previousSmall=22207,string? inputMod=null,string? outputStage=null){
  var hex=hexes.Single(hex=>hex.Id==owner);
  string input=Path.Combine(inputMod??Path.Combine(root,"dist/mod"),"regulation.bin"),directory=outputStage??Path.Combine(root,".work/ability-icons/staged");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(file=>file.ID,file=>file.Bytes.ToArray());
  // Standalone uses the normalized format; explicit overlays retain their
  // own already-loaded compression and are verified again after writing.
  if(inputMod==null)Check(binder.Compression==RegulationCompression.Format,"Unexpected regulation compression");
  var fields=new Dictionary<string,(string Field,int Value)>{
   ["SpEffectParam"]=("iconId",hex.SmallIcon),["AttachEffectParam"]=("statusIconId",hex.SmallIcon),
   ["PermanentBuffParam"]=("iconId",hex.LargeIcon),["EquipParamGoods"]=("iconId",hex.LargeIcon-1000)
  };
  foreach(var (table,field) in fields){
   var file=binder.Files.Single(file=>Name(file)==table+".param");var parameters=parse(table,file.Bytes);
   var row=parameters.Rows.Single(row=>row.ID==hex.Id);int current=Convert.ToInt32(row[field.Field].Value);
   int previous=table=="PermanentBuffParam"?previousLarge:table=="EquipParamGoods"?previousLarge-1000:previousSmall;
   Check(current==previous||current==field.Value,$"Unexpected icon {table}/{hex.Id}: {current}");
   row[field.Field].Value=Convert.ChangeType(field.Value,row[field.Field].Value.GetType());file.Bytes=parameters.Write();
  }
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  var changes=new List<object>();
  foreach(var pair in binder.Files.Zip(actual.Files)){
   var file=pair.Second;Check(pair.First.ID==file.ID&&pair.First.Name==file.Name&&pair.First.Flags==file.Flags&&pair.First.Bytes.Span.SequenceEqual(file.Bytes.Span),"Binder readback mismatch");
   string table=Name(file).Replace(".param","");
   if(!fields.TryGetValue(table,out var field)){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}
   var oldRows=parse(table,before[file.ID]).Rows;var newRows=parse(table,file.Bytes).Rows;Check(oldRows.Count==newRows.Count,"Row count changed");
   foreach(var rows in oldRows.Zip(newRows)){
    Check(rows.First.ID==rows.Second.ID&&rows.First.Name==rows.Second.Name,"Row identity changed");
    foreach(var cell in rows.First.Cells){
     object value=rows.Second[cell.Def.InternalName].Value;
     if(rows.First.ID==hex.Id&&cell.Def.InternalName==field.Field)Check(Convert.ToInt32(value)==field.Value,"Target icon readback failed");
     if(JsonSerializer.Serialize(cell.Value,CellJson)==JsonSerializer.Serialize(value,CellJson))continue;
     Check(rows.First.ID==hex.Id&&cell.Def.InternalName==field.Field,"Unrelated field changed");
     changes.Add(new{table,id=hex.Id,field=field.Field,before=cell.Value,after=value});
    }
   }
  }
  File.WriteAllText(Path.Combine(directory,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),owner=hex.Id,largeIcon=hex.LargeIcon,smallIcon=hex.SmallIcon,changes,unrelatedRowsAndFieldsPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: ability {owner} icons read back; all unrelated parameter fields preserved");
 }
}
