using SoulsFormats;
using System.Text.Json;

// Generate only changed ancestors. Original weapon generators and themed/class
// leaves remain native; no combinations of rewards or weapon effects are stored.
static class PotentialFission {
 public const int Owner=95003200,FirstLot=98000000,FirstTable=98000000;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static int N(PARAM.Row r,string f)=>Convert.ToInt32(r[f].Value);
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static void Check(bool ok,string message){if(!ok)throw new InvalidDataException(message);}
 static string Hash(string p)=>Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(p)));
 public static void Mechanics(Func<string,int,int,PARAM.Row> clone){
  for(int i=1;i<=3;i++){
   var r=clone("SpEffectParam",8000000,Owner+i);
   foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(r,f,v);
  }
 }
 public static object? Audit;
 public sealed record Root(int Source,int A,int B,int Count);
 public static Root[] Generate(Dictionary<string,PARAM> tables,string metadata){
  var table=tables["ItemTableParam"];var lot=tables["ItemLotParam_enemy"];
  var pools=table.Rows.Where(r=>r.ID<95000000||r.ID>=100000000).ToLookup(r=>r.ID);
  var weapons=tables["EquipParamCustomWeapon"].Rows.ToDictionary(r=>r.ID);
  var bases=tables["EquipParamWeapon"].Rows.ToDictionary(r=>r.ID);
  var known=new HashSet<int>{4200000,4200100,4200101,4200102,4200103,4200104,4200105,4200200,4200210,4200214,4200216,4200217,4200220,4200224,4200226,4200300,4200301,4200302,4200303,4200304,4200305};
  var special=new HashSet<int>{4200200,4200210,4200214,4200216,4200217,4200220,4200224,4200226};
  var high=new Dictionary<int,int>{{4200300,4200100},{4200301,4200101},{4200302,4200102},{4200303,4200103},{4200304,4200100},{4200305,4200100}};
  var reaches=new Dictionary<int,bool>();var checking=new HashSet<int>();
  bool Relevant(int id){
   if(known.Contains(id))return true;if(reaches.TryGetValue(id,out var found))return found;
   if(!checking.Add(id))throw new InvalidDataException("Cyclic native reward pool");
   bool value=pools[id].Any(r=>N(r,"chanceWeight")>0&&N(r,"itemCategory")==7&&Relevant(N(r,"itemId")));
   checking.Remove(id);return reaches[id]=value;
  }
  var sourceLots=lot.Rows.Where(r=>r.ID>=6000000&&r.ID<7000000).ToDictionary(r=>r.ID);
  var roots=new List<(int Id,PARAM.Row[] Rows)>();
  foreach(var row in sourceLots.Values.OrderBy(r=>r.ID)){
   if(sourceLots.ContainsKey(row.ID-1))continue;
   var group=Enumerable.Range(0,4).TakeWhile(i=>sourceLots.ContainsKey(row.ID+i)).Select(i=>sourceLots[row.ID+i]).ToArray();
   if(group.Length<2||group.Length>3)continue;
   if(group.Any(r=>Enumerable.Range(1,8).Any(i=>N(r,$"lotItemBasePoint{i:00}")>0&&N(r,$"lotItemCategory{i:00}")==7&&Relevant(N(r,$"lotItemId{i:00}")))))roots.Add((row.ID,group));
  }
  Check(roots.Count>0,"No native potential reward sources");
  var usedTables=table.Rows.Select(r=>r.ID).ToHashSet();var usedLots=lot.Rows.Select(r=>r.ID).ToHashSet();
  int nextTable=FirstTable;var cache=new Dictionary<(int,int),int>();var working=new HashSet<(int,int)>();
  int Publish(PARAM.Row[] rows){
   Check(!usedTables.Contains(nextTable),"Fission item-table collision");int id=nextTable++;usedTables.Add(id);
   foreach(var r in rows){r.ID=id;table.Rows.Add(r);}return id;
  }
  int Mix(int keep,int fallback){
   var a=new PARAM.Row(pools[4200100].First());Set(a,"itemCategory",7);Set(a,"itemId",keep);Set(a,"chanceWeight",1);Set(a,"onlyForClass",0);Set(a,"eventFlagId",0);Set(a,"unknown_6",0);Set(a,"unknown_7",0);
   var b=new PARAM.Row(a);Set(b,"itemId",fallback);return Publish(new[]{a,b});
  }
  var highGoods=high.Keys.SelectMany(k=>pools[k]).Where(r=>N(r,"itemCategory")==1).Select(r=>N(r,"itemId")).ToHashSet();
  var basicGoods=pools[4200100].Where(r=>N(r,"itemCategory")==1).Select(r=>N(r,"itemId")).ToHashSet();
  var specialGoods=special.SelectMany(k=>pools[k]).Where(r=>N(r,"itemCategory")==1).Select(r=>N(r,"itemId")).ToHashSet();
  var weaponOnly=new Dictionary<int,bool>();
  bool WeaponOnly(int id){
   if(weaponOnly.TryGetValue(id,out bool saved))return saved;
   var active=pools[id].Where(r=>N(r,"chanceWeight")>0).ToArray();
   return weaponOnly[id]=active.Length>0&&active.All(r=>N(r,"itemCategory")==6&&weapons.ContainsKey(N(r,"itemId"))||N(r,"itemCategory")==7&&WeaponOnly(N(r,"itemId")));
  }
  int ConvertPool(int id,int mode){
   if(cache.TryGetValue((id,mode),out int saved))return saved;
   if(!working.Add((id,mode)))throw new InvalidDataException("Cycle in conversion");
   var original=pools[id].ToArray();Check(original.Length>0,$"Missing native pool {id}");int target=id;
   // Replace native rarity selectors before descending into weapon families:
   // unique purple weapons may have no blue counterpart in their family.
   // Their native blue selector already redistributes those empty families.
   int lowerSelector=id switch {3000300=>3000200,3000310=>3000200,3000400=>3000300,2001503=>2000002,2001508=>2000002,_=>0};
   bool cursedSelector=id is 3000310 or 2001508;
   if(lowerSelector==0&&id>=2000000&&id<8000000&&id!=4200000){
    int suffix=id%10;
    if(suffix is 3 or 4 or 8){int candidate=id-(suffix==4?1:suffix==8?6:1);
     if(pools.Contains(candidate)&&pools[id].Any(r=>N(r,"chanceWeight")>0)&&pools[id].Where(r=>N(r,"chanceWeight")>0).All(r=>N(r,"itemCategory")==7)&&WeaponOnly(id)){
      lowerSelector=candidate;cursedSelector=suffix==8;
     }
    }
   }
   if(lowerSelector!=0)target=cursedSelector&&mode==1?Mix(id,lowerSelector):lowerSelector;
   else if(high.TryGetValue(id,out int basic))target=basic;
   else if(special.Contains(id))target=mode==0?4200100:Mix(id,4200100);
   else{
    var active=original.Where(r=>N(r,"chanceWeight")>0).ToArray();
    bool weaponLeaf=active.Length>0&&active.All(r=>N(r,"itemCategory")==6&&weapons.ContainsKey(N(r,"itemId")));
    if(weaponLeaf){
     var generators=active.Select(r=>weapons[N(r,"itemId")]).ToArray();
     bool cursed=generators.All(r=>N(r,"isCursed")!=0);
     var grades=generators.Select(r=>N(bases[N(r,"targetWeaponId")],"rarity")).Distinct().ToArray();
     if(cursed){
      Check(id>=10000000&&id%10==3,"Unrecognized cursed native weapon leaf: "+id);
      int blue=id-5001;Check(pools.Contains(blue)&&pools[blue].Any(r=>N(r,"chanceWeight")>0),"Missing themed blue cursed fallback: "+id);
      target=mode==0?blue:Mix(id,blue);
     }else if(grades.Length==1&&grades[0] is 2 or 3&&id>=10000000&&id%10==grades[0]+1){
      // Pool suffix3 is purple,4 gold; stop at the native target to avoid
      // applying purple->blue a second time to a gold->purple result.
      int lower=id-1;Check(pools.Contains(lower)&&pools[lower].Any(r=>N(r,"chanceWeight")>0),"Missing native lower-quality leaf: "+id);target=lower;
     }
    }else{
     var copy=original.Select(r=>new PARAM.Row(r)).ToArray();bool changed=false;
     foreach(var r in copy)if(N(r,"itemCategory")==1&&N(r,"chanceWeight")>0){
      int good=N(r,"itemId");
      if(highGoods.Contains(good)){
       int lower=good==8330104?8330103:good-1;
       if(basicGoods.Contains(lower))Set(r,"itemId",lower);else{Set(r,"itemCategory",7);Set(r,"itemId",4200100);}changed=true;
      }else if(specialGoods.Contains(good)){
       int replacement=4200100;
       if(mode==1){var keep=new PARAM.Row(r);Set(keep,"chanceWeight",1);Set(keep,"onlyForClass",0);Set(keep,"eventFlagId",0);var fallback=new PARAM.Row(keep);Set(fallback,"itemCategory",7);Set(fallback,"itemId",4200100);replacement=Publish(new[]{keep,fallback});}
       Set(r,"itemCategory",7);Set(r,"itemId",replacement);changed=true;
      }
     }
     foreach(var r in copy)if(N(r,"itemCategory")==7&&N(r,"chanceWeight")>0){int child=N(r,"itemId");if(child>=FirstTable&&child<nextTable||child==4200100&&N(original[Array.IndexOf(copy,r)],"itemCategory")==1)continue;int replacement=ConvertPool(child,mode);if(child!=replacement){Set(r,"itemId",replacement);changed=true;}}
     if(changed)target=Publish(copy);
    }
   }
   working.Remove((id,mode));return cache[(id,mode)]=target;
  }
  var result=new List<Root>();
  foreach(var root in roots){
   int a=FirstLot+result.Count*10,b=a+5;
   foreach(var(mode,start)in new[]{(0,a),(1,b)})for(int i=0;i<root.Rows.Length;i++){
    Check(usedLots.Add(start+i),"Fission loot collision");var row=new PARAM.Row(root.Rows[i]){ID=start+i};
    for(int slot=1;slot<=8;slot++)if(N(row,$"lotItemBasePoint{slot:00}")>0&&N(row,$"lotItemCategory{slot:00}")==7)Set(row,$"lotItemId{slot:00}",ConvertPool(N(row,$"lotItemId{slot:00}"),mode));
    lot.Rows.Add(row);
   }
   result.Add(new(root.Id,a,b,root.Rows.Length));
  }
  // Independent probability-mass audit over the final shared graph. It does
  // not enumerate weapon rolls/passives, and runs for every generated source.
  var allPools=table.Rows.ToLookup(r=>r.ID);var masses=new Dictionary<int,decimal[]>();
  decimal[] Mass(int id){
   if(masses.TryGetValue(id,out var saved))return saved;
   var value=new decimal[5];
   if(high.ContainsKey(id)){value[0]=1;return masses[id]=value;}
   if(special.Contains(id)){value[1]=1;return masses[id]=value;}
   var rows=allPools[id].Where(r=>N(r,"chanceWeight")>0).ToArray();decimal sum=rows.Sum(r=>(decimal)N(r,"chanceWeight"));
   foreach(var r in rows){var branch=new decimal[5];int category=N(r,"itemCategory"),item=N(r,"itemId");
    if(category==7)branch=Mass(item);
    else if(category==1){if(highGoods.Contains(item))branch[0]=1;else if(specialGoods.Contains(item))branch[1]=1;}
    else if(category==6&&weapons.TryGetValue(item,out var weapon)){
     if(N(weapon,"isCursed")!=0)branch[2]=1;
     else {int grade=N(bases[N(weapon,"targetWeaponId")],"rarity");if(grade==2)branch[3]=1;else if(grade==3)branch[4]=1;}
    }
    for(int i=0;i<5;i++)value[i]+=branch[i]*N(r,"chanceWeight")/sum;
   }return masses[id]=value;
  }
  decimal[] LotMass(int start){var value=new decimal[5];for(int index=0;index<3;index++){
   var row=lot.Rows.SingleOrDefault(r=>r.ID==start+index);if(row==null)break;
   decimal sum=Enumerable.Range(1,8).Sum(i=>(decimal)N(row,$"lotItemBasePoint{i:00}"));
   if(sum==0)continue;
   for(int i=1;i<=8;i++)if(N(row,$"lotItemBasePoint{i:00}")>0&&N(row,$"lotItemCategory{i:00}")==7){var branch=Mass(N(row,$"lotItemId{i:00}"));for(int k=0;k<5;k++)value[k]+=branch[k]*N(row,$"lotItemBasePoint{i:00}")/sum;}
  }return value;}
  foreach(var r in result){var before=LotMass(r.Source);foreach(var(mode,start)in new[]{(0,r.A),(1,r.B)}){
   var after=LotMass(start);decimal factor=mode==0?0m:0.5m;
   foreach(var(actual,wanted)in new[]{(after[0],0m),(after[1],before[1]*factor),(after[2],before[2]*factor),(after[3],before[4]),(after[4],0m)})
    Check(Math.Abs(actual-wanted)<0.000000000001m,$"Downgrade probability audit failed: {r.Source}/{mode}: {actual} != {wanted}");
  }}
  Audit=new{allRootsProbabilityMassChecked=true,conversions=cache.OrderBy(k=>k.Key.Item1).ThenBy(k=>k.Key.Item2).Select(k=>new{source=k.Key.Item1,mode=k.Key.Item2,target=k.Value}).ToArray(),specialAndCursedA=0,specialAndCursedB=0.5,goldDropsExactlyOneTier=true};
  Check(table.Rows.Count<65536&&lot.Rows.Count<65536,"Fission row capacity exceeded");
  table.Rows=table.Rows.OrderBy(r=>r.ID).ToList();lot.Rows=lot.Rows.OrderBy(r=>r.ID).ToList();
  Directory.CreateDirectory(Path.GetDirectoryName(metadata)!);
  File.WriteAllText(metadata,"// Generated shared A/B native reward roots; never enumerate reward outcomes.\n"+
   "pub const ROOTS:&[(i32,i32,i32,u8)]=&[\n"+string.Join("\n",result.Select(r=>$" ({r.Source},{r.A},{r.B},{r.Count}),"))+"\n];\n");
  return result.ToArray();
 }
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  var dir=Path.Combine(root,"work/potential-fission/staged");Directory.CreateDirectory(dir);
  var input=Path.Combine(root,"dist/mod/regulation.bin");var binder=SFUtil.DecryptNightreignRegulation(input);var before=binder.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam","ItemLotParam_enemy","EquipParamCustomWeapon","EquipParamWeapon"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,binder.Files.Single(f=>Name(f)==n+".param").Bytes));
  PARAM.Row Add(string n,int source,int id){var p=tables[n];Check(!p.Rows.Any(r=>r.ID==id),$"ID collision {n}/{id}");var r=new PARAM.Row(p.Rows.First(r=>r.ID==source)){ID=id};p.Rows.Add(r);return r;}
  var h=hexes.Single(h=>h.Id==Owner);
  var owner=Add("SpEffectParam",95002700,Owner);Set(owner,"iconId",h.SmallIcon);Set(owner,"spEffectTextId_1",Owner);
  Mechanics(Add);
  var attach=Add("AttachEffectParam",95002700,Owner);Set(attach,"passiveSpEffectId_1",Owner);Set(attach,"attachTextId",Owner);Set(attach,"statusIconId",h.SmallIcon);Set(attach,"overrideBaseEffectId",Owner);
  var permanent=Add("PermanentBuffParam",95002700,Owner);Set(permanent,"spEffectId",Owner);Set(permanent,"iconId",h.LargeIcon);
  var good=Add("EquipParamGoods",95002700,Owner);Set(good,"vagrantItemLotId",Owner);Set(good,"iconId",h.LargeIcon-1000);
  var table=Add("ItemTableParam",RewardCatalog.Table(95002700),RewardCatalog.Table(Owner));Set(table,"itemId",Owner);
  var roots=Generate(tables,Path.Combine(dir,"fission_catalog.rs"));
  // The appended history bit is still in the existing fourth byte; no new
  // packed history rows or reset-event changes are needed for this ability.
  Check(RewardCatalog.PackedIds(RewardCatalog.HistoricalOwners.Length).All(id=>tables["SpEffectParam"].Rows.Any(r=>r.ID==id)),"History byte expansion requires full staging");
  foreach(var n in names.Take(6)){var p=tables[n];p.Rows=p.Rows.OrderBy(r=>r.ID).ToList();binder.Files.Single(f=>Name(f)==n+".param").Bytes=p.Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,binder,binder.Compression);var readback=SFUtil.DecryptNightreignRegulation(output);
  Check(readback.Version==binder.Version&&readback.Compression==binder.Compression&&readback.Files.Count==binder.Files.Count,"Binder metadata changed");
  var added=new Dictionary<string,int>();
  foreach(var file in readback.Files){var wanted=binder.Files.Single(f=>f.ID==file.ID);Check(file.Name==wanted.Name&&file.Flags==wanted.Flags&&file.Bytes.Span.SequenceEqual(wanted.Bytes.Span),"Readback mismatch");string n=Name(file).Replace(".param","");if(!names.Take(6).Contains(n)){Check(file.Bytes.Span.SequenceEqual(before[file.ID]),"Unrelated table changed");continue;}
   var old=parse(n,before[file.ID]);var actual=parse(n,file.Bytes);var oldGroups=old.Rows.ToLookup(r=>r.ID);foreach(var group in oldGroups){var current=actual.Rows.Where(r=>r.ID==group.Key).ToArray();Check(current.Length==group.Count()&&group.Zip(current).All(p=>CageAI.Same(p.First,p.Second)),"Existing row changed: "+n+"/"+group.Key);}added[n]=actual.Rows.Count-old.Rows.Count;
  }
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),added,roots,audit=Audit,weaponGeneratorsUnchanged=true,allOriginalRowsAndOtherTablesUnchanged=true,itemTableRows=tables["ItemTableParam"].Rows.Count,itemLotRows=tables["ItemLotParam_enemy"].Rows.Count,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine($"PASS: {roots.Length} native roots converted with shared pools; {tables["ItemTableParam"].Rows.Count} item-table rows, {tables["ItemLotParam_enemy"].Rows.Count} loot rows");
 }
}
