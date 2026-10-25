using SoulsFormats;
using System.Text.Json;

static class MmvTae {
    static void Check(bool ok, string why) { if (!ok) throw new InvalidDataException(why); }
    static string Name(BinderFile f) => f.Name.Replace('\\', '/').Split('/').Last();
    static bool Equal(TAE.Event a, TAE.Event b, bool alignment = false) {
        if (a.Type != b.Type || a.Unk04 != b.Unk04 || a.StartTime != b.StartTime || a.EndTime != b.EndTime || a.Group?.GroupType != b.Group?.GroupType) return false;
        var x = a.GetParameterBytes(false); var y = b.GetParameterBytes(false);
        return x.SequenceEqual(y) || alignment && y.Length == x.Length + 8 && y.Take(x.Length).SequenceEqual(x) && y.Skip(x.Length).All(v => v == 0);
    }
    static object Event(TAE.Event e) => new { e.Type, e.Unk04, e.StartTime, e.EndTime, group = e.Group?.GroupType, data = Convert.ToHexString(e.GetParameterBytes(false)) };
    static bool Guard(TAE.Event e) => e.Type == 0 && e.GetParameterBytes(false).Length >= 12 && BitConverter.ToInt32(e.GetParameterBytes(false), 8) == 95002400;
    public static void Stage(string root, string mmv, Func<string, Memory<byte>, PARAM> parse) {
        const string relative = "chr/c0000.anibnd.dcx";
        var baseline = BND4.Read(Path.Combine(root, "assets/original", relative)).Files.ToDictionary(Name);
        var shared = BND4.Read(Path.Combine(root, "dist/mod", relative)).Files.ToDictionary(Name);
        var merged = BND4.Read(Path.Combine(mmv, "mod", relative));
        var before = merged.Files.ToDictionary(Name, f => f.Bytes.ToArray());
        var regulation = SFUtil.DecryptNightreignRegulation(Path.Combine(root, "work/compat/mmv/staged/mod/regulation.bin"));
        PARAM Table(string name) => parse(name, regulation.Files.Single(f => Name(f) == name + ".param").Bytes);
        PARAM.Row[] Unique(PARAM table, params string[] fields) => table.Rows.GroupBy(r => r.ID).Select(g => {
            Check(fields.All(field => g.Select(r => Convert.ToString(r[field].Value, System.Globalization.CultureInfo.InvariantCulture)).Distinct().Count() == 1), $"Ambiguous MMV parry parameter: {table.ParamType}/{g.Key}");
            return g.First();
        }).ToArray();
        var attackGroups = Table("AtkParam_Pc").Rows.GroupBy(r => r.ID).ToDictionary(g => g.Key, g => g.ToArray());
        var behaviors = Unique(Table("BehaviorParam_PC"), "refType", "refId", "behaviorJudgeId").Where(r => Convert.ToInt32(r["refType"].Value) == 0 && attackGroups.ContainsKey(Convert.ToInt32(r["refId"].Value))).OrderBy(r => r.ID).ToArray();
        var usedAttacks = behaviors.Select(r => Convert.ToInt32(r["refId"].Value)).ToHashSet();
        var attacks = attackGroups.Where(p => usedAttacks.Contains(p.Key)).ToDictionary(p => p.Key, p => {
            Check(p.Value.Select(r => Convert.ToSingle(r["atkSuperArmorCorrection"].Value)).Distinct().Count() == 1, $"Ambiguous MMV guarded attack: {p.Key}"); return p.Value[0];
        });
        var judges = behaviors.Select(r => Convert.ToInt32(r["behaviorJudgeId"].Value)).ToHashSet();
        Check(!Table("SpEffectParam").Rows.Any(r => Convert.ToInt32(r["stateInfo"].Value) is >= 14000 and < 24000), "MMV uses the private parry state range");
        bool NativeGuard(TAE.Event e) => e.Type == 0 && BitConverter.ToInt32(e.GetParameterBytes(false)) == 3;
        bool Weapon(TAE.Event e) {
            if (e.Type != 1 || e.EndTime <= e.StartTime || !float.IsFinite(e.StartTime) || !float.IsFinite(e.EndTime)) return false;
            var d = e.GetParameterBytes(false);
            return d.Length >= 16 && BitConverter.ToInt32(d) is >= 0 and <= 3 && d[13] <= 2 &&
                (judges.Contains(BitConverter.ToInt32(d, 8)) || judges.Contains(BitConverter.ToInt32(d, 8) % 1000));
        }
        (int Judge, byte Source, short Required, byte Extra) Key(TAE.Event e) {
            var d = e.GetParameterBytes(false); return (BitConverter.ToInt32(d, 8), d[13], BitConverter.ToInt16(d, 14), (byte)(d.Length > 16 && d[16] == 1 ? 1 : 0));
        }
        var parsed = merged.Files.Where(f => Name(f).EndsWith(".tae")).ToDictionary(Name, f => TAE.Read(f.Bytes));
        var keys = parsed.Values.SelectMany(t => t.Animations).Where(a => !a.Events.Any(NativeGuard)).SelectMany(a => a.Events.Where(Weapon)).Select(Key).Distinct().Order().ToArray();
        Check(keys.All(k => k.Required < 14000 || k.Required >= 24000) && 14000 + keys.Length * 7 <= 24000, "MMV parry condition range overflow/collision");
        var indices = keys.Select((k, i) => (k, i)).ToDictionary(p => p.k, p => p.i);
        var template = shared.Values.Where(f => Name(f).EndsWith(".tae")).SelectMany(f => TAE.Read(f.Bytes).Animations).SelectMany(a => a.Events).First(Guard);
        var reports = new List<object>(); int guardCount = 0, otherCount = 0;
        foreach (var f in merged.Files.Where(f => parsed.ContainsKey(Name(f)))) {
            string name = Name(f); var current = parsed[name]; var additions = new Dictionary<int, List<TAE.Event>>();
            List<TAE.Event> Extras(int i) { if (!additions.TryGetValue(i, out var list)) additions[i] = list = []; return list; }
            if (shared.TryGetValue(name, out var nf) && baseline.TryGetValue(name, out var bf) && !nf.Bytes.Span.SequenceEqual(bf.Bytes.Span)) {
                var originals = TAE.Read(bf.Bytes).Animations.ToDictionary(a => a.ID);
                foreach (var a in TAE.Read(nf.Bytes).Animations) {
                    Check(originals.TryGetValue(a.ID, out var old), $"New NRHex animation needs adapter: {name}/{a.ID}");
                    var extras = a.Events.ToList();
                    for (int j = 0; j < old!.Events.Count; j++) {
                        int found = extras.FindIndex(e => Equal(old.Events[j], e, j == 0));
                        Check(found >= 0, $"NRHex changed a native event: {name}/{a.ID}/{j}"); extras.RemoveAt(found);
                    }
                    extras.RemoveAll(Guard); if (extras.Count == 0) continue;
                    var matches = current.Animations.Select((v, i) => (v, i)).Where(p => p.v.ID == a.ID && p.v.Events.Count > 0).ToArray();
                    Check(matches.Length == 1, $"Ambiguous nonempty MMV animation: {name}/{a.ID}"); var target = matches[0];
                    foreach (var e in extras) {
                        float start = e.StartTime, end = e.EndTime;
                        if (e.Type == 302 && start > 0) {
                            // Ultimate markers follow native expenditure/release events.
                            // MMV can add alternate conditioned bullet IDs; match the
                            // original source/judge, or the original gauge event payload.
                            var anchors = old.Events.Where(x => x.StartTime == start && x.Type is 2 or 1000 or 1002).ToArray();
                            var onsets = anchors.SelectMany(anchor => target.v.Events.Where(x => x.Type == anchor.Type &&
                                (x.Type == 2 ? x.GetParameterBytes(false).Take(12).SequenceEqual(anchor.GetParameterBytes(false).Take(12)) :
                                x.GetParameterBytes(false).SequenceEqual(anchor.GetParameterBytes(false))))).Select(x => x.StartTime).Distinct().ToArray();
                            Check(onsets.Length == 1, $"Ambiguous MMV ultimate release: {name}/{a.ID}");
                            start = onsets[0]; end = start + (e.EndTime - e.StartTime);
                        } else {
                            Check(start == 0, $"Unreviewed nonzero extra animation event: {name}/{a.ID}");
                            // Retain fixed startup feedback/invulnerability intervals.
                            // If original matching events move, require a new review.
                            foreach (var anchor in old.Events) {
                                var peers = target.v.Events.Where(x => x.Type == anchor.Type && x.GetParameterBytes(false).SequenceEqual(anchor.GetParameterBytes(false))).ToArray();
                                Check(peers.Length == 0 || peers.Any(x => x.StartTime == anchor.StartTime && x.EndTime == anchor.EndTime), $"MMV changed startup timing: {name}/{a.ID}");
                            }
                        }
                        Extras(target.i).Add(new TAE.Event(start, end, e.Type, e.Unk04, e.GetParameterBytes(false), false) { Group = e.Group }); otherCount++;
                    }
                }
            }
            for (int i = 0; i < current.Animations.Count; i++) {
                var a = current.Animations[i]; if (a.Events.Any(NativeGuard)) continue;
                foreach (var e in a.Events.Where(Weapon)) for (int tier = 0; tier < 7; tier++) {
                    float half = Parry.GuardWindowMilliseconds[tier] / 2000f;
                    var d = template.GetParameterBytes(false).ToArray(); BitConverter.GetBytes((short)(14000 + indices[Key(e)] * 7 + tier)).CopyTo(d, 14);
                    Extras(i).Add(new TAE.Event(Math.Max(0, e.StartTime - half), e.StartTime + half, template.Type, template.Unk04, d, false) { Group = template.Group }); guardCount++;
                }
            }
            if (additions.Count == 0) continue;
            f.Bytes = TaeOverlay.Add(before[name], additions);
            reports.Add(new { file = name, animations = additions.Count, events = additions.Values.Sum(v => v.Count) });
        }
        // Every non-TAE member must be unchanged by NRHex; retain all MMV members.
        foreach (var (name, f) in shared.Where(p => !p.Key.EndsWith(".tae")))
            Check(baseline.TryGetValue(name, out var old) && f.Bytes.Span.SequenceEqual(old.Bytes.Span), $"New non-TAE animation resource: {name}");
        string output = Path.Combine(root, "work/compat/mmv/staged/mod", relative); Directory.CreateDirectory(Path.GetDirectoryName(output)!); merged.Write(output);
        var readback = BND4.Read(output); Check(readback.Files.Count == merged.Files.Count, "Animation binder count changed");
        foreach (var p in readback.Files.Zip(merged.Files)) Check(p.First.Name == p.Second.Name && p.First.ID == p.Second.ID && p.First.Flags == p.Second.Flags && p.First.Bytes.Span.SequenceEqual(p.Second.Bytes.Span), "Animation binder readback mismatch");
        string Float(object v) { float f = Convert.ToSingle(v); Check(float.IsFinite(f), "Nonfinite MMV parry value"); return f.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + "f32"; }
        var weapons = Unique(Table("EquipParamWeapon"), "saWeaponDamage").Where(r => Convert.ToSingle(r["saWeaponDamage"].Value) > 0).OrderBy(r => r.ID).ToArray();
        string data = "// Generated for the pinned MMV inputs; shared runtime logic.\npub const FIRST:u16=14000;\npub const MIN_TIER:usize=2;\npub const TIERS:usize=7;\npub const CONDITIONS:&[(i32,u8,u16,u8)]=&[\n" +
            string.Join("\n", keys.Select(k => $" ({k.Judge},{k.Source},{unchecked((ushort)k.Required)},{k.Extra}),")) + "\n];\npub const WEAPONS:&[(i32,f32)]=&[\n" +
            string.Join("\n", weapons.Select(r => $" ({r.ID},{Float(r["saWeaponDamage"].Value)}),")) + "\n];\npub const BEHAVIORS:&[(i32,f32)]=&[\n" +
            string.Join("\n", behaviors.Select(r => $" ({r.ID},{Float(attacks[Convert.ToInt32(r["refId"].Value)]["atkSuperArmorCorrection"].Value)}),")) + "\n];\n";
        File.WriteAllText(Path.Combine(root, "work/compat/mmv/staged/parry_data.rs"), data);
        File.WriteAllText(Path.Combine(root, "work/compat/mmv/staged/tae-validation.json"), JsonSerializer.Serialize(new { file = relative,
            sha256 = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(output))), guardCount, otherCount,
            conditionCount = keys.Length, weapons = weapons.Length, behaviors = behaviors.Length, reports, originalMmvPayloadsPreserved = true, gameplayVerified = false }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine($"Staged MMV animation overlay: {guardCount} guards, {otherCount} other events, {keys.Length} conditions");
    }
    public static void Audit(string root, string mmv) {
        const string relative = "chr/c0000.anibnd.dcx";
        var b = BND4.Read(Path.Combine(root, "assets/original", relative)).Files.ToDictionary(Name);
        var n = BND4.Read(Path.Combine(root, "dist/mod", relative));
        var m = BND4.Read(Path.Combine(mmv, "mod", relative)).Files.ToDictionary(Name);
        var additions = new List<object>(); var missing = new List<object>(); var duplicates = new List<object>();
        int guards = 0;
        foreach (var f in n.Files.Where(f => Name(f).EndsWith(".tae") && b.ContainsKey(Name(f)) && !f.Bytes.Span.SequenceEqual(b[Name(f)].Bytes.Span))) {
            var name = Name(f); var baseline = TAE.Read(b[name].Bytes).Animations.ToDictionary(a => a.ID);
            var target = TAE.Read(m[name].Bytes).Animations.GroupBy(a => a.ID).ToDictionary(g => g.Key, g => g.ToArray());
            foreach (var a in TAE.Read(f.Bytes).Animations) {
                if (!baseline.TryGetValue(a.ID, out var old)) { additions.Add(new { file = name, animation = a.ID, newAnimation = true }); continue; }
                var remaining = a.Events.ToList();
                for (int i = 0; i < old.Events.Count; i++) {
                    var e = old.Events[i]; int at = remaining.FindIndex(c => Equal(e, c, i == 0));
                    if (at >= 0) remaining.RemoveAt(at);
                    else missing.Add(new { file = name, animation = a.ID, index = i, before = Event(e), candidates = a.Events.Where(c => c.Type == e.Type).Select(Event) });
                }
                if (remaining.Count == 0) continue;
                guards += remaining.Count(Guard);
                if (!target.ContainsKey(a.ID) || target[a.ID].Length != 1)
                    duplicates.Add(new { file = name, animation = a.ID, count = target.GetValueOrDefault(a.ID)?.Length ?? 0,
                        original = old.Events.Select(Event), mmv = target.GetValueOrDefault(a.ID)?.Select(v => v.Events.Select(Event)) });
                var extra = remaining.Where(e => !Guard(e)).ToArray();
                if (extra.Length > 0) additions.Add(new { file = name, animation = a.ID, events = extra.Select(Event),
                    original = old.Events.Select(Event), mmv = target.GetValueOrDefault(a.ID)?.Select(v => v.Events.Select(Event)) });
            }
        }
        string output = Path.Combine(root, "work/compat/mmv/tae-audit.json");
        File.WriteAllText(output, JsonSerializer.Serialize(new { guards, additions, missing, duplicates }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine($"TAE audit: {guards} guard events, {additions.Count} non-guard changed animations, {missing.Count} unmatched native events, {duplicates.Count} ambiguous targets");
    }
}
