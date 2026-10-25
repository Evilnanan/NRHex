using SoulsFormats;
using System.Security.Cryptography;
using System.Text.Json;

// Read-only catalog for reviewing compatibility spell mappings and balance inputs.
static class MmvSpells {
    public static void ExportRanges(string root, Func<string, Memory<byte>, PARAM> parse, string? target = null) {
        var input = Path.Combine(target ?? Path.Combine(root, "dist"), "mod/regulation.bin");
        var before = SHA256.HashData(File.ReadAllBytes(input));
        var binder = SFUtil.DecryptNightreignRegulation(input);
        string Name(BinderFile f) => f.Name.Replace('\\', '/').Split('/').Last();
        var tables = new[] { "Magic", "Bullet", "AtkParam_Pc" }.ToDictionary(n => n,
            n => parse(n, binder.Files.Single(f => Name(f) == n + ".param").Bytes).Rows
                .Select(r => new { r.ID, fields = r.Cells.Where(c => c.Value is not byte[])
                    .ToDictionary(c => c.Def.InternalName, c => c.Value) }).ToArray());
        if (!before.SequenceEqual(SHA256.HashData(File.ReadAllBytes(input))))
            throw new InvalidDataException("Range input changed during export");
        var textPath = Path.Combine(target ?? Path.Combine(root, "dist"), "mod/msg/zhocn/item_dlc01.msgbnd.dcx");
        var names = BND4.Read(textPath).Files.Where(f => Name(f).StartsWith("MagicName"))
            .SelectMany(f => FMG.Read(f.Bytes).Entries).Where(e => !string.IsNullOrWhiteSpace(e.Text))
            .GroupBy(e => e.ID).ToDictionary(g => g.Key, g => g.Select(e => e.Text).Distinct().Single());
        var output = Path.Combine(root, target == null ? "work/sword-range-export/parameters.json" : "work/compat/mmv/sword-parameters.json");
        Directory.CreateDirectory(Path.GetDirectoryName(output)!);
        File.WriteAllText(output, JsonSerializer.Serialize(new { regulationSha256 = Convert.ToHexString(before), names, tables }));
        Console.WriteLine("Read-only installed dist range parameters exported: " + output);
    }
    public static void Export(string root, string mmv, BND4 original, Func<string, Memory<byte>, PARAM> parse) {
        string Name(BinderFile f) => f.Name.Replace('\\', '/').Split('/').Last();
        var regulation = Path.Combine(mmv, "mod/regulation.bin");
        var texts = Path.Combine(mmv, "mod/msg/zhocn/item_dlc01.msgbnd.dcx");
        string Hash(string p) => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(p)));
        var before = new[] { Hash(regulation), Hash(texts) };
        var binder = SFUtil.DecryptNightreignRegulation(regulation);
        object Rows(BND4 b, string table) => parse(table, b.Files.Single(f => Name(f) == table + ".param").Bytes)
            .Rows.Select(r => new { r.ID, r.Name, fields = r.Cells.Where(c => c.Value is not byte[])
                .ToDictionary(c => c.Def.InternalName, c => c.Value) }).ToArray();
        var fmg = BND4.Read(texts).Files.Where(f => Name(f).StartsWith("MagicName") || Name(f).StartsWith("GoodsName"))
            .ToDictionary(Name, f => FMG.Read(f.Bytes).Entries.Select(e => new { e.ID, e.Text }).ToArray());
        var report = new { regulationSha256 = before[0], textSha256 = before[1],
            magic = Rows(binder, "Magic"), originalMagic = Rows(original, "Magic"),
            magicTables = Rows(binder, "MagicTableParam"), names = fmg };
        if (!before.SequenceEqual(new[] { Hash(regulation), Hash(texts) }))
            throw new InvalidDataException("MMV inputs changed during export");
        var output = Path.Combine(root, "work/compat/mmv/spells.json");
        Directory.CreateDirectory(Path.GetDirectoryName(output)!);
        File.WriteAllText(output, JsonSerializer.Serialize(report, new JsonSerializerOptions {
            WriteIndented = true, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping }));
        Console.WriteLine("Read-only MMV spell catalog exported: " + output);
    }
}
