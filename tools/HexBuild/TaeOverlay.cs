using SoulsFormats;
using System.Text.Json;

// Append event tables without reserializing MMV's animation headers or payloads.
// Keys are animation indices, deliberately not IDs: some MMV IDs occur twice.
static class TaeOverlay {
    static void Check(bool ok, string why) { if (!ok) throw new InvalidDataException(why); }
    static string Group(TAE.EventGroup? g) => JsonSerializer.Serialize(g, new JsonSerializerOptions { IncludeFields = true });
    public static byte[] Add(byte[] source, IReadOnlyDictionary<int, List<TAE.Event>> additions) {
        Check(source[4] == 0 && source[7] == 0xff && BitConverter.ToInt32(source, 8) == 0x1000d, "Unsupported TAE raw layout");
        var original = TAE.Read(source); using var stream = new MemoryStream(); stream.Write(source);
        using var w = new BinaryWriter(stream); var allowed = new bool[source.Length];
        long L(int at) => BitConverter.ToInt64(source, at);
        int I(int at) => BitConverter.ToInt32(source, at);
        void Put(long at, long value) { stream.Position = at; w.Write(value); if (at < source.Length) Array.Fill(allowed, true, (int)at, 8); }
        void Int(long at, int value) { stream.Position = at; w.Write(value); if (at < source.Length) Array.Fill(allowed, true, (int)at, 4); }
        long Append(byte[] data) { stream.Position = stream.Length; while (stream.Position % 16 != 0) w.Write((byte)0); long at = stream.Position; w.Write(data); return at; }
        int headers = checked((int)L(0x58));
        foreach (var (index, added) in additions.Where(p => p.Value.Count > 0)) {
            var a = original.Animations[index];
            int body = checked((int)L(headers + index * 16 + 8)), events = checked((int)L(body)), groups = checked((int)L(body + 8));
            Check(L(headers + index * 16) == a.ID && I(body + 32) == a.Events.Count && I(body + 36) == a.EventGroups.Count, "TAE raw topology mismatch");
            Check(a.Events.Count > 0, "Adding to empty MMV animation needs a separate adapter");
            var all = added.Concat(a.Events).ToArray();
            var newGroups = added.Select(e => e.Group ?? throw new InvalidDataException("Added event lacks group")).Distinct().ToArray();
            var times = all.SelectMany(e => new[] { e.StartTime, e.EndTime }).Distinct().Order().ToArray();
            long timeTable = Append(times.SelectMany(BitConverter.GetBytes).ToArray());
            long table = Append(new byte[all.Length * 24]);
            // Keep the first old payload's original alignment and every old byte.
            int prefixBytes = added.Sum(e => 16 + e.GetParameterBytes(false).Length);
            int alignment = (int)((L(events + 16) - prefixBytes % 16 + 16) % 16);
            Append([]); stream.Position = stream.Length; w.Write(new byte[alignment]);
            for (int j = 0; j < all.Length; j++) {
                var e = all[j]; stream.Position = stream.Length; long data = stream.Position;
                w.Write(e.Type); w.Write(e.Unk04); w.Write(data + 16); w.Write(e.GetParameterBytes(false));
                Put(table + j * 24, timeTable + Array.IndexOf(times, e.StartTime) * 4);
                Put(table + j * 24 + 8, timeTable + Array.IndexOf(times, e.EndTime) * 4);
                Put(table + j * 24 + 16, data);
            }
            long groupTable = Append(new byte[(a.EventGroups.Count + newGroups.Length) * 32]);
            for (int j = 0; j < a.EventGroups.Count; j++) {
                int at = groups + j * 32, count = checked((int)L(at));
                stream.Position = groupTable + j * 32; w.Write(source, at, 32);
                if (count == 0) continue;
                int entries = checked((int)L(at + 8)); var values = new List<int>();
                for (int k = 0; k < count; k++) {
                    int old = I(entries + k * 4);
                    Check(old >= events && old < events + a.Events.Count * 24 && (old - events) % 24 == 0, "Invalid old event group index");
                    values.Add(checked((int)(table + added.Count * 24 + old - events)));
                }
                Put(groupTable + j * 32 + 8, Append(values.SelectMany(BitConverter.GetBytes).ToArray()));
            }
            for (int j = 0; j < newGroups.Length; j++) {
                var g = newGroups[j]; bool plain = original.Format is TAE.TAEFormat.DS3 or TAE.TAEFormat.SDT;
                Check(plain || (long)g.GroupData.DataType == g.GroupType, "Unknown added event group data");
                bool entity = !plain && g.GroupType == 128;
                long type = Append(new byte[entity ? 32 : 16]); Put(type, g.GroupType);
                if (entity) {
                    Put(type + 8, type + 16); stream.Position = type + 16;
                    w.Write((ushort)g.GroupData.CutsceneEntityType); w.Write(g.GroupData.CutsceneEntityIDPart1);
                    w.Write(g.GroupData.CutsceneEntityIDPart2); w.Write(g.GroupData.Block); w.Write(g.GroupData.Area);
                }
                var indices = added.Select((e, k) => (e, k)).Where(p => ReferenceEquals(p.e.Group, g)).Select(p => checked((int)(table + p.k * 24))).ToArray();
                long values = Append(indices.SelectMany(BitConverter.GetBytes).ToArray());
                long header = groupTable + (a.EventGroups.Count + j) * 32;
                Put(header, indices.Length); Put(header + 8, values); Put(header + 16, type);
            }
            Put(body, table); Put(body + 8, groupTable); Put(body + 16, timeTable);
            Int(body + 32, all.Length); Int(body + 36, a.EventGroups.Count + newGroups.Length); Int(body + 40, times.Length);
        }
        Append([]); Int(12, checked((int)stream.Length)); var result = stream.ToArray();
        for (int i = 0; i < source.Length; i++) Check(allowed[i] || source[i] == result[i], "TAE original byte outside permitted header fields changed");
        var parsed = TAE.Read(result); Check(parsed.Animations.Count == original.Animations.Count, "TAE animation count changed");
        for (int i = 0; i < original.Animations.Count; i++) {
            var a = original.Animations[i]; var actual = parsed.Animations[i];
            var extra = additions.GetValueOrDefault(i) ?? [];
            var expected = extra.Concat(a.Events).ToArray();
            Check(a.ID == actual.ID && a.AnimFileName == actual.AnimFileName && expected.Length == actual.Events.Count, "TAE animation identity changed");
            foreach (var p in expected.Zip(actual.Events)) Check(p.First.Type == p.Second.Type && p.First.Unk04 == p.Second.Unk04 &&
                p.First.StartTime == p.Second.StartTime && p.First.EndTime == p.Second.EndTime && Group(p.First.Group) == Group(p.Second.Group) &&
                p.First.GetParameterBytes(false).SequenceEqual(p.Second.GetParameterBytes(false)), $"TAE event readback mismatch in {a.ID}, type {p.First.Type}");
            Check(a.EventGroups.Select(Group).SequenceEqual(actual.EventGroups.Take(a.EventGroups.Count).Select(Group)), "Original TAE groups changed");
        }
        return result;
    }
}
