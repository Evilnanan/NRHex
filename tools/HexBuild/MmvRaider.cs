using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// This adapter changes DLL composition only. Pin the native factors and TAE
// windows it relies on; do not mutate MMV's incoming penalty or shared rows.
static class MmvRaider {
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Verify(string overlay,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(overlay,"mod/regulation.bin"),taePath=Path.Combine(overlay,"mod/chr/c0000.anibnd.dcx");
  var hash=Hash(input);var binder=SFUtil.DecryptNightreignRegulation(input);
  PARAM Table(string n)=>parse(n,binder.Files.Single(f=>Name(f)==n+".param").Bytes);
  var effects=Table("SpEffectParam");var rows=new List<object>();
  foreach(var(id,rate)in new[]{(7031601,1.5f),(7031602,2f)}){
   var row=effects.Rows.Single(r=>r.ID==id);
   Check(Convert.ToSingle(row["physicsAttackRate"].Value)==rate,"MMV Raider native rate changed");
   Check(Convert.ToInt32(row["invocationConditionsStateChange1"].Value)==2084,"MMV Raider owner condition changed");
   foreach(var e in new[]{"magic","fire","thunder","dark"})Check(Convert.ToSingle(row[e+"AttackRate"].Value)==1f,"MMV Raider elemental rates changed");
   foreach(var e in new[]{"physics","magic","fire","thunder","dark"})Check(Convert.ToSingle(row[e+"AttackPowerRate"].Value)==1f,"MMV Raider power correction changed");
   foreach(var e in new[]{"slash","blow","thrust","neutral","magic","fire","thunder","dark"})Check(Convert.ToSingle(row[e+"DamageCutRate"].Value)==1.3f,"MMV Raider incoming penalty changed");
   rows.Add(new{id,physicsAttackRate=rate,damageTaken=1.3f});
  }
  Check(Convert.ToInt32(effects.Rows.Single(r=>r.ID==7031600)["stateInfo"].Value)==2084,"MMV Raider owner state changed");
  var tool=Table("EquipParamWeapon").Rows.Single(r=>r.ID==60400000);
  Check(Convert.ToInt32(tool["attackBasePhysics"].Value)>0,"MMV Retaliate source tool changed");
  foreach(var e in new[]{"Magic","Fire","Thunder","Dark"})Check(Convert.ToInt32(tool["attackBase"+e].Value)==0,"MMV Retaliate is no longer physical-only");
  var tae=TAE.Read(BND4.Read(taePath).Files.Single(f=>Name(f)=="a904.tae").Bytes);
  var windows=new List<object>();
  foreach(var(anim,effect)in new[]{(541010,7031601),(541020,7031602)}){
   var a=tae.Animations.Single(a=>a.ID==anim);
   var buffs=a.Events.Where(e=>e.Type==67&&BitConverter.ToInt32(e.GetParameterBytes(false))==effect).ToArray();
   var hits=a.Events.Where(e=>e.Type==1).ToArray();Check(buffs.Length==1&&hits.Length>=2,"MMV Raider attack/buff timeline changed");
   Check(hits.All(e=>buffs[0].StartTime<=e.StartTime&&buffs[0].EndTime>=e.EndTime),"MMV Raider buff no longer covers hit window");
   windows.Add(new{animation=anim,effect,buffStart=buffs[0].StartTime,buffEnd=buffs[0].EndTime,hits=hits.Select(e=>new{e.StartTime,e.EndTime})});
  }
  Check(Hash(input)==hash,"Read-only Raider check changed regulation");
  File.WriteAllText(Path.Combine(overlay,"raider-validation.json"),JsonSerializer.Serialize(new{inputSha256=hash,taeSha256=Hash(taePath),rows,windows,parametersUnchanged=true,physicalSourceTool=60400000,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: MMV Raider native rates, incoming penalty and attack windows verified; no resources changed");
 }
}
