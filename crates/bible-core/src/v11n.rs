//! Versification systems: which books exist, how many chapters and verses each has,
//! and how a verse maps onto the index layout used by SWORD modules.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
struct RawBook {
    osis: String,
    name: String,
    pref: String,
    v: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSystem {
    ot: usize,
    books: Vec<RawBook>,
}

#[derive(Debug, Clone)]
pub struct Book {
    pub osis: String,
    pub name: String,
    pub abbr: String,
    /// verses[c-1] = number of verses in chapter c
    pub verses: Vec<u16>,
}

impl Book {
    pub fn chapters(&self) -> u16 {
        self.verses.len() as u16
    }
    pub fn verse_count(&self, chapter: u16) -> u16 {
        self.verses.get(chapter as usize - 1).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct System {
    pub name: String,
    pub books: Vec<Book>,
    /// number of books stored in the first (Old Testament) file
    pub ot_books: usize,
    /// cumulative index start of each book within its testament
    book_start: Vec<usize>,
}

/// Where a verse lives in a SWORD module: testament file (1 = ot, 2 = nt) and entry index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub testament: u8,
    pub index: usize,
}

impl System {
    fn from_raw(name: &str, raw: RawSystem) -> System {
        let books: Vec<Book> = raw
            .books
            .into_iter()
            .map(|b| Book {
                osis: b.osis,
                name: display_name(&b.name),
                abbr: b.pref,
                verses: b.v,
            })
            .collect();
        let mut book_start = Vec::with_capacity(books.len());
        let mut acc = 2usize; // 0 = module heading, 1 = testament heading
        for (i, b) in books.iter().enumerate() {
            if i == raw.ot {
                acc = 2;
            }
            book_start.push(acc);
            acc += 1 + b.verses.iter().map(|v| 1 + *v as usize).sum::<usize>();
        }
        System {
            name: name.to_string(),
            books,
            ot_books: raw.ot,
            book_start,
        }
    }

    pub fn book_index(&self, osis: &str) -> Option<usize> {
        self.books
            .iter()
            .position(|b| b.osis.eq_ignore_ascii_case(osis))
    }

    pub fn book(&self, osis: &str) -> Option<&Book> {
        self.book_index(osis).map(|i| &self.books[i])
    }

    /// Index slot for book (by position), chapter and verse. Verse 0 = chapter heading,
    /// chapter 0 = book heading.
    pub fn slot(&self, book: usize, chapter: u16, verse: u16) -> Option<Slot> {
        let b = self.books.get(book)?;
        let testament = if book < self.ot_books { 1 } else { 2 };
        let mut idx = self.book_start[book];
        if chapter == 0 {
            return Some(Slot {
                testament,
                index: idx,
            });
        }
        if chapter > b.chapters() || verse > b.verse_count(chapter) {
            return None;
        }
        idx += 1;
        for c in 1..chapter {
            idx += 1 + b.verse_count(c) as usize;
        }
        Some(Slot {
            testament,
            index: idx + verse as usize,
        })
    }

    pub fn slot_for(&self, osis: &str, chapter: u16, verse: u16) -> Option<Slot> {
        self.slot(self.book_index(osis)?, chapter, verse)
    }
}

/// SWORD's long names use roman numerals ("I John"); show "1 John" instead.
fn display_name(n: &str) -> String {
    let n = n.trim();
    for (r, a) in [("III ", "3 "), ("II ", "2 "), ("I ", "1 ")] {
        if let Some(rest) = n.strip_prefix(r) {
            return format!("{a}{rest}");
        }
    }
    match n {
        "Revelation of John" => "Revelation".into(),
        _ => n.to_string(),
    }
}

static SYSTEMS: Lazy<HashMap<String, System>> = Lazy::new(|| {
    let raw: HashMap<String, RawSystem> =
        serde_json::from_str(include_str!("../data/versification.json"))
            .expect("versification data");
    raw.into_iter()
        .map(|(k, v)| (k.clone(), System::from_raw(&k, v)))
        .collect()
});

/// Look up a versification system by name; unknown names fall back to KJV, as SWORD does.
pub fn system(name: &str) -> &'static System {
    SYSTEMS.get(name).unwrap_or_else(|| &SYSTEMS["KJV"])
}

pub fn kjv() -> &'static System {
    system("KJV")
}

/// Deuterocanonical / apocryphal books (OSIS ids). Hidden unless the user opts in.
pub const APOCRYPHA: &[&str] = &[
    "Tob", "Jdt", "AddEsth", "EsthGr", "Wis", "Sir", "Bar", "EpJer", "PrAzar", "Sus", "Bel",
    "1Macc", "2Macc", "3Macc", "4Macc", "1Esd", "2Esd", "PrMan", "Ps151", "AddPs", "Odes",
    "PssSol", "EpLao", "AddDan", "1En", "Jub", "4Ezra", "5Ezra", "6Ezra", "Esd", "Mal4",
];

pub fn is_apocrypha(osis: &str) -> bool {
    APOCRYPHA.iter().any(|a| a.eq_ignore_ascii_case(osis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kjv_shape() {
        let k = kjv();
        assert_eq!(k.books.len(), 66);
        assert_eq!(k.ot_books, 39);
        let total: usize = k
            .books
            .iter()
            .map(|b| b.verses.iter().map(|v| *v as usize).sum::<usize>())
            .sum();
        assert_eq!(total, 31102);
    }

    #[test]
    fn slots_match_sword_layout() {
        let k = kjv();
        assert_eq!(
            k.slot_for("Gen", 1, 1),
            Some(Slot {
                testament: 1,
                index: 4
            })
        );
        assert_eq!(
            k.slot_for("Matt", 1, 1),
            Some(Slot {
                testament: 2,
                index: 4
            })
        );
        // last NT verse is the last entry of an 8246-entry index
        assert_eq!(
            k.slot_for("Rev", 22, 21),
            Some(Slot {
                testament: 2,
                index: 8245
            })
        );
        assert_eq!(
            k.slot_for("Mal", 4, 6),
            Some(Slot {
                testament: 1,
                index: 24114
            })
        );
    }

    #[test]
    fn apocrypha_in_kjva() {
        let a = system("KJVA");
        assert!(a.book("Tob").is_some());
        assert!(is_apocrypha("Tob"));
        assert!(!is_apocrypha("John"));
    }
}
