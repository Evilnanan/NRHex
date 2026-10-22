using SoulsFormats;
using System.Text.Json;
static class SwordCharged {
 public static void DamageProbe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var report=new Dictionary<string,object>();
  var directory=Path.Combine(root,"data/reference/combat");Directory.CreateDirectory(directory);
  foreach(var name in new[]{"SpEffectParam","AtkParam_Pc"}){
   var parameters=parse(name,original.Files.Single(file=>Path.GetFileName(file.Name)==name+".param").Bytes);
   var ids=name=="SpEffectParam"?new[]{321300,330900,8320100,8320101,8320102}:new[]{47200,47205};
   var rows=parameters.Rows.Where(row=>ids.Contains(row.ID)).ToArray();
   if(rows.Length!=ids.Length)throw new InvalidDataException("Missing charged damage reference rows");
   report[name]=rows.Select(row=>new{row.ID,fields=row.Cells.Where(cell=>cell!=null).ToDictionary(cell=>cell.Def.InternalName,cell=>cell.Value)}).ToArray();
   if(name=="SpEffectParam")foreach(var row in rows){
    parameters.Rows.Clear();parameters.Rows.Add(row);File.WriteAllBytes(Path.Combine(directory,$"effect-{row.ID}.param"),parameters.Write());
   }
  }
  File.WriteAllText(Path.Combine(directory,"sword-charge-damage.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
 }
 public static void Probe(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  var report=new Dictionary<string,object>();
  foreach(string name in new[]{"AttachEffectParam","SpEffectParam"}){
   var p=parse(name,original.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
   var rows=p.Rows.Where(r=>r.ID>=8320400&&r.ID<=8320429||r.ID is 330900 or 8600200 or 8600205||r.ID>=8330200&&r.ID<=8330302).ToArray();
   report[name]=rows.Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}).ToArray();
   if(name=="SpEffectParam"){
    var dir0=Path.Combine(root,".work/sword-charged");Directory.CreateDirectory(dir0);
    foreach(var row in rows){p.Rows.Clear();p.Rows.Add(row);File.WriteAllBytes(Path.Combine(dir0,$"effect-{row.ID}.param"),p.Write());}
   }
  }
  var dir=Path.Combine(root,".work/sword-charged");Directory.CreateDirectory(dir);
  var installed=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var sp=parse("SpEffectParam",installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var pact=sp.Rows.Single(r=>r.ID==95002600);
  report["Contract"]=new[]{new{pact.ID,fields=pact.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}};
  sp.Rows.Clear();sp.Rows.Add(pact);File.WriteAllBytes(Path.Combine(dir,"effect-95002600.param"),sp.Write());
  File.WriteAllText(Path.Combine(dir,"native.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("Native charged-hit damage negation parameters exported");
 }
}
