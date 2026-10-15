using SoulsFormats;
using System.Text.Json;
static class SwordCharged {
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
