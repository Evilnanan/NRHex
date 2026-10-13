using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class BalloonAttribute {
 const int Attack=95001760,Bullet=95001750;
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static void Set(PARAM.Row row,string field,int value){row[field].Value=Convert.ChangeType(value,row[field].Value.GetType());}
 public static void Configure(PARAM.Row attack,PARAM.Row bullet){
  // Damage components and the elemental expression are separate. Keep the
  // native standard physical subtype, but remove Golden Shockwave's Holy tag.
  Set(attack,"spAttribute",0);
  if(Convert.ToInt32(bullet["spAttribute"].Value)==13)Set(bullet,"spAttribute",0);
 }
 public static void Probe(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/balloon-attribute-01774");Directory.CreateDirectory(dir);
  var bnd=SFUtil.DecryptNightreignRegulation(input);
  var attacks=parse("AtkParam_Pc",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="AtkParam_Pc.param").Bytes);
  var bullets=parse("Bullet",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="Bullet.param").Bytes);
  object Dump(PARAM.Row row)=>new{row.ID,fields=row.Cells.ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  File.WriteAllText(Path.Combine(dir,"probe.json"),JsonSerializer.Serialize(new{sourceSha256=Hash(input),attacks=attacks.Rows.Where(r=>r.ID==Attack||r.ID==12064105||r.ID==100000000).Select(Dump),bullets=bullets.Rows.Where(r=>r.ID==Bullet||r.ID==12641050).Select(Dump)},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Read-only Balloon/Golden Shockwave attack and projectile attribute data exported");
 }
 public static void Stage(string root,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/balloon-attribute-01774/staged");Directory.CreateDirectory(dir);
  var bnd=SFUtil.DecryptNightreignRegulation(input);var before=bnd.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"AtkParam_Pc","Bullet"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  var original=names.ToDictionary(n=>n,n=>parse(n,before[bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").ID]));
  var attack=tables["AtkParam_Pc"].Rows.Single(r=>r.ID==Attack);var bullet=tables["Bullet"].Rows.Single(r=>r.ID==Bullet);
  Check(Convert.ToInt32(attack["spAttribute"].Value) is 0 or 13,"Unexpected Balloon special attribute");
  Check(Convert.ToInt32(bullet["spAttribute"].Value) is 0 or 13 or 254,"Unexpected Balloon projectile special attribute");
  Check(Convert.ToInt32(attack["atkAttribute"].Value)==3&&Convert.ToInt32(attack["dmgLevel"].Value)==7,"Unexpected physical subtype/impact");
  Check(Convert.ToInt32(attack["atkPhys"].Value)==1&&new[]{"atkMag","atkFire","atkThun","atkDark"}.All(f=>Convert.ToInt32(attack[f].Value)==0),"Unexpected Balloon damage components");
  Configure(attack,bullet);
  foreach(string name in names)bnd.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes=tables[name].Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,bnd,bnd.Compression);
  var read=SFUtil.DecryptNightreignRegulation(output);
  Check(read.Version==bnd.Version&&read.Compression==bnd.Compression&&read.Files.Count==bnd.Files.Count,"Binder metadata changed");
  foreach(var member in read.Files){
   var expected=bnd.Files.Single(f=>f.ID==member.ID);Check(member.Name==expected.Name&&member.Flags==expected.Flags&&member.Bytes.Span.SequenceEqual(expected.Bytes.Span),"Binder readback failed");
   if(!names.Contains(Path.GetFileNameWithoutExtension(member.Name)))Check(member.Bytes.Span.SequenceEqual(before[member.ID]),"Unrelated table changed");
  }
  var changes=new List<object>();
  foreach(string name in names){
   var actual=parse(name,read.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
   var oldRows=original[name].Rows.OrderBy(r=>r.ID).ToArray();var newRows=actual.Rows.OrderBy(r=>r.ID).ToArray();
   Check(oldRows.Length==newRows.Length,"Row count changed");
   foreach(var pair in oldRows.Zip(newRows)){
    Check(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name,"Row identity/order changed");
    foreach(var cell in pair.First.Cells){
     var value=pair.Second[cell.Def.InternalName].Value;
     if(JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(value))continue;
     Check(cell.Def.InternalName=="spAttribute"&&pair.First.ID==(name=="AtkParam_Pc"?Attack:Bullet)&&Convert.ToInt32(cell.Value)==13&&Convert.ToInt32(value)==0,"Unrelated field changed");
     changes.Add(new{table=name,id=pair.First.ID,field=cell.Def.InternalName,before=cell.Value,after=value});
    }
   }
  }
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changes,physicalSubtype=3,specialAttribute=0,impactLevel=7,unrelatedRowsAndFieldsPreserved=true,otherTablesPreserved=true,gameplayVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Balloon Holy expression removed; all other damage, impact, visual and parameter fields preserved");
 }
}
