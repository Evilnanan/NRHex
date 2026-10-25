using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Preserve MMV event bodies. The three shared edits below have reviewed insertion
// boundaries; all other simultaneous edits fail rather than selecting one mod.
static class MmvEvents {
    static void Require(bool ok, string why) { if (!ok) throw new InvalidDataException(why); }
    static bool Same(EMEVD.Instruction a, EMEVD.Instruction b) => a.Bank == b.Bank && a.ID == b.ID &&
        a.Layer == b.Layer && a.ArgData.SequenceEqual(b.ArgData);
    static EMEVD.Instruction Copy(EMEVD.Instruction i) => new(i.Bank, i.ID, i.ArgData.ToArray()) { Layer = i.Layer };
    static bool Header(EMEVD.Event a, EMEVD.Event b) => a.ID == b.ID && a.Name == b.Name && a.RestBehavior == b.RestBehavior &&
        JsonSerializer.Serialize(a.Parameters) == JsonSerializer.Serialize(b.Parameters);
    static bool Metadata(EMEVD a, EMEVD b) => a.Format == b.Format && a.StringData.SequenceEqual(b.StringData) &&
        a.LinkedFileOffsets.SequenceEqual(b.LinkedFileOffsets);

    static EMEVD.Event CageOpener(EMEVD.Event mmv, EMEVD.Event nrhex) {
        Require(mmv.ID == 90045000 && mmv.Instructions.Count == 25 && nrhex.ID == Cages.OpenEvent && nrhex.Instructions.Count == 48,
            "Gaol opener layout requires review");
        Require(mmv.Instructions[1].Bank == 1012 && mmv.Instructions[1].ID == 3 &&
            mmv.Instructions[9].Bank == 3 && mmv.Instructions[9].ID == 4 &&
            mmv.Instructions[21].Bank == 2003 && mmv.Instructions[21].ID == 100 &&
            mmv.Instructions[24].Bank == 1000 && mmv.Instructions[24].ID == 4 &&
            !mmv.Instructions.Any(i => i.Bank == 1014), "MMV gaol control flow changed");
        var result = new EMEVD.Event(nrhex.ID, nrhex.RestBehavior) { Name = nrhex.Name };
        var copied = new List<(int Output, int Source)>();
        void Range(EMEVD.Event input, int start, int end) {
            for (int i = start; i < end; i++) {
                var at = result.Instructions.Count; result.Instructions.Add(Copy(input.Instructions[i]));
                if (ReferenceEquals(input, mmv)) copied.Add((at, i));
                foreach (var p in input.Parameters.Where(p => p.InstructionIndex == i)) result.Parameters.Add(new EMEVD.Parameter {
                    InstructionIndex = at, TargetStartByte = p.TargetStartByte, SourceStartByte = p.SourceStartByte,
                    ByteCount = p.ByteCount, UnkID = p.UnkID
                });
            }
        }
        // MMV's activation, key check, consumption, dialog and animation survive.
        // Only owners with unused cage capacity take NRHex's existing alternate path.
        Range(mmv, 0, 9); Range(nrhex, 10, 18); Range(mmv, 9, 25);
        Range(nrhex, 34, 43); Range(mmv, 21, 24); Range(nrhex, 46, 48);
        Require(result.Instructions.Count == 47 && copied.Select(p => p.Source).Take(25).SequenceEqual(Enumerable.Range(0, 25)),
            "MMV gaol instruction coverage failed");
        foreach (var (output, source) in copied) {
            Require(Same(result.Instructions[output], mmv.Instructions[source]), "MMV gaol instruction changed");
            var expected = mmv.Parameters.Where(p => p.InstructionIndex == source).Select(p => (p.TargetStartByte, p.SourceStartByte, p.ByteCount, p.UnkID));
            var actual = result.Parameters.Where(p => p.InstructionIndex == output).Select(p => (p.TargetStartByte, p.SourceStartByte, p.ByteCount, p.UnkID));
            Require(expected.SequenceEqual(actual), "MMV gaol parameter binding changed");
        }
        return result;
    }

