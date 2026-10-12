using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class HexIcons {
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex hex){
  void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
  string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
  using var catalog=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"data/reference/ui/icon-ids.json")));
  var icons=catalog.RootElement.EnumerateArray().Select(e=>(e.GetProperty("kind").GetString(),e.GetProperty("id").GetInt32())).ToHashSet();
  Check(icons.Contains(("item",hex.LargeIcon))&&icons.Contains(("status",hex.SmallIcon)),"Requested icon missing from native atlas");
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/hex-icons",hex.Id.ToString());Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var originals=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var changes=new Dictionary<int,(string Table,string Field,int Value)>();
  foreach(var (table,field,value) in new[]{("SpEffectParam","iconId",hex.SmallIcon),("AttachEffectParam","statusIconId",hex.SmallIcon),("PermanentBuffParam","iconId",hex.LargeIcon),("EquipParamGoods","iconId",hex.LargeIcon-1000)}){
   var file=binder.Files.Single(f=>Path.GetFileName(f.Name)==table+".param");var rows=parse(table,file.Bytes);
   var row=rows.Rows.Single(r=>r.ID==hex.Id);row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
   file.Bytes=rows.Write();changes.Add(file.ID,(table,field,value));
  }
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);
  Check(final.Files.Count==binder.Files.Count&&final.Version==binder.Version&&final.Compression==binder.Compression,"Binder metadata changed");
  foreach(var file in final.Files){
   var expected=binder.Files.Single(f=>f.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder readback mismatch");
   if(!changes.TryGetValue(file.ID,out var change)){Check(file.Bytes.Span.SequenceEqual(originals[file.ID]),"Unrelated table changed");continue;}
   var before=parse(change.Table,originals[file.ID]);var after=parse(change.Table,file.Bytes).Rows.ToDictionary(r=>r.ID);
   Check(before.Rows.Count==after.Count,"Parameter row count changed");
   foreach(var row in before.Rows)foreach(var cell in row.Cells.Where(c=>c!=null)){
    var actual=after[row.ID][cell.Def.InternalName].Value;
    if(row.ID==hex.Id&&cell.Def.InternalName==change.Field)Check(Convert.ToInt32(actual)==change.Value,"Wrong icon after serialization");
    else Check(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(actual),"Unrelated parameter field changed");
   }
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{hexId=hex.Id,largeIcon=hex.LargeIcon,smallIcon=hex.SmallIcon,inputSha256=Hash(input),outputSha256=Hash(output),nativeAtlasEntriesPresent=true,onlyFourIconFieldsChanged=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {hex.Id} icons {hex.LargeIcon}/{hex.SmallIcon}; native atlas and four serialized fields verified; all other fields preserved");
 }
}
