using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// MMV retains the conditional Claw Shot TAE events but omits the outgoing
// bleed-only attack and its behavior. Restore only those two native rows.
static class MmvWylderBleed {
 const int Outgoing = 600000030;
 static readonly string[] Tables = { "BehaviorParam_PC", "AtkParam_Pc" };
 static string Name(BinderFile f) => f.Name.Replace('\\', '/').Split('/').Last();
 static void Check(bool ok, string why) { if (!ok) throw new InvalidDataException(why); }
 static string Hash(string path) => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
 static bool Same(PARAM.Row a, PARAM.Row b, bool names = true) => a.ID == b.ID && (!names || a.Name == b.Name) &&
  a.Cells.Select(c => (c.Def.InternalName, JsonSerializer.Serialize(c.Value)))
   .SequenceEqual(b.Cells.Select(c => (c.Def.InternalName, JsonSerializer.Serialize(c.Value))));

 public static Dictionary<string, PARAM> Restore(BND4 original, BND4 target,
  Func<string, Memory<byte>, PARAM> parse, string animationPath) {
  Check(original.Version == target.Version, "Wylder bleed parameter version changed");
  PARAM Table(BND4 b, string n) => parse(n, b.Files.Single(f => Name(f) == n + ".param").Bytes);
  var nativeEffects = Table(original, "SpEffectParam");
  var effects = Table(target, "SpEffectParam");
  Check(Convert.ToInt32(effects.Rows.Single(r => r.ID == 7500000)["stateInfo"].Value) == 2035,
   "Wylder bleed ownership condition changed; review repair");
  foreach (int id in new[] { 7500001, 7500002 })
   Check(Same(nativeEffects.Rows.Single(r => r.ID == id), effects.Rows.Single(r => r.ID == id), names: false),
    "Wylder bleed effect changed; review repair");
  var tae = TAE.Read(BND4.Read(animationPath).Files.Single(f => Name(f) == "a900.tae").Bytes);
  foreach (var (animation, judge) in new[] { (500000, 6030), (505000, 6030), (500005, 6035), (505005, 6035) }) {
   var events = tae.Animations.Single(a => a.ID == animation).Events.Where(e => e.Type == 1 &&
    BitConverter.ToInt32(e.GetParameterBytes(false), 8) == judge).ToArray();
   string data = judge == 6030 ? "00000000010000008E170000FF03F30700000000000000000000000000000000"
    : "000000000100000093170000FF03F30700000000000000000000000000000000";
   Check(events.Length == 1 && Convert.ToHexString(events[0].GetParameterBytes(false)) == data &&
    events[0].StartTime == (judge == 6030 ? 0.16666667f : 0.3f) && events[0].EndTime == 0.46666667f,
    "Wylder bleed animation/ownership window changed; review repair");
  }
  var changed = new Dictionary<string, PARAM>();
  // Validate both tables before mutating either binder member.
  foreach (string name in Tables) {
   var native = Table(original, name);
   var donor = native.Rows.Single(r => r.ID == Outgoing);
   var current = Table(target, name);
   Check(Same(native.Rows.Single(r => r.ID == 600000035), current.Rows.Single(r => r.ID == 600000035), names: false),
    "Wylder pull attack/behavior changed; review repair");
   var existing = current.Rows.Where(r => r.ID == Outgoing).ToArray();
   Check(existing.Length <= 1 && (existing.Length == 0 || Same(donor, existing[0], names: false)),
    "Wylder outgoing bleed row conflicts with native repair: " + name);
   if (existing.Length == 1) continue;
   int index = current.Rows.FindIndex(r => r.ID > Outgoing);
   current.Rows.Insert(index < 0 ? current.Rows.Count : index, new PARAM.Row(donor));
   changed.Add(name, current);
  }
  foreach (var (name, table) in changed)
   target.Files.Single(f => Name(f) == name + ".param").Bytes = table.Write();
  return changed;
 }

 public static void Stage(BND4 original, Func<string, Memory<byte>, PARAM> parse, string inputMod, string output) {
  string input = Path.Combine(inputMod, "regulation.bin");
  string animation = Path.Combine(inputMod, "chr/c0000.anibnd.dcx");
  var inputHash = Hash(input); var animationHash = Hash(animation);
  var target = SFUtil.DecryptNightreignRegulation(input);
  var before = target.Files.ToDictionary(f => f.ID, f => f.Bytes.ToArray());
  var changed = Restore(original, target, parse, animation);
  Directory.CreateDirectory(output);
  string path = Path.Combine(output, "regulation.bin");
  Check(Path.GetFullPath(path) != Path.GetFullPath(input), "Stage repair outside the input package");
  SFUtil.EncryptNightreignRegulation(path, target, target.Compression);
  var read = SFUtil.DecryptNightreignRegulation(path);
  Check(read.Version == target.Version && read.Compression == target.Compression && read.Files.Count == target.Files.Count,
   "Wylder repair binder metadata changed");
  foreach (var (expected, actual) in target.Files.Zip(read.Files)) {
   Check(expected.ID == actual.ID && expected.Name == actual.Name && expected.Flags == actual.Flags &&
    expected.Bytes.Span.SequenceEqual(actual.Bytes.Span), "Wylder repair readback differs");
   string name = Path.GetFileNameWithoutExtension(Name(actual));
   if (!changed.ContainsKey(name)) {
    Check(actual.Bytes.Span.SequenceEqual(before[actual.ID]), "Unrelated parameter table changed");
    continue;
   }
   var oldRows = parse(name, before[actual.ID]).Rows;
   var rows = parse(name, actual.Bytes).Rows;
   Check(rows.Count == oldRows.Count + 1 && rows.Where(r => r.ID != Outgoing).Zip(oldRows).All(p => Same(p.First, p.Second)),
    "Existing parameter rows/order changed");
   var donor = parse(name, original.Files.Single(f => Name(f) == name + ".param").Bytes).Rows.Single(r => r.ID == Outgoing);
   Check(Same(donor, rows.Single(r => r.ID == Outgoing)), "Restored row differs from native");
  }
  Check(Restore(original, read, parse, animation).Count == 0, "Repair is not idempotent");
  Check(Hash(input) == inputHash && Hash(animation) == animationHash, "Repair changed its inputs");
  File.WriteAllText(Path.Combine(output, "validation.json"), JsonSerializer.Serialize(new {
   inputSha256 = inputHash, outputSha256 = Hash(path), animationSha256 = animationHash,
   restoredRow = Outgoing, changedTables = changed.Keys, existingRowsAndOtherTablesPreserved = true,
   nativeRowsRestored = true, idempotencePassed = true, gameplayVerified = false
  }, new JsonSerializerOptions { WriteIndented = true }));
  Console.WriteLine($"PASS: Wylder outgoing bleed restored in {changed.Count} tables; readback, preservation and idempotence checked");
 }
}
