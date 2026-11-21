//! A timeline of biblical, world and church history, for the Timeline section.
//! Data in data/timeline.json (built by tools/mktimeline.py).

use crate::refs::parse_osis;
use crate::Ref;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Era {
    pub title: String,
    pub start: i32,
    pub end: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lane {
    pub id: String,
    pub title: String,
    /// Where the dates come from and how certain they are
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub lane: String,
    pub title: String,
    pub start: i32,
    #[serde(default)]
    pub end: Option<i32>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default)]
    pub approx: bool,
    #[serde(default)]
    pub note: Option<String>,
}

impl Event {
    pub fn end(&self) -> i32 {
        self.end.unwrap_or(self.start)
    }
}

#[derive(Debug, Deserialize)]
pub struct Timeline {
    pub eras: Vec<Era>,
    pub lanes: Vec<Lane>,
    pub events: Vec<Event>,
    /// book -> [[from chapter, start year, end year], ...]
    books: HashMap<String, Vec<[i32; 3]>>,
}

impl Timeline {
    /// The years a passage is set in (or, for letters, written), if known.
    pub fn when(&self, book: &str, chapter: u16) -> Option<(i32, i32)> {
        let spans = self.books.get(book)?;
        let mut found = None;
        for s in spans {
            if chapter as i32 >= s[0] {
                found = Some((s[1], s[2]));
            }
        }
        found
    }

    pub fn lane(&self, id: &str) -> Option<&Lane> {
        self.lanes.iter().find(|l| l.id == id)
    }

    pub fn era_at(&self, year: i32) -> Option<&Era> {
        self.eras.iter().find(|e| year >= e.start && year < e.end)
    }
}

pub fn timeline() -> &'static Timeline {
    static T: Lazy<Timeline> = Lazy::new(|| {
        serde_json::from_str(include_str!("../data/timeline.json")).expect("timeline.json")
    });
    &T
}

/// "1446 BC", "AD 30", "c. 1446 BC"
pub fn year(y: i32, approx: bool) -> String {
    let c = if approx { "c. " } else { "" };
    if y < 0 {
        format!("{c}{} BC", -y)
    } else if y < 1000 {
        format!("{c}AD {y}")
    } else {
        format!("{c}{y}")
    }
}

pub fn span(start: i32, end: i32, approx: bool) -> String {
    if start == end {
        return year(start, approx);
    }
    let c = if approx { "c. " } else { "" };
    match (start < 0, end < 0) {
        (true, true) => format!("{c}{}–{} BC", -start, -end),
        (false, false) if end < 1000 => format!("{c}AD {start}–{end}"),
        (false, false) => format!("{c}{start}–{end}"),
        _ => format!("{c}{} BC – AD {end}", -start),
    }
}

/// A reference from the data, with a readable label. Ranges across chapters
/// ("Exod.12.1-Exod.15.21") become "Exodus 12–15".
pub fn ref_label(osis: &str) -> Option<(Ref, String)> {
    let r = parse_osis(osis)?;
    if let Some((_, b)) = osis.split_once('-') {
        if let Some(r2) = parse_osis(b) {
            if r2.chapter != r.chapter && r2.book == r.book {
                return Some((
                    Ref::new(&r.book, r.chapter, r.verse),
                    format!("{} {}–{}", r.book_name(), r.chapter, r2.chapter),
                ));
            }
        }
    }
    let label = r.to_string();
    Some((r, label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads() {
        let t = timeline();
        assert!(t.events.len() > 150);
        assert_eq!(t.when("1Kgs", 15), Some((-931, -853)));
        assert_eq!(t.when("Gen", 3), None);
        assert_eq!(t.when("Gen", 20), Some((-2091, -1805)));
        for e in &t.events {
            assert!(t.lane(&e.lane).is_some(), "{}", e.lane);
            for r in &e.refs {
                assert!(ref_label(r).is_some(), "{r}");
            }
        }
        assert_eq!(span(-931, -910, false), "931–910 BC");
        assert_eq!(span(-5, 30, true), "c. 5 BC – AD 30");
        assert_eq!(ref_label("Exod.12.1-Exod.15.21").unwrap().1, "Exodus 12–15");
    }
}
