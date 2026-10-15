using SoulsFormats;
using System.Text.Json;

static class CageHealth {
 public static void Probe(string root,Func<string,Memory<byte>,PARAM> parse){
  var binder=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
  var tables=new[]{"SpEffectParam","NpcParam","MultiPlayCorrectionParam"}.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
  object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
  int[] effects={7999,7844,7944,7845,7945,99150,99152,99153,95001602};
  var report=new{effects=tables["SpEffectParam"].Rows.Where(r=>effects.Contains(r.ID)).Select(Dump),npcs=tables["NpcParam"].Rows.Where(r=>r.ID is 95300001 or 95300019 or 95300072).Select(Dump),corrections=tables["MultiPlayCorrectionParam"].Rows.Where(r=>r.ID is 7744 or 7745).Select(Dump)};
  string dir=Path.Combine(root,".work/cage-hp-122");Directory.CreateDirectory(dir);File.WriteAllText(Path.Combine(dir,"parameters.json"),JsonSerializer.Serialize(report));
  var common=EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx"));
  File.WriteAllText(Path.Combine(dir,"native-events.json"),JsonSerializer.Serialize(common.Events.Where(e=>e.ID is 90015022 or 90045004 or 90065000 or 90065031).Select(e=>new{e.ID,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})})));
  Console.WriteLine("PASS: read-only native correction and gaol event evidence exported");
 }
}
