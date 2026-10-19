using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;
using SoulsFormats;
var root=Path.GetFullPath(args[0]);
var drive=Path.GetPathRoot(root)!;
var smith=Environment.GetEnvironmentVariable("NRHEX_SMITHBOX")??Path.Combine(drive,"Program Files/Smithbox");
var game=Environment.GetEnvironmentVariable("NRHEX_GAME")??Path.Combine(drive,"Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game");
AppDomain.CurrentDomain.AssemblyResolve+=(_,a)=>{var p=Path.Combine(smith,new AssemblyName(a.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};
NativeLibrary.SetDllImportResolver(typeof(BND4).Assembly,(n,a,p)=>{var f=Path.Combine(game,n.EndsWith(".dll")?n:n+".dll");return File.Exists(f)?NativeLibrary.Load(f):IntPtr.Zero;});
Directory.SetCurrentDirectory(game);
if(args.Contains("--storm-audio-probe")){StormAudio.Probe(root,smith,game);return;}
// Text-only work does not require parsing parameters or rebuilding gameplay assets.
if(args.Contains("--stage-translations")){
 HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(root,".work/texts"),allRewards:true);return;
}
if(args.Contains("--snack-text-stage")){
 HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(root,".work/snack-price/texts"),allRewards:true,onlyIds:new[]{Snack.Owner});return;
}
if(args.Contains("--combat-protection-texts")){
 HexTexts.Stage(root,HexCatalog.Load(root),Path.Combine(root,"work/combat-protection/texts"),allRewards:true,onlyIds:new[]{95003000});return;
}
if(args.Contains("--check-descriptions")){
 HexTexts.Verify(root,HexCatalog.Load(root));return;
}
if(args.Contains("--regulation-compression")){RegulationCompression.Normalize(root);return;}
if(args.Contains("--cage-presentation")){CagePresentation.Stage(root);return;}
if(args.Contains("--cage-demand")){CageDemand.Stage(root);return;}
if(args.Contains("--cage-music-probe")){CageMusic.Probe(root);return;}
if(args.Contains("--cage-music-stage")){CageMusic.Stage(root);return;}
if(args.Contains("--cage-music-shown-fix")){CageMusic.Stage(root,true);return;}
if(args.Contains("--cage-body-fog")){CagePresentation.StageBodyFog(root);return;}

