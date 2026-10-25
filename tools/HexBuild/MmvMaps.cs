using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;
using System.Collections;

static class MmvMaps {
    static string Hash(byte[] bytes) => Convert.ToHexString(SHA256.HashData(bytes));
    static void Require(bool ok, string message) { if (!ok) throw new InvalidDataException(message); }
    static void Remove(object container, object entry) {
        var list = container.GetType().GetProperties().Where(p => p.GetIndexParameters().Length == 0)
            .Select(p => p.GetValue(container)).OfType<IList>().Single(l => l.Contains(entry));
        list.Remove(entry);
    }
    static string Signature(object value) => JsonSerializer.Serialize(value, value.GetType(), new JsonSerializerOptions { IncludeFields = true });
    static IEnumerable<uint> Ids(object entry) {
        foreach (string name in new[] { "EntityID", "Common_EntityID" }) {
            var value = entry.GetType().GetProperty(name)?.GetValue(entry);
            if (value != null) { uint id = Convert.ToUInt32(value); if (id != 0 && id != uint.MaxValue) yield return id; }
        }
        var entity = entry.GetType().GetProperty("EntityData")?.GetValue(entry);
        if (entity != null) {
            foreach (var id in Ids(entity)) yield return id;
            if (entity.GetType().GetProperty("EntityGroupIDs")?.GetValue(entity) is uint[] groups)
                foreach (var id in groups.Where(i => i != 0 && i != uint.MaxValue)) yield return id;
        }
    }
    public static void Stage(string root, string mmv) {
        var reports = new List<object>(); var output = Path.Combine(root, "work/compat/mmv/staged/mod/map/mapstudio");
        Directory.CreateDirectory(output);
        foreach (var path in Directory.GetFiles(Path.Combine(root, "dist/mod/map/mapstudio"), "*.msb.dcx")) {
            var name = Path.GetFileName(path); var originalPath = Path.Combine(root, "assets/original/map/mapstudio", name);
            var mmvPath = Path.Combine(mmv, "mod/map/mapstudio", name); var input = File.Exists(mmvPath) ? mmvPath : originalPath;
            var baseline = MSB_NR.Read(originalPath); var nrhex = MSB_NR.Read(path); var target = MSB_NR.Read(input);
            var raw = DCX.Decompress(input).ToArray();
            Require(DCX.Decompress(target.Write()).Span.SequenceEqual(raw), $"MSB cannot preserve original raw input: {name}");
            int block = int.Parse(name.Substring(4, 2)); uint prefix = (uint)(4600 + block) * 10000;
            var originalStone = baseline.Parts.Assets.Single(a => a.EntityData.EntityID == prefix + 1200);
            var mmvStone = target.Parts.Assets.Single(a => a.EntityData.EntityID == prefix + 1200);
            Require(originalStone.Position == mmvStone.Position && originalStone.Rotation == mmvStone.Rotation,
                $"MMV gaol entrance moved; spawn layout needs an adapter: {name}");
            var existingIds = target.Parts.GetEntries().Cast<object>().Concat(target.Regions.GetEntries()).Concat(target.Events.GetEntries())
                .SelectMany(Ids).ToHashSet();
            var added = new Dictionary<string, HashSet<string>>();
            void Merge<T>(IEnumerable<T> b, IEnumerable<T> n, IEnumerable<T> m, object nrContainer, Action<T> append, Func<T, string> getName, string kind) where T : class {
                var originalNames = b.Select(getName).ToHashSet(); var existing = m.ToDictionary(getName);
                var additions = n.Where(e => !originalNames.Contains(getName(e))).ToArray();
                added[kind] = new HashSet<string>();
                foreach (var entry in additions) {
                    var entryName = getName(entry);
                    if (existing.TryGetValue(entryName, out var collision)) {
                        Require(kind == "models" && Signature(entry) == Signature(collision), $"MSB entry name collision: {name}/{entryName}");
                    } else {
                        Require(!Ids(entry).Any(existingIds.Contains), $"MMV entity/group ID collision: {name}/{entryName}");
                        append(entry); added[kind].Add(entryName);
                    }
                    Remove(nrContainer, entry);
                }
            }
            Merge(baseline.Models.GetEntries(), nrhex.Models.GetEntries(), target.Models.GetEntries(), nrhex.Models,
                e => target.Models.Add(e.DeepCopy()), e => e.Name, "models");
            Merge(baseline.Parts.GetEntries(), nrhex.Parts.GetEntries(), target.Parts.GetEntries(), nrhex.Parts,
                e => target.Parts.Add(e.DeepCopy()), e => e.Name, "parts");
            Merge(baseline.Regions.GetEntries(), nrhex.Regions.GetEntries(), target.Regions.GetEntries(), nrhex.Regions,
                e => target.Regions.Add(e.DeepCopy()), e => e.Name, "regions");
            Merge(baseline.Events.GetEntries(), nrhex.Events.GetEntries(), target.Events.GetEntries(), nrhex.Events,
                e => target.Events.Add(e.DeepCopy()), e => e.Name, "events");
            Require(DCX.Decompress(nrhex.Write()).Span.SequenceEqual(DCX.Decompress(baseline.Write()).Span),
                $"NRHex changes existing map entries, requires an adapter: {name}");
            var destination = Path.Combine(output, name); target.Write(destination); var reread = MSB_NR.Read(destination);
            Require(DCX.Decompress(reread.Write()).Span.SequenceEqual(DCX.Decompress(destination).Span), "Merged MSB readback mismatch");
            foreach (var entry in reread.Models.GetEntries().Where(e => added["models"].Contains(e.Name)).ToArray()) Remove(reread.Models, entry);
            foreach (var entry in reread.Parts.GetEntries().Where(e => added["parts"].Contains(e.Name)).ToArray()) Remove(reread.Parts, entry);
            foreach (var entry in reread.Regions.GetEntries().Where(e => added["regions"].Contains(e.Name)).ToArray()) Remove(reread.Regions, entry);
            foreach (var entry in reread.Events.GetEntries().Where(e => added["events"].Contains(e.Name)).ToArray()) Remove(reread.Events, entry);
            Require(DCX.Decompress(reread.Write()).Span.SequenceEqual(raw), $"MSB inverse did not restore MMV bytes: {name}");
            reports.Add(new { file = name, added = added.ToDictionary(p => p.Key, p => p.Value.Count),
                sha256 = Hash(File.ReadAllBytes(destination)), inverseRestoresRawMmvMap = true });
        }
        File.WriteAllText(Path.Combine(root, "work/compat/mmv/staged/maps-validation.json"), JsonSerializer.Serialize(new { files = reports, gameplayVerified = false }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine($"Staged {reports.Count} MSB files; removing NRHex additions restores the exact MMV map bytes");
    }
    public static void Audit(string root, string mmv) {
        var reports = new List<object>();
        foreach (var path in Directory.GetFiles(Path.Combine(root, "dist/mod/map/mapstudio"), "*.msb.dcx")) {
            var name = Path.GetFileName(path);
            var mmvPath = Path.Combine(mmv, "mod/map/mapstudio", name);
            var input = File.Exists(mmvPath) ? mmvPath : Path.Combine(root, "assets/original/map/mapstudio", name);
            var raw = DCX.Decompress(input).ToArray(); var parsed = MSB_NR.Read(input); var roundtrip = DCX.Decompress(parsed.Write()).ToArray();
            var baseline = MSB_NR.Read(Path.Combine(root, "assets/original/map/mapstudio", name));
            var nrhex = MSB_NR.Read(path);
            reports.Add(new { name, mmvInput = File.Exists(mmvPath), rawBytes = raw.Length, writtenBytes = roundtrip.Length,
                rawHash = Hash(raw), roundtripHash = Hash(roundtrip), exactRoundtrip = raw.SequenceEqual(roundtrip),
                parts = parsed.Parts.GetEntries().Count, regions = parsed.Regions.GetEntries().Count,
                models = parsed.Models.GetEntries().Count, events = parsed.Events.GetEntries().Count,
                newParts = nrhex.Parts.GetEntries().Where(p => !baseline.Parts.GetEntries().Any(b => b.Name == p.Name)).Select(p => new { p.Name, type = p.GetType().Name }),
                newRegions = nrhex.Regions.GetEntries().Where(p => !baseline.Regions.GetEntries().Any(b => b.Name == p.Name)).Select(p => new { p.Name, type = p.GetType().Name }),
                newEvents = nrhex.Events.GetEntries().Where(p => !baseline.Events.GetEntries().Any(b => b.Name == p.Name)).Select(p => new { p.Name, type = p.GetType().Name }) });
        }
        File.WriteAllText(Path.Combine(root, "work/compat/mmv/maps-audit.json"), JsonSerializer.Serialize(reports, new JsonSerializerOptions { WriteIndented = true }));
    }
}
