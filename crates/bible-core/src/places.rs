//! Biblical places with coordinates, and the base map geography.
//!
//! Places come from STEPBible's TIPNR (CC BY 4.0, STEPBible.org / Tyndale House,
//! geolocations from OpenBible.info). Geography is Natural Earth (public domain),
//! clipped to the lands of the Bible. Both are converted by tools/ in the source tree.

use crate::refs::parse_osis;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::io::Read;

pub const CREDIT: &str = "Places: STEPBible.org TIPNR (CC BY 4.0), locations from OpenBible.info. Map data: Natural Earth (public domain).";

fn gunzip(bytes: &[u8]) -> String {
    let mut s = String::new();
    let _ = flate2::read::GzDecoder::new(bytes).read_to_string(&mut s);
    s
}

#[derive(Debug, Clone)]
pub struct Place {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub brief: String,
    /// OSIS references, "Gen.12.6"
    pub refs: Vec<String>,
}

pub struct Places {
    pub list: Vec<Place>,
    /// (book, chapter) -> [(place index, verses)]
    by_chapter: HashMap<(String, u16), Vec<(usize, Vec<u16>)>>,
}

impl Places {
    fn parse(text: &str) -> Places {
        let mut list = Vec::new();
        let mut by_chapter: HashMap<(String, u16), Vec<(usize, Vec<u16>)>> = HashMap::new();
        for line in text.lines() {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 5 {
                continue;
            }
            let (Ok(lat), Ok(lon)) = (c[1].parse(), c[2].parse()) else {
                continue;
            };
            let refs: Vec<String> = c[4]
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            let idx = list.len();
            for r in &refs {
                let Some(rf) = parse_osis(r) else { continue };
                let e = by_chapter.entry((rf.book.clone(), rf.chapter)).or_default();
                match e.iter_mut().find(|(i, _)| *i == idx) {
                    Some((_, vs)) => vs.push(rf.verse),
                    None => e.push((idx, vec![rf.verse])),
                }
            }
            list.push(Place {
                name: c[0].to_string(),
                lat,
                lon,
                brief: c[3].to_string(),
                refs,
            });
        }
        Places { list, by_chapter }
    }

    /// Places mentioned in a chapter, with the verses that mention them.
    pub fn in_chapter(&self, book: &str, chapter: u16) -> Vec<(&Place, &[u16])> {
        self.by_chapter
            .get(&(book.to_string(), chapter))
            .map(|v| {
                v.iter()
                    .map(|(i, vs)| (&self.list[*i], vs.as_slice()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Places named anywhere in a book, most-mentioned first, with how many verses.
    pub fn in_book(&self, book: &str) -> Vec<(&Place, usize)> {
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for ((b, _), list) in &self.by_chapter {
            if b == book {
                for (i, vs) in list {
                    *counts.entry(*i).or_default() += vs.len();
                }
            }
        }
        let mut v: Vec<(&Place, usize)> = counts
            .into_iter()
            .map(|(i, n)| (&self.list[i], n))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.name.cmp(&b.0.name)));
        v
    }

    pub fn find(&self, name: &str) -> Vec<&Place> {
        let n = name.to_lowercase();
        self.list
            .iter()
            .filter(|p| p.name.to_lowercase() == n)
            .collect()
    }
}

pub fn places() -> &'static Places {
    static P: Lazy<Places> =
        Lazy::new(|| Places::parse(&gunzip(include_bytes!("../data/places.tsv.gz"))));
    &P
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Land,
    /// A hole in the land polygon before it (inland sea)
    Hole,
    /// A lake
    Water,
    River,
}

pub struct Shape {
    pub kind: ShapeKind,
    /// (lon, lat)
    pub pts: Vec<(f64, f64)>,
}

pub fn geography() -> &'static [Shape] {
    static G: Lazy<Vec<Shape>> = Lazy::new(|| {
        gunzip(include_bytes!("../data/geo.txt.gz"))
            .lines()
            .filter_map(|l| {
                let mut it = l.split(' ');
                let kind = match it.next()? {
                    "L" => ShapeKind::Land,
                    "H" => ShapeKind::Hole,
                    "W" => ShapeKind::Water,
                    "R" => ShapeKind::River,
                    _ => return None,
                };
                let pts = it
                    .filter_map(|p| {
                        let (x, y) = p.split_once(',')?;
                        Some((x.parse().ok()?, y.parse().ok()?))
                    })
                    .collect();
                Some(Shape { kind, pts })
            })
            .collect()
    });
    &G
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_places() {
        let p = places();
        assert!(p.list.len() > 900);
        let names: Vec<_> = p
            .in_chapter("John", 3)
            .iter()
            .map(|(p, _)| p.name.clone())
            .collect();
        assert!(names.iter().any(|n| n == "Aenon"), "{names:?}");
        assert!(!p.find("Jerusalem").is_empty());
        assert!(
            geography()
                .iter()
                .filter(|s| s.kind == ShapeKind::Land)
                .count()
                > 5
        );
    }
}
