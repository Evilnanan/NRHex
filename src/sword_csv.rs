use std::collections::{BTreeMap, BTreeSet};

pub fn read(text: &str, expected_ids: impl IntoIterator<Item = i32>) -> Result<BTreeMap<i32, f32>, String> {
    let expected: BTreeSet<_> = expected_ids.into_iter().collect();
    let mut rows = text.trim_start_matches('\u{feff}').lines();
    let header = csv_row(rows.next().ok_or("Empty spell bonus table")?)?;
    let id_col = header.iter().position(|h| h == "MagicID").ok_or("Missing column MagicID")?;
    let value_cols: Vec<_> = ["加成", "吸血百分比", "效果百分比", "增伤百分比"]
        .iter().filter_map(|name| header.iter().position(|h| h == name)).collect();
    if value_cols.len() != 1 { return Err("Expected one shared percentage column".into()); }
    let mut percentages = BTreeMap::new();
    for line in rows.filter(|l| !l.trim().is_empty()) {
        let row = csv_row(line)?;
        if row.len() != header.len() { return Err("CSV column count mismatch".into()); }
        let id: i32 = row[id_col].trim().parse().map_err(|_| "Invalid MagicID")?;
        if !expected.contains(&id) || percentages.contains_key(&id) {
            return Err(format!("Unknown or duplicate MagicID {id}"));
        }
        let raw = row[value_cols[0]].trim();
        let n: f32 = if raw.is_empty() { 0. } else {
            raw.parse().map_err(|_| format!("Invalid bonus for {id}"))?
        };
        if !n.is_finite() || !(0.0..=100.).contains(&n) {
            return Err(format!("Bonus must be finite and 0..100 for {id}"));
        }
        percentages.insert(id, n);
    }
    if percentages.len() != expected.len() { return Err("Spell bonus table is incomplete".into()); }
    Ok(percentages)
}

fn csv_row(line: &str) -> Result<Vec<String>, String> {
    let mut cols = Vec::new(); let mut value = String::new(); let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => if quoted && chars.peek() == Some(&'"') { value.push('"'); chars.next(); }
                   else { quoted = !quoted; },
            ',' if !quoted => cols.push(std::mem::take(&mut value)),
            _ => value.push(c),
        }
    }
    if quoted { return Err("Unterminated CSV quote".into()); }
    cols.push(value); Ok(cols)
}