if(args.Contains("--cage-network-selector")){CageNetwork.Stage(root);return;}
if(args.Contains("--cage-usage")){CageUsage.Stage(root);return;}
if(args.Contains("--cage-without-prelude")){CageUsage.StageWithoutPrelude(root);return;}
if(args.Contains("--audit-peer-lifecycle")){Cages.StagePeerLifecycle(root);return;}
if(args.Contains("--cage-peer-generation-only")){Cages.StagePeerGeneration(root);return;}
if(args.Contains("--cage-group-metadata")){CageGroups.ExportMetadata(root);return;}
if(args.Contains("--cage-group-research")){
 CageGroups.Export(root);return;
}
if(args.Contains("--cage-birth-only")){
 Cages.StageBirthPatch(root);
 Console.WriteLine("PASS: staged four Tree Crucible birth events; only its forced intro animation removed; dist untouched");return;
}
if(args.Contains("--cage-fog-only")){
 Cages.StageFogPatch(root);
 Console.WriteLine("PASS: staged all 336 single/group/mount birth events; only fresh/ongoing snapshot control flow changed; dist untouched");return;
}
if(args.Contains("--cage-selector-only")){
 Cages.StageSelectorPatch(root);
 Console.WriteLine("PASS: staged four gaol selectors; only selectors changed, all flag gotos point forward; dist untouched");return;
}
string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
void Require(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
var source=Path.Combine(game,"regulation.bin");
Require(Hash(source)=="876A3CA279A4561D0C69F81FE5E510C75C59AB9A8A201694F8CF4A42C91E0268","Unsupported regulation hash");
var bnd=SFUtil.DecryptNightreignRegulation(source);
Require(bnd.Version=="10350000","Unsupported regulation version");
var original=bnd.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
var defs=new Dictionary<string,PARAMDEF>();
PARAM Parse(string name,Memory<byte> bytes){
 var p=PARAM.Read(bytes);
 if(!defs.TryGetValue(name,out var d)){
  var dn=name.StartsWith("ItemLot")?"ItemLotParam":name.StartsWith("AtkParam_")?"AtkParam":name.StartsWith("BehaviorParam_")?"BehaviorParam":name;
  var xml=XDocument.Load(Path.Combine(smith,"Assets/PARAM/NR/Defs",dn+".xml"));
  foreach(var field in xml.Root!.Element("Fields")!.Elements("Field").ToArray()){
   ulong first=(ulong?)field.Attribute("FirstVersion")??0;
   ulong removed=(ulong?)field.Attribute("RemovedVersion")??ulong.MaxValue;
   if(first>10350000||removed<=10350000)field.Remove();
   else{field.Attribute("FirstVersion")?.Remove();field.Attribute("RemovedVersion")?.Remove();}
  }
  var dp=Path.Combine(root,".work/defs",dn+".xml");Directory.CreateDirectory(Path.GetDirectoryName(dp)!);xml.Save(dp);
  d=PARAMDEF.XmlDeserialize(dp);defs[name]=d;
 }
 p.ApplyParamdef(d);return p;
}
if(args.Contains("--momentum-probe")){Momentum.Probe(root,bnd,Parse);return;}
if(args.Contains("--momentum-stage")){Momentum.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--momentum-check")){Momentum.Verify(root,bnd,Parse);return;}
if(args.Contains("--momentum-delivery-stage")){Momentum.FixDelivery(root,bnd,Parse);return;}
if(args.Contains("--snack-stage")){Snack.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--snack-check")){Snack.Verify(root,bnd,Parse);return;}
if(args.Contains("--snack-rarity-stage")){Snack.StageRarity(root,bnd,Parse);return;}
if(args.Contains("--echo-cast-stage")){EchoCast.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--echo-cast-cost-stage")){EchoCast.CostResources(root,Parse,HexCatalog.Load(root),true);return;}
if(args.Contains("--echo-cast-cost-check")){EchoCast.CostResources(root,Parse,HexCatalog.Load(root),false);return;}
if(args.Contains("--echo-cast-claw-probe")){EchoCast.ClawProbe(root,bnd,Parse);return;}
if(args.Contains("--echo-cast-areas-stage")){EchoCast.AreaResources(root,Parse,true);return;}
if(args.Contains("--echo-cast-areas-check")){EchoCast.AreaResources(root,Parse,false);return;}
if(args.Contains("--ambulance-stage")){Ambulance.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--reward-template-stage")){RewardTemplate.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--potential-fission-stage")){PotentialFission.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--demons-call-catalog")){DemonsCall.Catalog(bnd,Parse,Path.Combine(root,"work/demons-call/demons_data.rs"));return;}
if(args.Contains("--demons-call-stage")){DemonsCall.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--ability-icons-stage")){AbilityIcons.Stage(root,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--erosion-stage")){Erosion.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--erosion-tuning-stage")){Erosion.Tune(root,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--death-dance-stage")){DeathDance.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--death-dance-probe")){DeathDance.Probe(root,bnd,Parse);return;}
if(args.Contains("--death-dance-flask-check")){DeathDance.FlaskCheck(root,bnd,Parse);return;}
if(args.Contains("--sword-regain-stage")){SwordRegain.Stage(root,bnd,Parse);return;}
if(args.Contains("--sword-regain-probe")){SwordRegain.Export(root,bnd,Parse);return;}
if(args.Contains("--sword-charged-probe")){SwordCharged.Probe(root,bnd,Parse);return;}
if(args.Contains("--archmage-data")){Archmage.WriteData(root,bnd,Parse);return;}
if(args.Contains("--essence-reaver-probe")){EssenceReaver.Probe(root,bnd,Parse);return;}
if(args.Contains("--essence-reaver-single")){EssenceReaver.StageSingle(root,Parse);return;}
if(args.Contains("--essence-reaver-check")){EssenceReaver.ExportChecks(root,Parse);return;}
if(args.Contains("--essence-reaver-hide-icon")){EssenceReaver.StageHiddenIcon(root,Parse);return;}
if(args.Contains("--contract-heart-probe")){ContractHeart.Probe(root,bnd,Parse);return;}
if(args.Contains("--contract-heart-check")){ContractHeart.ExportChecks(root,Parse);return;}
if(args.Contains("--contract-damage-marker")){ContractHeart.StageDamageMarker(root,Parse);return;}
if(args.Contains("--contract-native-hit")){ContractHeart.StageNativeHit(root,bnd,Parse);return;}
if(args.Contains("--contract-hit-scope")){ContractHeart.AuditNativeHit(root,bnd,Parse);return;}
if(args.Contains("--reward-cycle")){RewardCycle.Stage(root,Parse);return;}
if(args.Contains("--reward-reroll")){RewardReroll.Stage(root,Parse);return;}
if(args.Contains("--cat-wall-check")){CatWall.Probe(root,bnd,Parse);return;}
if(args.Contains("--heart-two-returns")){ContractHeart.StageTwoReturns(root,Parse);return;}
if(args.Contains("--cage-hp-probe")){CageHealth.Probe(root,Parse);return;}
if(args.Contains("--cage-hp-sync")){CageCoop.Stage(root,Parse);return;}
if(args.Contains("--cage-hp-check")){CageCoop.Export(root,Parse);return;}
if(args.Contains("--parry-counter-probe")){Parry.ProbeCounterCost(root,bnd,Parse);return;}
if(args.Contains("--archmage-secondary")){Archmage.StageSecondary(root,Parse);return;}
if(args.Contains("--archmage-probe")){Archmage.Probe(root,bnd,Parse);return;}
if(args.Contains("--cage-group-npcs")){
 CageGroups.ExportNpcs(root,Parse("NpcParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="NpcParam.param").Bytes),Hash(source));
 return;
}
if(args.Contains("--spirit-placement-probe")){SpiritPlacement.Probe(root,Parse);return;}
if(args.Contains("--storm-vfx-probe")){StormVfx.Probe(root,bnd,Parse);return;}
if(args.Contains("--sword-vfx-probe")){SwordVfx.Probe(root,bnd,Parse);return;}
if(args.Contains("--sword-model-probe")){SwordVfx.ModelProbe(root,smith,game);return;}
if(args.Contains("--sword-vfx-stage")){SwordVfx.Stage(root,Parse);return;}
if(args.Contains("--sword-storm-probe")){SwordVfx.StormProbe(root);return;}
if(args.Contains("--sword-fxr-stage")){SwordVfx.StageFxr(root);return;}
if(args.Contains("--storm-archive-probe")){StormVfx.ArchiveProbe(smith);return;}
if(args.Contains("--storm-extract")){StormVfx.Extract(root,smith,game);return;}
if(args.Contains("--storm-resources")){StormVfx.Resources(root);return;}
if(args.Contains("--storm-vfx-stage")){StormVfx.Stage(root);return;}
if(args.Contains("--devils-gaze-resources")){DevilsGaze.Resources(root,smith,game);return;}
if(args.Contains("--devils-gaze-remnant")){DevilsGazeRemnant.Stage(root,bnd,Parse);return;}
if(args.Contains("--slingshot-tune")){Slingshot.Tune(root,Parse);return;}
if(args.Contains("--devils-gaze-tune")){DevilsGaze.Tune(root,Parse);return;}
if(args.Contains("--devils-gaze-hitfix")){DevilsGaze.StageHitFix(root,Parse);return;}
if(args.Contains("--devils-gaze-probe")){DevilsGaze.Probe(root,bnd,Parse);return;}
if(args.Contains("--devils-gaze-resources-stage")){DevilsGaze.StageResources(root);return;}
if(args.Contains("--balloon-attribute-probe")){BalloonAttribute.Probe(root,Parse);return;}
if(args.Contains("--balloon-attribute-stage")){BalloonAttribute.Stage(root,Parse);return;}
if(args.Contains("--ability-buffs-probe")){AbilityBuffs.Probe(root,Parse);return;}
if(args.Contains("--red-envelope-rewards")){RedEnvelope.StageRewards(root,bnd,Parse);return;}
if(args.Contains("--red-envelope-rewards-check")){RedEnvelope.CheckRewards(root,bnd,Parse);return;}
if(args.Contains("--red-envelope-probe")){RedEnvelope.Probe(root,bnd,Parse);return;}
if(args.Contains("--red-envelope-only")){RedEnvelope.Stage(root,Parse);return;}
if(args.Contains("--ultimate-vfx-probe")){UltimateRefreshVfx.Probe(root,bnd,Parse);return;}
if(args.Contains("--ultimate-vfx-only")){UltimateRefreshVfx.Stage(root,Parse);return;}
if(args.Contains("--escape-duchess-probe")){EscapeVfx.Probe(root,bnd,Parse);return;}
if(args.Contains("--escape-resource-probe")){EscapeVfx.ResourceProbe(root);return;}
if(args.Contains("--escape-audio-probe")){EscapeVfx.AudioProbe(root,game);return;}
if(args.Contains("--escape-duchess-only")){EscapeVfx.Stage(root,Parse);return;}
if(args.Contains("--coop-01749-probe")){CoopAudit.Probe(root,Parse,Parse("NpcParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="NpcParam.param").Bytes),Hash(source));return;}
if(args.Contains("--coop-01749-stage")){CoopAudit.Stage(root,Parse);return;}
if(args.Contains("--spirit-corpse-cue-only")){SpiritPlacement.Stage(root,Parse);return;}
if(args.Contains("--spirit-cue-visibility-only")){SpiritPlacement.StageVisibility(root,Parse);return;}
var names=new[]{"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy","ItemLotParam_map","Bullet","AtkParam_Pc","AtkParam_Npc","BehaviorParam_PC","SpEffectVfxParam","NpcParam","NpcThinkParam","MultiPlayCorrectionParam","ChaosMatchingCorrectParam","ChaosMatchingMutationCategoryParam","ChaosMatchingMutationEnemyTableParam","SpEffectSetParam","LotBaseMapPatternFlag"};
var files=names.ToDictionary(n=>n,n=>bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param"));
var pars=files.ToDictionary(k=>k.Key,k=>Parse(k.Key,k.Value.Bytes));
if(args.Contains("--spirit-rework-probe")){SpiritRework.Probe(root,bnd,Parse);return;}
if(args.Contains("--angel-chant-probe")){AngelChant.Probe(root,bnd,Parse);return;}
if(args.Contains("--new-hexes-probe")){
 NewHexes.Probe(root,bnd,Parse,Hash(source));return;
}
if(args.Contains("--stance-break-probe")){
 var output=Path.Combine(root,".work/fixes-01731");Directory.CreateDirectory(output);
 File.WriteAllText(Path.Combine(output,"stance-effects.json"),JsonSerializer.Serialize(pars["SpEffectParam"].Rows.Where(r=>new[]{36,50,51}.Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only native stance-break effects exported");return;
}
if(args.Contains("--crystal-rebound-probe")){
 var weapons=Parse("EquipParamWeapon",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="EquipParamWeapon.param").Bytes);
 string[] fields={"wepType","guardBaseRepel","attackBaseRepel","guardLevel"};
 var output=Path.Combine(root,".work/crystal-rebound-01730");Directory.CreateDirectory(output);
 File.WriteAllText(Path.Combine(output,"native-weapons.json"),JsonSerializer.Serialize(weapons.Rows.Where(r=>Convert.ToInt32(r["wepType"].Value)>=50&&Convert.ToInt32(r["wepType"].Value)<=70).Select(r=>new{r.ID,fields=fields.ToDictionary(n=>n,n=>r[n].Value)}),new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only native shield hardness exported");return;
}
if(args.Contains("--hud-order-probe")){
 var input=Path.Combine(root,"dist/mod/regulation.bin");
 var current=SFUtil.DecryptNightreignRegulation(input);
 var hudEffects=Parse("SpEffectParam",current.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
 string[] fields={"iconId","secondaryIconId","effectEndurance","spCategory","categoryPriority","saveCategory","stateInfo","displayIconForPermamentEffect","dispDartDebuffStacks","spEffectTextId_1","replaceSpEffectId","cycleOccurrenceSpEffectId"};
 object DumpHud(PARAM.Row r)=>new{r.ID,fields=fields.ToDictionary(n=>n,n=>r[n].Value)};
 var output=Path.Combine(root,".work/hud-order-01728");Directory.CreateDirectory(output);
 File.WriteAllText(Path.Combine(output,"parameters.json"),JsonSerializer.Serialize(new{
  sourceRegulationSha256=Hash(source),installedRegulationSha256=Hash(input),
  native=pars["SpEffectParam"].Rows.Where(r=>new[]{9700,9701,700600,8610702,1681,703500,5300}.Contains(r.ID)).Select(DumpHud),
  custom=hudEffects.Rows.Where(r=>r.ID>=95000100&&r.ID<95006000).Select(DumpHud)
 },new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only current HUD icon parameters exported; no package writes");return;
}
if(args.Contains("--coop-fixes-only")){
 string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(root,".work/fixes-01727/staged/mod");Directory.CreateDirectory(output);
 var installed=SFUtil.DecryptNightreignRegulation(input);Require(installed.Version==bnd.Version,"Fix regulation version mismatch");
 var before=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var tables=new Dictionary<string,PARAM>(pars);
 foreach(var name in new[]{"NpcParam","NpcThinkParam","SpEffectParam"})tables[name]=Parse(name,installed.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
 var stanceEffects=tables["SpEffectParam"];var originalEffects=Parse("SpEffectParam",installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
 foreach(int id in new[]{6999804,6999805}){
  Require(!stanceEffects.Rows.Any(r=>r.ID==id),"Fix must start from unpatched 0.17.26 package");
  var row=new PARAM.Row(stanceEffects.Rows.Single(r=>r.ID==6999801)){ID=id};
  row["effectEndurance"].Value=Convert.ChangeType(id==6999804?-1f:.001f,row["effectEndurance"].Value.GetType());
  row["spCategory"].Value=Convert.ChangeType(20,row["spCategory"].Value.GetType());stanceEffects.Rows.Add(row);
 }
 var effectFile=installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");effectFile.Bytes=stanceEffects.Write();
 string destination=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(destination,installed,installed.Compression);var reopened=SFUtil.DecryptNightreignRegulation(destination);
 Require(reopened.Version==installed.Version&&reopened.Files.Count==installed.Files.Count&&reopened.Files.Zip(installed.Files).All(p=>p.First.ID==p.Second.ID&&p.First.Name==p.Second.Name&&p.First.Flags==p.Second.Flags&&p.First.Bytes.Span.SequenceEqual(p.Second.Bytes.Span)),"Fix binder round-trip failed");
 foreach(var file in reopened.Files.Where(f=>f.ID!=effectFile.ID))Require(file.Bytes.Span.SequenceEqual(before[file.ID]),"Fix changed unrelated parameter table");
 var actual=Parse("SpEffectParam",reopened.Files.Single(f=>f.ID==effectFile.ID).Bytes);Require(actual.Rows.Count==originalEffects.Rows.Count+2,"Unexpected added effects");
 foreach(var row in originalEffects.Rows)Require(CageAI.Same(row,actual.Rows.Single(r=>r.ID==row.ID)),"Fix changed preexisting effect");
 foreach(var id in new[]{6999804,6999805})Require(CageAI.Same(stanceEffects.Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"Stance effect readback failed");
 // Check the actual native scaling contract before adding it to cloned actors.
 var native=EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx")).Events.Single(e=>e.ID==90015022);
 Require(native.Instructions.Count==6&&native.Instructions[0].Bank==2000&&native.Instructions[0].ID==2&&native.Instructions[3].Bank==2009&&native.Instructions[3].ID==4&&native.Instructions[4].Bank==2004&&native.Instructions[4].ID==8&&BitConverter.ToInt32(native.Instructions[4].ArgData,4)==7999&&native.Parameters.Any(p=>p.InstructionIndex==3&&p.TargetStartByte==0&&p.SourceStartByte==0&&p.ByteCount==4),"Native multiplayer activation contract changed");
 var nativeBoss=EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx")).Events.Single(e=>e.ID==9005800);
 Require(nativeBoss.Instructions[43].Bank==1003&&nativeBoss.Instructions[43].ID==13&&nativeBoss.Instructions[43].ArgData[1]==1&&nativeBoss.Instructions[45].Bank==2004&&nativeBoss.Instructions[45].ID==28&&BitConverter.ToInt32(nativeBoss.Instructions[45].ArgData,4)==8192,"Native host boss authority contract changed");
 var bosses=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
 var cooperative=Parse("MultiPlayCorrectionParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="MultiPlayCorrectionParam.param").Bytes);
 var actors=Enumerable.Range(0,bosses.Length).Select(i=>(index:i,source:bosses[i].npc)).Concat(CageEncounters.Load(root).groups.SelectMany(g=>g.members).Select(m=>(index:m.index,source:m.npc))).ToArray();
 var corrections=actors.Select(actor=>{
  var npc=tables["NpcParam"].Rows.Single(r=>r.ID==95300000+actor.index);int id=Convert.ToInt32(npc["multiPlayCorrectionParamId"].Value);
  Require(id==Convert.ToInt32(pars["NpcParam"].Rows.Single(r=>r.ID==actor.source)["multiPlayCorrectionParamId"].Value),"Cage multiplayer correction reference changed");
  var correction=cooperative.Rows.SingleOrDefault(r=>r.ID==id);
  var clients=new[]{1,2,3}.Select(count=>{
   int effect=correction==null?-1:Convert.ToInt32(correction[$"client{count}SpEffectId"].Value);var row=tables["SpEffectParam"].Rows.SingleOrDefault(r=>r.ID==effect);
   return new{count,effect,hpMultiplier=row==null?(float?)null:Convert.ToSingle(row["maxHpRate"].Value),attackMultiplier=row==null?(float?)null:Convert.ToSingle(row["physicsAttackPowerRate"].Value)};
  }).ToArray();return new{actor.index,actor.source,npc=npc.ID,correction=id,clients};
 }).ToArray();
 File.WriteAllText(Path.Combine(root,".work/fixes-01727/parameter-patch.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(destination),addedEffects=new[]{6999804,6999805},originalRowsAndOtherTablesPreserved=true,nativeMultiplayerEvent=Cages.Dump(native),corrections},new JsonSerializerOptions{WriteIndented=true}));
 Directory.CreateDirectory(Path.Combine(output,"event"));File.Copy(Path.Combine(root,"dist/mod/event/common.emevd.dcx"),Path.Combine(output,"event/common.emevd.dcx"),true);
 Cages.Build(root,bosses,tables,output);
 // Changing events must not move actors, rewards, riding or AI references.
 foreach(var path in Directory.GetFiles(Path.Combine(output,"map/mapstudio"),"*.msb.dcx"))Require(Hash(path)==Hash(Path.Combine(root,"dist/mod/map/mapstudio",Path.GetFileName(path))),"Fix unexpectedly changed a map");
 var currentCommon=EMEVD.Read(Path.Combine(root,"dist/mod/event/common_func.emevd.dcx"));var fixedCommon=EMEVD.Read(Path.Combine(output,"event/common_func.emevd.dcx"));
 Require(currentCommon.Events.Count==fixedCommon.Events.Count&&currentCommon.Events.Where(e=>e.ID!=Cages.OpenEvent).All(e=>Cages.Same(e,fixedCommon.Events.Single(v=>v.ID==e.ID))),"Fix changed unrelated common function");
 Require(Hash(Path.Combine(output,"event/common.emevd.dcx"))==Hash(Path.Combine(root,"dist/mod/event/common.emevd.dcx")),"Fix changed common events");
 foreach(int block in new[]{50,60,70,80}){
  string map=$"m46_{block:00}_00_00";long prefix=(4600L+block)*10000;
  var current=EMEVD.Read(Path.Combine(root,"dist/mod/event",map+".emevd.dcx"));var fixedEvents=EMEVD.Read(Path.Combine(output,"event",map+".emevd.dcx"));
  bool Intended(long id)=>id==prefix+9700||id>=prefix+9710&&id<prefix+9760||id>=prefix+9900&&id<prefix+9984||id>=prefix+9137&&id<prefix+9184;
  Require(current.Events.Select(e=>e.ID).SequenceEqual(fixedEvents.Events.Select(e=>e.ID)),"Fix changed event order or identities");
  Require(current.Events.Where(e=>!Intended(e.ID)).All(e=>Cages.Same(e,fixedEvents.Events.Single(v=>v.ID==e.ID))),"Fix changed unrelated map event or constructor");
  Require(current.Format==fixedEvents.Format&&current.Compression==fixedEvents.Compression&&current.StringData.SequenceEqual(fixedEvents.StringData)&&current.LinkedFileOffsets.SequenceEqual(fixedEvents.LinkedFileOffsets),"Fix changed map event metadata");
 }
 Console.WriteLine("PASS: staged host-authoritative cages, native multiplayer activation and two hidden stance markers; all maps and original parameter rows preserved");return;
}
if(args.Contains("--cage-groups-only")){
 string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(root,".work/cage-groups/staged");Directory.CreateDirectory(output);
 var installed=SFUtil.DecryptNightreignRegulation(input);Require(installed.Version==bnd.Version,"Group regulation version mismatch");
 var before=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());var tables=new Dictionary<string,PARAM>(pars);
 foreach(var name in new[]{"NpcParam","NpcThinkParam","SpEffectParam"})tables[name]=Parse(name,installed.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
 var catalog=CageEncounters.Load(root);
 foreach(var member in catalog.groups.SelectMany(g=>g.members))foreach(var name in new[]{"NpcParam","NpcThinkParam"}){
  int id=name=="NpcParam"?member.npc:member.think;
  Require(CageAI.Same(pars[name].Rows.Single(r=>r.ID==id),tables[name].Rows.Single(r=>r.ID==id)),"Original group source parameter was modified");
 }
 CageEncounters.AddNpcs(root,tables);
 foreach(var name in new[]{"NpcParam","NpcThinkParam"}){
  var file=installed.Files.Single(f=>Path.GetFileName(f.Name)==name+".param");var old=Parse(name,before[file.ID]);file.Bytes=tables[name].Write();var actual=Parse(name,file.Bytes);
  var newIds=catalog.groups.SelectMany(g=>g.members).Select(m=>95300000+m.index).ToHashSet();
  Require(!old.Rows.Any(r=>newIds.Contains(r.ID))&&actual.Rows.Count==old.Rows.Count+newIds.Count,"Group patch requires a single-boss parameter base");
  foreach(var row in old.Rows)Require(CageAI.Same(row,actual.Rows.Single(r=>r.ID==row.ID)),"Group patch changed preexisting parameter row");
  foreach(int id in newIds)Require(CageAI.Same(tables[name].Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"Group parameter readback failed");
  tables[name]=actual;
 }
 string target=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,installed,installed.Compression);var reopened=SFUtil.DecryptNightreignRegulation(target);
 Require(reopened.Version==installed.Version&&reopened.Files.Count==installed.Files.Count&&reopened.Files.Zip(installed.Files).All(p=>p.First.ID==p.Second.ID&&p.First.Name==p.Second.Name&&p.First.Flags==p.Second.Flags&&p.First.Bytes.Span.SequenceEqual(p.Second.Bytes.Span)),"Group binder round-trip failed");
 foreach(var file in installed.Files.Where(f=>Path.GetFileName(f.Name) is not ("NpcParam.param" or "NpcThinkParam.param")))Require(file.Bytes.Span.SequenceEqual(before[file.ID]),"Group patch changed unrelated table");
 File.WriteAllText(Path.Combine(root,".work/cage-groups/parameter-patch.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input).ToLowerInvariant(),outputSha256=Hash(target).ToLowerInvariant(),addedActors=catalog.actorCount-37,originalRowsPreserved=true,unrelatedTablesPreserved=true},new JsonSerializerOptions{WriteIndented=true}));
 CageEncounters.Build(root,tables,output);Console.WriteLine("PASS: staged 13 groups, 84 independent actors, native group loot and riding relations; dist untouched");return;
}
if(args.Contains("--cage-ai-probe")){
 var think=Parse("NpcThinkParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="NpcThinkParam.param").Bytes);
 var bosses=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
 var keys=new[]{"logicId","battleGoalID","eye_dist","searchEye_dist","ear_dist","nose_dist","BattleStartDist","isUpdateBattleSight","battleEye_updateDist","maxBackhomeDist","backhomeDist","backhomeBattleDist","SightTargetForgetTime","MemoryTargetForgetTime"};
 var output=Path.Combine(root,".work/cage-ai");Directory.CreateDirectory(output);
 var records=bosses.Select(b=>new{b.name,b.map,b.think,fields=keys.ToDictionary(k=>k,k=>think.Rows.Single(r=>r.ID==b.think)[k].Value),variants=think.Rows.Where(r=>r.ID/1000==b.think/1000).Select(r=>new{r.ID,fields=keys.ToDictionary(k=>k,k=>r[k].Value)})});
 File.WriteAllText(Path.Combine(output,"native.json"),JsonSerializer.Serialize(new{sourceRegulationSha256=Hash(source),records},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only cage AI ranges and same-archetype native variants exported");return;
}
if(args.Contains("--cage-ai-only")){
 string input=Path.Combine(root,"dist/mod/regulation.bin"),output=Path.Combine(root,".work/cage-ai/staged");Directory.CreateDirectory(output);
 var installed=SFUtil.DecryptNightreignRegulation(input);Require(installed.Version==bnd.Version,"Cage AI regulation version mismatch");
 var before=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var member=installed.Files.Single(f=>Path.GetFileName(f.Name)=="NpcThinkParam.param");
 var think=Parse("NpcThinkParam",member.Bytes);var oldThink=Parse("NpcThinkParam",member.Bytes);
 var bosses=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
 CageAI.Add(root,think,bosses);member.Bytes=think.Write();CageAI.Validate(oldThink,think,Parse("NpcThinkParam",member.Bytes));
 string target=Path.Combine(output,"regulation.bin");SFUtil.EncryptNightreignRegulation(target,installed,installed.Compression);
 var reopened=SFUtil.DecryptNightreignRegulation(target);
 Require(reopened.Version==installed.Version&&reopened.Files.Count==installed.Files.Count,"Cage AI binder metadata changed");
 foreach(var pair in reopened.Files.Zip(installed.Files))Require(pair.First.ID==pair.Second.ID&&pair.First.Name==pair.Second.Name&&pair.First.Flags==pair.Second.Flags&&pair.First.Bytes.Span.SequenceEqual(pair.First.ID==member.ID?member.Bytes.Span:before[pair.First.ID].AsSpan()),"Cage AI changed unrelated parameter/binder content");
 var actual=Parse("NpcThinkParam",reopened.Files.Single(f=>f.ID==member.ID).Bytes);CageAI.Validate(oldThink,think,actual);
 var maps=CageAI.StageMaps(root,bosses);
 File.WriteAllText(Path.Combine(root,".work/cage-ai/patch.json"),JsonSerializer.Serialize(new{installed=false,inputRegulationSha256=Hash(input).ToLowerInvariant(),outputRegulationSha256=Hash(target).ToLowerInvariant(),limits=CageAI.Limits,rows=CageAI.Report(pars["NpcThinkParam"],actual,bosses),maps,originalThinkRowsPreserved=true,unrelatedParameterTablesPreserved=true,all37CombatScriptsPreserved=true},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("PASS: staged 37 independent cage Think rows and 148 map references; original AI/combat scripts/other tables/maps preserved; dist untouched");return;
}
if(args.Contains("--regen-balance-only")){
 var output=Path.Combine(root,".work/regen-balance");Directory.CreateDirectory(output);
 var destination=Path.Combine(root,"dist/mod/regulation.bin");var inputHash=Hash(destination);
 var installed=SFUtil.DecryptNightreignRegulation(destination);
 Require(installed.Version==bnd.Version,"Regeneration regulation version mismatch");
 var before=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var member=installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
 var table=Parse("SpEffectParam",member.Bytes);var pulse=table.Rows.Single(r=>r.ID==95000601);
 var oldHp=Convert.ToInt32(pulse["changeHpPoint"].Value);var oldFp=Convert.ToInt32(pulse["changeMpPoint"].Value);
 Require(((oldHp is -15 or -20)&&oldFp==5||oldHp==-16&&oldFp==4)&&Convert.ToInt32(pulse["soul"].Value)==0&&Convert.ToSingle(pulse["effectEndurance"].Value)==0f,"Unexpected regeneration pulse");
 pulse["changeHpPoint"].Value=Convert.ChangeType(-16,pulse["changeHpPoint"].Value.GetType());
 pulse["changeMpPoint"].Value=Convert.ChangeType(4,pulse["changeMpPoint"].Value.GetType());
 member.Bytes=table.Write();var checkedTable=Parse("SpEffectParam",member.Bytes);
 var oldTable=Parse("SpEffectParam",before[member.ID]);
 Require(checkedTable.Rows.Count==oldTable.Rows.Count,"Regeneration row count changed");
 foreach(var row in oldTable.Rows){var actual=checkedTable.Rows.Single(r=>r.ID==row.ID);
  foreach(var cell in row.Cells.Where(c=>c!=null)){
   var expected=row.ID==95000601&&cell.Def.InternalName=="changeHpPoint"?(object)(-16):row.ID==95000601&&cell.Def.InternalName=="changeMpPoint"?(object)4:cell.Value;
   Require(JsonSerializer.Serialize(actual[cell.Def.InternalName].Value)==JsonSerializer.Serialize(expected),"Unrelated effect field changed: "+row.ID+" "+cell.Def.InternalName);
  }
 }
 var temporary=Path.Combine(output,"patched-regulation.bin");
 SFUtil.EncryptNightreignRegulation(temporary,installed,installed.Compression);
 var reopened=SFUtil.DecryptNightreignRegulation(temporary);
 Require(reopened.Files.Count==installed.Files.Count,"Regeneration binder count changed");
 foreach(var file in reopened.Files)Require(file.Bytes.Span.SequenceEqual(file.ID==member.ID?member.Bytes.Span:before[file.ID].AsSpan()),"Regeneration binder round-trip changed a table");
 var finalPulse=Parse("SpEffectParam",reopened.Files.Single(f=>f.ID==member.ID).Bytes).Rows.Single(r=>r.ID==95000601);
 Require(Convert.ToInt32(finalPulse["changeHpPoint"].Value)==-16&&Convert.ToInt32(finalPulse["changeMpPoint"].Value)==4,"Regeneration ratio readback failed");
 if(inputHash!=Hash(temporary)){File.Copy(destination,Path.Combine(output,"before-regulation.bin"),true);File.Copy(temporary,destination,true);}
 File.WriteAllText(Path.Combine(output,"parameter-patch.json"),JsonSerializer.Serialize(new{inputRegulationSha256=inputHash,outputRegulationSha256=Hash(destination),effectId=95000601,changes=new[]{new{field="changeHpPoint",before=oldHp,after=-16},new{field="changeMpPoint",before=oldFp,after=4}},fpCost=4,hpRestore=16,fpToHpRatio="1:4",unrelatedRowsAndFieldsUnchanged=true,unrelatedTablesUnchanged=true,encryptedRegulationReadbackPassed=true},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("PASS: regeneration spends 4 FP to restore 16 HP; all other fields and tables preserved");return;
}
if(args.Contains("--sword-role-probe")){
 var output=Path.Combine(root,".work/sword-balance");Directory.CreateDirectory(output);
 var charas=Parse("CharaInitParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="CharaInitParam.param").Bytes);
 File.WriteAllText(Path.Combine(output,"native-heroes.json"),JsonSerializer.Serialize(charas.Rows.Select(r=>new{r.ID,heroId=r["heroId"].Value}),new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only native CharaInitParam hero identities exported");return;
}
if(args.Contains("--drinking-hud-only")){
 var output=Path.Combine(root,".work/drinking-hud");Directory.CreateDirectory(output);
 var destination=Path.Combine(root,"dist/mod/regulation.bin");
 var patchReport=Path.Combine(output,"parameter-patch.json");var inputHash=Hash(destination);
 if(File.Exists(patchReport)&&JsonDocument.Parse(File.ReadAllText(patchReport)).RootElement.GetProperty("outputRegulationSha256").GetString()==inputHash){Console.WriteLine("Drinking HUD parameter patch is already installed");return;}
 Require(inputHash=="2962D93B0B3195095BF21CEB6C9BCCAF0B0B427F61A0BD40B7586F9F7D7C9B1D","Drinking HUD patch requires inspected 0.17.14 parameters; use full build for other inputs");
 var nativeAttack=pars["SpEffectParam"].Rows.Single(r=>r.ID==1681);
 Require(Convert.ToInt32(nativeAttack["iconId"].Value)==20006,"Native temporary attack-up icon changed");
 var installed=SFUtil.DecryptNightreignRegulation(destination);var oldFiles=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var member=installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
 var table=Parse("SpEffectParam",member.Bytes);var oldTable=Parse("SpEffectParam",oldFiles[member.ID]);
 var ids=Enumerable.Range(1,79).Select(slot=>95005000+slot).Prepend(95000501).ToHashSet();
 foreach(var id in ids){
  var row=table.Rows.Single(r=>r.ID==id);
  Require(Convert.ToInt32(row["iconId"].Value)==21206&&Convert.ToSingle(row["effectEndurance"].Value)==15f&&Convert.ToInt32(row["dispDartDebuffStacks"].Value)==0,"Unexpected Drinking stack display/duration");
  row["iconId"].Value=Convert.ChangeType(20006,row["iconId"].Value.GetType());
 }
 member.Bytes=table.Write();var readback=Parse("SpEffectParam",member.Bytes);
 Require(readback.Rows.Count==oldTable.Rows.Count,"Unexpected effect row count");
 foreach(var row in oldTable.Rows){
  var actual=readback.Rows.Single(r=>r.ID==row.ID);
  foreach(var cell in row.Cells.Where(c=>c!=null)){
   var expected=ids.Contains(row.ID)&&cell.Def.InternalName=="iconId"?JsonSerializer.Serialize(20006):JsonSerializer.Serialize(cell.Value);
   Require(JsonSerializer.Serialize(actual[cell.Def.InternalName].Value)==expected,"Unexpected effect change: "+row.ID+" "+cell.Def.InternalName);
  }
 }
 var temporary=Path.Combine(output,"patched-regulation.bin");SFUtil.EncryptNightreignRegulation(temporary,installed,installed.Compression);
 var checkedBinder=SFUtil.DecryptNightreignRegulation(temporary);Require(checkedBinder.Files.Count==installed.Files.Count,"Parameter binder count changed");
 foreach(var f in checkedBinder.Files)Require(f.ID==member.ID?f.Bytes.Span.SequenceEqual(member.Bytes.Span):f.Bytes.Span.SequenceEqual(oldFiles[f.ID]),"Unexpected table change or readback failure");
 File.Copy(destination,Path.Combine(output,"before-regulation.bin"),true);File.Copy(temporary,destination,true);
 File.WriteAllText(patchReport,JsonSerializer.Serialize(new{inputRegulationSha256=inputHash,outputRegulationSha256=Hash(destination),changedEffectIds=ids.OrderBy(id=>id),field="iconId",previousIcon=21206,hudIcon=20006,nativeReference=1681,allOtherEffectFieldsUnchanged=true,unrelatedTablesUnchanged=true},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("PASS: 80 Drinking layers use native attack-up HUD icon; all other effect fields and tables preserved");return;
}
if(args.Contains("--escape-vfx-only")){
 var output=Path.Combine(root,".work/escape-vfx");Directory.CreateDirectory(output);
 var destination=Path.Combine(root,"dist/mod/regulation.bin");
 var patchReport=Path.Combine(output,"parameter-patch.json");var inputHash=Hash(destination);
 if(File.Exists(patchReport)&&JsonDocument.Parse(File.ReadAllText(patchReport)).RootElement.GetProperty("outputRegulationSha256").GetString()==inputHash){Console.WriteLine("Escape visual parameter patch is already installed");return;}
 Require(inputHash=="CCB55EEA7A3876166FD050F63CCF6F6C16274F87FDE52C7F740B5A5DE527C04D","Escape VFX patch requires inspected 0.17.12 input; use full build for other parameters");
 var aromaticGood=pars["EquipParamGoods"].Rows.Single(r=>r.ID==3550);
 Require(Convert.ToInt32(aromaticGood["refCategory"].Value)==1&&Convert.ToInt32(aromaticGood["refId_default"].Value)==10355000,"Bloodboil item reference changed");
 var aromaticBullet=pars["Bullet"].Rows.Single(r=>r.ID==10355000);
 Require(Convert.ToInt32(aromaticBullet["spEffectId0"].Value)==503550,"Bloodboil bullet effect changed");
 var aromaticEffect=pars["SpEffectParam"].Rows.Single(r=>r.ID==503550);
 Require(Convert.ToInt32(aromaticEffect["vfxId"].Value)==5280,"Bloodboil ongoing visual changed");
 var originalVfx=pars["SpEffectVfxParam"].Rows.Single(r=>r.ID==5280);
 Require(Convert.ToInt32(originalVfx["midstSfxId"].Value)==4530&&Convert.ToInt32(originalVfx["midstDmyId"].Value)==905,"Bloodboil ongoing SFX binding changed");
 var installed=SFUtil.DecryptNightreignRegulation(destination);var oldFiles=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var member=installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
 var table=Parse("SpEffectParam",member.Bytes);Require(!table.Rows.Any(r=>r.ID==95001204),"Escape visual ID collision");
 var visual=new PARAM.Row(pars["SpEffectParam"].Rows.Single(r=>r.ID==8000000)){ID=95001204};
 void SetVisual(string field,object value){var cell=visual[field];cell.Value=Convert.ChangeType(value,cell.Value.GetType());}
 SetVisual("maxHpRate",1f);SetVisual("saveCategory",-1);SetVisual("effectEndurance",10f);SetVisual("spCategory",20);
 SetVisual("iconId",-1);SetVisual("spEffectTextId_1",-1);SetVisual("displayIconForPermamentEffect",0);SetVisual("vfxId",5280);
 table.Rows.Add(visual);member.Bytes=table.Write();var readback=Parse("SpEffectParam",member.Bytes);
 var oldTable=Parse("SpEffectParam",oldFiles[member.ID]);Require(readback.Rows.Count==oldTable.Rows.Count+1,"Unexpected effect row count");
 foreach(var row in oldTable.Rows){var actual=readback.Rows.Single(r=>r.ID==row.ID);Require(actual.Cells.Zip(row.Cells).All(pair=>pair.First==null&&pair.Second==null||pair.First!=null&&pair.Second!=null&&JsonSerializer.Serialize(pair.First.Value)==JsonSerializer.Serialize(pair.Second.Value)),"Existing special effect changed: "+row.ID);}
 var temporary=Path.Combine(output,"patched-regulation.bin");SFUtil.EncryptNightreignRegulation(temporary,installed,installed.Compression);
 var checkedBinder=SFUtil.DecryptNightreignRegulation(temporary);Require(checkedBinder.Files.Count==installed.Files.Count,"Parameter binder count changed");
 foreach(var f in checkedBinder.Files)Require(f.ID==member.ID?f.Bytes.Span.SequenceEqual(member.Bytes.Span):f.Bytes.Span.SequenceEqual(oldFiles[f.ID]),"Unexpected table change or readback failure");
 File.Copy(destination,Path.Combine(output,"before-regulation.bin"),true);File.Copy(temporary,destination,true);
 File.WriteAllText(patchReport,JsonSerializer.Serialize(new{inputRegulationSha256=inputHash,outputRegulationSha256=Hash(destination),addedEffect=95001204,nativeGoods=3550,nativeBullet=10355000,nativeEffect=503550,nativeVfx=5280,midstSfxId=4530,midstDmyId=905,durationSeconds=10,originalEffectsUnchanged=true,unrelatedTablesUnchanged=true},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("PASS: one neutral ten-second visual effect added; all existing effects and unrelated tables preserved");return;
}
if(args.Contains("--effect-sync-only")){
 var output=Path.Combine(root,".work/effect-sync");Directory.CreateDirectory(output);
 var destination=Path.Combine(root,"dist/mod/regulation.bin");
 var installed=SFUtil.DecryptNightreignRegulation(destination);
 var inputHash=Hash(destination);
 if(inputHash=="CCB55EEA7A3876166FD050F63CCF6F6C16274F87FDE52C7F740B5A5DE527C04D"&&File.Exists(Path.Combine(output,"parameter-patch.json"))){Console.WriteLine("Inspected effect sync regulation is already installed");return;}
 Require(inputHash=="4C8122BD2F4C155A14B125BC01B87C85132F0B25401CB9DD461A1B55EF5D8E63","Effect sync patch requires the inspected 0.17.9 regulation; use full build for other inputs");
 var inputFiles=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var file=installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param");
 var table=Parse("SpEffectParam",file.Bytes);
 var owners=new[]{95000100,95000200,95000300,95000400,95000500,95000600,95000700,95000800,95000900,95001100,95001200,95001300,95001400,95001500,95001600};
 var ready=new[]{95000403,95000701,95001203};
 var changes=new List<object>();
 foreach(var row in table.Rows.Where(r=>owners.Contains(r.ID)||ready.Contains(r.ID))){
  Require(Convert.ToInt32(row["spCategory"].Value)==10,"Unexpected original status category");
  changes.Add(new{id=row.ID,field="spCategory",before=10,after=20});row["spCategory"].Value=(short)20;
  if(ready.Contains(row.ID)){
   Require(Convert.ToSingle(row["effectEndurance"].Value)==-1f,"Unexpected original ready duration");
   changes.Add(new{id=row.ID,field="effectEndurance",before=-1f,after=2f});row["effectEndurance"].Value=2f;
  }
 }
 Require(changes.Count==21,"Status patch row count mismatch");
 var oldTable=Parse("SpEffectParam",inputFiles[file.ID]);
 file.Bytes=table.Write();var checkedTable=Parse("SpEffectParam",file.Bytes);
 foreach(var row in checkedTable.Rows){
  var old=oldTable.Rows.Single(r=>r.ID==row.ID);
  foreach(var cell in row.Cells.Where(c=>c!=null)){
   var field=cell.Def.InternalName;
   var allowed=field=="spCategory"&&(owners.Contains(row.ID)||ready.Contains(row.ID))||field=="effectEndurance"&&ready.Contains(row.ID);
   Require(allowed||JsonSerializer.Serialize(cell.Value)==JsonSerializer.Serialize(old[field].Value),"Unrelated status field changed: "+row.ID+" "+field);
  }
 }
 var beforePath=Path.Combine(output,"before-regulation.bin");File.Copy(destination,beforePath,true);
 var temporary=Path.Combine(output,"patched-regulation.bin");SFUtil.EncryptNightreignRegulation(temporary,installed,installed.Compression);
 var checkedBinder=SFUtil.DecryptNightreignRegulation(temporary);
 Require(checkedBinder.Files.Count==installed.Files.Count,"Parameter binder count changed");
 foreach(var f in checkedBinder.Files)Require(f.ID==file.ID||f.Bytes.Span.SequenceEqual(inputFiles[f.ID]),"Unrelated parameter table changed");
 var finalTable=Parse("SpEffectParam",checkedBinder.Files.Single(f=>f.ID==file.ID).Bytes);
 foreach(var id in owners.Concat(ready))Require(Convert.ToInt32(finalTable.Rows.Single(r=>r.ID==id)["spCategory"].Value)==20,"Category readback failed");
 foreach(var id in ready)Require(Convert.ToSingle(finalTable.Rows.Single(r=>r.ID==id)["effectEndurance"].Value)==2f,"Lease readback failed");
 File.Copy(temporary,destination,true);
 File.WriteAllText(Path.Combine(output,"parameter-patch.json"),JsonSerializer.Serialize(new{inputRegulationSha256=inputHash,outputRegulationSha256=Hash(destination),changes,unrelatedTablesUnchanged=true,unrelatedFieldsUnchanged=true},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("PASS: 18 same-ID status rows, three native two-second ready leases; all other fields/tables preserved");return;
}
if(args.Contains("--effect-sync-probe")){
 var output=Path.Combine(root,".work/effect-sync");Directory.CreateDirectory(output);
 var installed=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
 var fields=new[]{"iconId","saveCategory","effectEndurance","spCategory","categoryPriority","spEffectTextId_1","displayIconForPermamentEffect","unknown_239a","unknown_239b","unknown_239c","unknown_239e","unknown_239f","unknown_239g","unknown_239h","replaceSpEffectId","cycleOccurrenceSpEffectId","stateInfo","vfxId"};
 object DumpSync(PARAM.Row row)=>new{row.ID,fields=row.Cells.Where(c=>c!=null&&fields.Contains(c.Def.InternalName)).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
 var current=Parse("SpEffectParam",installed.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
 var permanentIds=pars["PermanentBuffParam"].Rows.Select(r=>r.ID).ToHashSet();
 var permanentEffects=pars["PermanentBuffParam"].Rows.Select(r=>Convert.ToInt32(r["spEffectId"].Value)).ToHashSet();
 var nativeRows=pars["SpEffectParam"].Rows.Where(r=>permanentEffects.Contains(r.ID)||new[]{700600,707115,102100,102150}.Contains(r.ID));
 File.WriteAllText(Path.Combine(output,"parameters.json"),JsonSerializer.Serialize(new{sourceRegulationSha256=Hash(source),installedRegulationSha256=Hash(Path.Combine(root,"dist/mod/regulation.bin")),native=nativeRows.Select(DumpSync),current=current.Rows.Where(r=>r.ID>=95000000&&r.ID<95001700).Select(DumpSync)},new JsonSerializerOptions{WriteIndented=true}));
 foreach(var table in new[]{"AttachEffectParam","PermanentBuffParam"}){
  var currentTable=Parse(table,installed.Files.Single(f=>Path.GetFileName(f.Name)==table+".param").Bytes);
  File.WriteAllText(Path.Combine(output,table+".json"),JsonSerializer.Serialize(new{native=pars[table].Rows.Where(r=>permanentIds.Contains(r.ID)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),current=currentTable.Rows.Where(r=>r.ID>=95000000&&r.ID<95001700).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})},new JsonSerializerOptions{WriteIndented=true}));
 }
 Console.WriteLine("Read-only native/current special-effect display and permanent power references exported");return;
}
if(args.Contains("--cage-probe")){
 var output=Path.Combine(root,".work/cage");Directory.CreateDirectory(output);
 var events=EMEVD.Read(Path.Combine(root,"assets/original/event/common_func.emevd.dcx"));
 var selected=events.Events.Where(e=>new long[]{90045000,90045001,90045002,90045004,90015000,90015023,90065910}.Contains(e.ID)||e.Instructions.Any(i=>i.Bank==2006&&i.ID==3)&&e.Instructions.Any(i=>i.Bank==2000&&i.ID==2));
 File.WriteAllText(Path.Combine(output,"native-events.json"),JsonSerializer.Serialize(selected.Select(e=>new{e.ID,e.RestBehavior,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})}),new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only native cage, HUD and local SFX events exported");return;
}
if(args.Contains("--cage-only")){
 // Rebuild only cage maps/events against the installed, verified NPC copies.
 var installed=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
 Require(installed.Version==bnd.Version,"Cage-only regulation version mismatch");
 var serializedCage=new Dictionary<string,PARAM>(pars);
 foreach(var name in new[]{"NpcParam","SpEffectParam","NpcThinkParam"})serializedCage[name]=Parse(name,installed.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes);
 var bosses=JsonSerializer.Deserialize<Cages.Boss[]>(File.ReadAllText(Path.Combine(root,"data/reference/world/cage-bosses.json")))!;
 var towers=Cages.TowerSources(root,bosses);
 foreach(var (boss,index) in bosses.Select((boss,index)=>(boss,index))){
  var nativeRow=pars["NpcParam"].Rows.Single(r=>r.ID==boss.npc);var copy=serializedCage["NpcParam"].Rows.Single(r=>r.ID==95300000+index);
  Require(Convert.ToInt32(copy["disableRespawn"].Value)==0&&copy.Cells.Where(c=>c!=null&&c.Def.InternalName!="disableRespawn").All(c=>
   towers.Any(t=>t.sourceNpc==boss.npc)&&c.Def.InternalName=="rewardItemLot_2"?new[]{boss.nativeRewardLot,Cages.Reward(boss,towers)}.Contains(Convert.ToInt32(c.Value)):
   c.Def.InternalName=="chaosMatchingRewardLotId"&&Convert.ToInt32(nativeRow[c.Def.InternalName].Value)==-1?new[]{-1,boss.nativeRewardLot,Cages.Reward(boss,towers)}.Contains(Convert.ToInt32(c.Value)):
   c.Value is byte[] bytes?bytes.SequenceEqual((byte[])nativeRow[c.Def.InternalName].Value):Equals(c.Value,nativeRow[c.Def.InternalName].Value)),"Cage-only NPC copy mismatch");
 }
 var before=installed.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 Cages.MutationParameters(serializedCage,towers);
 CageAI.Add(root,serializedCage["NpcThinkParam"],bosses);
 foreach(var name in new[]{"NpcParam","SpEffectParam","NpcThinkParam"})installed.Files.Single(f=>Path.GetFileName(f.Name)==name+".param").Bytes=serializedCage[name].Write();
 foreach(var file in installed.Files){
  var name=Path.GetFileNameWithoutExtension(file.Name);
  if(name is not ("NpcParam" or "SpEffectParam" or "NpcThinkParam")){Require(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated cage parameter table changed");continue;}
  var old=Parse(name,before[file.ID]);var readback=Parse(name,file.Bytes);
  if(name=="NpcThinkParam"){CageAI.Validate(old,serializedCage[name],readback);serializedCage[name]=readback;continue;}
  Require(readback.Rows.Count==old.Rows.Count+(name=="SpEffectParam"&&!old.Rows.Any(r=>r.ID==Cages.Mutation)?1:0),"Unexpected mutation row count");
  foreach(var row in old.Rows){var actual=readback.Rows.Single(r=>r.ID==row.ID);foreach(var cell in row.Cells.Where(c=>c!=null)){
   var tower=towers.SingleOrDefault(t=>t.cloneNpc==row.ID);
   var expected=name=="NpcParam"&&tower!=null&&cell.Def.InternalName is "rewardItemLot_2" or "chaosMatchingRewardLotId"?(object)tower.firstLot:
    name=="NpcParam"&&row.ID>=95300000&&row.ID<95300037&&cell.Def.InternalName=="chaosMatchingRewardLotId"&&Convert.ToInt32(cell.Value)==-1?row["rewardItemLot_2"].Value:cell.Value;
   Require(JsonSerializer.Serialize(actual[cell.Def.InternalName].Value)==JsonSerializer.Serialize(expected),"Unexpected mutation field change: "+name+" "+row.ID+" "+cell.Def.InternalName);
  }}
  serializedCage[name]=readback;
 }
 var destination=Path.Combine(root,"dist/mod/regulation.bin");var temporary=Path.Combine(root,".work/cage/mutation-regulation.bin");
 SFUtil.EncryptNightreignRegulation(temporary,installed,installed.Compression);
 var reopened=SFUtil.DecryptNightreignRegulation(temporary);Require(reopened.Files.Count==installed.Files.Count&&reopened.Files.All(f=>f.Bytes.Span.SequenceEqual(installed.Files.Single(v=>v.ID==f.ID).Bytes.Span)),"Mutation regulation round-trip failed");
 File.Copy(temporary,destination,true);
 File.WriteAllText(Path.Combine(root,".work/cage/parameter-patch.json"),JsonSerializer.Serialize(new{combatFieldsPreserved=37,towerRewardOverrides=towers,addedMarker=Cages.Mutation,unrelatedRowsAndTablesPreserved=true,outputRegulationSha256=Hash(destination)},new JsonSerializerOptions{WriteIndented=true}));
 Cages.Build(root,bosses,serializedCage);return;
}
if(args.Contains("--noisy-probe")){
 var output=Path.Combine(root,".work/noisy");Directory.CreateDirectory(output);
 object DumpNoisy(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
 var ids=pars["SpEffectParam"].Rows.Where(r=>r.ID>=8630500&&r.ID<8630600).Select(r=>r.ID).ToHashSet();ids.Add(8000000);
 for(int pass=0;pass<5;pass++)foreach(var r in pars["SpEffectParam"].Rows.Where(r=>ids.Contains(r.ID)).ToArray())foreach(var f in new[]{"replaceSpEffectId","cycleOccurrenceSpEffectId","atkOccurrenceSpEffectId"}){
  int id=Convert.ToInt32(r[f].Value);if(id>0)ids.Add(id);
 }
 var noisyReport=new{sourceSha256=Hash(source),version=bnd.Version,effects=pars["SpEffectParam"].Rows.Where(r=>ids.Contains(r.ID)).Select(DumpNoisy),bullets=pars["Bullet"].Rows.Select(DumpNoisy),attacks=pars["AtkParam_Pc"].Rows.Select(DumpNoisy),behaviors=pars["BehaviorParam_PC"].Rows.Where(r=>r.ID>=863050000&&r.ID<863060000).Select(DumpNoisy)};
 File.WriteAllText(Path.Combine(output,"parameters.json"),JsonSerializer.Serialize(noisyReport));
 var keys=Parse("KeyAssignParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="KeyAssignParam.param").Bytes);
 File.WriteAllText(Path.Combine(output,"keys.json"),JsonSerializer.Serialize(keys.Rows.Select(DumpNoisy)));
 var textBundle=BND4.Read(Path.Combine(root,"assets/original/msg/zhocn/menu_dlc01.msgbnd.dcx"));
 foreach(var f in textBundle.Files.Where(f=>f.Name.EndsWith(".fmg")))foreach(var e in FMG.Read(f.Bytes).Entries.Where(e=>e.Text!=null&&(e.Text.Contains("标记")||e.Text.Contains("自己位置"))))Console.WriteLine(Path.GetFileName(f.Name)+" "+e.ID+"="+e.Text);
 var common=Parse("PlayerCommonParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="PlayerCommonParam.param").Bytes);
 File.WriteAllText(Path.Combine(output,"player-common.json"),JsonSerializer.Serialize(common.Rows.Select(DumpNoisy)));
 var events=EMEVD.Read(Path.Combine(root,"assets/original/event/common.emevd.dcx"));
 File.WriteAllText(Path.Combine(output,"events.json"),JsonSerializer.Serialize(events.Events.Where(e=>e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>ids.Contains(BitConverter.ToInt32(i.ArgData,n*4))))).Select(e=>new{e.ID,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData)})}),new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Original lava effects, bullets, attacks, behaviors and referencing events exported.");return;
}
if(args.Contains("--spiritspring-probe")){
 var mapNames=new[]{"WorldMapPointIconParam","WorldMapPointParam","TutorialParam","ChrPhysicsVelocityChangeParam","FallControlParam"};
 var tables=mapNames.ToDictionary(n=>n,n=>Parse(n,bnd.Files.Single(f=>Path.GetFileName(f.Name)==n+".param").Bytes));
 bool NumberEquals(object? value,int id)=>value is not byte[]&&value!=null&&double.TryParse(value.ToString(),out var number)&&number==id;
 object DumpSpring(PARAM.Row row)=>new{row.ID,fields=row.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
 var iconRows=tables["WorldMapPointIconParam"].Rows.Where(r=>r.Cells.Any(c=>c!=null&&NumberEquals(c.Value,42407))).ToArray();
 var iconIds=iconRows.Select(r=>r.ID).ToHashSet();
 var points=tables["WorldMapPointParam"].Rows.Where(r=>r.Cells.Any(c=>c!=null&&((c.Def.InternalName=="worldMapPointIconId"&&iconIds.Any(id=>NumberEquals(c.Value,id)))||NumberEquals(c.Value,42407)))).ToArray();
 var direct=tables.ToDictionary(k=>k.Key,k=>k.Value.Rows.Where(r=>r.Cells.Any(c=>c!=null&&NumberEquals(c.Value,42407))).Select(DumpSpring).ToArray());
 var physics=tables["ChrPhysicsVelocityChangeParam"].Rows.Where(r=>r.ID>=100&&r.ID<=400||r.ID>=1000&&r.ID<=3500).Select(DumpSpring).ToArray();
 var fallRows=tables["FallControlParam"].Rows.Where(r=>r.ID>=100&&r.ID<=400).Select(DumpSpring).ToArray();
 var regionTypes=typeof(BND4).Assembly.GetTypes().Where(t=>t.FullName!=null&&t.FullName.Contains("MSB")&&(t.Name.Contains("Wind")||t.Name.Contains("Jump")||t.Name.Contains("Spring"))).Select(t=>new{name=t.FullName,fields=t.GetFields().Select(f=>new{f.Name,type=f.FieldType.FullName}),properties=t.GetProperties().Select(p=>new{p.Name,type=p.PropertyType.FullName})}).ToArray();
 var mapRegions=new List<object>();var crystalAssets=new List<object>();
 var mapFolder=Path.Combine(root,"assets/original/map/mapstudio");
 if(Directory.Exists(mapFolder))foreach(var mapFile in Directory.EnumerateFiles(mapFolder,"*.msb.dcx")){
  var map=MSB_NR.Read(mapFile);
  foreach(var part in map.Parts.GetEntries().Where(p=>new[]{"結晶","水晶","霊気","大晶"}.Any(term=>p.Name.Contains(term)))){
   var t=part.GetType();crystalAssets.Add(new{file=Path.GetFileName(mapFile),part.Name,part.ModelName,position=new[]{part.Position.X,part.Position.Y,part.Position.Z},entity=t.GetProperty("Common_EntityID")?.GetValue(part),type=t.FullName});
  }
  foreach(var region in map.Regions.GetEntries().Where(r=>r.GetType().Name.Contains("Jump")||new[]{"結晶","水晶","気流","霊気","ジャンプ"}.Any(term=>r.Name.Contains(term)))){
   var type=region.GetType();var regionFields=type.GetProperties(BindingFlags.Public|BindingFlags.Instance|BindingFlags.DeclaredOnly).ToDictionary(p=>p.Name,p=>p.GetValue(region));
   mapRegions.Add(new{file=Path.GetFileName(mapFile),sourceSha256=Hash(mapFile),type=type.FullName,region.Name,region.RegionID,region.Common_EntityID,position=new[]{region.Position.X,region.Position.Y,region.Position.Z},fields=regionFields});
  }
 }
 var regionEnums=typeof(MSB_NR).GetNestedTypes(BindingFlags.Public|BindingFlags.NonPublic).Where(t=>t.IsEnum&&t.Name.Contains("Region")).Select(t=>new{name=t.FullName,values=Enum.GetNames(t).Where(n=>n.Contains("Jump")).ToDictionary(n=>n,n=>Convert.ToInt64(Enum.Parse(t,n)))});
 var specialJumpEffects=pars["SpEffectParam"].Rows.Where(r=>new[]{98215,98216,98217,98218,99215,99216,99217,99218,102500,102501,102502}.Contains(r.ID)).Select(DumpSpring).ToArray();
 var crystalEvents=new List<object>();var eventFolder=Path.Combine(root,"assets/original/event");
 if(Directory.Exists(eventFolder))foreach(var eventFile in Directory.EnumerateFiles(eventFolder,"*.emevd.dcx")){
  var crystalEventBundle=EMEVD.Read(eventFile);
  foreach(var e in crystalEventBundle.Events.Where(e=>e.ID==1047402505||e.ID==1047402506||e.Instructions.Any(i=>Enumerable.Range(0,i.ArgData.Length/4).Any(n=>new[]{98215,98216,98217,98218,99215,99216,99217,99218,102501,1047402502}.Contains(BitConverter.ToInt32(i.ArgData,n*4)))))){
   crystalEvents.Add(new{file=Path.GetFileName(eventFile),sourceSha256=Hash(eventFile),e.ID,e.RestBehavior,e.Parameters,instructions=e.Instructions.Select(i=>new{i.Bank,i.ID,hex=Convert.ToHexString(i.ArgData),values=Enumerable.Range(0,i.ArgData.Length/4).Select(n=>BitConverter.ToInt32(i.ArgData,n*4))})});
  }
 }
 var result=new{sourceRegulationSha256=Hash(source),version=bnd.Version,iconId=42407,direct,iconRows=iconRows.Select(DumpSpring),linkedMapPoints=points.Select(DumpSpring),jumpPhysics=physics,fallRows,regionTypes,mapRegions,regionEnums,specialJumpEffects,crystalEvents,crystalAssets};
 File.WriteAllText(Path.Combine(root,"data/reference/actions/spiritspring.json"),JsonSerializer.Serialize(result,new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine($"Read-only spiritspring probe: {iconRows.Length} icon rows, {points.Length} linked map points; original jump/fall parameters exported.");return;
}
if(args.Contains("--escape-vfx-probe")){
 var output=Path.Combine(root,".work/escape-vfx");Directory.CreateDirectory(output);
 object DumpVfx(PARAM.Row row)=>new{row.ID,fields=row.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
 var aromaticGood=pars["EquipParamGoods"].Rows.Single(r=>r.ID==3550);
 Require(Convert.ToInt32(aromaticGood["refCategory"].Value)==1,"3550 must reference a bullet");
 var aromaticBullet=pars["Bullet"].Rows.Single(r=>r.ID==Convert.ToInt32(aromaticGood["refId_default"].Value));
 var effectIds=aromaticBullet.Cells.Where(c=>c!=null&&c.Def.InternalName.Contains("spEffect",StringComparison.OrdinalIgnoreCase)).Select(c=>Convert.ToInt32(c.Value)).Where(id=>id>0).ToHashSet();
 for(int pass=0;pass<8;pass++)foreach(var row in pars["SpEffectParam"].Rows.Where(r=>effectIds.Contains(r.ID)).ToArray())foreach(var field in new[]{"replaceSpEffectId","cycleOccurrenceSpEffectId","atkOccurrenceSpEffectId"}){
  var id=Convert.ToInt32(row[field].Value);if(id>=0)effectIds.Add(id);
 }
 var aromaticEffects=pars["SpEffectParam"].Rows.Where(r=>effectIds.Contains(r.ID)).ToArray();
 var vfxIds=aromaticEffects.Select(r=>Convert.ToInt32(r["vfxId"].Value)).ToHashSet();
 var aromaticVfx=pars["SpEffectVfxParam"].Rows.Where(r=>vfxIds.Contains(r.ID)).ToArray();
 var current=SFUtil.DecryptNightreignRegulation(Path.Combine(root,"dist/mod/regulation.bin"));
 var currentEffects=Parse("SpEffectParam",current.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes);
 var currentVisual=currentEffects.Rows.SingleOrDefault(r=>r.ID==95001204);
 var originalEscapeVfx=pars["SpEffectVfxParam"].Rows.Single(r=>r.ID==340800);
 Console.WriteLine("Original Escape VFX category/priority: "+originalEscapeVfx["playCategory"].Value+"/"+originalEscapeVfx["playPriority"].Value);
 File.WriteAllText(Path.Combine(output,"parameters.json"),JsonSerializer.Serialize(new{sourceRegulationSha256=Hash(source),installedRegulationSha256=Hash(Path.Combine(root,"dist/mod/regulation.bin")),nativeGood=DumpVfx(aromaticGood),nativeBullet=DumpVfx(aromaticBullet),nativeEffects=aromaticEffects.Select(DumpVfx),nativeVfx=aromaticVfx.Select(DumpVfx),nativeNeutral=DumpVfx(pars["SpEffectParam"].Rows.Single(r=>r.ID==8000000)),currentEffect=DumpVfx(currentEffects.Rows.Single(r=>r.ID==95001201)),currentVisual=currentVisual==null?null:DumpVfx(currentVisual)},new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Read-only original 3550 visual chain and current Escape active effect exported");return;
}
if(args.Contains("--escape-probe")){
 object Dump(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
 File.WriteAllText(Path.Combine(root,".work/effects/native-low-hp.json"),JsonSerializer.Serialize(new[]{8000000,340800,7012300,8660100,8660000,700600,700601}.Select(id=>Dump(pars["SpEffectParam"].Rows.Single(r=>r.ID==id))),new JsonSerializerOptions{WriteIndented=true}));return;
}
if(args.Contains("--cat-jump-probe")){
 var velocity=Parse("ChrPhysicsVelocityChangeParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="ChrPhysicsVelocityChangeParam.param").Bytes);
 var fall=Parse("FallControlParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="FallControlParam.param").Bytes);
 object Fields(PARAM.Row r)=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)};
 var reportJump=new{sourceRegulationSha256=Hash(source),nativeJumpEffects=new[]{321800,8320300,8320301,8320302}.Select(id=>Fields(pars["SpEffectParam"].Rows.Single(r=>r.ID==id))),jumpVelocityRows=new[]{100,101,111,121}.Select(id=>Fields(velocity.Rows.Single(r=>r.ID==id))),fallControlRows=new[]{100,101}.Select(id=>Fields(fall.Rows.Single(r=>r.ID==id))),jumpHeightEstimateMetres=2.5,threeJumpBaselineMetres=7.5,estimateNotLiveMeasured=true};
 File.WriteAllText(Path.Combine(root,"data/reference/combat/jump-parameters.json"),JsonSerializer.Serialize(reportJump,new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine("Original-regulation jump/velocity/fall parameters exported; 2.5 m is a provisional estimate.");return;
}
if(args.Contains("--sword-category-probe")){
 var categoryIds=args.Contains("--sword-test-probe")?new[]{8000000,8320500,8350000,7006600,7006700}:new[]{8000000,8320500,8350000};
 var categoryRows=categoryIds.Select(id=>pars["SpEffectParam"].Rows.Single(r=>r.ID==id)).ToArray();
 var categoryProbe=new{effects=categoryRows.Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})};
 File.WriteAllText(Path.Combine(root,args.Contains("--sword-test-probe")?".work/combat/sword-test-categories.json":"data/reference/combat/sword-categories.json"),JsonSerializer.Serialize(categoryProbe,new JsonSerializerOptions{WriteIndented=true}));
 var neutralCategory=categoryRows[0];
 foreach(var rowCategory in categoryRows.Skip(1))Console.WriteLine(rowCategory.ID+" "+string.Join(", ",rowCategory.Cells.Where(c=>c!=null&&!Equals(c.Value,neutralCategory[c.Def.InternalName].Value)).Select(c=>c.Def.InternalName+"="+c.Value)));
 if(args.Contains("--export-native-fixtures")){
  var paramFixture=pars["SpEffectParam"];
  foreach(var rowFixture in categoryRows){
   paramFixture.Rows.Clear();paramFixture.Rows.Add(rowFixture);
   File.WriteAllBytes(Path.Combine(root,"data/reference/combat",$"effect-{rowFixture.ID}.param"),paramFixture.Write());
  }
 }
 return;
}
if(args.Contains("--shell-native-probe")){
 var ids=new[]{8000000,3507,8610700,8610701,8610702};
 var nativeVfx=Parse("SpEffectVfxParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectVfxParam.param").Bytes);
 var shellProbeReport=new{
 effects=ids.Select(id=>pars["SpEffectParam"].Rows.Single(r=>r.ID==id)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),
 visuals=new[]{5866,5867}.Select(id=>nativeVfx.Rows.Single(r=>r.ID==id)).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})};
 File.WriteAllText(Path.Combine(root,"data/reference/combat/shell-chain.json"),JsonSerializer.Serialize(shellProbeReport,new JsonSerializerOptions{WriteIndented=true}));
 foreach(var langProbe in new[]{"zhocn","engus"}){
  var menuSource=Path.Combine(root,"assets/original/msg",langProbe,"menu_dlc01.msgbnd.dcx");
  var menuProbe=BND4.Read(menuSource);
  Console.WriteLine(langProbe+" menu hash "+Hash(menuSource));
  Console.WriteLine(string.Join("\n",menuProbe.Files.Where(f=>Path.GetFileName(f.Name).Contains("SpEffect")).Select(f=>f.ID+" "+f.Name)));
 }
 Console.WriteLine("Read-only native 8610700 shell chain saved");return;
}
if(args.Contains("--reward-probe")){
 var reportData=new{
  effects=new[]{8000000,8000001,99821}.Select(id=>pars["SpEffectParam"].Rows.Single(r=>r.ID==id)).Select(r=>new{r.ID,fields=new[]{"saveCategory","effectEndurance","spCategory","stateInfo","maxHpRate"}.ToDictionary(n=>n,n=>r[n].Value)}),
  goods=pars["EquipParamGoods"].Rows.Where(r=>Convert.ToInt32(r["goodsType"].Value)==11).Select(r=>new{r.ID,fields=new[]{"rarity","vagrantItemLotId","iconId"}.ToDictionary(n=>n,n=>r[n].Value)}),
  lots=pars["ItemLotParam_enemy"].Rows.Where(r=>r.ID>=6100000&&r.ID<6100200).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),
  attach=pars["AttachEffectParam"].Rows.Where(r=>r.ID>=8000000&&r.ID<8000010).Select(r=>new{r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})
 };
 File.WriteAllText(Path.Combine(root,".work/rewards/native-parameters.json"),JsonSerializer.Serialize(reportData,new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("Read-only reward parameters saved");return;
}
if(args.Contains("--stealth-probe")){
 foreach(var id in new[]{1766,7031803,703500,8000000,8360000,8810200,7037806,3507}){
  var r=pars["SpEffectParam"].Rows.Single(r=>r.ID==id);
  Console.WriteLine(id+" "+string.Join(", ",new[]{"iconId","secondaryIconId","effectEndurance","stateInfo","saveCategory","displayIconForPermamentEffect","hearingSearchEnemyRate","sightSearchEnemyRate","vfxId","spCategory","categoryPriority"}.Select(n=>n+"="+r[n].Value)));
 }
 return;
}
if(args.Contains("--probe")){
 foreach(var n in new[]{"EquipParamGoods","PermanentBuffParam","ItemTableParam"}){
  var row=pars[n].Rows.First(r=>r.ID==(n=="ItemTableParam"?4200000:8000001));
  Console.WriteLine(n+" "+row.ID+" "+string.Join(", ",row.Cells.Where(c=>c!=null).Select(c=>c.Def.InternalName+"="+c.Value)));
 }
 var tb=BND4.Read(Path.Combine(root,"assets/original/msg/zhocn/item_dlc01.msgbnd.dcx"));
 foreach(var f in tb.Files.Where(f=>Path.GetFileName(f.Name).Contains("PermanentBuff")||Path.GetFileName(f.Name).Contains("GoodsName"))){var fm=FMG.Read(f.Bytes);Console.WriteLine(Path.GetFileName(f.Name)+" : "+string.Join(" | ",fm.Entries.Where(e=>!string.IsNullOrEmpty(e.Text)).Take(4).Select(e=>e.ID+"="+e.Text)));Console.WriteLine("8000000="+fm.Entries.FirstOrDefault(e=>e.ID==8000000)?.Text);}
 return;
}
var added=names.ToDictionary(n=>n,n=>new List<int>());
PARAM.Row Clone(string name,int from,int id){
 var p=pars[name];Require(!p.Rows.Any(r=>r.ID==id),$"ID collision: {name} {id}");
 var row=new PARAM.Row(p.Rows.First(r=>r.ID==from)){ID=id};p.Rows.Add(row);added[name].Add(id);return row;
}
void Set(PARAM.Row row,string name,object value){var c=row[name]??throw new InvalidDataException("Missing field "+name);c.Value=Convert.ChangeType(value,c.Value.GetType());}
if(args.Contains("--export-descriptions")){
 var text=BND4.Read(Path.Combine(root,"dist/mod/msg/zhocn/item_dlc01.msgbnd.dcx"));
 var namesFmg=FMG.Read(text.Files.Single(f=>Path.GetFileName(f.Name)=="PermanentBuffName.fmg").Bytes);
 var infoFmg=FMG.Read(text.Files.Single(f=>Path.GetFileName(f.Name)=="PermanentBuffInfo.fmg").Bytes);
 var descriptions=namesFmg.Entries.Where(e=>e.ID>=95000100&&e.ID<=95001600&&e.ID%100==0&&e.ID!=95001000).ToDictionary(e=>e.Text!,e=>infoFmg.Entries.Single(v=>v.ID==e.ID).Text);
 Require(descriptions.Count==15,"Incomplete current hex descriptions");
 var exportPath=Path.Combine(root,".work/effects/hex-descriptions.json");Directory.CreateDirectory(Path.GetDirectoryName(exportPath)!);
 File.WriteAllText(exportPath,JsonSerializer.Serialize(descriptions,new JsonSerializerOptions{WriteIndented=true,Encoder=System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping}));
 Console.WriteLine("Exported 15 current Chinese descriptions from the working Mod package.");return;
}
var hexes=HexCatalog.Load(root);
if(args.Contains("--essence-reaver-stage")){EssenceReaver.Stage(root,bnd,Parse,hexes);return;}
if(args.Contains("--parry-stage")){Parry.Stage(root,bnd,Parse,hexes);Parry.BuildActions(root,bnd,Parse);return;}
if(args.Contains("--parry-followup")){ParryFollowup.Stage(root,Parse);return;}
if(args.Contains("--parry-followup-actions")){ParryFollowup.Actions(root,Parse("SpEffectParam",bnd.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectParam.param").Bytes));return;}
if(args.Contains("--contract-heart")){ContractHeart.Stage(root,bnd,Parse,hexes);return;}
if(args.Contains("--night-blessing")){NightBlessing.Stage(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--night-blessing-rework")){NightBlessing.Rework(root,bnd,Parse,HexCatalog.Load(root));return;}
if(args.Contains("--night-blessing-actions")){NightBlessing.Assets(root);return;}
if(args.Contains("--parry-actions")){Parry.BuildActions(root,bnd,Parse);return;}
if(args.Contains("--parry-support")){Parry.StageSupport(root,Parse);return;}
if(args.Contains("--parry-counter")){Parry.StageCounter(root,Parse);return;}
if(args.Contains("--slingshot-stage")){Slingshot.Stage(root,bnd,Parse,hexes);return;}
if(args.Contains("--archmage-stage")){Archmage.Stage(root,bnd,Parse,hexes);return;}
if(args.Contains("--devils-gaze-stage")){DevilsGaze.Stage(root,bnd,Parse,hexes);return;}
if(args.Contains("--hex-icons-only")){HexIcons.Stage(root,Parse,hexes.Single(h=>h.Id==int.Parse(args[Array.IndexOf(args,"--hex-icons-only")+1])));return;}
if(args.Contains("--glass-cannon-only")){GlassCannon.Stage(root,Parse,HexCatalog.Load(root));return;}
if((args.Contains("--balloon-balance-only")||args.Contains("--balloon-resistance-only")||args.Contains("--balloon-01754-only"))){BalloonBalance.Stage(root,Parse,hexes);return;}
if(args.Contains("--balance-01750-only")){Balance1750.Stage(root,Parse,hexes);return;}
if(args.Contains("--chant-01746-only")){SpiritDamageProbe.Stage(root,Parse,hexes);return;}
if(args.Contains("--spirit-damage-probe")){SpiritDamageProbe.Run(root,Parse);return;}
if(args.Contains("--balance-01745-only")){Balance1745.Stage(root,Parse,hexes);return;}
if(args.Contains("--angel-chant-only"))throw new InvalidOperationException("Historical 0.17.40 stage is retired; use the current full build.");
if(args.Contains("--spirit-ultimate-only")){SpiritUltimate.Stage(root,Parse,hexes);return;}
if(args.Contains("--spirit-rework-only")){SpiritRework.Stage(root,Parse,hexes);return;}
if(args.Contains("--new-hexes-only")){int sourceArgument=Array.IndexOf(args,"--new-hexes-source");NewHexes.Stage(root,bnd,Parse,hexes,sourceArgument<0?null:Path.GetFullPath(args[sourceArgument+1]));return;}
if(args.Contains("--texts-only")){HexTexts.Stage(root,hexes);return;}
if(args.Contains("--strength-texts-only")){HexTexts.Stage(root,hexes,Path.Combine(root,".work/strength-stars/texts"),onlyIds:new[]{95001100});return;}
if(args.Contains("--status-texts-only")){HexTexts.Stage(root,hexes,Path.Combine(root,".work/status-labels-01731/texts"),statusOnly:true);return;}
if(args.Contains("--coop-texts-only")){HexTexts.Stage(root,hexes,Path.Combine(root,".work/fixes-01731/texts"));return;}
if(args.Contains("--jiahao-only")||args.Contains("--sword-only")){
 bool swordPatch=args.Contains("--sword-only");int patchId=swordPatch?95000900:95001300;
 var patchLabel=swordPatch?"Sword":"Jiahao";
 var output=Path.Combine(root,swordPatch?".work/sword-balance":".work/jiahao");Directory.CreateDirectory(output);
 var patchText=hexes.Single(h=>h.Id==patchId);
 var reports=new List<object>();
 foreach(var language in new[]{"zhocn","engus"}){
  var destination=Path.Combine(root,"dist/mod/msg",language,"item_dlc01.msgbnd.dcx");
  var textBinder=BND4.Read(destination);var oldFiles=textBinder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  var changedNames=new[]{"AttachEffectInfo.fmg","PermanentBuffInfo.fmg","PermanentBuffCaption.fmg"};
  Require(changedNames.All(n=>textBinder.Files.Count(f=>Path.GetFileName(f.Name)==n)==1),"Missing Jiahao description FMG");
  foreach(var member in textBinder.Files.Where(f=>changedNames.Contains(Path.GetFileName(f.Name)))){
   var textFmg=FMG.Read(member.Bytes);var oldFmg=FMG.Read(oldFiles[member.ID]);
   textFmg.Entries.Single(e=>e.ID==patchId).Text=language=="zhocn"?patchText.ZhInfo:patchText.EnInfo;
   member.Bytes=textFmg.Write();var reopened=FMG.Read(member.Bytes);
   Require(reopened.Entries.Count==oldFmg.Entries.Count,"Jiahao FMG entry count changed");
   Require(reopened.Entries.Zip(oldFmg.Entries).All(pair=>pair.First.ID==pair.Second.ID&&(pair.First.ID==patchId||pair.First.Text==pair.Second.Text)),"Unrelated text changed");
  }
  var temporary=Path.Combine(output,language+"-item_dlc01.msgbnd.dcx");textBinder.Write(temporary);
  var checkedText=BND4.Read(temporary);Require(checkedText.Files.Count==textBinder.Files.Count,"Jiahao text binder count changed");
  foreach(var member in checkedText.Files){
   var intended=textBinder.Files.Single(f=>f.ID==member.ID);
   Require(member.Name==intended.Name&&member.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Jiahao text readback failed");
   Require(changedNames.Contains(Path.GetFileName(member.Name))||member.Bytes.Span.SequenceEqual(oldFiles[member.ID]),"Unrelated text member changed");
  }
  File.Copy(temporary,destination,true);
  reports.Add(new{language,outputSha256=Hash(destination),effectId=patchId,description=language=="zhocn"?patchText.ZhInfo:patchText.EnInfo,changedFiles=changedNames,unrelatedEntriesUnchanged=true,unrelatedMembersUnchanged=true});
 }
 File.WriteAllText(Path.Combine(output,"text-validation.json"),JsonSerializer.Serialize(reports,new JsonSerializerOptions{WriteIndented=true}));
 Console.WriteLine($"PASS: only {patchLabel} description in three Chinese/English FMGs updated and read back");return;
}
// Native dormant powers are goods of type 11. ItemTable category 1 resolves
// EquipParamGoods first; vagrantItemLotId then references PermanentBuffParam.
// Without this wrapper the reward UI falls back to an unknown ordinary good.
Archmage.WriteData(root,bnd,Parse);
var nativeGood=pars["EquipParamGoods"].Rows.Single(r=>r.ID==8000001);
Require(Convert.ToInt32(nativeGood["goodsType"].Value)==11&&Convert.ToInt32(nativeGood["vagrantItemLotId"].Value)==8000001&&Convert.ToInt32(nativeGood["isEquip"].Value)==0,"Native permanent-buff goods template changed");
foreach(var h in hexes){
 var s=Clone("SpEffectParam",8000000,h.Id);
 Set(s,"maxHpRate",1f);Set(s,"spCategory",20);Set(s,"iconId",h.SmallIcon);Set(s,"spEffectTextId_1",h.Id);Set(s,"displayIconForPermamentEffect",0);
 if(h.Id==95000200){foreach(var f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(s,f,.75f);foreach(var f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(s,f,1);foreach(var f in new[]{"addLifeForceStatus","addWillpowerStatus","addEndureStatus","addStrengthStatus","addDexterityStatus","addMagicStatus","addFaithStatus","addLuckStatus"})Set(s,f,10);}
 if(h.Id==95000300){foreach(var f in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(s,f,1.2f);foreach(var f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Set(s,f,1);foreach(var f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Set(s,f,10f);}
 if(h.Id==95001700){Set(s,"maxHpRate",1.5f);foreach(string field in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Set(s,field,field=="thrustDamageCutRate"?BalloonBalance.ThrustDamageRate:1.3f);}
 if(h.Id==ContractHeart.Contract)ContractHeart.Owners(s);
 if(h.Id==EssenceReaver.Owner)EssenceReaver.Owners(s);
 if(h.Id==DeathDance.Owner)DeathDance.Owners(s);
 if(h.Id==Erosion.Owner)Erosion.Owners(s);
 if(h.Id==DemonsCall.Owner)Set(s,"stateInfo",2278);
 if(h.Id==95000500)Set(s,"changeHpEstusFlaskCorrectRate",.5f);
 if(h.Id==95001400){Set(s,"artsConsumptionRate",0f);}
 if(h.Id==EchoCast.Owner)EchoCast.OwnerEffect(s);
 if(h.Id==Momentum.Owner)Momentum.OwnerEffect(s);
 if(h.Id==95002100){Set(s,"magicConsumptionRate",1f);Set(s,"miracleConsumptionRate",1f);}
 if(h.Id==95000900){
  var nativeMelee=pars["SpEffectParam"].Rows.Single(r=>r.ID==8320500);
  Require(Convert.ToInt32(nativeMelee["magicSubCategoryChange1"].Value)==130,"Native melee category changed");
  Set(s,"magicSubCategoryChange1",nativeMelee["magicSubCategoryChange1"].Value);
  foreach(var field in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Set(s,field,1.15f);
 }

 // Copy only silence fields, not the bush relic's near-zero sight range.
 if(h.Id==95000800){var silence=pars["SpEffectParam"].Rows.Single(r=>r.ID==1766);foreach(var f in new[]{"stateInfo","hearingSearchEnemyRate","vfxId"})Set(s,f,silence[f].Value);}
 var a=Clone("AttachEffectParam",8000000,h.Id);
 Set(a,"passiveSpEffectId_1",h.Id);Set(a,"attachTextId",h.Id);Set(a,"statusIconId",h.SmallIcon);
 Set(a,"compatibilityId",-1);Set(a,"overrideBaseEffectId",h.Id);Set(a,"overrideEffectId",-1);
 Set(a,"isNumericEffect",0);Set(a,"displayPercentageSymbol",0);Set(a,"displayedModifierValue",0);
 if(h.Id==95000900)Set(a,"passiveSpEffectId_2",95000912);
 // The DLL maintains the exact-f32 action marker; latent powers do not apply this extra metadata passive.
 if(h.Id==95001300)Set(a,"passiveSpEffectId_2",6999700);
 var p=Clone("PermanentBuffParam",8000000,h.Id);Set(p,"spEffectId",h.Id);Set(p,"iconId",h.LargeIcon);Set(p,"displayNumericValue",2);Set(p,"displayPercentageSymbol",0);Set(p,"numericValue",0);
 var good=Clone("EquipParamGoods",8000001,h.Id);Set(good,"vagrantItemLotId",h.Id);Set(good,"iconId",h.LargeIcon-1000);Set(good,"rarity",3);
 var table=Clone("ItemTableParam",4200000,RewardCatalog.Table(h.Id));Set(table,"disableParam_NT",0);Set(table,"itemId",h.Id);Set(table,"chanceWeight",1000);
}
Archmage.Markers((source,id)=>Clone("SpEffectParam",source,id));
Slingshot.Mechanics((source,id)=>Clone("SpEffectParam",source,id));
Parry.Support(Clone);
NightBlessing.ValidateNative(bnd,Parse);
NightBlessing.Mechanics(Clone);
RewardCycle.Parameters(Clone,RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length));
RewardReroll.Parameters(Clone);
EssenceReaver.Mechanics(Clone);
DeathDance.Mechanics(Clone);
Snack.Mechanics(Clone);
Momentum.Mechanics(Clone,pars["SpEffectParam"]);
Ambulance.RecoveryEffect(Clone,pars["SpEffectParam"]);
Erosion.Mechanics(Clone);
PotentialFission.Mechanics(Clone);
DemonsCall.Mechanics(Clone);
DemonsCall.Catalog(bnd,Parse,Path.Combine(root,"src/demons_data.rs"));
foreach(var weaponParam in new[]{"EquipParamCustomWeapon","EquipParamWeapon"})pars[weaponParam]=Parse(weaponParam,bnd.Files.Single(f=>Path.GetFileName(f.Name)==weaponParam+".param").Bytes);
PotentialFission.Generate(pars,Path.Combine(root,"src/fission_catalog.rs"));
foreach(var fissionParam in new[]{"ItemTableParam","ItemLotParam_enemy"})added[fissionParam].AddRange(pars[fissionParam].Rows.Where(r=>r.ID>=PotentialFission.FirstTable&&r.ID<99000000).Select(r=>r.ID));
EchoCast.Areas(root,Clone,pars);
ContractHeart.Mechanics(Clone);ContractHeart.Branches(pars["Bullet"]);ContractHeart.Lots(pars["ItemLotParam_map"],Clone);
var cageBosses=Cages.AddNpcs(root,pars,Clone,Set);
foreach(var m in CageEncounters.Load(root).groups.SelectMany(g=>g.members))foreach(string n in new[]{"NpcParam","NpcThinkParam"})added[n].Add(95300000+m.index);
Cages.MutationParameters(pars,Cages.TowerSources(root,cageBosses));
added["SpEffectParam"].Add(Cages.Mutation);
CageCoop.Parameters(pars,Clone);
var cageTerrain=Clone("SpEffectParam",8000000,Cages.Hollow);
Set(cageTerrain,"maxHpRate",1f);Set(cageTerrain,"saveCategory",-1);Set(cageTerrain,"iconId",-1);Set(cageTerrain,"spEffectTextId_1",-1);Set(cageTerrain,"displayIconForPermamentEffect",0);
// Exact f32 HKS ownership token and neutral short-lived action notifications.
foreach(int id in new[]{6999700,6999701,6999705}){
 var marker=Clone("SpEffectParam",8000000,id);Set(marker,"maxHpRate",1f);Set(marker,"saveCategory",-1);Set(marker,"iconId",-1);Set(marker,"spEffectTextId_1",-1);Set(marker,"displayIconForPermamentEffect",0);
 if(id!=6999700)Set(marker,"effectEndurance",.001f);
 if(id==6999700)Set(marker,"stateInfo",25013);
}
foreach(int id in new[]{6999800,6999801,6999802,6999803,6999804,6999805}){
 var marker=Clone("SpEffectParam",8000000,id);Set(marker,"maxHpRate",1f);Set(marker,"saveCategory",-1);Set(marker,"iconId",-1);Set(marker,"spEffectTextId_1",-1);Set(marker,"displayIconForPermamentEffect",0);
 if(id!=6999800&&id!=6999804)Set(marker,"effectEndurance",.001f);
 if(id==6999804){Set(marker,"spCategory",20);Set(marker,"effectEndurance",-1f);}
 if(id!=6999800)Set(marker,"isDisableNetSync",1);
}
Require(!pars["SpEffectParam"].Rows.Any(r=>r.ID!=6999700&&Convert.ToInt32(r["stateInfo"].Value)==25013),"Jiahao private TAE gate state is already used");
// Hidden per-expedition receipts use the native permanent ability save category.
foreach(int id in Enumerable.Range(95001000,5).Concat(Enumerable.Range(95001010,4))){
 var marker=Clone("SpEffectParam",8000000,id);Set(marker,"maxHpRate",1f);Set(marker,"iconId",-1);Set(marker,"spEffectTextId_1",-1);
 if(id==95001004){Set(marker,"saveCategory",-1);Set(marker,"effectEndurance",1.5f);}
}
// Independent finite effect: healing does not end the ten-second window.
var nativeLowHp=pars["SpEffectParam"].Rows.Single(r=>r.ID==8660100);
Require(Convert.ToSingle(nativeLowHp["conditionHp"].Value)==40f&&Convert.ToSingle(nativeLowHp["conditionHpRate"].Value)==-1f,"Native low-HP threshold changed");
var escape=Clone("SpEffectParam",8000000,95001201);
Set(escape,"maxHpRate",1f);Set(escape,"saveCategory",-1);Set(escape,"effectEndurance",10f);Set(escape,"iconId",-1);Set(escape,"displayIconForPermamentEffect",0);Set(escape,"spEffectTextId_1",95001201);
Set(escape,"vfxId",nativeLowHp["vfxId"].Value);
var bloodboilGoods=pars["EquipParamGoods"].Rows.Single(r=>r.ID==3550);
Require(Convert.ToInt32(bloodboilGoods["refCategory"].Value)==1&&Convert.ToInt32(bloodboilGoods["refId_default"].Value)==10355000,"Bloodboil goods reference changed");
Require(Convert.ToInt32(pars["Bullet"].Rows.Single(r=>r.ID==10355000)["spEffectId0"].Value)==503550,"Bloodboil bullet reference changed");
var escapeVisual=Clone("SpEffectParam",8000000,95001204);
Set(escapeVisual,"maxHpRate",1f);Set(escapeVisual,"saveCategory",-1);Set(escapeVisual,"effectEndurance",10f);Set(escapeVisual,"spCategory",20);
Set(escapeVisual,"iconId",-1);Set(escapeVisual,"spEffectTextId_1",-1);Set(escapeVisual,"displayIconForPermamentEffect",0);Set(escapeVisual,"vfxId",5280);
EscapeVfx.Apply(pars["SpEffectParam"],pars["SpEffectVfxParam"],pars["Bullet"]);
added["SpEffectVfxParam"].Add(EscapeVfx.Visual);
foreach(var f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Set(escape,f,.4f);
var escapeCooldown=Clone("SpEffectParam",8000000,95001202);
Set(escapeCooldown,"maxHpRate",1f);Set(escapeCooldown,"effectEndurance",60f);Set(escapeCooldown,"iconId",-1);Set(escapeCooldown,"displayIconForPermamentEffect",0);Set(escapeCooldown,"spEffectTextId_1",95001202);
var sixthSense=pars["SpEffectParam"].Rows.Single(r=>r.ID==700600);
Require(Convert.ToInt32(sixthSense["displayIconForPermamentEffect"].Value)==1,"Native Sixth Sense HUD flag changed");
var escapeReady=Clone("SpEffectParam",8000000,95001203);
Set(escapeReady,"maxHpRate",1f);Set(escapeReady,"saveCategory",-1);Set(escapeReady,"iconId",20222);Set(escapeReady,"displayIconForPermamentEffect",sixthSense["displayIconForPermamentEffect"].Value);Set(escapeReady,"spEffectTextId_1",95001203);
Set(escapeReady,"spCategory",20);Set(escapeReady,"effectEndurance",2f);
Require(Convert.ToSingle(escape["conditionHp"].Value)==-1f&&Convert.ToInt32(escape["stateInfo"].Value)==0,"Escape must not inherit a conditional damage/animation state");
var paid=Clone("SpEffectParam",8000000,95000101);Set(paid,"maxHpRate",1f);Set(paid,"iconId",-1);Set(paid,"spEffectTextId_1",-1);
var money=Clone("SpEffectParam",3269,95000102);Set(money,"soul",88888);
RedEnvelope.Apply(pars["SpEffectParam"],pars["SpEffectVfxParam"]);
RedEnvelope.Rewards(Clone);
// Retain the old refill row for compatibility; the DLL uses the native
// percentage API in 0.2.2. Only the saved cooldown remains active here.
var refill=Clone("SpEffectParam",99823,95000401);
var cooldown=Clone("SpEffectParam",8000000,95000402);Set(cooldown,"maxHpRate",1f);Set(cooldown,"effectEndurance",45f);Set(cooldown,"iconId",-1);Set(cooldown,"displayIconForPermamentEffect",0);Set(cooldown,"spEffectTextId_1",95000400);
UltimateRefreshVfx.Apply(pars["SpEffectParam"],pars["SpEffectVfxParam"]);
var ultimateReady=Clone("SpEffectParam",8000000,95000403);Set(ultimateReady,"maxHpRate",1f);Set(ultimateReady,"saveCategory",-1);Set(ultimateReady,"iconId",20297);Set(ultimateReady,"displayIconForPermamentEffect",1);Set(ultimateReady,"spEffectTextId_1",95000403);
Set(ultimateReady,"spCategory",20);Set(ultimateReady,"effectEndurance",2f);
// Native finite-effect HUD merges the same icon and counts live instances.
// Keep ordinary instance counting: Dart's flag instead reads ID % 100.
var drinking=Clone("SpEffectParam",8000000,95000501);Set(drinking,"maxHpRate",1f);Set(drinking,"effectEndurance",15f);Set(drinking,"saveCategory",-1);Set(drinking,"iconId",20006);Set(drinking,"spEffectTextId_1",95000500);
// Native "Improved Attack Power at Full HP" uses AttackRate, not
// AttackPowerRate. Match that terminology and its spell applicability flags,
// without copying the HP condition or the native visual.
var fullHpAttack=pars["SpEffectParam"].Rows.Single(r=>r.ID==8670000);
var drinkingRates=new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"};
var drinkingPowerRates=new[]{"physicsAttackPowerRate","magicAttackPowerRate","fireAttackPowerRate","thunderAttackPowerRate","darkAttackPowerRate"};
foreach(var f in drinkingRates){Require(Convert.ToSingle(fullHpAttack[f].Value)==1.07f,"Native full-HP damage multiplier changed");Set(drinking,f,1.05f);}
foreach(var f in drinkingPowerRates){Require(Convert.ToSingle(fullHpAttack[f].Value)==1f,"Native full-HP attack-power multiplier changed");Set(drinking,f,1f);}
foreach(var f in new[]{"magParamChange","miracleParamChange","shamanParamChange"}){Require(Convert.ToInt32(fullHpAttack[f].Value)==1,"Native full-HP spell applicability changed");Set(drinking,f,fullHpAttack[f].Value);}
// Separate finite native effects implement multiplicative stacking without
// depending on same-ID application semantics. Runtime recycles expired slots.
for(int slot=1;slot<80;slot++)Clone("SpEffectParam",95000501,95005000+slot);
// One neutral visual shared by all live stacks: avoid adding the native
// full-HP row's extra attack multiplier or layering 80 identical auras.
var drinkingGlow=Clone("SpEffectParam",8000000,95000509);
Set(drinkingGlow,"maxHpRate",1f);Set(drinkingGlow,"saveCategory",-1);Set(drinkingGlow,"iconId",-1);Set(drinkingGlow,"spEffectTextId_1",-1);
Require(Convert.ToInt32(fullHpAttack["vfxId"].Value)==320500,"Native full-HP visual changed");
Set(drinkingGlow,"vfxId",fullHpAttack["vfxId"].Value);
var regen=Clone("SpEffectParam",3269,95000601);Set(regen,"soul",0);Set(regen,"changeHpPoint",-16);Set(regen,"changeMpPoint",4);
var fpFlask=Clone("SpEffectParam",3269,95000602);Set(fpFlask,"soul",0);Set(fpFlask,"changeHpPoint",0);Set(fpFlask,"changeMpPoint",0);Set(fpFlask,"changeMpRate",-50);
var nativeFpFlask=pars["SpEffectParam"].Rows.Single(r=>r.ID==8885003);
Require(Convert.ToInt32(nativeFpFlask["vfxId"].Value)==5891,"Native flask FP visual changed");
Set(fpFlask,"vfxId",nativeFpFlask["vfxId"].Value);
var boon=pars["SpEffectParam"].Rows.Single(r=>r.ID==1643000);
Require(Convert.ToInt32(boon["vfxId"].Value)==1643000,"Blessing's Boon visual changed");
var regenGlow=Clone("SpEffectParam",8000000,95000603);
Set(regenGlow,"maxHpRate",1f);Set(regenGlow,"saveCategory",-1);Set(regenGlow,"iconId",-1);Set(regenGlow,"spEffectTextId_1",-1);Set(regenGlow,"vfxId",boon["vfxId"].Value);
// Acquisition completion survives a map reload, like Red Envelope's paid row.
var flaskPaid=Clone("SpEffectParam",8000000,95000502);Set(flaskPaid,"maxHpRate",1f);Set(flaskPaid,"iconId",-1);Set(flaskPaid,"spEffectTextId_1",-1);
foreach(var id in Enumerable.Range(95000520,21).Concat(new[]{95000504,95000505,95000506,95000507,95000508})){
 var marker=Clone("SpEffectParam",8000000,id);Set(marker,"maxHpRate",1f);Set(marker,"iconId",-1);Set(marker,"spEffectTextId_1",-1);Set(marker,"saveCategory",-1);
 if(id!=95000505&&id<95000520)Set(marker,"effectEndurance",0f);
}
// Visual only. Native 8610702 deletes the shell on a damage event; copy
// only that cleanup criterion, never its mitigation or attack accumulator.
// Absorption itself uses the native per-hit attack-nullify bit in the DLL.
Require(Convert.ToInt32(pars["SpEffectParam"].Rows.Single(r=>r.ID==8610702)["deleteCriteriaDamage"].Value)==1,"Native one-hit shell criterion changed");
var shell=Clone("SpEffectParam",8000000,95000701);Set(shell,"maxHpRate",1f);Set(shell,"effectEndurance",-1f);Set(shell,"deleteCriteriaDamage",1);Set(shell,"isContractSpEffectLife",1);Set(shell,"saveCategory",-1);Set(shell,"iconId",20408);Set(shell,"spEffectTextId_1",95000701);Set(shell,"vfxId",pars["SpEffectParam"].Rows.Single(r=>r.ID==3507)["vfxId"].Value);
Set(shell,"spCategory",20);Set(shell,"effectEndurance",2f);
var shellCooldown=Clone("SpEffectParam",8000000,95000702);Set(shellCooldown,"maxHpRate",1f);Set(shellCooldown,"effectEndurance",40f);Set(shellCooldown,"iconId",-1);Set(shellCooldown,"spEffectTextId_1",-1);
// Native Finale state 600 is consumed by rendering/targeting code. Keep this
// short effect outside category 201 so it cannot replace Duchess's 15s Art.
var invisible=Clone("SpEffectParam",703500,95000801);Set(invisible,"effectEndurance",1f);Set(invisible,"saveCategory",-1);Set(invisible,"spCategory",20);Set(invisible,"categoryPriority",0);Set(invisible,"spEffectTextId_1",95000800);
// A separate exactly representable f32 ID is queried by the surgical HKS
// casting guard. The permanent ability's native int ID remains 95000900.
var castOwner=Clone("SpEffectParam",8000000,95000912);Set(castOwner,"maxHpRate",1f);Set(castOwner,"saveCategory",-1);Set(castOwner,"iconId",-1);Set(castOwner,"spEffectTextId_1",-1);SwordRegain.Apply(castOwner);
var castPulse=Clone("SpEffectParam",8000000,95000904);
Set(castPulse,"maxHpRate",1f);Set(castPulse,"effectEndurance",0f);Set(castPulse,"saveCategory",-1);Set(castPulse,"iconId",-1);Set(castPulse,"spEffectTextId_1",-1);
SwordVfx.Build(Clone);
var castGuard=Clone("SpEffectParam",8330902,95000908);
Set(castGuard,"effectEndurance",-1f);Set(castGuard,"saveCategory",-1);Set(castGuard,"categoryPriority",0);Set(castGuard,"iconId",-1);Set(castGuard,"secondaryIconId",-1);Set(castGuard,"spEffectTextId_1",-1);
foreach(var field in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Set(castGuard,field,1f);
// Native 507C40 only reads reaction remaps from category 1001, choosing the
// lowest priority. Keep this separate from category 20 casting protection.
var castPoise=Clone("SpEffectParam",5300,95000916);
Set(castPoise,"saveCategory",-1);Set(castPoise,"categoryPriority",0);Set(castPoise,"spEffectTextId_1",-1);
Set(castPoise,"defFlickPower",pars["SpEffectParam"].Rows.Single(r=>r.ID==1650)["defFlickPower"].Value);
Set(castPoise,"saReceiveDamageRate",pars["SpEffectParam"].Rows.Single(r=>r.ID==45)["saReceiveDamageRate"].Value);
var healVisual=Clone("SpEffectParam",8630001,95000910);Set(healVisual,"changeHpPoint",0);Set(healVisual,"changeMpPoint",0);Set(healVisual,"saveCategory",-1);
// Native instant HP pulses in binary denominations avoid writing HP memory or
// guessing a native HP-mutator ABI. Every queued heal is consumed only once.
for(int bit=0;bit<16;bit++){
 var heal=Clone("SpEffectParam",3269,95000920+bit);Set(heal,"soul",0);Set(heal,"changeHpPoint",-(1<<bit));Set(heal,"changeMpPoint",0);Set(heal,"vfxId",-1);
}
// Private copy of the native 8630500 lava chain: preserve phases, SFX,
// collision and scattering. The placement seed has no damage; the eruption
// uses base 100 fire attack; remaining patches use base 20 (native mitigation).
var lavaEmit=Clone("SpEffectParam",8630502,95001501);
Set(lavaEmit,"behaviorId",950015000);Set(lavaEmit,"replaceSpEffectId",-1);
Set(lavaEmit,"saveCategory",-1);Set(lavaEmit,"iconId",-1);Set(lavaEmit,"spEffectTextId_1",-1);
var lavaBehavior=Clone("BehaviorParam_PC",863050000,950015000);Set(lavaBehavior,"refId",95001550);
for(int i=0;i<7;i++){
 var attack=Clone("AtkParam_Pc",12048210+i,95001560+i);
 Set(attack,"atkFire",i==4?200:i==6?40:0);
 Require(Convert.ToInt32(attack["isAddBaseAtk"].Value)==0,"Lava must use flat native attack, not held-weapon attack");
 Require(Convert.ToInt32(attack["selfTarget"].Value)==0&&Convert.ToInt32(attack["friendlyTarget"].Value)==0,"Lava source/friendly targeting mismatch");
}
for(int i=0;i<8;i++){
 var bullet=Clone("Bullet",12482050+i,95001550+i);
 Set(bullet,"atkId_Bullet",95001560+(i<=4?i:i==7?6:5));
 Set(bullet,"HitBulletID",i==7?-1:95001551+i);
}
NewHexes.AddMechanics(pars,Clone,Path.Combine(root,"src"));
DevilsGaze.Mechanics(pars,Clone);
foreach(int id in new[]{6999920,6999921}){
 var row=Clone("SpEffectParam",8000000,id);Set(row,"maxHpRate",1f);Set(row,"saveCategory",-1);Set(row,"spCategory",20);Set(row,"iconId",-1);Set(row,"spEffectTextId_1",-1);Set(row,"displayIconForPermamentEffect",0);Set(row,"effectEndurance",id==6999920?-1f:.001f);
}
// One native three-choice template; the DLL fills the selected goods before creation.
var rewardEntries=new[]{new RewardCatalog.Entry(RewardCatalog.TemplateLot,new[]{0,1,2})};
var orders=rewardEntries.Select(e=>e.Order).ToArray();
Require(orders.Length==1,"Expected one native reward template");
RewardCatalog.Write(Path.Combine(root,"src/hex_pool.rs"),hexes);
foreach(var entry in RewardCatalog.Template(hexes))for(int slot=0;slot<3;slot++){
 var r=Clone("ItemLotParam_enemy",6100002,entry.Lot+slot);Set(r,"lotItemId01",RewardCatalog.Table(entry.Owners[slot]));Set(r,"lotItem_Rarity",3);
}
bool Eq(PARAM.Row a,PARAM.Row b)=>a.ID==b.ID&&a.Name==b.Name&&a.Cells.Count==b.Cells.Count&&a.Cells.Zip(b.Cells).All(v=>v.First==null?v.Second==null:v.Second!=null&&(v.First.Value is byte[] ba?v.Second.Value is byte[] bb&&ba.SequenceEqual(bb):Equals(v.First.Value,v.Second.Value)));
foreach(var n in names){
 pars[n].Rows=pars[n].Rows.OrderBy(r=>r.ID).ToList();files[n].Bytes=pars[n].Write();
 var check=Parse(n,files[n].Bytes);var before=Parse(n,original[files[n].ID]);
 Require(check.Rows.Count==before.Rows.Count+added[n].Count,"Unexpected row count in "+n);
 var old=check.Rows.Where(r=>!added[n].Contains(r.ID)).ToArray();
 Require(old.Length==before.Rows.Count&&old.Zip(before.Rows).All(v=>{
  if(n=="Bullet"&&new[]{10335000,13335000,13335010}.Contains(v.First.ID)){
   var copy=new PARAM.Row(v.First);Require(Convert.ToInt32(copy["spEffectIDForShooter"].Value)==ContractHeart.Used,"Branch shooter notification missing");Set(copy,"spEffectIDForShooter",v.Second["spEffectIDForShooter"].Value);return Eq(copy,v.Second);
  }return Eq(v.First,v.Second);
 }),"Original rows changed in "+n);
 foreach(var id in added[n].Distinct()){var expected=pars[n].Rows.Where(r=>r.ID==id).ToArray();var actual=check.Rows.Where(r=>r.ID==id).ToArray();Require(expected.Length==actual.Length&&expected.Zip(actual).All(v=>Eq(v.First,v.Second)),"Added row failed round-trip: "+n+" "+id);}
 if(added[n].Count==0&&n!="Bullet")files[n].Bytes=original[files[n].ID];
}
// Check the serialized reward graph, including the actual UI classification.
var serialized=files.ToDictionary(k=>k.Key,k=>Parse(k.Key,k.Value.Bytes));
EchoCast.VerifyAreas(root,serialized);
for(int g=0;g<orders.Length;g++)for(int slot=0;slot<3;slot++){
 var lot=serialized["ItemLotParam_enemy"].Rows.Single(r=>r.ID==rewardEntries[g].Lot+slot);
 Require(Convert.ToInt32(lot["lotItemCategory01"].Value)==7&&Convert.ToInt32(lot["lotItemId01"].Value)==RewardCatalog.Table(hexes[orders[g][slot]].Id)&&Convert.ToInt32(lot["lotItemBasePoint01"].Value)==1000,"Serialized choice pool mismatch");
 Require(Convert.ToInt32(lot["lotItem_Rarity"].Value)==3&&Convert.ToInt32(lot["GameClearOffset"].Value)==-1,"Personal gold reward lot rarity/variant mismatch");
}
using var iconCatalog=JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"data/reference/ui/icon-ids.json")));
var availableIcons=iconCatalog.RootElement.EnumerateArray().Select(e=>(kind:e.GetProperty("kind").GetString(),id:e.GetProperty("id").GetInt32())).ToHashSet();
foreach(var h in hexes){
 var table=serialized["ItemTableParam"].Rows.Single(r=>r.ID==RewardCatalog.Table(h.Id));
 Require(Convert.ToInt32(table["itemCategory"].Value)==1&&Convert.ToInt32(table["itemId"].Value)==h.Id,"Reward table does not resolve the expected good");
 var good=serialized["EquipParamGoods"].Rows.Single(r=>r.ID==h.Id);
 Require(Convert.ToInt32(good["goodsType"].Value)==11&&Convert.ToInt32(good["isEquip"].Value)==0&&Convert.ToInt32(good["vagrantItemLotId"].Value)==h.Id,"Reward is not a directly acquired permanent ability");
 Require(Convert.ToInt32(good["rarity"].Value)==3,"Dormant power choice must use native legendary/gold rarity");
 Require(Convert.ToInt32(good["iconId"].Value)==h.LargeIcon-1000,"Reward goods icon category mismatch");
 Require(availableIcons.Contains(("item",h.LargeIcon)),"Large icon is missing from the original item atlas");
 Require(availableIcons.Contains(("status",h.SmallIcon)),"Small icon is missing from the original status atlas");
 var perm=serialized["PermanentBuffParam"].Rows.Single(r=>r.ID==h.Id);
 Require(Convert.ToInt32(perm["spEffectId"].Value)==h.Id&&serialized["SpEffectParam"].Rows.Any(r=>r.ID==h.Id),"Permanent ability has no effect");
 Require(Convert.ToInt32(perm["iconId"].Value)==h.LargeIcon&&Convert.ToInt32(serialized["AttachEffectParam"].Rows.Single(r=>r.ID==h.Id)["statusIconId"].Value)==h.SmallIcon,"Serialized large/small icon mapping mismatch");
 var owner=serialized["SpEffectParam"].Rows.Single(r=>r.ID==h.Id);
 Require(Convert.ToInt32(owner["iconId"].Value)==h.SmallIcon&&Convert.ToInt32(owner["displayIconForPermamentEffect"].Value)==0,"Permanent icon mapping or HUD visibility mismatch");
}
var effects=serialized["SpEffectParam"];
PARAM.Row Effect(int id)=>effects.Rows.Single(r=>r.ID==id);
for(int id=95001000;id<95001004;id++)Require(Convert.ToInt32(Effect(id)["saveCategory"].Value)==10&&Convert.ToSingle(Effect(id)["effectEndurance"].Value)==-1f&&Convert.ToSingle(Effect(id)["maxHpRate"].Value)==1f&&Convert.ToInt32(Effect(id)["iconId"].Value)==-1,"Level receipt must be neutral, hidden and natively saved");
Require(Convert.ToInt32(serialized["AttachEffectParam"].Rows.Single(r=>r.ID==95000900)["passiveSpEffectId_2"].Value)==95000912,"Sword HKS marker is not attached");
Require(Convert.ToInt32(serialized["SpEffectParam"].Rows.Single(r=>r.ID==95000912)["stateInfo"].Value)==449,"Sword native regain state missing");
Require(Convert.ToInt32(Effect(95000900)["magicSubCategoryChange1"].Value)==130,"Sword permanent melee bonus must use native category 130");
foreach(var field in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Require(Convert.ToSingle(Effect(95000900)[field].Value)==1.15f,"Sword melee bonus must be exactly 15%");

Require(Convert.ToSingle(Effect(95000904)["effectEndurance"].Value)==0f&&Convert.ToSingle(Effect(95000904)["maxHpRate"].Value)==1f,"Sword casting marker must be instant");
Require(Convert.ToSingle(Effect(95000908)["effectEndurance"].Value)==-1f&&Convert.ToInt32(Effect(95000908)["saveCategory"].Value)==-1&&Convert.ToInt32(Effect(95000908)["stateInfo"].Value)==0,"Sword protection must last until explicitly removed");
foreach(var field in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Require(Convert.ToSingle(Effect(95000908)[field].Value)==1f,"Sword casting VFX row must be neutral; the active spell supplies reduction");
foreach(var field in new[]{"physicsAttackRate","magicAttackRate","fireAttackRate","thunderAttackRate","darkAttackRate"})Require(Convert.ToSingle(Effect(8821000)[field].Value)==.01f,"Native glancing damage reference changed");
Require(Convert.ToInt32(Effect(95000908)["vfxId"].Value)==Convert.ToInt32(Effect(8330902)["vfxId"].Value),"Sword casting VFX differs from native casting reduction");
Require(Convert.ToInt32(Effect(95000916)["spCategory"].Value)==1001&&Convert.ToInt32(Effect(95000916)["categoryPriority"].Value)==0&&Convert.ToSingle(Effect(95000916)["saReceiveDamageRate"].Value)==0f&&Convert.ToSingle(Effect(95000916)["effectEndurance"].Value)==-1f&&Convert.ToInt32(Effect(95000916)["saveCategory"].Value)==-1,"Sword native poise category/priority/lifetime mismatch");
foreach(var field in new[]{"dmgLv_None","dmgLv_S","dmgLv_M","dmgLv_L","dmgLv_BlowM","dmgLv_Push","dmgLv_Strike","dmgLv_BlowS","dmgLv_Min","dmgLv_Uppercut","dmgLv_BlowLL","dmgLv_Breath"})Require(Equals(Effect(95000916)[field].Value,Effect(5300)[field].Value),"Sword no-stagger remap differs from native");
Require(Convert.ToInt32(Effect(95000910)["vfxId"].Value)==Convert.ToInt32(Effect(8630001)["vfxId"].Value)&&Convert.ToInt32(Effect(95000910)["changeHpPoint"].Value)==0&&Convert.ToInt32(Effect(95000910)["changeMpPoint"].Value)==0&&Convert.ToSingle(Effect(95000910)["effectEndurance"].Value)==0f,"Sword HP-only VFX must not also restore HP/FP");
for(int bit=0;bit<16;bit++)Require(Convert.ToInt32(Effect(95000920+bit)["changeHpPoint"].Value)==-(1<<bit)&&Convert.ToInt32(Effect(95000920+bit)["changeMpPoint"].Value)==0&&Convert.ToInt32(Effect(95000920+bit)["soul"].Value)==0&&Convert.ToSingle(Effect(95000920+bit)["effectEndurance"].Value)==0f,"Sword native healing denomination mismatch");
foreach(var f in drinkingRates)Require(Convert.ToSingle(Effect(95000200)[f].Value)==.75f,"Astral Body damage multiplier must be 0.75");
foreach(var f in drinkingPowerRates)Require(Convert.ToSingle(Effect(95000200)[f].Value)==1f,"Astral Body must not also change pre-defense attack power");
foreach(var f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Require(Convert.ToInt32(Effect(95000200)[f].Value)==1,"Astral Body spell applicability mismatch");
foreach(var f in new[]{"addLifeForceStatus","addWillpowerStatus","addEndureStatus","addStrengthStatus","addDexterityStatus","addMagicStatus","addFaithStatus","addLuckStatus"})Require(Convert.ToInt32(Effect(95000200)[f].Value)==10,"Astral Body must grant all eight attributes +10");
foreach(var f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Require(Convert.ToSingle(Effect(95000300)[f].Value)==10f,"Glass Cannon must take 1000% of normal damage, not +1000%");
foreach(var f in drinkingRates)Require(Convert.ToSingle(Effect(95000300)[f].Value)==1.2f,"Glass Cannon damage multiplier must be 1.2");
foreach(var f in drinkingPowerRates)Require(Convert.ToSingle(Effect(95000300)[f].Value)==1f,"Glass Cannon must not also change pre-defense attack power");
foreach(var f in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Require(Convert.ToInt32(Effect(95000300)[f].Value)==1,"Glass Cannon spell applicability mismatch");
Require(Convert.ToSingle(Effect(95000401)["ultimateArtGauge"].Value)==100f&&Convert.ToSingle(Effect(95000401)["effectEndurance"].Value)==0f,"Ultimate pulse is not an instant +100 refill");
Require(Convert.ToSingle(Effect(95000402)["effectEndurance"].Value)==45f,"Ultimate cooldown is not 45 seconds");
Require(Convert.ToInt32(Effect(95000400)["iconId"].Value)==22297&&Convert.ToInt32(Effect(95000400)["displayIconForPermamentEffect"].Value)==0&&Convert.ToInt32(Effect(95000402)["iconId"].Value)==-1&&Convert.ToInt32(Effect(95000402)["displayIconForPermamentEffect"].Value)==0&&Convert.ToInt32(Effect(95000403)["iconId"].Value)==20297&&Convert.ToInt32(Effect(95000403)["displayIconForPermamentEffect"].Value)==1,"Ultimate must have its special-effects icon and only its ready HUD icon");
Require(Convert.ToInt32(Effect(95000800)["stateInfo"].Value)==54&&Convert.ToSingle(Effect(95000800)["hearingSearchEnemyRate"].Value)==0f&&Convert.ToSingle(Effect(95000800)["sightSearchEnemyRate"].Value)==1f,"Cat silence must not introduce permanent invisibility");
Require(Convert.ToInt32(Effect(95000801)["stateInfo"].Value)==600&&Convert.ToSingle(Effect(95000801)["effectEndurance"].Value)==1f&&Convert.ToInt32(Effect(95000801)["vfxId"].Value)==503500&&Convert.ToInt32(Effect(95000801)["spCategory"].Value)==20,"Cat invisibility must be one second and must not replace native Art category");
foreach(var id in new[]{95000808,95000816,95000824,95000832})Require(!effects.Rows.Any(r=>r.ID==id),"Wall diagnostic effects must be removed");
Require(Convert.ToSingle(Effect(95000500)["changeHpEstusFlaskCorrectRate"].Value)==.5f&&Convert.ToSingle(Effect(95000501)["effectEndurance"].Value)==15f,"Drinking duration/healing multiplier mismatch");
foreach(var field in drinkingRates)Require(Convert.ToSingle(Effect(95000501)[field].Value)==1.05f,"Drinking native-style attack multiplier mismatch");
foreach(var field in drinkingPowerRates)Require(Convert.ToSingle(Effect(95000501)[field].Value)==1f,"Drinking must not change pre-defense attack-power multipliers");
foreach(var field in new[]{"magParamChange","miracleParamChange","shamanParamChange"})Require(Convert.ToInt32(Effect(95000501)[field].Value)==1,"Drinking spell applicability mismatch");
Require(Convert.ToSingle(Effect(95000501)["conditionHpRate"].Value)==-1f,"Drinking must not require full HP");
Require(Convert.ToInt32(Effect(95000501)["iconId"].Value)==20006&&Convert.ToInt32(Effect(95000501)["dispDartDebuffStacks"].Value)==0,"Drinking must use native attack-up HUD instance counting");
for(int slot=1;slot<80;slot++){
 var row=Effect(95005000+slot);
 Require(row.Cells.Zip(Effect(95000501).Cells).All(v=>Equals(v.First.Value,v.Second.Value)),"Independent drinking stack differs from the first layer");
}
Require(Convert.ToInt32(Effect(95000509)["vfxId"].Value)==320500&&Convert.ToSingle(Effect(95000509)["conditionHpRate"].Value)==-1f&&Convert.ToSingle(Effect(95000509)["effectEndurance"].Value)==-1f,"Drinking visual must stay active without a full-HP condition");
foreach(var field in drinkingRates.Concat(drinkingPowerRates))Require(Convert.ToSingle(Effect(95000509)[field].Value)==1f,"Drinking visual must not add damage");
Require(Convert.ToInt32(Effect(95000601)["changeHpPoint"].Value)==-16&&Convert.ToInt32(Effect(95000601)["changeMpPoint"].Value)==4&&Convert.ToInt32(Effect(95000601)["soul"].Value)==0&&Convert.ToSingle(Effect(95000601)["effectEndurance"].Value)==0f,"Regen HP/FP pulse mismatch");
Require(Convert.ToInt32(Effect(95000603)["vfxId"].Value)==1643000&&Convert.ToSingle(Effect(95000603)["effectEndurance"].Value)==-1f&&Convert.ToInt32(Effect(95000603)["saveCategory"].Value)==-1&&Convert.ToInt32(Effect(95000603)["stateInfo"].Value)==0,"Regen visual must be neutral Blessing's Boon VFX");
foreach(var f in new[]{"changeHpPoint","changeHpRate","changeMpPoint","changeMpRate"})Require(Convert.ToSingle(Effect(95000603)[f].Value)==0f,"Regen visual must not itself heal or spend FP");
foreach(var f in drinkingRates.Concat(drinkingPowerRates))Require(Convert.ToSingle(Effect(95000603)[f].Value)==1f,"Regen visual must not change damage");
Require(Convert.ToInt32(Effect(95000602)["changeMpRate"].Value)==-50&&Convert.ToInt32(Effect(95000602)["changeHpPoint"].Value)==0&&Convert.ToInt32(Effect(95000602)["changeMpPoint"].Value)==0&&Convert.ToInt32(Effect(95000602)["soul"].Value)==0&&Convert.ToSingle(Effect(95000602)["effectEndurance"].Value)==0f,"Full-HP flask FP pulse mismatch");
Require(Convert.ToInt32(Effect(95000602)["vfxId"].Value)==5891,"Full-HP flask must display native FP restoration visual");
Require(Convert.ToSingle(Effect(95000702)["effectEndurance"].Value)==40f&&Convert.ToInt32(Effect(95000702)["saveCategory"].Value)==10,"Shell cooldown must last 40 seconds and use the native saved-effect category");
Require(Convert.ToInt32(Effect(95000701)["vfxId"].Value)==5866&&Convert.ToSingle(Effect(95000701)["effectEndurance"].Value)==2f&&Convert.ToInt32(Effect(95000701)["stateInfo"].Value)==0&&Convert.ToInt32(Effect(95000701)["deleteCriteriaDamage"].Value)==1&&Convert.ToInt32(Effect(95000701)["isContractSpEffectLife"].Value)==1,"Shell must retain native one-hit visual removal with native leased renewal and unchanged hit criteria");
foreach(var field in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Require(Convert.ToSingle(Effect(95000701)[field].Value)==1f&&Convert.ToSingle(Effect(95000700)[field].Value)==1f,"Shell rows must not change damage multipliers");
// Validate actual serialized chain, including visual-only growth's untouched
// radii. Runtime never mutates these collision values.
foreach(int i in Enumerable.Range(0,8)){
 var row=serialized["Bullet"].Rows.Single(r=>r.ID==95001550+i);var sourceRow=pars["Bullet"].Rows.Single(r=>r.ID==12482050+i);
 foreach(var f in new[]{"life","hitRadius","hitRadiusMax","dmgHitRecordLifeTime","sfxId_Bullet","sfxId_Hit","sfxId_Flick"})Require(Equals(row[f].Value,sourceRow[f].Value),"Lava phase/collision/SFX mismatch: "+i+" "+f);
}
Require(Convert.ToInt32(Effect(95001501)["stateInfo"].Value)==275&&Convert.ToInt32(Effect(95001501)["replaceSpEffectId"].Value)==-1&&Convert.ToInt32(Effect(95001501)["behaviorId"].Value)==950015000,"Lava emission must use private native behavior without the native kill accumulator reset");
Require(!effects.Rows.Any(r=>r.ID==95001502),"Lava must rearm on native cleanup without a fixed cooldown effect");
Directory.CreateDirectory(Path.Combine(root,".work/noisy"));
File.WriteAllText(Path.Combine(root,".work/noisy/serialized.json"),JsonSerializer.Serialize(new{effects=new[]{95001501}.Select(id=>new{id,fields=Effect(id).Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),bullets=serialized["Bullet"].Rows.Where(r=>r.ID>=95001550&&r.ID<=95001557).Select(r=>new{id=r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),attacks=serialized["AtkParam_Pc"].Rows.Where(r=>r.ID>=95001560&&r.ID<=95001566).Select(r=>new{id=r.ID,fields=r.Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)})}));
var escapeIds=new[]{95001200,95001201,95001202,95001203,95001204};
foreach(int id in escapeIds)Require(Convert.ToSingle(Effect(id)["maxHpRate"].Value)==1f,"Escape adds an unintended max-HP bonus");
Require(Convert.ToSingle(Effect(95001201)["effectEndurance"].Value)==10f&&Convert.ToSingle(Effect(95001202)["effectEndurance"].Value)==60f,"Escape duration/cooldown mismatch");
Require(Convert.ToInt32(Effect(95001204)["vfxId"].Value)==EscapeVfx.Visual&&Convert.ToSingle(Effect(95001204)["effectEndurance"].Value)==0.2f&&Convert.ToInt32(Effect(95001204)["saveCategory"].Value)==-1&&Convert.ToInt32(Effect(95001204)["iconId"].Value)==-1&&Convert.ToInt32(Effect(95001204)["stateInfo"].Value)==0,"Escape Restage cue must be neutral, hidden and transient");
foreach(var f in new[]{"slashDamageCutRate","blowDamageCutRate","thrustDamageCutRate","neutralDamageCutRate","magicDamageCutRate","fireDamageCutRate","thunderDamageCutRate","darkDamageCutRate"})Require(Convert.ToSingle(Effect(95001201)[f].Value)==.4f,"Escape must reduce all damage types by 60%");
Require(Convert.ToInt32(Effect(95001203)["iconId"].Value)==20222&&Convert.ToInt32(Effect(95001203)["displayIconForPermamentEffect"].Value)==1&&Convert.ToInt32(Effect(95001203)["saveCategory"].Value)==-1,"Escape ready HUD icon mismatch");
File.WriteAllText(Path.Combine(root,".work/effects/escape-serialized.json"),JsonSerializer.Serialize(escapeIds.Select(id=>new{id,fields=Effect(id).Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),new JsonSerializerOptions{WriteIndented=true}));
File.WriteAllText(Path.Combine(root,".work/actions/jiahao-serialized.json"),JsonSerializer.Serialize(new[]{95001300,6999700,6999701,6999705}.Select(id=>new{id,fields=Effect(id).Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),new JsonSerializerOptions{WriteIndented=true}));
Require(Convert.ToSingle(Effect(95002100)["magicConsumptionRate"].Value)==1f&&Convert.ToSingle(Effect(95002100)["miracleConsumptionRate"].Value)==1f,"Archmage owner must leave primary spell FP consumption unchanged");
Require(Convert.ToSingle(Effect(EchoCast.Owner)["magicConsumptionRate"].Value)==EchoCast.SpellFpRate&&Convert.ToSingle(Effect(EchoCast.Owner)["miracleConsumptionRate"].Value)==EchoCast.SpellFpRate&&Convert.ToSingle(Effect(EchoCast.Owner)["artsConsumptionRate"].Value)==1f,"Echo spell FP rates or unaffected skill FP rate mismatch");
Require(Convert.ToSingle(Effect(95001400)["artsConsumptionRate"].Value)==0f&&Convert.ToSingle(Effect(95001400)["magicConsumptionRate"].Value)==1f,"Free skills must not affect spells");
File.WriteAllText(Path.Combine(root,".work/arts/serialized.json"),JsonSerializer.Serialize(new[]{95001400,6999800,6999801,6999802,6999803}.Select(id=>new{id,fields=Effect(id).Cells.Where(c=>c!=null).ToDictionary(c=>c.Def.InternalName,c=>c.Value)}),new JsonSerializerOptions{WriteIndented=true}));
bnd.Compression=RegulationCompression.Format;
var dest=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(Path.GetDirectoryName(dest)!);SFUtil.EncryptNightreignRegulation(dest,bnd,bnd.Compression);
var reread=SFUtil.DecryptNightreignRegulation(dest);Require(reread.Version==bnd.Version&&reread.Files.Count==bnd.Files.Count,"Binder metadata mismatch");
foreach(var f in reread.Files)Require(f.Bytes.Span.SequenceEqual(bnd.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Binder readback mismatch "+f.Name);
var changed=reread.Files.Where(f=>!f.Bytes.Span.SequenceEqual(original[f.ID])).Select(f=>Path.GetFileNameWithoutExtension(f.Name)).Order().ToArray();
var expectedChanged=names.Where(n=>added[n].Count>0||n=="Bullet").Order().ToArray();
Require(changed.SequenceEqual(expectedChanged),"Unexpected changed parameter files: "+string.Join(", ",changed));
var textReports=new List<object>();
foreach(var lang in new[]{"zhocn","engus"}){
 var input=Path.Combine(root,"assets/original/msg",lang,"item_dlc01.msgbnd.dcx");var tb=BND4.Read(input);var originals=tb.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var textNames=new[]{"AttachEffectName.fmg","AttachEffectInfo.fmg","PermanentBuffName.fmg","PermanentBuffInfo.fmg","PermanentBuffCaption.fmg"};
 Require(textNames.All(n=>tb.Files.Count(f=>Path.GetFileName(f.Name)==n)==1),"Missing or duplicate permanent ability text file");
 foreach(var f in tb.Files.Where(f=>textNames.Contains(Path.GetFileName(f.Name)))){
  var fm=FMG.Read(f.Bytes);
  foreach(var h in hexes){Require(!fm.Entries.Any(e=>e.ID==h.Id),"FMG ID collision "+h.Id);var isName=Path.GetFileName(f.Name).Contains("Name");string text=lang=="zhocn"?(isName?h.Zh:h.ZhInfo):(isName?h.En:h.EnInfo);fm.Entries.Add(new FMG.Entry(fm,h.Id,text));}
  fm.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));f.Bytes=fm.Write();var outFm=FMG.Read(f.Bytes);var origFm=FMG.Read(originals[f.ID]);
  Require(origFm.Entries.All(e=>outFm.Entries.Any(v=>v.ID==e.ID&&v.Text==e.Text))&&outFm.Entries.Count==origFm.Entries.Count+hexes.Length,"FMG changed outside new entries");
 }
 var output=Path.Combine(root,"dist/mod/msg",lang,"item_dlc01.msgbnd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(output)!);tb.Write(output);var read=BND4.Read(output);Require(read.Files.Count==tb.Files.Count,"Text binder count changed");
 foreach(var f in read.Files)Require(f.Bytes.Span.SequenceEqual(tb.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Text binder readback failed");
 textReports.Add(new{language=lang,inputSha256=Hash(input),outputSha256=Hash(output),changedFiles=tb.Files.Where(f=>!f.Bytes.Span.SequenceEqual(originals[f.ID])).Select(f=>Path.GetFileName(f.Name)).ToArray()});
}
// Temporary SpEffect labels live in the menu binder, unlike permanent rewards.
foreach(var lang in new[]{"zhocn","engus"}){
 var menuInput=Path.Combine(root,"assets/original/msg",lang,"menu_dlc01.msgbnd.dcx");
 Require(Hash(menuInput)==(lang=="zhocn"?"A56F88BEBA7D8ABA36D2D436E9EF2E08E28A2189EEE87075F792B39B774A9E52":"E33F1C338878DA423D95CF54589072D99425F2A92C10603E5C93BDA9323184DB"),"Unsupported menu text source");
 var tb=BND4.Read(menuInput);var originals=tb.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
 var statusNames=tb.Files.Where(f=>Path.GetFileName(f.Name)=="SpEffectName.fmg").ToArray();
 Require(statusNames.Length==1,"Missing native SpEffectName text file");
 var statusName=statusNames[0];var statusFmg=FMG.Read(statusName.Bytes);
 var statuses=HexTexts.Statuses(root,hexes,lang);
 foreach(var h in hexes.Where(h=>h.Id==Erosion.Owner))statuses[h.Id]=lang=="zhocn"?(h.Zh,h.ZhInfo):(h.En,h.EnInfo);
 var statusInfoFile=tb.Files.Single(f=>Path.GetFileName(f.Name)=="SpEffectInfo.fmg");
 var statusInfoFmg=FMG.Read(statusInfoFile.Bytes);
 foreach(var (id,text) in statuses){
  Require(!statusFmg.Entries.Any(e=>e.ID==id)&&!statusInfoFmg.Entries.Any(e=>e.ID==id),"Custom status text collision");
  statusFmg.Entries.Add(new FMG.Entry(statusFmg,id,text.Name));
  statusInfoFmg.Entries.Add(new FMG.Entry(statusInfoFmg,id,text.Info));
 }
 statusInfoFmg.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));
 statusInfoFile.Bytes=statusInfoFmg.Write();
 var checkStatusInfo=FMG.Read(statusInfoFile.Bytes);var originalStatusInfo=FMG.Read(originals[statusInfoFile.ID]);
 Require(checkStatusInfo.Entries.Count==originalStatusInfo.Entries.Count+statuses.Count&&originalStatusInfo.Entries.All(e=>checkStatusInfo.Entries.Any(v=>v.ID==e.ID&&v.Text==e.Text)),"Shield description changed original status text");
 statusFmg.Entries.Sort((a,b)=>a.ID.CompareTo(b.ID));statusName.Bytes=statusFmg.Write();
 var checkStatus=FMG.Read(statusName.Bytes);var originalStatus=FMG.Read(originals[statusName.ID]);
 Require(checkStatus.Entries.Count==originalStatus.Entries.Count+statuses.Count&&originalStatus.Entries.All(e=>checkStatus.Entries.Any(v=>v.ID==e.ID&&v.Text==e.Text)),"Shield name changed original status text");
 RewardReroll.Text(root,tb,lang);
 var menuOutput=Path.Combine(root,"dist/mod/msg",lang,"menu_dlc01.msgbnd.dcx");tb.Write(menuOutput);
 var menuRead=BND4.Read(menuOutput);Require(menuRead.Files.Count==tb.Files.Count,"Menu binder count changed");
 foreach(var f in menuRead.Files)Require(f.Bytes.Span.SequenceEqual(tb.Files.Single(v=>v.ID==f.ID).Bytes.Span),"Menu text readback failed");
 Require(tb.Files.Where(f=>f.ID!=statusName.ID&&f.ID!=statusInfoFile.ID&&Path.GetFileName(f.Name)!="CL_KeyGuide.fmg").All(f=>f.Bytes.Span.SequenceEqual(originals[f.ID])),"Unrelated menu text changed");
 textReports.Add(new{language=lang,inputSha256=Hash(menuInput),outputSha256=Hash(menuOutput),changedFiles=new[]{"SpEffectName.fmg","SpEffectInfo.fmg","CL_KeyGuide.fmg"}});
}
var eventSource=Path.Combine(root,"assets/original/event/common.emevd.dcx");Require(Hash(eventSource)=="0018635B07959514CD9F1DF4E4889959B649C04E175567E6D8A8E2F11B0E2805","Unsupported common event hash");
var ev=EMEVD.Read(eventSource);var origEv=EMEVD.Read(eventSource);Require(!ev.Events.Any(e=>e.ID==9100900),"Event ID collision");var parent=ev.Events.Single(e=>e.ID==0);
Require(parent.Instructions.Count>0&&parent.Parameters.Count==0,"Unexpected common constructor");
// Initialize before any original constructor branches can end early.
parent.Instructions.Insert(0,new EMEVD.Instruction(2000,0,new object[]{0,9100900,0}));
var reward=new EMEVD.Event(9100900,EMEVD.Event.RestBehaviorType.Restart);
EMEVD.Instruction Has(int group,int effect,bool on)=>new(4,5,new object[]{(sbyte)group,20000u,effect,(byte)(on?1:0),(sbyte)0,1f});
reward.Instructions.AddRange([
 new(2000,2,new object[]{(byte)0}),Has(1,95000100,true),Has(1,95000101,false),
 new(0,0,new object[]{(sbyte)0,(byte)1,(sbyte)1}),
 new(2004,8,new object[]{20000u,95000101}),new(2004,8,new object[]{20000u,95000102}),
 Has(0,95000100,false),new(1000,4,new object[]{(byte)1})
]);
ev.Events.Add(reward);
Require(!ev.Events.Any(e=>e.ID==9100901),"Capacity probe event collision");
parent.Instructions.Insert(0,new EMEVD.Instruction(2000,0,new object[]{0,9100901,0}));
var capacityProbe=new EMEVD.Event(9100901,EMEVD.Event.RestBehaviorType.Restart);
capacityProbe.Instructions.AddRange([new(2000,2,new object[]{(byte)0}),Has(1,95000500,true),Has(1,95000502,false),Has(1,95000505,true),new(0,0,new object[]{(sbyte)0,(byte)1,(sbyte)1}),new(2004,87,Array.Empty<object>()),new(1001,1,new object[]{1}),new(2004,8,new object[]{20000u,95000506}),Has(0,95000502,true),Has(0,95000500,false),new(1000,4,new object[]{(byte)1})]);
ev.Events.Add(capacityProbe);
for(int n=0;n<=20;n++){
 int id=9100910+n;Require(!ev.Events.Any(e=>e.ID==id),"Acquisition event collision");
 parent.Instructions.Insert(0,new EMEVD.Instruction(2000,0,new object[]{0,id,0}));
 var acquisition=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Restart);
 acquisition.Instructions.AddRange([new(2000,2,new object[]{(byte)0}),Has(1,95000500,true),Has(1,95000502,false),Has(1,95000520+n,true),new(0,0,new object[]{(sbyte)0,(byte)1,(sbyte)1})]);
 for(int j=0;j<n;j++)acquisition.Instructions.Add(new(2004,86,Array.Empty<object>()));
 // Refill exposes the actual native capacity, including any native cap.
 if(n>0)acquisition.Instructions.AddRange([new(2004,87,Array.Empty<object>()),new(1001,1,new object[]{1})]);
 acquisition.Instructions.AddRange([new(2004,8,new object[]{20000u,95000504}),Has(0,95000502,true),Has(0,95000500,false),new(1000,4,new object[]{(byte)1})]);
 ev.Events.Add(acquisition);
}
// Add the bonus after the native grant's completion flag, allowing native
// capacity recomputation to finish before adding the owner's extra charge.
EMEVD.Instruction[] Bonus()=>[
 new(2004,8,new object[]{20000u,95000508}),new(1001,1,new object[]{3}),
 new(2004,86,Array.Empty<object>()),new(2004,87,Array.Empty<object>()),
 new(1001,1,new object[]{1}),new(2004,8,new object[]{20000u,95000507})];
void DoubleNight(EMEVD.Event e){
 var old=e.Instructions.ToArray();var mapping=new int[old.Length];var rebuilt=new List<EMEVD.Instruction>();
 for(int i=0;i<old.Length;i++){
  mapping[i]=rebuilt.Count;rebuilt.Add(old[i]);
  if(old[i].Bank==2003&&old[i].ID==66&&BitConverter.ToInt32(old[i].ArgData,4)==9051){
   rebuilt.Add(new(1004,0,new object[]{(byte)6,20000u,95000500,(byte)0,(sbyte)0,1}));
   rebuilt.AddRange(Bonus());
  }
 }
 foreach(var p in e.Parameters)p.InstructionIndex=mapping[p.InstructionIndex];
 if(e.ID==1304){Require(old[2].Bank==1003&&old[2].ID==1&&old[2].ArgData[0]==4,"Unexpected second-night branch");old[2].ArgData[0]=(byte)(mapping[7]-mapping[2]-1);}
 e.Instructions=rebuilt;
}
DoubleNight(ev.Events.Single(e=>e.ID==1304));
// Native 1111 returns immediately when deployment-completed flag 9011 is ON.
// Fresh deployment waits for flag 7500, then the local player exists. Clear
// receipts at that point; deaths/relevelling cannot enter this initialization.
var deployment=ev.Events.Single(e=>e.ID==1111);
Require(deployment.Parameters.Count==0&&deployment.Instructions.Count==32,"Unexpected deployment event structure");
Require(deployment.Instructions[1].Bank==1003&&deployment.Instructions[1].ID==1&&Convert.ToHexString(deployment.Instructions[1].ArgData)=="0200000033230000","Deployment completion guard changed");
Require(deployment.Instructions[8].Bank==3&&deployment.Instructions[8].ID==0&&Convert.ToHexString(deployment.Instructions[8].ArgData)=="000100004C1D0000","Deployment player-ready wait changed");
deployment.Instructions.InsertRange(9,Enumerable.Range(95001000,4).Concat(Enumerable.Range(95001010,4)).Select(id=>new EMEVD.Instruction(2004,21,new object[]{20000u,id})));
// Native PlayerInoutMap uses the original common event 1902 hub maps. Two local events:
// keep a short gate outside the hub; clear saved receipts whenever in the hub.
EMEVD.Instruction InMap(int group,bool inside,int area)=>new(3,8,new object[]{(sbyte)group,(byte)(inside?1:0),(sbyte)area,(sbyte)0,(sbyte)0,(sbyte)0});
EMEVD.Instruction WaitGroup(int group)=>new(0,0,new object[]{(sbyte)0,(byte)1,(sbyte)group});
foreach(int id in new[]{9100940,9100941}){Require(!ev.Events.Any(e=>e.ID==id),"Level lifecycle event collision");parent.Instructions.Insert(0,new(2000,0,new object[]{0,id,0}));}
var gate=new EMEVD.Event(9100940,EMEVD.Event.RestBehaviorType.Restart);
gate.Instructions.AddRange([new(2000,2,new object[]{(byte)0}),InMap(1,false,10),InMap(1,false,11),WaitGroup(1),new(2004,8,new object[]{20000u,95001004}),new(1001,0,new object[]{.5f}),new(1000,4,new object[]{(byte)1})]);ev.Events.Add(gate);
var reset=new EMEVD.Event(9100941,EMEVD.Event.RestBehaviorType.Restart);
reset.Instructions.AddRange([new(2000,2,new object[]{(byte)0}),InMap(-1,true,10),InMap(-1,true,11),WaitGroup(-1)]);
foreach(int id in Enumerable.Range(95001000,5).Concat(Enumerable.Range(95001010,4)))reset.Instructions.Add(new(2004,21,new object[]{20000u,id}));
reset.Instructions.AddRange([InMap(1,false,10),InMap(1,false,11),WaitGroup(1),new(1000,4,new object[]{(byte)1})]);ev.Events.Add(reset);
Cages.Terrain(ev);
ContractHeart.Events(ev);
NightBlessing.ResetReceipt(ev);
RewardCycle.Events(ev,RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length));
RewardReroll.Events(ev);
EssenceReaver.Events(ev);
RedEnvelope.RewardEvents(ev);
var eventDest=Path.Combine(root,"dist/mod/event/common.emevd.dcx");Directory.CreateDirectory(Path.GetDirectoryName(eventDest)!);ev.Write(eventDest);var outEv=EMEVD.Read(eventDest);
bool Same(EMEVD.Event a,EMEVD.Event b)=>a.ID==b.ID&&a.RestBehavior==b.RestBehavior&&a.Name==b.Name&&a.Instructions.Count==b.Instructions.Count&&a.Instructions.Zip(b.Instructions).All(v=>v.First.Bank==v.Second.Bank&&v.First.ID==v.Second.ID&&v.First.Layer==v.Second.Layer&&v.First.ArgData.SequenceEqual(v.Second.ArgData))&&JsonSerializer.Serialize(a.Parameters)==JsonSerializer.Serialize(b.Parameters);
Require(outEv.Format==origEv.Format&&outEv.Compression==origEv.Compression&&outEv.StringData.SequenceEqual(origEv.StringData)&&outEv.LinkedFileOffsets.SequenceEqual(origEv.LinkedFileOffsets),"EMEVD metadata changed");
Require(outEv.Events.Count==origEv.Events.Count+29&&origEv.Events.Where(e=>e.ID!=0&&e.ID!=1304&&e.ID!=1111&&e.ID!=9100941).All(e=>Same(e,outEv.Events.Single(o=>o.ID==e.ID))),"Existing events changed");Require(ev.Events.All(e=>Same(e,outEv.Events.Single(o=>o.ID==e.ID))),"Edited events failed readback");
var churchReports=new List<object>();
var churchSources=new[]{
 ("m41_00_00_00",41004700u,"8C9B4F43717A9800E02E1497075AE9C2874808877671364BD5BD0EF7D38D57CA"),
 ("m41_01_00_00",41014700u,"8D3EF0816FF2FCF1EDAF76A865D22BE946F4F2A77899349364605506433F2566"),
 ("m41_30_00_00",41304700u,"F5229F95A6BC5B1030418FB5D60EB4D433B7403C4AFDE8C3120AC9FA9E4AC998"),
 ("m51_00_00_00",51004700u,"9EB8A2A6FF0299767DFBA9AAF8A64A80EF81DCDC975C3212DB31C503DB5553D0")};
for(int index=0;index<churchSources.Length;index++){
 var (name,flag,hash)=churchSources[index];var input=Path.Combine(root,"assets/original/event",name+".emevd.dcx");Require(Hash(input)==hash,"Unsupported church source "+name);
 var map=EMEVD.Read(input);var before=EMEVD.Read(input);
 bool rotted=name=="m41_30_00_00";
 int id=rotted?41302530:(int)(flag-2180);
 Require(map.Events.Single(e=>e.ID==0).Parameters.Count==0,"Unexpected church constructor");
 Require(map.Events.Any(e=>e.ID!=0&&e.Instructions.Any(i=>i.Bank==3&&i.ID==0&&BitConverter.ToUInt32(i.ArgData,4)==flag)),"Missing church completion flag");
 EMEVD.Event bonus;
 if(!rotted){
  // Follow the original, already-initialized pickup event in its own map
  // namespace. It supplies the correct instance context and reload guard.
  bonus=map.Events.Single(e=>e.ID==id);
  Require(bonus.Parameters.Count==0&&bonus.Instructions.Count==5&&bonus.Instructions[^1].Bank==2004&&bonus.Instructions[^1].ID==8&&BitConverter.ToInt32(bonus.Instructions[^1].ArgData,4)==99270,"Native church pickup tail changed "+name);
 }else{
  // The unused native 41302510 is not initialized. Use a local map-range ID
  // for this variant without activating its unrelated enemy-effect event.
  Require(!map.Events.Any(e=>e.ID==id),"Rotted church event collision");
  map.Events.Single(e=>e.ID==0).Instructions.Insert(0,new(2000,0,new object[]{0,id,0}));
  bonus=new EMEVD.Event(id,EMEVD.Event.RestBehaviorType.Default);
  bonus.Instructions.AddRange([new(2000,2,new object[]{(byte)0}),new(1003,2,new object[]{(byte)0,(byte)1,(byte)0,flag}),new(3,0,new object[]{(sbyte)0,(byte)1,(byte)0,flag})]);map.Events.Add(bonus);
 }
 bonus.Instructions.Add(new(1004,2,new object[]{(byte)0,20000u,95000500,(byte)0,(sbyte)0,1}));
 bonus.Instructions.AddRange(Bonus());
 var output=Path.Combine(root,"dist/mod/event",name+".emevd.dcx");map.Write(output);var check=EMEVD.Read(output);
 Require(check.Events.Count==before.Events.Count+(rotted?1:0)&&before.Events.Where(e=>e.ID!=(rotted?0:id)).All(e=>Same(e,check.Events.Single(o=>o.ID==e.ID)))&&map.Events.All(e=>Same(e,check.Events.Single(o=>o.ID==e.ID))),"Church event readback mismatch "+name);
 Require(check.Format==before.Format&&check.Compression==before.Compression&&check.StringData.SequenceEqual(before.StringData)&&check.LinkedFileOffsets.SequenceEqual(before.LinkedFileOffsets),"Church metadata changed "+name);
 churchReports.Add(new{name,flag,eventId=id,nativePickupTail=!rotted,mapNamespace=true,inputSha256=Hash(input),outputSha256=Hash(output)});
}
Cages.Build(root,cageBosses,serialized);
var drinkingFields=drinkingRates.Concat(drinkingPowerRates).Concat(new[]{"magParamChange","miracleParamChange","shamanParamChange","conditionHpRate","effectEndurance"}).ToArray();
var report=new{version="0.17.26",codeVersion="0.17.26",gameTested=false,sourceRegulationSha256=Hash(source),outputRegulationSha256=Hash(dest),changedParams=changed,addedRows=added,textReports,eventSourceSha256=Hash(eventSource),eventOutputSha256=Hash(eventDest),churchReports,modifiedEvents=new[]{0,1111,1304},newEvents=Enumerable.Range(9100910,21).Prepend(9100901).Prepend(9100900).Concat(new[]{9100940,9100941,9100960}),levelRewards=new{levels=new[]{2,7,10,13},receipts=new[]{95001000,95001001,95001002,95001003},goldRarity=3,perPlayer=true},hexes,orders,drinkingNativeReference=new{id=8670000,fields=drinkingFields.ToDictionary(f=>f,f=>fullHpAttack[f].Value)},drinkingSerialized=new{id=95000501,fields=drinkingFields.ToDictionary(f=>f,f=>Effect(95000501)[f].Value)},validation="Reopened outputs; original param rows, unrelated text and events preserved; Drinking matches native full-HP AttackRate and spell flags without its condition; stolen-flask events 1145/1146 and first-night 1303 unchanged; native church pickup tails and map-local event namespace; already-completed churches skipped; neutral native full-HP red aura."};
File.WriteAllText(Path.Combine(root,"dist/validation.json"),JsonSerializer.Serialize(report,new JsonSerializerOptions{WriteIndented=true}));Console.WriteLine("Built and verified "+dest);
