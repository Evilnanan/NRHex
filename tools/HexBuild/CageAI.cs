using System.Security.Cryptography;
using System.Text.Json;
using SoulsFormats;

// Separate Think rows keep original tower/field encounters unchanged.
static class CageAI {
 public static readonly Dictionary<string,double> Limits=new(){
  ["eye_dist"]=30,["searchEye_dist"]=20,["ear_dist"]=30,["nose_dist"]=0,
  ["BattleStartDist"]=30,["battleEye_updateDist"]=30,
  ["backhomeDist"]=40,["maxBackhomeDist"]=60,["backhomeBattleDist"]=20,
  ["SightTargetForgetTime"]=8,["MemoryTargetForgetTime"]=5
 };
 public static bool Same(PARAM.Row a,PARAM.Row b)=>a.ID==b.ID&&a.Cells.Zip(b.Cells).All(p=>p.First==null?p.Second==null:p.Second!=null&&JsonSerializer.Serialize(p.First.Value)==JsonSerializer.Serialize(p.Second.Value));
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static string Hash(string path)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
 public static void Add(string root,PARAM table,Cages.Boss[] bosses,Func<int,int,PARAM.Row>? clone=null){
  var towers=Cages.TowerSources(root,bosses);
  foreach(var (boss,index) in bosses.Select((b,i)=>(b,i))){
   int id=95300000+index;var source=table.Rows.Single(r=>r.ID==boss.think);
   var tower=towers.SingleOrDefault(t=>t.cloneNpc==id);
   var baseline=tower==null?source:table.Rows.Single(r=>r.ID==tower.alternateThink);
   Check(Equals(source["logicId"].Value,baseline["logicId"].Value)&&Equals(source["battleGoalID"].Value,baseline["battleGoalID"].Value),"Tower alternate AI has a different combat script");
   var expected=new PARAM.Row(source){ID=id};
   foreach(var (field,limit) in Limits){
    double value=Math.Min(limit,Math.Min(Convert.ToDouble(source[field].Value),Convert.ToDouble(baseline[field].Value)));
    Check(value>=0&&double.IsFinite(value),"Unexpected negative AI range: "+field);
    expected[field].Value=Convert.ChangeType(value,source[field].Value.GetType());
   }
   Check(Convert.ToDouble(expected["backhomeBattleDist"].Value)<=Convert.ToDouble(expected["backhomeDist"].Value)&&Convert.ToDouble(expected["backhomeDist"].Value)<=Convert.ToDouble(expected["maxBackhomeDist"].Value),"Invalid cage return thresholds");
   var existing=table.Rows.SingleOrDefault(r=>r.ID==id);
   if(existing!=null){
    Check(source.Cells.Where(c=>c!=null&&!Limits.ContainsKey(c.Def.InternalName)).All(c=>JsonSerializer.Serialize(existing[c.Def.InternalName].Value)==JsonSerializer.Serialize(c.Value)),"Cage AI ID collision / unexpected combat change");
    table.Rows[table.Rows.IndexOf(existing)]=expected;
   }else if(clone!=null){
    var row=clone(boss.think,id);foreach(var field in Limits.Keys)row[field].Value=expected[field].Value;
   }else table.Rows.Add(expected);
  }
 }
 public static void Validate(PARAM before,PARAM expected,PARAM actual){
  var ids=Enumerable.Range(95300000,37).ToHashSet();
  Check(actual.Rows.Count==before.Rows.Count(r=>!ids.Contains(r.ID))+37,"Unexpected AI row count");
  foreach(var row in before.Rows.Where(r=>!ids.Contains(r.ID)))Check(Same(row,actual.Rows.Single(r=>r.ID==row.ID)),"Original AI row changed: "+row.ID);
  foreach(int id in ids)Check(Same(expected.Rows.Single(r=>r.ID==id),actual.Rows.Single(r=>r.ID==id)),"Cloned AI failed readback: "+id);
 }
 public static object StageMaps(string root,Cages.Boss[] bosses){
  string output=Path.Combine(root,".work/cage-ai/staged/map/mapstudio");Directory.CreateDirectory(output);
  var reports=new List<object>();
  foreach(int block in new[]{50,60,70,80}){
   string name=$"m46_{block:00}_00_00.msb.dcx",input=Path.Combine(root,"dist/mod/map/mapstudio",name),target=Path.Combine(output,name);
   uint prefix=(uint)(4600+block)*10000;
   var map=MSB_NR.Read(input);var before=map.Write();var oldThinks=new Dictionary<uint,int>();
   foreach(var (boss,index) in bosses.Select((b,i)=>(b,i))){
    var enemy=map.Parts.Enemies.Single(p=>p.EntityData.EntityID==prefix+9000+(uint)index);
    Check(enemy.NpcParamId==95300000+index&&enemy.NpcThinkParamId is var think&&(think==boss.think||think==95300000+index),"Unexpected cage AI binding");
    oldThinks[enemy.EntityData.EntityID]=enemy.NpcThinkParamId;enemy.NpcThinkParamId=95300000+index;
   }
   map.Write(target);var check=MSB_NR.Read(target);
   foreach(var (entity,oldThink) in oldThinks){var enemy=check.Parts.Enemies.Single(p=>p.EntityData.EntityID==entity);Check(enemy.NpcThinkParamId==95300000+(int)(entity-prefix-9000),"Cage AI map reference failed readback");enemy.NpcThinkParamId=oldThink;}
   Check(check.Write().SequenceEqual(before),"AI patch changed unrelated map content");
   reports.Add(new{name,inputSha256=Hash(input),outputSha256=Hash(target),changedThinkReferences=37,unrelatedMapContentPreserved=true});
  }
  return reports;
 }
 public static object Report(PARAM native,PARAM actual,Cages.Boss[] bosses)=>bosses.Select((boss,index)=>new{
  boss.name,boss.map,sourceThink=boss.think,cloneThink=95300000+index,
  fields=Limits.Keys.ToDictionary(k=>k,k=>new{before=native.Rows.Single(r=>r.ID==boss.think)[k].Value,after=actual.Rows.Single(r=>r.ID==95300000+index)[k].Value}),
  combatScriptPreserved=Equals(actual.Rows.Single(r=>r.ID==95300000+index)["logicId"].Value,native.Rows.Single(r=>r.ID==boss.think)["logicId"].Value)&&Equals(actual.Rows.Single(r=>r.ID==95300000+index)["battleGoalID"].Value,native.Rows.Single(r=>r.ID==boss.think)["battleGoalID"].Value)
 });
}