    static void MergeShared(string name, EMEVD.Event baseline, EMEVD.Event nrhex, EMEVD.Event mmv) {
        Require(Header(baseline, nrhex) && Header(baseline, mmv) && baseline.Parameters.Count == 0,
            $"Shared event header/parameters changed: {name}/{baseline.ID}");
        int added = nrhex.Instructions.Count - baseline.Instructions.Count;
        Require(added > 0, "Expected NRHex event additions");
        if (name == "common" && baseline.ID == 0) {
            Require(nrhex.Instructions.Skip(added).Zip(baseline.Instructions).All(p => Same(p.First, p.Second)),
                "Common initialization is no longer a pure NRHex prefix");
            mmv.Instructions.InsertRange(0, nrhex.Instructions.Take(added).Select(Copy));
        } else if (name == "common" && baseline.ID == 1111) {
            Require(baseline.Instructions.Count == 32 && mmv.Instructions.Count == 32 &&
                nrhex.Instructions.Take(9).Zip(baseline.Instructions).All(p => Same(p.First, p.Second)) &&
                nrhex.Instructions.Skip(9 + added).Zip(baseline.Instructions.Skip(9)).All(p => Same(p.First, p.Second)),
                "Departure cleanup no longer has the reviewed insertion point");
            Require(Same(baseline.Instructions[8], mmv.Instructions[8]) &&
                Convert.ToHexString(mmv.Instructions[8].ArgData) == "000100004C1D0000",
                "MMV player-ready wait changed");
            // In this reviewed prefix, skip instructions at 1 and 5 target 4/8;
            // neither crosses the insertion at 9. Later source and target indices
            // move together, so their relative distances must remain untouched.
            for (int i = 0; i < 9; i++) {
                var ins = mmv.Instructions[i];
                if (ins.Bank is >= 1000 and < 2000) {
                    if (i == 3) Require(ins.Bank == 1000 && ins.ID == 4 && Same(ins, baseline.Instructions[i]), "Departure end guard changed");
                    else {
                        Require((i == 1 && ins.Bank == 1003 && ins.ID == 1 || i == 5 && ins.Bank == 1000 && ins.ID == 1) &&
                            i + 1 + ins.ArgData[0] < 9, "Unreviewed branch crosses departure insertion");
                    }
                }
            }
            var cleanup = nrhex.Instructions.Skip(9).Take(added).ToArray();
            Require(cleanup.All(i => i.Bank == 2004 && i.ID is 8 or 21 && i.ArgData.Length == 8 &&
                BitConverter.ToInt32(i.ArgData, 0) == 20000),
                "Cleanup additions contain new control flow and need review");
            mmv.Instructions.InsertRange(9, cleanup.Select(Copy));
        } else if (name == "m46_70_00_00" && baseline.ID == 0) {
            var changed = new List<int>();
            for (int i = 0; i < baseline.Instructions.Count; i++) {
                var before = baseline.Instructions[i]; var desired = nrhex.Instructions[added + i];
                if (Same(before, desired)) continue;
                Require(before.Bank == 2000 && before.ID == 6 && before.ArgData.Length == 24 &&
                    BitConverter.ToInt32(before.ArgData, 4) == 90045000 && desired.Bank == 2000 && desired.ID == 6 &&
                    desired.ArgData.Length == 40 && BitConverter.ToInt32(desired.ArgData, 4) == Cages.OpenEvent,
                    "Unreviewed NRHex gaol constructor modification");
                var matches = mmv.Instructions.Select((value, at) => (value, at)).Where(p => Same(p.value, before)).ToArray();
                Require(matches.Length == 1, "MMV gaol opener missing, duplicated or modified");
                mmv.Instructions[matches[0].at] = Copy(desired); changed.Add(i);
            }
            Require(changed.SequenceEqual(new[] { 3, 12, 21, 30, 39 }), "Gaol opener set changed");
            mmv.Instructions.InsertRange(0, nrhex.Instructions.Take(added).Select(Copy));
        } else {
            throw new InvalidDataException($"No simultaneous event adapter: {name}/{baseline.ID}");
        }
    }

