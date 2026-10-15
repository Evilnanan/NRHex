using SoulsFormats;
using System.Text.Json;

static partial class ContractHeart {
 public const int NativeHitReference=8600200; // FP Restoration upon Landing Attacks.
 public static void AuditNativeHit(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin");string before=Hash(input);
  var native=parse("SpEffectParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var binder=SFUtil.DecryptNightreignRegulation(input);var installed=parse("SpEffectParam",binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
  var reference=native.Rows.Single(r=>r.ID==NativeHitReference);var contract=installed.Rows.Single(r=>r.ID==Contract);
  var equivalent=new PARAM.Row(contract){ID=reference.ID};
  foreach(string field in new[]{"iconId","atkOccurrenceSpEffectId","saveCategory","spEffectTextId_1"})Set(equivalent,field,reference[field].Value);
  Check(CageAI.Same(reference,equivalent),"Installed Contract differs from native FP on-hit filters");
  var tiers=new List<object>();
  for(int tier=0;tier<3;tier++){
   var hp=native.Rows.Single(r=>r.ID==8600100+tier);var fp=native.Rows.Single(r=>r.ID==NativeHitReference+tier);var normalized=new PARAM.Row(hp){ID=fp.ID};Set(normalized,"atkOccurrenceSpEffectId",fp["atkOccurrenceSpEffectId"].Value);
   Check(CageAI.Same(fp,normalized),"Native HP and FP on-hit criteria differ");
   tiers.Add(new{hpOwner=hp.ID,fpOwner=fp.ID,onlyDifference="atkOccurrenceSpEffectId"});
  }
  string[] fields={"stateInfo","wepParamChange","magParamChange","miracleParamChange","shamanParamChange","magicSubCategoryChange1","magicSubCategoryChange2","magicSubCategoryChange3","triggerOnWepType","triggerAttachedWeapon","isIgnoreNoDamage"};
  Check(Hash(input)==before,"Read-only audit changed regulation");
  string dir=Path.Combine(root,".work/pact-fp-reference");Directory.CreateDirectory(dir);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{nativeReference=NativeHitReference,nativeName="FP Restoration upon Landing Attacks",regulationSha256=before,tiers,filters=fields.ToDictionary(f=>f,f=>reference[f].Value),installedContractNativeFiltersMatch=true,hpAndFpTriggerCriteriaIdentical=true,assetsUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  ExportChecks(root,parse);
  Console.WriteLine("PASS: installed Contract matches native FP-on-hit filters; all three HP/FP tiers have identical hit criteria; assets unchanged");
 }
 public static void StageNativeHit(string root,BND4 original,Func<string,Memory<byte>,PARAM> parse){
  string input=Path.Combine(root,"dist/mod/regulation.bin"),dir=Path.Combine(root,".work/pact-native-hit/staged");Directory.CreateDirectory(dir);
  var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var member=binder.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
  var old=parse("SpEffectParam",member.Bytes);var sp=parse("SpEffectParam",member.Bytes);
  var native=parse("SpEffectParam",original.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes).Rows.Single(r=>r.ID==NativeHitReference);
  var contract=sp.Rows.Single(r=>r.ID==Contract);
  Check(Convert.ToInt32(contract["stateInfo"].Value)==2103&&Convert.ToInt32(contract["atkOccurrenceSpEffectId"].Value)==SelfDamage,"Contract on-hit route changed");
  string[] fields={"magParamChange","miracleParamChange","shamanParamChange"};
  foreach(string field in fields){Check(Convert.ToInt32(native[field].Value)==0,"Native spell applicability changed");Set(contract,field,native[field].Value);}
  // Normalize only identity/display/persistence before comparing every field.
  var normalized=new PARAM.Row(contract){ID=native.ID};
  foreach(string field in new[]{"iconId","atkOccurrenceSpEffectId","saveCategory","spEffectTextId_1"})Set(normalized,field,native[field].Value);
  Check(CageAI.Same(native,normalized),"Contract native hit filters differ");
  member.Bytes=sp.Write();string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);
  var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==binder.Version&&actual.Compression==binder.Compression&&actual.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){var wanted=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback failed");if(f.ID!=member.ID)Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");}
  var final=parse("SpEffectParam",actual.Files.Single(f=>f.ID==member.ID).Bytes);Check(final.Rows.Count==old.Rows.Count,"Effect row count changed");
  foreach(var pair in old.Rows.Zip(final.Rows)){
   var restored=new PARAM.Row(pair.Second);
   if(restored.ID==Contract)foreach(string field in fields)Set(restored,field,pair.First[field].Value);
   Check(CageAI.Same(pair.First,restored),"Unrelated row/field changed");
  }
  ExportChecks(root,parse,output);
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),changedEffect=Contract,changedFields=fields,nativeReference=NativeHitReference,allNativeHitFiltersMatch=true,otherFieldsRowsAndTablesPreserved=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native on-hit spell exclusions restored; only three applicability bits changed, all native hit filters and other assets preserved");
 }
}
