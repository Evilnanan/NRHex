using System.Text.RegularExpressions;

record Hex(int Id,string Zh,string En,string ZhInfo,string EnInfo,int LargeIcon,int SmallIcon);

static class HexCatalog {
 public static Dictionary<int,(string Name,string Description)> Read(string path){
  var text=File.ReadAllText(path).Replace("\r\n","\n");
  var sections=Regex.Matches(text,@"(?m)^<!-- hex:(\d+) -->\n## ([^\n]+)\n");
  void Check(bool value,string why){if(!value)throw new InvalidDataException($"{path}: {why}");}
  Check(sections.Count>0&&Regex.Matches(text,@"(?m)^## ").Count==sections.Count&&Regex.Matches(text,@"<!--\s*hex:").Count==sections.Count,"Every hex heading needs exactly one stable hex ID comment");
  var result=new Dictionary<int,(string Name,string Description)>();
  for(int i=0;i<sections.Count;i++){
   var section=sections[i];var id=int.Parse(section.Groups[1].Value);var name=section.Groups[2].Value.Trim();
   int start=section.Index+section.Length,end=i+1<sections.Count?sections[i+1].Index:text.Length;
   var description=text[start..end].Trim();
   Check(name.Length>0&&description.Length>0&&!description.Contains("<!-- hex:"),"Missing or malformed hex text: "+id);
   Check(result.TryAdd(id,(name,description)),"Duplicate hex text ID: "+id);
  }
  Check(result.Values.Select(t=>t.Name).Distinct().Count()==result.Count,"Duplicate hex names");
  return result;
 }

 public static Hex[] Load(string root){
  var zh=Read(Path.Combine(root,"docs/hexes.md"));
  var en=Read(Path.Combine(root,"docs/hexes.en.md"));
  var icons=new (int Id,int Large,int Small)[]{
   (95000100,21267,22267),
   (95000200,21208,22208),
   (95000300,21206,22206),
   (95000400,21297,22297),
   (95000500,21291,22291),
   (95000600,21291,22291),
   (95000700,21208,22208),
   (95000800,21222,22222),
   (95000900,21206,22206),
   (95001100,21206,22206),
   (95001200,21222,22222),
   (95001300,21222,22222),
   (95001400,21295,22295),
   (95001500,21229,22229),
   (95001600,21267,22267),
   (95001700,21229,22229),
   (95001800,21297,22297),
   (95002000,21207,22207),
   (95002100,21207,22207),
   (95002200,21700,20700),
   (95002300,21222,22222),
   (95002400,21295,22295),
   (95002500,21690,20690),
   (95002600,21229,22229),
   (95002700,21267,22267),
   (95002900,21292,22292),
   (95003000,21291,22291),
    (95003100,21229,22229),
   (95003200,21267,22267),
   (95003400,21293,22293),
   (95003500,21207,22207),
   (95003600,21267,22267),
   (95003700,21206,22206),
   (95003800,21297,22297),
   (95003900,21297,22297),
   (95004000,21297,22297),
   (95004200,21295,22295),
   (95004300,21229,22229),
   (95004400,21297,22297),
   (95004500,21278,22278),
   (95004600,21208,22208),
  };
  foreach(var (file,texts) in new[]{("docs/hexes.md",zh),("docs/hexes.en.md",en)}){
   var missing=icons.Select(h=>h.Id).Except(texts.Keys).ToArray();
   var extra=texts.Keys.Except(icons.Select(h=>h.Id)).ToArray();
   if(missing.Length>0||extra.Length>0)throw new InvalidDataException($"{file}: hex IDs must match implemented abilities; missing [{string.Join(", ",missing)}], extra [{string.Join(", ",extra)}]");
  }
  return icons.Select(h=>new Hex(h.Id,zh[h.Id].Name,en[h.Id].Name,zh[h.Id].Description,en[h.Id].Description,h.Large,h.Small)).ToArray();
 }
}
