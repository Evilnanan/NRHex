using SoulsFormats;
using System.Text;
using System.Text.Json;

// Candidate catalogues, never Cartesian products of loadouts. The DLL rolls
// and materializes immutable CharaInit/weapon rows on demand on every peer.
static class DemonsCall {
 public const int Owner=95003300,Drain=95003301,Cooldown=95003302,FirstNpc=98300000;
 static void Check(bool value,string message){if(!value)throw new InvalidDataException(message);}
 static int N(PARAM.Row row,string key)=>Convert.ToInt32(row[key].Value);
 static void Set(PARAM.Row row,string key,object value){row[key].Value=Convert.ChangeType(value,row[key].Value.GetType());}
 static string Name(BinderFile file)=>file.Name.Replace('\\','/').Split('/').Last();
 static string Hash(string path)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> add){
  var drain=add("SpEffectParam",705200,Drain);
  Set(drain,"changeHpRate",100f/60);Set(drain,"changeHpPoint",0);Set(drain,"bCurrHPIndependeMaxHP",0);
  Set(drain,"isExtendSpEffectLife",0);Set(drain,"isContractSpEffectLife",0);
  Set(drain,"vfxId",95001804); // Available native Necromancy appearance effect.
  var cooldown=add("SpEffectParam",95002700,Cooldown);
  Set(cooldown,"effectEndurance",60f);Set(cooldown,"saveCategory",10);Set(cooldown,"dontDeleteOnDead",1);
  Set(cooldown,"effectTargetPcDeceased",1);
  Set(cooldown,"iconId",-1);Set(cooldown,"spEffectTextId_1",-1);Set(cooldown,"isDisableNetSync",1);
  for(int hero=0;hero<10;hero++)for(int skin=0;skin<6;skin++){
   var npc=add("NpcParam",700035000+hero*100+skin,FirstNpc+hero*6+skin);
   Set(npc,"teamType",29);Set(npc,"getSoul",0);Set(npc,"isSoulGetByBoss",0);Set(npc,"excludeGroupRewardCheck",1);Set(npc,"affectedByNecromancy",0);Set(npc,"isCalculatePvPDamage",1);
   foreach(var cell in npc.Cells.Where(c=>c!=null&&(c.Def.InternalName.Contains("LotId",StringComparison.OrdinalIgnoreCase)||c.Def.InternalName.StartsWith("rewardItemLot_"))))Set(npc,cell.Def.InternalName,-1);
   foreach(var cell in npc.Cells.Where(c=>c!=null&&c.Def.InternalName.Contains("LotPoint")))Set(npc,cell.Def.InternalName,0);
   // Remove Libra's on-death pickup bullet and his 80% resistance to other
   // enemies. Native class/skin/equipment effects remain unchanged.
   for(int i=0;i<32;i++)if(N(npc,$"spEffectID{i}") is 49173 or 49152)Set(npc,$"spEffectID{i}",-1);
  }
 }
 // Serialize marked rows to verify offsets against the *version-filtered*
 // PARAMDEF and actual binary writer, including bit fields and padding.
 static (int Size,Dictionary<string,int> Offsets) Layout(PARAM table,int source,string first,string[] fields){
  var rows=table.Rows;var a=new PARAM.Row(rows.First(r=>r.ID==source)){ID=1,Name=null};var b=new PARAM.Row(a){ID=2};
  try{
   table.Rows=new(){a,b};Set(a,first,0x13579bdf);Set(b,first,0x2468ace0);
   byte[] bytes=table.Write();
   int Find(byte[] marker){var hits=Enumerable.Range(0,bytes.Length-marker.Length+1).Where(i=>bytes.AsSpan(i,marker.Length).SequenceEqual(marker)).ToArray();Check(hits.Length==1,"Ambiguous layout marker");return hits[0];}
   int start=Find(BitConverter.GetBytes(0x13579bdf)),second=Find(BitConverter.GetBytes(0x2468ace0));
   int size=table.AppliedParamdef.GetRowSize(10350000);Check(second-start==size,"Unexpected raw-row stride");
   // 'soul' starts CI rows; weapon rows begin with a four-byte enable mask.
   int firstOffset=first=="behaviorVariationId"?4:0;start-=firstOffset;
   var offsets=new Dictionary<string,int>();
   foreach(string key in fields){var old=a[key].Value;Set(a,key,0x3579bdf1);bytes=table.Write();int offset=Find(BitConverter.GetBytes(0x3579bdf1))-start;Check(offset>=0&&offset+4<=size,"Field outside raw row: "+key);offsets[key]=offset;a[key].Value=old;}
   return(size,offsets);
  }finally{table.Rows=rows;}
 }
 public static void Catalog(BND4 native,Func<string,Memory<byte>,PARAM> parse,string destination){
  PARAM Read(string name)=>parse(name,native.Files.Single(f=>Name(f)==name+".param").Bytes);
  var weapon=Read("EquipParamWeapon");var weapons=weapon.Rows.ToDictionary(r=>r.ID);var ci=Read("CharaInitParam");
  var generators=Read("EquipParamCustomWeapon").Rows.Where(r=>N(r,"isCursed")==0&&weapons.ContainsKey(N(r,"targetWeaponId"))).ToArray();
  var arts=Read("SwordArtsTableParam").Rows.ToLookup(r=>r.ID);var magic=Read("MagicTableParam").Rows.ToLookup(r=>r.ID);
  var validArts=Read("SwordArtsParam").Rows.Select(r=>r.ID).ToHashSet();var validMagic=Read("Magic").Rows.Select(r=>r.ID).ToHashSet();
  var candidates=generators.Select(r=>N(r,"targetWeaponId")).Distinct().Where(id=>id>0&&id<50000000).Order().ToArray();
  Check(candidates.Length>300,"Unexpectedly small native weapon pool");
  Check(!weapons.Keys.Any(id=>id>=100000000&&id<204857600)&&!ci.Rows.Any(r=>r.ID>=1000000000&&r.ID<1000524288),"Virtual parameter namespace collision");
  var cLayout=Layout(ci,35000,"soul",new[]{"equip_Wep_Right_1","equip_Wep_Right_2","equip_Wep_Right_3","equip_Wep_Left_1","equip_Wep_Left_2","equip_Wep_Left_3"});
  var wLayout=Layout(weapon,candidates[0],"behaviorVariationId",new[]{"swordArtsParamId","equippedSpell_R2"});
  var output=new StringBuilder("// Generated native candidate pools and verified raw field offsets; no loadout combinations.\nuse crate::demons::Weapon;\n");
  output.AppendLine($"pub const CI_SIZE:usize={cLayout.Size};\npub const WEAPON_SIZE:usize={wLayout.Size};");
  output.AppendLine("pub const CI_WEAPONS:[usize;6]=["+string.Join(",",new[]{"equip_Wep_Right_1","equip_Wep_Right_2","equip_Wep_Right_3","equip_Wep_Left_1","equip_Wep_Left_2","equip_Wep_Left_3"}.Select(k=>cLayout.Offsets[k]))+"];");
  output.AppendLine($"pub const WEAPON_ART:usize={wLayout.Offsets["swordArtsParamId"]};\npub const WEAPON_SPELL:usize={wLayout.Offsets["equippedSpell_R2"]};");
  var types=new List<int>();var leftTypes=new List<int>();
  for(int hero=0;hero<10;hero++){
   var start=ci.Rows.Single(r=>r.ID==30000+hero*100);int right=N(start,"equip_Wep_Right_1"),left=N(start,"equip_Wep_Left_1");
   types.Add(N(weapons[right],"wepType"));leftTypes.Add(left<0?-1:N(weapons[left],"wepType"));
   for(int skin=0;skin<6;skin++)Check(N(ci.Rows.Single(r=>r.ID==35000+hero*100+skin),"heroId")==hero+1,"Libra hero/skin template changed");
  }
  output.AppendLine("pub const MAIN_TYPES:[i32;10]=["+string.Join(",",types)+"];");
  output.AppendLine("pub const LEFT_TYPES:[i32;10]=["+string.Join(",",leftTypes)+"];");
  output.AppendLine("pub const WEAPONS:&[Weapon]=&[");
  int artCount=0,spellCount=0;
  foreach(int id in candidates){
   var w=weapons[id];int table=N(w,"swordArtsTableId");
   var compatible=arts[table].Where(r=>N(r,"chanceWeight")>0&&validArts.Contains(N(r,"swordArtsId"))).GroupBy(r=>N(r,"swordArtsId")).OrderBy(g=>g.Key).Select(g=>(Id:g.Key,Weight:g.Sum(r=>N(r,"chanceWeight")))).ToArray();
   if(table>=0)Check(compatible.Length>0,"Missing compatible native weapon-art pool: "+id);
   // Secondary spells use this weapon's native generator pool, not all Magic
   // rows (which include NPC-only and incompatible spells).
   var poolIds=generators.Where(r=>N(r,"targetWeaponId")==id).Select(r=>N(r,"magicTableId_2")).Where(i=>i>=0).Distinct().ToArray();
   // Recluse's melee starting staff has no secondary generator. Use the
   // native general pool for its casting type rather than leaving it blank.
   if(poolIds.Length==0&&N(w,"enableMagic")!=0)poolIds=new[]{3300100};
   if(poolIds.Length==0&&N(w,"enableMiracle")!=0)poolIds=new[]{3400100};
   var spells=poolIds.SelectMany(i=>magic[i]).Where(r=>N(r,"chanceWeight")>0&&validMagic.Contains(N(r,"magicId"))&&N(r,"magicId")!=N(w,"equippedSpell_R1")).GroupBy(r=>N(r,"magicId")).OrderBy(g=>g.Key).Select(g=>(Id:g.Key,Weight:g.Max(r=>N(r,"chanceWeight")))).ToArray();
   if(N(w,"enableMagic")!=0||N(w,"enableSorcery")!=0||N(w,"enableMiracle")!=0)Check(spells.Length>0,"Missing native secondary spell pool: "+id);
   int spellTable=poolIds.Order().FirstOrDefault(-1);
   output.AppendLine($" Weapon{{id:{id},kind:{N(w,"wepType")},art:{N(w,"swordArtsParamId")},spell:{N(w,"equippedSpell_R2")},primary:{N(w,"equippedSpell_R1")},art_table:{table},spell_table:{spellTable}}},");
   artCount+=compatible.Length;spellCount+=spells.Length;
  }
  output.AppendLine("];");Directory.CreateDirectory(Path.GetDirectoryName(destination)!);File.WriteAllText(destination,output.ToString());
  Console.WriteLine($"PASS: {candidates.Length} native weapons, {artCount} compatible art entries, {spellCount} secondary spell entries; CI {cLayout.Size} / weapon {wLayout.Size} bytes");
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var dir=Path.Combine(root,"work/demons-call/staged");Directory.CreateDirectory(dir);
  string input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","NpcParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Name(f)==n+".param").Bytes));
  PARAM.Row Add(string name,int source,int id){Check(!tables[name].Rows.Any(r=>r.ID==id),$"Parameter collision: {name}/{id}");var row=new PARAM.Row(tables[name].Rows.First(r=>r.ID==source)){ID=id};tables[name].Rows.Add(row);return row;}
  var h=hexes.Single(h=>h.Id==Owner);var owner=Add("SpEffectParam",7035400,Owner);Set(owner,"saveCategory",10);Set(owner,"spCategory",20);Set(owner,"iconId",h.SmallIcon);Set(owner,"spEffectTextId_1",Owner);
  // Reuse state 2278. HKS/native gesture logic still adds 7035401/7035402
  // through its original path, with its original stacking and timing.
  var a=Add("AttachEffectParam",95002700,Owner);Set(a,"passiveSpEffectId_1",Owner);Set(a,"attachTextId",Owner);Set(a,"statusIconId",h.SmallIcon);Set(a,"overrideBaseEffectId",Owner);
  var p=Add("PermanentBuffParam",95002700,Owner);Set(p,"spEffectId",Owner);Set(p,"iconId",h.LargeIcon);
  var g=Add("EquipParamGoods",95002700,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);
  var t=Add("ItemTableParam",RewardCatalog.Table(95002700),RewardCatalog.Table(Owner));Set(t,"itemId",Owner);
  Mechanics(Add);Catalog(native,parse,Path.Combine(dir,"demons_data.rs"));RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  foreach(string n in names){tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();Check(tables[n].Rows.Count<65536,"PARAM row capacity");binder.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var back=SFUtil.DecryptNightreignRegulation(output);
  var added=new Dictionary<string,int>();Check(back.Version==binder.Version&&back.Compression==binder.Compression&&back.Files.Count==binder.Files.Count,"Binder metadata changed");
  foreach(var f in back.Files){var wanted=binder.Files.Single(v=>v.ID==f.ID);Check(f.Name==wanted.Name&&f.Flags==wanted.Flags&&f.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Member readback mismatch");string n=Name(f).Replace(".param","");
   if(!names.Contains(n)){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated parameter changed");continue;}
   var old=parse(n,before[f.ID]);var current=parse(n,f.Bytes);var oldGroups=old.Rows.ToLookup(r=>r.ID);
   foreach(var group in oldGroups){var now=current.Rows.Where(r=>r.ID==group.Key).ToArray();Check(now.Length==group.Count()&&group.Zip(now).All(v=>CageAI.Same(v.First,v.Second)),"Existing row changed: "+n+"/"+group.Key);}
   added[n]=current.Rows.Count-old.Rows.Count;
  }
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),added,npcRows=60,generatedLoadouts=0,gestureChainUnchanged=true,allOriginalRowsAndOtherTablesUnchanged=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: native sinner templates, native gesture ownership, lifetime/cooldown and reward wrappers staged/read back");
 }
}
