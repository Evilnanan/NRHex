using SoulsFormats;
using System.Text.Json;
using System.Security.Cryptography;

static class Balance1750 {
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,".work/balance-01750");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin"),target=Path.Combine(dir,"regulation.bin");
  var binder=SFUtil.DecryptNightreignRegulation(input);
  var original=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var changed=new Dictionary<int,(string table,string field,int value)>();
  var balloon=hexes.Single(h=>h.Id==95001700);var noisy=hexes.Single(h=>h.Id==95001500);
  Check(balloon.LargeIcon==noisy.LargeIcon&&balloon.SmallIcon==noisy.SmallIcon,"Balloon icon catalog differs from Noisy Ghost");
  foreach(var (table,field,value) in new[]{("SpEffectParam","iconId",noisy.SmallIcon),("AttachEffectParam","statusIconId",noisy.SmallIcon),("PermanentBuffParam","iconId",noisy.LargeIcon),("EquipParamGoods","iconId",noisy.LargeIcon-1000)}){
   var file=binder.Files.Single(f=>Path.GetFileName(f.Name)==table+".param");
   var rows=parse(table,file.Bytes);var row=rows.Rows.Single(r=>r.ID==balloon.Id);
   Check(Convert.ToInt32(rows.Rows.Single(r=>r.ID==noisy.Id)[field].Value)==value,"Installed Noisy Ghost icon differs from catalog");
   row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());
   file.Bytes=rows.Write();changed.Add(file.ID,(table,field,value));
  }
  SFUtil.EncryptNightreignRegulation(target,binder,binder.Compression);
  var reopened=SFUtil.DecryptNightreignRegulation(target);
  Check(reopened.Version==binder.Version&&reopened.Compression==binder.Compression&&reopened.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var file in reopened.Files){
   var expected=binder.Files.Single(f=>f.ID==file.ID);
   Check(file.Name==expected.Name&&file.Flags==expected.Flags&&file.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder roundtrip mismatch");
   if(!changed.TryGetValue(file.ID,out var change)){Check(file.Bytes.Span.SequenceEqual(original[file.ID]),"Unrelated table changed");continue;}
   var before=parse(change.table,original[file.ID]);var after=parse(change.table,file.Bytes).Rows.ToDictionary(r=>r.ID);
   Check(before.Rows.Count==after.Count,"Row count changed");
   foreach(var row in before.Rows)foreach(var cell in row.Cells){
    var actual=after[row.ID][cell.Def.InternalName].Value;
    if(row.ID==balloon.Id&&cell.Def.InternalName==change.field)Check(Convert.ToInt32(actual)==change.value,"Icon readback mismatch");
    else Check(JsonSerializer.Serialize(actual)==JsonSerializer.Serialize(cell.Value),$"Unrelated parameter field changed: {change.table}/{row.ID}/{cell.Def.InternalName}");
   }
  }
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:[95001800]);
  File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(new{version="0.17.50",inputSha256=Hash(input),outputSha256=Hash(target),icons=changed.Values.Select(c=>new{c.table,c.field,c.value}),unrelatedRowsAndFieldsPreserved=true,hexes=hexes.Where(h=>h.Id is 95001700 or 95001800),gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Balloon icons match Noisy Ghost in all four tables; spirit texts updated; unrelated data preserved");
 }
}
