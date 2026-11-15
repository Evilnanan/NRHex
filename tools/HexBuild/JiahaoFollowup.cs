using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Extend only the native hand-attack cancel windows, gated by Jiahao's existing state.
static class JiahaoFollowup {
    public const short OwnerState = 25013;
    public const float Advance = 4f / 30f;
    static readonly long[] Clips = [27000, 27010, 27020, 27030];
    static void Check(bool ok, string message) { if (!ok) throw new InvalidDataException(message); }
    static string Name(BinderFile f) => f.Name.Replace('\\', '/').Split('/').Last();
    static int Flag(TAE.Event e) => BitConverter.ToInt32(e.GetParameterBytes(false));
    public static bool IsExtra(long clip, TAE.Event e) => Clips.Contains(clip) && e.Type == 0 &&
        e.GetParameterBytes(false).Length >= 16 && Flag(e) is 4 or 16 &&
        BitConverter.ToInt16(e.GetParameterBytes(false), 14) == OwnerState;
    static TAE.Event Anchor(TAE.Animation a, int flag) {
        var candidates = a.Events.Where(e => e.Type == 0 && e.GetParameterBytes(false).Length >= 16 &&
            Flag(e) == flag && BitConverter.ToInt16(e.GetParameterBytes(false), 14) == 0).ToArray();
        Check(candidates.Length == 1, $"Ambiguous native backstep attack cancel: {a.ID}/{flag}");
        var native = candidates[0];
        Check(native.Group != null && native.Group.GroupType == 0 && native.StartTime > Advance && native.EndTime > native.StartTime,
            $"Unexpected backstep attack event: {a.ID}/{flag}");
        return native;
    }
    public static (float Start, float End) Window(TAE.Animation a, TAE.Event extra) {
        var native = Anchor(a, Flag(extra));
        var data = native.GetParameterBytes(false).ToArray();
        BitConverter.GetBytes(OwnerState).CopyTo(data, 14);
        Check(data.SequenceEqual(extra.GetParameterBytes(false)), "MMV changed backstep attack cancel arguments");
        return (native.StartTime - Advance, native.StartTime);
    }
    static TAE.Event Create(TAE.Animation a, int flag) {
        var native = Anchor(a, flag);
        var data = native.GetParameterBytes(false).ToArray();
        BitConverter.GetBytes(OwnerState).CopyTo(data, 14);
        return new TAE.Event(native.StartTime - Advance, native.StartTime, 0, native.Unk04, data, false) {
            Group = new TAE.EventGroup(native.Group.GroupType) { GroupData = native.Group.GroupData }
        };
    }
    public static void Stage(string root, bool mmv) {
        string relative = "mod/chr/c0000.anibnd.dcx";
        string input = Path.Combine(root, mmv ? "dist/mmv" : "dist", relative);
        string folder = Path.Combine(root, "work/jiahao-followup", mmv ? "mmv" : "standalone");
        Directory.CreateDirectory(folder);
        var bundle = BND4.Read(input);
        var source = bundle.Files.ToDictionary(Name, f => f.Bytes.ToArray());
        var baseline = BND4.Read(Path.Combine(root, "assets/original/chr/c0000.anibnd.dcx"));
        var report = new List<object>(); int added = 0;
        foreach (var f in baseline.Files.Where(f => Name(f).EndsWith(".tae"))) {
            var ids = TAE.Read(f.Bytes).Animations.Where(a => Clips.Contains(a.ID)).Select(a => a.ID).ToArray();
            if (ids.Length == 0) continue;
            var target = bundle.Files.Single(x => Name(x) == Name(f));
            var tae = TAE.Read(target.Bytes); var extras = new Dictionary<int, List<TAE.Event>>();
            foreach (var id in ids) {
                var matches = tae.Animations.Select((a, i) => (a, i)).Where(p => p.a.ID == id && p.a.Events.Count > 0).ToArray();
                Check(matches.Length == 1, $"Ambiguous backstep clip: {Name(f)}/{id}");
                var (a, i) = matches[0];
                foreach (int flag in new[] { 4, 16 }) {
                    var e = Create(a, flag);
                    var old = a.Events.Where(x => IsExtra(id, x) && Flag(x) == flag).ToArray();
                    Check(old.Length <= 1 && old.All(x => x.StartTime == e.StartTime && x.EndTime == e.EndTime &&
                        x.GetParameterBytes(false).SequenceEqual(e.GetParameterBytes(false))), "Existing early backstep event differs");
                    if (old.Length == 0) {
                        if (!extras.ContainsKey(i)) extras[i] = [];
                        extras[i].Add(e); added++;
                    }
                    report.Add(new { file = Name(f), clip = id, flag, start = e.StartTime, nativeStart = e.EndTime });
                }
            }
            if (extras.Count > 0) target.Bytes = TaeOverlay.Add(source[Name(f)], extras);
        }
        Check(report.Count == 72, "Expected both hand cancel windows on 36 backstep clips");
        string output = Path.Combine(folder, relative); Directory.CreateDirectory(Path.GetDirectoryName(output)!);
        if (added == 0) File.Copy(input, output, true); else bundle.Write(output);
        var readback = BND4.Read(output);
        Check(readback.Files.Count == bundle.Files.Count, "Binder count changed");
        foreach (var (a, b) in bundle.Files.Zip(readback.Files))
            Check(a.ID == b.ID && a.Name == b.Name && a.Flags == b.Flags && a.Bytes.Span.SequenceEqual(b.Bytes.Span), "Binder readback mismatch");
        string Hash(string path) => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
        File.WriteAllText(Path.Combine(folder, "validation.json"), JsonSerializer.Serialize(new {
            input, output, sourceSha256 = Hash(input), sha256 = Hash(output), ownerState = OwnerState,
            advanceSeconds = Advance, added, originalEventsAndAnimationBytesPreserved = true,
            gameplayVerified = false, windows = report
        }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine($"PASS: 72 owner-gated backstep attack windows validated; {added} added. {output}");
    }
}
