using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

static class MmvBinders {
    static string Name(BinderFile f) => f.Name.Replace('\\', '/').Split('/').Last();
    static void Check(bool ok, string why) { if (!ok) throw new InvalidDataException(why); }
    static string Hash(byte[] data) => Convert.ToHexString(SHA256.HashData(data));
    // Explicitly keep MMV's shared Storm Ruler appearance. Sword no longer applies
    // its retired visual effects; these resources are still used by the weapon art.
    static readonly HashSet<string> KeepMmv = ["f000450940.fxr", "f000450950.fxr", "s88350.flver",
        "f000450940.ffxreslist", "f000450950.ffxreslist"];
    public static void Stage(string root, string mmv, string originalSfx) {
        var reports = new List<object>();
        foreach (string relative in new[] { "sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx", "script/talk/m00_00_00_00.talkesdbnd.dcx" }) {
            bool sfx = relative.StartsWith("sfx/");
            string baselinePath = sfx ? originalSfx : Path.Combine(root, "assets/original", relative);
            var b = BND4.Read(baselinePath).Files.ToDictionary(Name);
            var n = BND4.Read(Path.Combine(root, "dist/mod", relative)).Files.ToDictionary(Name);
            string mmvPath = Path.Combine(mmv, "mod", relative);
            var target = BND4.Read(File.Exists(mmvPath) ? mmvPath : baselinePath);
            var m = target.Files.ToDictionary(Name);
            var retained = target.Files.ToDictionary(Name, f => (f.ID, f.Name, f.Flags, Bytes: f.Bytes.ToArray()));
            var changed = new HashSet<string>(); var preserved = new HashSet<string>(); var added = new List<string>();
            Check(b.Keys.All(n.ContainsKey), $"Deleted binder member needs an adapter: {relative}");
            foreach (var (name, wanted) in n) {
                b.TryGetValue(name, out var old);
                if (old != null && old.Bytes.Span.SequenceEqual(wanted.Bytes.Span)) continue;
                if (m.TryGetValue(name, out var current)) {
                    if (current.Bytes.Span.SequenceEqual(wanted.Bytes.Span)) continue;
                    if (sfx && KeepMmv.Contains(name)) { preserved.Add(name); continue; }
                    Check(old != null && current.Bytes.Span.SequenceEqual(old.Bytes.Span), $"Unreviewed binder collision: {relative}/{name}");
                    current.Bytes = wanted.Bytes.ToArray(); changed.Add(name);
                } else {
                    Check(old == null, $"MMV removed a modified member: {relative}/{name}");
                    int id = wanted.ID;
                    if (target.Files.Any(f => f.ID == id)) {
                        int group = sfx ? wanted.ID / 100000 * 100000 : 0;
                        id = target.Files.Where(f => !sfx || f.ID >= group && f.ID < group + 100000).Max(f => f.ID) + 1;
                        Check(!sfx || id < group + 100000, "SFX binder ID group exhausted");
                    }
                    var copy = new BinderFile(wanted.Flags, id, wanted.Name, wanted.Bytes.ToArray());
                    target.Files.Add(copy); m.Add(name, copy); added.Add(name);
                }
            }
            if (sfx) Check(preserved.SetEquals(KeepMmv), "Shared Storm conflicts changed; review appearance policy");
            if (sfx) {
                // The selected DLC baseline is supplied together with its native
                // common binder. Pin that companion before using its dependencies.
                string common = Path.Combine(Path.GetDirectoryName(originalSfx)!, "sfxbnd_commoneffects.ffxbnd.dcx");
                Check(File.Exists(common) && Hash(File.ReadAllBytes(common)) == "F2306FC8657E6785B05DDD57251C3B5BAF985CCBE553B6256076FBF4FCE5ABE9", "Original shared SFX companion missing or changed");
                string mmvCommon = Path.Combine(mmv, "mod/sfx/sfxbnd_commoneffects.ffxbnd.dcx");
                var available = BND4.Read(File.Exists(mmvCommon) ? mmvCommon : common).Files.Select(Name).Concat(m.Keys).ToHashSet(StringComparer.OrdinalIgnoreCase);
                foreach (string name in added.Where(n => n.EndsWith(".ffxreslist"))) {
                    foreach (string line in System.Text.Encoding.UTF8.GetString(m[name].Bytes.Span).Split(['\r', '\n'], StringSplitOptions.RemoveEmptyEntries)) {
                        string resource = line.Trim(); string ext = Path.GetExtension(resource).ToLowerInvariant();
                        Check(ext is ".tif" or ".sib", $"Unknown SFX dependency type: {resource}");
                        string member = Path.GetFileNameWithoutExtension(resource) + (ext == ".tif" ? ".tpf" : ".flver");
                        Check(available.Contains(member), $"Missing added SFX dependency: {name}/{member}");
                    }
                }
            }
            Check(target.Files.Select(f => f.ID).Distinct().Count() == target.Files.Count, "Duplicate merged binder ID");
            string output = Path.Combine(root, "work/compat/mmv/staged/mod", relative);
            Directory.CreateDirectory(Path.GetDirectoryName(output)!); target.Write(output);
            var actual = BND4.Read(output);
            Check(actual.Version == target.Version && actual.Format == target.Format && actual.Compression == target.Compression &&
                actual.BigEndian == target.BigEndian && actual.Unicode == target.Unicode && actual.Files.Count == target.Files.Count,
                "Merged binder metadata readback mismatch");
            foreach (var pair in target.Files.Zip(actual.Files)) {
                Check(pair.First.ID == pair.Second.ID && pair.First.Name == pair.Second.Name && pair.First.Flags == pair.Second.Flags &&
                    pair.First.Bytes.Span.SequenceEqual(pair.Second.Bytes.Span), "Merged binder member readback mismatch");
            }
            var readback = actual.Files.ToDictionary(Name);
            foreach (var (name, old) in retained.Where(p => !changed.Contains(p.Key))) {
                var f = readback[name];
                Check(f.ID == old.ID && f.Name == old.Name && f.Flags == old.Flags && f.Bytes.Span.SequenceEqual(old.Bytes),
                    $"Unrelated MMV binder member changed: {relative}/{name}");
            }
            reports.Add(new { file = relative, sha256 = Hash(File.ReadAllBytes(output)), added, changed, preservedMmvOverrides = preserved,
                unchangedMmvMembers = retained.Count - changed.Count, readbackVerified = true });
        }
        File.WriteAllText(Path.Combine(root, "work/compat/mmv/staged/binders-validation.json"),
            JsonSerializer.Serialize(new { files = reports, gameplayVerified = false }, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine("Staged SFX/talk binders; retained MMV shared Storm appearance and verified every member");
    }
}