    public static void Stage(string root, string mmvRoot) {
        var dir = Path.Combine(root, "work/compat/mmv/staged/mod/event"); Directory.CreateDirectory(dir);
        var reports = new List<object>();
        foreach (var path in Directory.GetFiles(Path.Combine(root, "dist/mod/event"), "*.emevd.dcx")) {
            var file = Path.GetFileName(path); var name = file.Replace(".emevd.dcx", "");
            var originalPath = Path.Combine(root, "assets/original/event", file);
            Require(File.Exists(originalPath), $"Original event input missing: {file}");
            var baseline = EMEVD.Read(originalPath); var nrhex = EMEVD.Read(path);
            var mmvPath = Path.Combine(mmvRoot, "mod/event", file);
            var input = File.Exists(mmvPath) ? mmvPath : originalPath;
            var prior = EMEVD.Read(input); var result = EMEVD.Read(input);
            Require(Metadata(baseline, nrhex), $"NRHex event metadata requires an adapter: {file}");
            // Shared string tables are retained; copied NRHex instructions may
            // still contain offsets into the original table, so require equality.
            Require(Metadata(baseline, result), $"MMV event metadata requires an adapter: {file}");
            EMEVD.Event? rebasedOpener = null;
            if (name == "common_func") {
                File.WriteAllText(Path.Combine(root, "work/compat/mmv/gaol-opener.json"), JsonSerializer.Serialize(new {
                    original = Cages.Dump(baseline.Events.Single(e => e.ID == 90045000)),
                    mmv = Cages.Dump(result.Events.Single(e => e.ID == 90045000)),
                    nrhex = Cages.Dump(nrhex.Events.Single(e => e.ID == Cages.OpenEvent))
                }, new JsonSerializerOptions { WriteIndented = true }));
                rebasedOpener = CageOpener(result.Events.Single(e => e.ID == 90045000), nrhex.Events.Single(e => e.ID == Cages.OpenEvent));
            }
            var originals = baseline.Events.ToDictionary(e => e.ID);
            var wanted = nrhex.Events.ToDictionary(e => e.ID);
            Require(originals.Keys.All(wanted.ContainsKey), $"NRHex removes original events: {file}");
            var targets = result.Events.ToDictionary(e => e.ID);
            var changed = new HashSet<long>(); var simultaneous = new List<long>();
            foreach (var (id, wantedEvent) in wanted) {
                var desired = id == Cages.OpenEvent && rebasedOpener != null ? rebasedOpener : wantedEvent;
                originals.TryGetValue(id, out var original);
                if (original != null && Cages.Same(original, desired)) continue;
                if (!targets.TryGetValue(id, out var current)) {
                    Require(original == null, $"MMV deleted a changed event: {file}/{id}");
                    result.Events.Add(desired); changed.Add(id); continue;
                }
                if (Cages.Same(current, desired)) continue;
                Require(original != null, $"New event ID collision: {file}/{id}");
                if (Cages.Same(original!, current)) result.Events[result.Events.IndexOf(current)] = desired;
                else { MergeShared(name, original!, desired, current); simultaneous.Add(id); }
                changed.Add(id);
            }
            var output = Path.Combine(dir, file); result.Write(output); var check = EMEVD.Read(output);
            Require(Metadata(result, check) && result.Compression == check.Compression &&
                result.Events.Count == check.Events.Count && result.Events.Zip(check.Events).All(p => Cages.Same(p.First, p.Second)),
                $"Event write/readback mismatch: {file}");
            Require(prior.Events.Where(e => !changed.Contains(e.ID)).All(e => Cages.Same(e, check.Events.Single(x => x.ID == e.ID))),
                $"Unrelated MMV event changed: {file}");
            reports.Add(new { file, changedEvents = changed.Count, simultaneous, mmvInput = File.Exists(mmvPath),
                sha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(output))) });
        }
        File.WriteAllText(Path.Combine(root, "work/compat/mmv/staged/events-validation.json"),
            JsonSerializer.Serialize(new { files = reports, unrelatedMmvEventsPreserved = true, gameplayVerified = false }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine($"Staged {reports.Count} EMEVD files; MMV events preserved and merged output read back");
    }
}
