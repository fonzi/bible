//! Keyed modules: dictionaries, lexicons and daily devotionals
//! (RawLD, RawLD4 and zLD drivers).

use crate::conf::decode;
use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LdKind {
    Raw { size4: bool },
    Z,
}

#[derive(Debug)]
pub struct Ld {
    prefix: PathBuf,
    kind: LdKind,
    /// (key, offset into .dat, size)
    keys: Vec<(String, u32, u32)>,
}

fn u32le(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

impl Ld {
    pub fn open(prefix: &Path, kind: LdKind) -> std::io::Result<Ld> {
        let idx = fs::read(prefix.with_extension("idx"))
            .or_else(|_| fs::read(format!("{}.idx", prefix.display())))?;
        let dat = fs::read(format!("{}.dat", prefix.display()))?;
        let width = match kind {
            LdKind::Raw { size4: false } => 6,
            _ => 8,
        };
        let mut keys = Vec::with_capacity(idx.len() / width);
        for c in idx.chunks_exact(width) {
            let off = u32le(c, 0);
            let size = if width == 6 {
                u16::from_le_bytes([c[4], c[5]]) as u32
            } else {
                u32le(c, 4)
            };
            let start = off as usize;
            let end = (start + size as usize).min(dat.len());
            if start >= dat.len() {
                continue;
            }
            let entry = &dat[start..end];
            let kend = entry
                .iter()
                .position(|&b| b == b'\n')
                .unwrap_or(entry.len());
            let key = decode(&entry[..kend]).trim_end_matches('\r').to_string();
            keys.push((key, off, size));
        }
        Ok(Ld {
            prefix: PathBuf::from(format!("{}", prefix.display())),
            kind,
            keys,
        })
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.keys.iter().map(|(k, _, _)| k.as_str())
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn find(&self, key: &str) -> Option<usize> {
        let k = key.trim().to_uppercase();
        self.keys.iter().position(|(x, _, _)| x.to_uppercase() == k)
    }

    /// Entry text for a key, trying the padded forms SWORD uses for Strong's numbers.
    pub fn get(&self, key: &str) -> Option<(String, String)> {
        let mut cands = vec![key.trim().to_string()];
        let up = key.trim().to_uppercase();
        if let Some(rest) = up.strip_prefix('G').or_else(|| up.strip_prefix('H')) {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                let p = &up[..1];
                cands.extend([
                    format!("{p}{n:04}"),
                    format!("{n:05}"),
                    format!("{n:04}"),
                    format!("{p}{n}"),
                    n.to_string(),
                ]);
            }
        }
        for c in cands {
            if let Some(i) = self.find(&c) {
                return self.read(i).map(|t| (self.keys[i].0.clone(), t));
            }
        }
        None
    }

    fn read(&self, i: usize) -> Option<String> {
        let (_, off, size) = self.keys[i];
        let dat = fs::read(format!("{}.dat", self.prefix.display())).ok()?;
        let entry = dat.get(off as usize..(off + size) as usize)?;
        let kend = entry.iter().position(|&b| b == b'\n')? + 1;
        match self.kind {
            LdKind::Raw { .. } => Some(
                decode(&entry[kend..])
                    .trim_end_matches(['\0', '\n', '\r'])
                    .to_string(),
            ),
            LdKind::Z => {
                let rest = &entry[kend..];
                if rest.len() < 8 {
                    return None;
                }
                let block = u32le(rest, 0) as usize;
                let ent = u32le(rest, 4) as usize;
                let zdx = fs::read(format!("{}.zdx", self.prefix.display())).ok()?;
                let boff = u32le(&zdx, block * 8) as u64;
                let bsize = u32le(&zdx, block * 8 + 4) as usize;
                let zdt = fs::read(format!("{}.zdt", self.prefix.display())).ok()?;
                let comp = zdt.get(boff as usize..boff as usize + bsize)?;
                let mut buf = Vec::new();
                ZlibDecoder::new(comp).read_to_end(&mut buf).ok()?;
                let count = u32le(&buf, 0) as usize;
                if ent >= count {
                    return None;
                }
                let eo = u32le(&buf, 4 + ent * 8) as usize;
                let es = u32le(&buf, 8 + ent * 8) as usize;
                let text = buf.get(eo..eo + es)?;
                Some(
                    decode(text)
                        .trim_end_matches(['\0', '\n', '\r'])
                        .to_string(),
                )
            }
        }
    }

    /// Today's entry in a daily devotional (keys are "MM.DD").
    pub fn for_date(&self, month: u32, day: u32) -> Option<(String, String)> {
        self.get(&format!("{month:02}.{day:02}"))
    }
}
