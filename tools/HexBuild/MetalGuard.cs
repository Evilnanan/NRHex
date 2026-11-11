using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class MetalGuard {
 public const int EffectId=95004603;
 public const float Multiplier=0.5f,PoiseMultiplier=1.25f;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static string Value(object v)=>v switch{float f=>"f32:"+BitConverter.SingleToInt32Bits(f).ToString("X8"),double d=>"f64:"+BitConverter.DoubleToInt64Bits(d).ToString("X16"),byte[] b=>Convert.ToHexString(b),_=>JsonSerializer.Serialize(v)};
 static bool Same(PARAM.Row a,PARAM.Row b)=>a.ID==b.ID&&a.Name==b.Name&&a.Cells.Select(c=>(c.Def.InternalName,Value(c.Value))).SequenceEqual(b.Cells.Select(c=>(c.Def.InternalName,Value(c.Value))));
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static readonly string[] SpellFlags={"magParamChange","miracleParamChange","shamanParamChange"};
 public static void Effect(PARAM.Row r){
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("effectEndurance",2f),("saveCategory",-1),("stateInfo",158),("guardStaminaMult",Multiplier),("saAttackPowerRate",PoiseMultiplier),("isDisableNetSync",0),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);
  foreach(var f in SpellFlags)Set(r,f,1);
 }
 public static void Validate(PARAM.Row r){
  Check(SpellFlags.All(f=>Convert.ToInt32(r[f].Value)==1),"Missing Metal spell applicability");
  Check(r.ID==EffectId&&Convert.ToSingle(r["guardStaminaMult"].Value)==Multiplier&&Convert.ToSingle(r["saAttackPowerRate"].Value)==PoiseMultiplier&&Convert.ToInt32(r["stateInfo"].Value)==158&&Convert.ToSingle(r["effectEndurance"].Value)==2f&&Convert.ToInt32(r["saveCategory"].Value)==-1&&Convert.ToInt32(r["isDisableNetSync"].Value)==0,"Invalid Metal guard lease");
 }
 static bool Removal(EMEVD.Instruction i)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==EffectId;
 public static void Events(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0&&!e.Instructions.Any(Removal),"Parameterized or already patched lifecycle");
   int at=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(at>0&&e.Instructions.Take(at).Any(i=>i.Bank==3),"Missing guarded cleanup anchor");
   e.Instructions.Insert(at,new(2004,21,new object[]{20000u,EffectId}));
  }
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,string mod,string dir){
  Directory.CreateDirectory(dir);string input=Path.Combine(mod,"regulation.bin");var b=SFUtil.DecryptNightreignRegulation(input);
  var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var member=b.Files.Single(f=>Name(f)=="SpEffectParam.param");
  var old=parse("SpEffectParam",member.Bytes);var rows=parse("SpEffectParam",member.Bytes);
  var template=parse("SpEffectParam",native.Files.Single(f=>Name(f)=="SpEffectParam.param").Bytes).Rows.Single(r=>r.ID==8000000);
  var existing=rows.Rows.SingleOrDefault(r=>r.ID==EffectId);bool updating=existing!=null;
  var added=new PARAM.Row(template){ID=EffectId};Effect(added);Validate(added);
  if(existing!=null){var prior=new PARAM.Row(added);Set(prior,"saAttackPowerRate",1f);foreach(var f in SpellFlags)Set(prior,f,template[f].Value);Check(Same(existing,prior)||Same(existing,added),"Unexpected existing Metal effect; refusing unrelated overwrite");Effect(existing);}
  else rows.Rows.Add(added);
  rows.Rows=rows.Rows.OrderBy(r=>r.ID).ToList();member.Bytes=rows.Write();
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Regulation metadata changed");
  foreach(var f in actual.Files){
   var intended=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter readback mismatch");
   if(f.ID!=member.ID){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var now=parse("SpEffectParam",f.Bytes);Check(now.Rows.Count==old.Rows.Count+(updating?0:1)&&old.Rows.Where(r=>r.ID!=EffectId).OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>r.ID!=EffectId).OrderBy(r=>r.ID)).All(p=>Same(p.First,p.Second)),"Existing effect row changed");
   Check(Same(added,now.Rows.Single(r=>r.ID==EffectId)),"Private guard row changed in serialization");
   now.Rows=now.Rows.Where(r=>r.ID is EffectId or 701535 or 701550).ToList();File.WriteAllBytes(Path.Combine(dir,"guard-effects.param"),now.Write());
  }
  string eventInput=Path.Combine(mod,"event/common.emevd.dcx");var original=EMEVD.Read(eventInput);var ev=EMEVD.Read(eventInput);if(!updating)Events(ev);
  string eventOutput=Path.Combine(dir,"common.emevd.dcx");if(updating)File.Copy(eventInput,eventOutput,true);else ev.Write(eventOutput);var final=EMEVD.Read(eventOutput);
  Check(final.Format==original.Format&&final.Compression==original.Compression&&final.Events.Count==original.Events.Count&&final.StringData.SequenceEqual(original.StringData)&&final.LinkedFileOffsets.SequenceEqual(original.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in original.Events){var now=final.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(now,ev.Events.Single(x=>x.ID==e.ID)),"Event readback mismatch");if(e.ID is 1111 or 9100941){Check(now.Instructions.Count(Removal)==1,"Missing guard cleanup");if(!updating)now.Instructions.RemoveAll(Removal);}Check(Cages.Same(e,now),"Unrelated event changed");}
  HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(dir,"texts"),allRewards:true,onlyIds:[Metal.Owner],inputMod:mod);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{owner=Metal.Owner,effect=EffectId,guardStaminaMultiplier=Multiplier,poiseDamageMultiplier=PoiseMultiplier,effectSeconds=2,stateInfo=158,inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),allOtherParameterRowsPreserved=true,updatedExistingEffect=updating,onlyTwoCleanupInstructionsAdded=!updating,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Metal guard effect and lifecycle staged; all unrelated rows and events preserved");
 }
}
