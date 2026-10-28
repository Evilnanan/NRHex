using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
static class DeliberatePunch {
 public const int Owner=95004800;
 static string Name(BinderFile f)=>f.Name.Replace('\\','/').Split('/').Last();
 static void Check(bool ok,string why){if(!ok)throw new InvalidDataException(why);}
 static void Set(PARAM.Row r,string f,object v)=>r[f].Value=Convert.ChangeType(v,r[f].Value.GetType());
 static string Hash(string p)=>Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
 public static void Events(EMEVD ev){
  foreach(int id in new[]{1111,9100941}){
   var e=ev.Events.Single(e=>e.ID==id);Check(e.Parameters.Count==0,"Parameterized lifecycle");
   int at=id==1111?9:e.Instructions.FindLastIndex(i=>i.Bank==2004&&i.ID==21)+1;
   Check(at>0&&e.Instructions.Take(at).Any(i=>i.Bank==3),"Missing guarded lifecycle anchor");
   Check(!e.Instructions.Any(i=>Removal(i,Owner)),"Duplicate owner cleanup");
   e.Instructions.Insert(at,new(2004,21,new object[]{20000u,Owner}));
  }
 }
 static bool Removal(EMEVD.Instruction i,int id)=>i.Bank==2004&&i.ID==21&&i.ArgData.Length==8&&BitConverter.ToInt32(i.ArgData)==20000&&BitConverter.ToInt32(i.ArgData,4)==id;
 public static void Stage(string root,BND4 native,Func<string,Memory<byte>,PARAM> parse,Hex[] hexes){
  string dir=Path.Combine(root,"work/deliberate-punch/staged"),input=Path.Combine(root,"dist/mod/regulation.bin");Directory.CreateDirectory(dir);
  var b=SFUtil.DecryptNightreignRegulation(input);var before=b.Files.ToDictionary(f=>f.ID,f=>f.Bytes.ToArray());
  string[] names={"SpEffectParam","AttachEffectParam","PermanentBuffParam","EquipParamGoods","ItemTableParam"};
  var tables=names.ToDictionary(n=>n,n=>parse(n,b.Files.Single(f=>Name(f)==n+".param").Bytes));
  var origins=names.ToDictionary(n=>n,n=>parse(n,native.Files.Single(f=>Name(f)==n+".param").Bytes));var added=names.ToDictionary(n=>n,n=>new HashSet<int>());
  PARAM.Row Add(string n,int from,int id){Check(!tables[n].Rows.Any(r=>r.ID==id),$"Deliberate Punch ID collision: {n}/{id}");var r=new PARAM.Row(origins[n].Rows.First(r=>r.ID==from)){ID=id};tables[n].Rows.Add(r);added[n].Add(id);return r;}
  var h=hexes.Single(h=>h.Id==Owner);var s=Add("SpEffectParam",8000000,Owner);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("iconId",h.SmallIcon),("spEffectTextId_1",Owner),("displayIconForPermamentEffect",0)})Set(s,f,v);
  var a=Add("AttachEffectParam",8000000,Owner);foreach(var(f,v)in new[]{("passiveSpEffectId_1",Owner),("attachTextId",Owner),("statusIconId",h.SmallIcon),("compatibilityId",-1),("overrideBaseEffectId",Owner),("overrideEffectId",-1),("isNumericEffect",0),("displayPercentageSymbol",0),("displayedModifierValue",0)})Set(a,f,v);
  var p=Add("PermanentBuffParam",8000000,Owner);foreach(var(f,v)in new[]{("spEffectId",Owner),("iconId",h.LargeIcon),("displayNumericValue",2),("displayPercentageSymbol",0),("numericValue",0)})Set(p,f,v);
  var g=Add("EquipParamGoods",8000001,Owner);Set(g,"vagrantItemLotId",Owner);Set(g,"iconId",h.LargeIcon-1000);Set(g,"rarity",3);
  var t=Add("ItemTableParam",4200000,RewardCatalog.Table(Owner));Set(t,"disableParam_NT",0);Set(t,"itemId",Owner);Set(t,"chanceWeight",1000);
  Check(RewardCatalog.HistoryBit(Owner)==45,"History must append at bit 45");
  int marker=DoubleDip.FirstMarker+RewardCatalog.HistoryBit(Owner);
  var hidden=Add("SpEffectParam",8000000,marker);
  foreach(var(f,v)in new(string,object)[]{("maxHpRate",1f),("spCategory",20),("saveCategory",10),("effectEndurance",-1f),("dontDeleteOnDead",1),("isDisableNetSync",1),("iconId",-1),("spEffectTextId_1",-1),("displayIconForPermamentEffect",0)})Set(hidden,f,v);
  foreach(string n in names){Check(tables[n].Rows.Count<65536,"PARAM capacity exceeded");tables[n].Rows=tables[n].Rows.OrderBy(r=>r.ID).ToList();b.Files.Single(f=>Name(f)==n+".param").Bytes=tables[n].Write();}
  string output=Path.Combine(dir,"regulation.bin");SFUtil.EncryptNightreignRegulation(output,b,b.Compression);var actual=SFUtil.DecryptNightreignRegulation(output);
  Check(actual.Version==b.Version&&actual.Compression==b.Compression&&actual.Files.Count==b.Files.Count,"Binder metadata changed");
  foreach(var f in actual.Files){
   var intended=b.Files.Single(x=>x.ID==f.ID);Check(f.Name==intended.Name&&f.Flags==intended.Flags&&f.Bytes.Span.SequenceEqual(intended.Bytes.Span),"Parameter roundtrip failed");
   var n=names.SingleOrDefault(n=>Name(f)==n+".param");if(n==null){Check(f.Bytes.Span.SequenceEqual(before[f.ID]),"Unrelated table changed");continue;}
   var old=parse(n,before[f.ID]);var now=parse(n,f.Bytes);Check(now.Rows.Count==old.Rows.Count+added[n].Count&&old.Rows.OrderBy(r=>r.ID).Zip(now.Rows.Where(r=>!added[n].Contains(r.ID)).OrderBy(r=>r.ID)).All(pair=>CageAI.Same(pair.First,pair.Second)),"Existing parameter row changed");
  }
  string eventInput=Path.Combine(root,"dist/mod/event/common.emevd.dcx");var ev=EMEVD.Read(eventInput);var previous=EMEVD.Read(eventInput);Events(ev);
  var parent=ev.Events.Single(e=>e.ID==0);Check(parent.Parameters.Count==0,"Parameterized constructor");
  var inits=parent.Instructions.Where(i=>i.Bank==2000&&i.ID==0&&i.ArgData.Length>=8&&BitConverter.ToInt32(i.ArgData,4)==DoubleDip.GrantEvent).ToArray();
  Check(inits.Length==hexes.Length-2&&inits.All(i=>i.ArgData.Length==16)&&!inits.Any(i=>BitConverter.ToInt32(i.ArgData,12)==Owner),"Unexpected Double Surprise worker instances");
  parent.Instructions.Insert(0,new(2000,0,new object[]{inits.Length,DoubleDip.GrantEvent,marker,Owner}));
  foreach(int id in new[]{1111,9100941}){var e=ev.Events.Single(e=>e.ID==id);int at=e.Instructions.FindIndex(i=>Removal(i,Owner));e.Instructions.Insert(at+1,new(2004,21,new object[]{20000u,marker}));}
  string eventOutput=Path.Combine(dir,"common.emevd.dcx");ev.Write(eventOutput);var final=EMEVD.Read(eventOutput);
  Check(final.Format==previous.Format&&final.Compression==previous.Compression&&final.Events.Count==previous.Events.Count&&final.StringData.SequenceEqual(previous.StringData)&&final.LinkedFileOffsets.SequenceEqual(previous.LinkedFileOffsets),"Event metadata changed");
  foreach(var e in previous.Events){
   var now=final.Events.Single(x=>x.ID==e.ID);Check(Cages.Same(now,ev.Events.Single(x=>x.ID==e.ID)),"Event roundtrip mismatch");
   if(e.ID is 1111 or 9100941){Check(now.Instructions.Count(i=>Removal(i,Owner))==1&&now.Instructions.Count(i=>Removal(i,marker))==1,"Missing cleanup");now.Instructions.RemoveAll(i=>Removal(i,Owner)||Removal(i,marker));}
   if(e.ID==0)now.Instructions.RemoveAt(0);Check(Cages.Same(e,now),"Unrelated event changed");
  }
  RewardCatalog.Write(Path.Combine(dir,"hex_pool.rs"),hexes);
  HexTexts.Stage(root,hexes,Path.Combine(dir,"texts"),allRewards:true,onlyIds:new[]{Owner});
  File.WriteAllText(Path.Combine(dir,"validation.json"),JsonSerializer.Serialize(new{inputSha256=Hash(input),outputSha256=Hash(output),eventInputSha256=Hash(eventInput),eventOutputSha256=Hash(eventOutput),hexCount=hexes.Length,historyBit=45,historyBytes=6,addedRows=added.ToDictionary(x=>x.Key,x=>x.Value.Order().ToArray()),allExistingRowsAndUnrelatedTablesUnchanged=true,onlyThreeExistingEventsExtended=true,gameplayVerified=false,multiplayerVerified=false},new JsonSerializerOptions{WriteIndented=true}));
  Console.WriteLine("PASS: Deliberate Punch neutral owner, append-only reward, Double Surprise instance and guarded cleanup staged");
 }
}
