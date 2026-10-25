using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Stage the two formats for which an entry-level merge is already defined.
// These files are incomplete work products, never an installable MMV overlay.
static class MmvStage {
    static string Name(BinderFile file) => file.Name.Replace('\\', '/').Split('/').Last();
    static string Value(object? value) => value switch {
        float number => "f32:" + BitConverter.SingleToInt32Bits(number).ToString("X8"),
        double number => "f64:" + BitConverter.DoubleToInt64Bits(number).ToString("X16"),
        _ => JsonSerializer.Serialize(value)
    };
    static bool Same(PARAM.Row a, PARAM.Row b) => a.Name == b.Name &&
        a.Cells.Select(c => (c.Def.InternalName, Value(c.Value)))
            .SequenceEqual(b.Cells.Select(c => (c.Def.InternalName, Value(c.Value))));
    static void Require(bool condition, string message) {
        if (!condition) throw new InvalidDataException(message);
    }
    static void CheckBinder(BND4 expected, BND4 actual) {
        Require(expected.Version == actual.Version && expected.Compression == actual.Compression &&
            expected.Files.Count == actual.Files.Count, "Merged binder metadata mismatch");
        foreach (var pair in expected.Files.Zip(actual.Files)) {
            Require(pair.First.ID == pair.Second.ID && pair.First.Name == pair.Second.Name &&
                pair.First.Flags == pair.Second.Flags && pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span),
                $"Merged binder readback mismatch: {pair.First.Name}");
        }
    }

    public static void Stage(string root, string mmv, BND4 original, Func<string, Memory<byte>, PARAM> parse) {
        var output = Path.Combine(root, "work/compat/mmv/staged/mod");
        Directory.CreateDirectory(output);
        var manifest = Path.Combine(root, "work/compat/mmv/staged/manifest.json");
        // A previous successful preview must not attest partially refreshed files.
        if (File.Exists(manifest)) File.Delete(manifest);
        var ours = SFUtil.DecryptNightreignRegulation(Path.Combine(root, "dist/mod/regulation.bin"));
        var merged = SFUtil.DecryptNightreignRegulation(Path.Combine(mmv, "mod/regulation.bin"));
        Require(original.Version == ours.Version && ours.Version == merged.Version, "Parameter version mismatch");
        var changedTables = new Dictionary<string, PARAM>();
        var baseFiles = original.Files.ToDictionary(Name);
        var ourFiles = ours.Files.ToDictionary(Name);
        var theirFiles = merged.Files.ToDictionary(Name);
        Require(baseFiles.Keys.All(ourFiles.ContainsKey), "Removed parameter table requires an adapter");
        foreach (var (name, file) in ourFiles) {
            Require(baseFiles.TryGetValue(name, out var baseline), $"New parameter table requires an adapter: {name}");
            if (file.Bytes.Span.SequenceEqual(baseline!.Bytes.Span)) continue;
            Require(theirFiles.TryGetValue(name, out var target), $"MMV parameter table missing: {name}");
            var tableName = Path.GetFileNameWithoutExtension(name);
            var b = parse(tableName, baseline.Bytes).Rows.GroupBy(r => r.ID).ToDictionary(g => g.Key, g => g.ToArray());
            var n = parse(tableName, file.Bytes).Rows.GroupBy(r => r.ID).ToDictionary(g => g.Key, g => g.ToArray());
            var result = parse(tableName, target!.Bytes);
            var m = result.Rows.GroupBy(r => r.ID).ToDictionary(g => g.Key, g => g.ToArray());
            Require(b.Keys.All(n.ContainsKey), $"Removed parameter rows require an adapter: {name}");
            bool changed = false;
            foreach (var (id, rows) in n) {
                if (b.TryGetValue(id, out var before) && before.Length == rows.Length &&
                    before.Zip(rows).All(pair => Same(pair.First, pair.Second))) continue;
                if (before == null) {
                    if (m.TryGetValue(id, out var existing)) {
                        Require(existing.Length == rows.Length && existing.Zip(rows).All(p => Same(p.First, p.Second)),
                            $"New row collision: {name}/{id}");
                    } else {
                        // Some native table formats intentionally have repeated IDs.
                        // Add the complete ordered group, never collapse it to one row.
                        var insertion = result.Rows.FindIndex(row => row.ID > id);
                        result.Rows.InsertRange(insertion < 0 ? result.Rows.Count : insertion,
                            rows.Select(row => new PARAM.Row(row)));
                        changed = true;
                    }
                    continue;
                }
                Require(before.Length == 1 && rows.Length == 1 && m.TryGetValue(id, out var _) && m[id].Length == 1,
                    $"Ambiguous or missing changed row: {name}/{id}");
                var wanted = rows[0]; var old = before[0]; var current = m[id][0];
                if (wanted.Name != old.Name) {
                    Require(current.Name == old.Name || current.Name == wanted.Name, $"Row name collision: {name}/{id}");
                    current.Name = wanted.Name; changed = true;
                }
                foreach (var cell in wanted.Cells) {
                    var key = cell.Def.InternalName;
                    if (Value(cell.Value) == Value(old[key].Value)) continue;
                    Require(Value(current[key].Value) == Value(old[key].Value) || Value(current[key].Value) == Value(cell.Value),
                        $"Parameter field collision: {name}/{id}/{key}");
                    current[key].Value = cell.Value; changed = true;
                }
            }
            if (!changed) continue;
            // Existing MMV row order, including repeated IDs, remains untouched.
            target.Bytes = result.Write();
            changedTables.Add(tableName, result);
        }
        var regulation = Path.Combine(output, "regulation.bin");
        SFUtil.EncryptNightreignRegulation(regulation, merged, merged.Compression);
        var readback = SFUtil.DecryptNightreignRegulation(regulation);
        CheckBinder(merged, readback);
        foreach (var (name, expected) in changedTables) {
            var actual = parse(name, readback.Files.Single(f => Name(f) == name + ".param").Bytes);
            Require(expected.Rows.Count == actual.Rows.Count && expected.Rows.Zip(actual.Rows).All(p => Same(p.First, p.Second)),
                $"Parameter row readback failed: {name}");
        }

        var written = new List<string> { "regulation.bin" };
        foreach (var language in new[] { "engus", "zhocn" }) foreach (var category in new[] { "item", "menu" }) {
            var relative = $"msg/{language}/{category}_dlc01.msgbnd.dcx";
            var baseline = BND4.Read(Path.Combine(root, "assets/original", relative)).Files.ToDictionary(Name);
            var desired = BND4.Read(Path.Combine(root, "dist/mod", relative)).Files.ToDictionary(Name);
            var result = BND4.Read(Path.Combine(mmv, "mod", relative));
            var targets = result.Files.ToDictionary(Name);
            Require(baseline.Keys.All(desired.ContainsKey), "Removed message binder member needs an adapter");
            foreach (var (name, file) in desired) {
                Require(baseline.TryGetValue(name, out var old), $"New message member needs an adapter: {relative}/{name}");
                if (file.Bytes.Span.SequenceEqual(old!.Bytes.Span)) continue;
                Require(name.EndsWith(".fmg") && targets.TryGetValue(name, out var _), $"Unsupported message member: {relative}/{name}");
                var b = FMG.Read(old.Bytes).Entries.ToDictionary(e => e.ID, e => e.Text);
                var n = FMG.Read(file.Bytes).Entries.ToDictionary(e => e.ID, e => e.Text);
                var fmg = FMG.Read(targets[name].Bytes);
                var m = fmg.Entries.ToDictionary(e => e.ID);
                Require(b.Keys.All(n.ContainsKey), $"Deleted text entry needs an adapter: {relative}/{name}");
                foreach (var (id, text) in n) {
                    bool existsBefore = b.TryGetValue(id, out var before);
                    if (existsBefore && text == before) continue;
                    if (m.TryGetValue(id, out var current)) {
                        Require(current.Text == text || existsBefore && current.Text == before, $"Text collision: {relative}/{name}/{id}");
                        current.Text = text;
                    } else {
                        Require(!existsBefore, $"MMV deleted a changed text entry: {relative}/{name}/{id}");
                        fmg.Entries.Add(new FMG.Entry(fmg, id, text));
                    }
                }
                fmg.Entries.Sort((a, b) => a.ID.CompareTo(b.ID));
                targets[name].Bytes = fmg.Write();
                var parsed = FMG.Read(targets[name].Bytes);
                Require(fmg.Entries.Select(e => (e.ID, e.Text)).SequenceEqual(parsed.Entries.Select(e => (e.ID, e.Text))), "Text readback mismatch");
            }
            var path = Path.Combine(output, relative);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            result.Write(path); CheckBinder(result, BND4.Read(path)); written.Add(relative);
        }
        var hashes = written.ToDictionary(p => p, p => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Path.Combine(output, p)))));
        File.WriteAllText(manifest, JsonSerializer.Serialize(new { schemaVersion = 1, stage = "partial-preview", releaseReady = false,
            gameplayVerified = false, parameterTables = changedTables.Count, files = hashes }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine($"Staged and read back {written.Count} merged parameter/text files; incomplete preview only: {output}");
    }
}
