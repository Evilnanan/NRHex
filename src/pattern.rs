//! AOB ("array of bytes") signature scanning with `?` / `??` wildcards.
//!
//! The upstream mod stores its signatures XOR-obfuscated and scans the game's
//! `.text` at load time.  We keep the signatures in clear text (this is a
//! re-implementation, not a redistribution) and scan every executable section.

#![allow(dead_code)]

/// A compiled signature: literal bytes plus a per-byte "must match" mask.
#[derive(Debug, Clone)]
pub struct Pattern {
    bytes: Vec<u8>,
    mask: Vec<bool>,
    /// Index of the first fixed byte, used as the search anchor.
    anchor: usize,
    source: String,
}

impl Pattern {
    /// Parse `"48 8B 05 ? ? ? ? C3"` (also accepts `??` and `*`).
    pub fn parse(src: &str) -> Result<Pattern, String> {
        let mut bytes = Vec::new();
        let mut mask = Vec::new();
        for tok in src.split_whitespace() {
            match tok {
                "?" | "??" | "*" | "**" => {
                    bytes.push(0);
                    mask.push(false);
                }
                hex => {
                    let v = u8::from_str_radix(hex.trim_start_matches("0x"), 16)
                        .map_err(|_| format!("bad hex byte {hex:?} in pattern"))?;
                    bytes.push(v);
                    mask.push(true);
                }
            }
        }
        if bytes.is_empty() {
            return Err("empty pattern".into());
        }
        let anchor = mask
            .iter()
            .position(|&m| m)
            .ok_or_else(|| "pattern is entirely wildcards".to_string())?;
        Ok(Pattern {
            bytes,
            mask,
            anchor,
            source: src.to_string(),
        })
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Render the pattern as `48 8B ?? ..`, handy for logs.
    pub fn display(&self) -> String {
        self.bytes
            .iter()
            .zip(&self.mask)
            .map(|(b, m)| {
                if *m {
                    format!("{b:02X}")
                } else {
                    "??".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[inline]
    fn matches_at(&self, hay: &[u8], start: usize) -> bool {
        let n = self.mask.len();
        if start + n > hay.len() {
            return false;
        }
        for i in 0..n {
            if self.mask[i] && hay[start + i] != self.bytes[i] {
                return false;
            }
        }
        true
    }

    /// All match offsets inside `hay`.
    pub fn find_all(&self, hay: &[u8]) -> Vec<usize> {
        let mut hits = Vec::new();
        if self.mask.len() > hay.len() {
            return hits;
        }
        let anchor_byte = self.bytes[self.anchor];
        let last_start = hay.len() - self.mask.len();
        let mut i = 0usize;
        while i <= last_start + self.anchor {
            // Scan for the anchor byte; everything before `i` has been ruled out.
            let rel = match hay[i..].iter().position(|&b| b == anchor_byte) {
                Some(r) => r,
                None => break,
            };
            let pos = i + rel;
            if pos >= self.anchor {
                let start = pos - self.anchor;
                if start <= last_start && self.matches_at(hay, start) {
                    hits.push(start);
                }
            }
            i = pos + 1;
        }
        hits
    }

    pub fn find_first(&self, hay: &[u8]) -> Option<usize> {
        self.find_all(hay).first().copied()
    }
}

/// A match resolved back to a virtual address.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub region_rva: u32,
    pub offset_in_region: usize,
    pub rva: u32,
}

impl Hit {
    pub fn va(&self, image_base: u64) -> u64 {
        image_base + self.rva as u64
    }
}

/// Scan every executable region and return (region index, hit) pairs.
pub fn scan_regions(regions: &[super::pe::ExecRegion<'_>], pattern: &Pattern) -> Vec<(usize, Hit)> {
    let mut out = Vec::new();
    for (idx, r) in regions.iter().enumerate() {
        for off in pattern.find_all(r.bytes) {
            out.push((
                idx,
                Hit {
                    region_rva: r.rva,
                    offset_in_region: off,
                    rva: r.rva + off as u32,
                },
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wildcards() {
        let p = Pattern::parse("48 8B 05 ? ? ? ? C3").unwrap();
        assert_eq!(p.len(), 8);
        assert_eq!(p.display(), "48 8B 05 ?? ?? ?? ?? C3");
    }

    #[test]
    fn finds_matches() {
        let p = Pattern::parse("AA ?? CC").unwrap();
        let hay = [0x00, 0xAA, 0x11, 0xCC, 0xAA, 0x22, 0xCD, 0xAA, 0x33, 0xCC];
        assert_eq!(p.find_all(&hay), vec![1, 7]);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Pattern::parse("").is_err());
        assert!(Pattern::parse("? ?").is_err());
        assert!(Pattern::parse("ZZ").is_err());
    }
}
