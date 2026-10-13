using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Restore the native pickup after the explosion, without cloning its gameplay
// effects or treating a beneficial contact as another all-team damaging hit.
static class DevilsGazeRemnant {
 static void Check(bool ok,string message){if(!ok)throw new InvalidDataException(message);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 public static void Stage(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),directory=Path.Combine(root,".work/gaze-remnant-01792");Directory.CreateDirectory(directory);
  var binder=SFUtil.DecryptNightreignRegulation(input);Check(binder.Version==original.Version,"Regulation version changed");
  var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  PARAM Table(BND4 b,string name)=>parse(name,b.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
  var bullets=Table(binder,"Bullet");var expected=Table(binder,"Bullet");
  var pickup=bullets.Rows.Single(r=>r.ID==7560369);
  Check(Convert.ToInt32(pickup["atkId_Bullet"].Value)==7560902&&Convert.ToSingle(pickup["life"].Value)==12&&Convert.ToSingle(pickup["hitRadius"].Value)==1&&Convert.ToInt32(pickup["isPenetrateChr"].Value)==0&&Convert.ToInt32(pickup["HitBulletID"].Value)==-1,"Native pickup chain changed");
  Check(new[]{0,1,2}.Select(i=>Convert.ToInt32(pickup[$"spEffectId{i}"].Value)).SequenceEqual(new[]{45890,45891,45893}),"Native pickup effects changed");
  var preserved=new Dictionary<string,int[]>{["Bullet"]=[7560369],["AtkParam_Npc"]=[7560902],["SpEffectParam"]=[45890,45891,45893],["SpEffectVfxParam"]=[54460,54461,54462]};
  foreach(var (name,ids) in preserved){var installed=Table(binder,name);var native=Table(original,name);foreach(int id in ids)Check(CageAI.Same(installed.Rows.Single(r=>r.ID==id),native.Rows.Single(r=>r.ID==id)),"Native pickup dependency modified: "+name+" "+id);}
  var attack=Table(binder,"AtkParam_Npc").Rows.Single(r=>r.ID==7560902);
  Check(new[]{"atkPhys","atkMag","atkFire","atkThun","atkDark","atkStam","atkSuperArmor","dmgLevel"}.All(f=>Convert.ToSingle(attack[f].Value)==0),"Pickup must not deal damage or stagger");
  var changed=new List<int>();
  for(int depth=0;depth<=5;depth++){
   int id=DevilsGaze.FirstBullet+depth*10+2;var row=bullets.Rows.Single(r=>r.ID==id);
   Check(Convert.ToInt32(row["HitBulletID"].Value) is -1 or 7560369,"Unexpected explosion child");
   Check(Convert.ToInt32(row["atkId_Bullet"].Value)==DevilsGaze.Owner+depth,"Unexpected explosion attack");
   row["HitBulletID"].Value=7560369;expected.Rows.Single(r=>r.ID==id)["HitBulletID"].Value=7560369;changed.Add(id);
  }
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="Bullet.param");member.Bytes=bullets.Write();
  string output=Path.Combine(directory,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var final=SFUtil.DecryptNightreignRegulation(output);Check(final.Files.Count==binder.Files.Count,"Member count changed");
  foreach(var f in final.Files){var intended=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Readback mismatch");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var actual=Table(final,"Bullet");Check(actual.Rows.Count==expected.Rows.Count&&actual.Rows.Zip(expected.Rows).All(p=>CageAI.Same(p.First,p.Second)),"Unrelated bullet field changed");
  File.WriteAllText(Path.Combine(directory,"parameter-validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedExplosionIds=changed,onlySixChildLinksChanged=true,nativePickup=7560369,pickupLifetimeSeconds=12,pickupRadius=1,pickupEffects=new[]{45890,45891,45893},nativePickupRowsUnchanged=true,pickupHasNoDamageOrStagger=true,pickupConsumedOnContact=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: six explosion links restored to native pickup; all other rows/fields and native pickup effects preserved");
 }
}
