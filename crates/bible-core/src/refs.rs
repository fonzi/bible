//! Bible references: parsing ("Jn 3:16", "Rom. 8:28–30", "John.3.16"), formatting,
//! links (bible://John.3.16) and detection inside free text.

use crate::v11n;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ref {
    pub book: String, // OSIS id
    pub chapter: u16,
    pub verse: u16,     // 0 = whole chapter
    pub verse_end: u16, // 0 = single verse
}

impl Ref {
    pub fn new(book: &str, chapter: u16, verse: u16) -> Ref {
        Ref {
            book: book.to_string(),
            chapter,
            verse,
            verse_end: 0,
        }
    }

    pub fn book_name(&self) -> String {
        v11n::kjv()
            .book(&self.book)
            .map(|b| b.name.clone())
            .or_else(|| {
                v11n::system("KJVA")
                    .book(&self.book)
                    .map(|b| b.name.clone())
            })
            .unwrap_or_else(|| self.book.clone())
    }

    /// bible://John.3.16 or bible://John.3.16-18
    pub fn uri(&self) -> String {
        let mut s = format!("bible://{}.{}", self.book, self.chapter);
        if self.verse > 0 {
            s.push_str(&format!(".{}", self.verse));
            if self.verse_end > self.verse {
                s.push_str(&format!("-{}", self.verse_end));
            }
        }
        s
    }

    pub fn from_uri(uri: &str) -> Option<Ref> {
        let rest = uri.strip_prefix("bible://")?;
        parse_osis(rest)
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.book_name(), self.chapter)?;
        if self.verse > 0 {
            write!(f, ":{}", self.verse)?;
            if self.verse_end > self.verse {
                write!(f, "–{}", self.verse_end)?;
            }
        }
        Ok(())
    }
}

/// Parse an OSIS reference such as "John.3.16", "John.3.16-John.3.18" or "Ps.23".
pub fn parse_osis(s: &str) -> Option<Ref> {
    let s = s.trim();
    let (first, second) = match s.split_once('-') {
        Some((a, b)) => (a, Some(b)),
        None => (s, None),
    };
    let mut parts = first.split('.');
    let book = parts.next()?.to_string();
    let chapter: u16 = parts.next().and_then(|c| c.parse().ok()).unwrap_or(1);
    let verse: u16 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let verse_end = second
        .and_then(|b| b.rsplit('.').next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    canonical_book(&book).map(|osis| Ref {
        book: osis,
        chapter,
        verse,
        verse_end,
    })
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

const EXTRA: &[(&str, &[&str])] = &[
    ("Gen", &["ge", "gn", "gen", "genesis"]),
    ("Exod", &["ex", "exo", "exod", "exodus"]),
    ("Lev", &["le", "lv", "lev", "leviticus"]),
    ("Num", &["nu", "nm", "num", "numbers"]),
    ("Deut", &["dt", "deu", "deut", "deuteronomy"]),
    ("Josh", &["jos", "josh", "joshua"]),
    ("Judg", &["jdg", "jdgs", "judg", "judges"]),
    ("Ruth", &["ru", "rth", "ruth"]),
    (
        "1Sam",
        &["1sa", "1sam", "1samuel", "isam", "isamuel", "1stsamuel"],
    ),
    (
        "2Sam",
        &["2sa", "2sam", "2samuel", "iisam", "iisamuel", "2ndsamuel"],
    ),
    ("1Kgs", &["1ki", "1kgs", "1kings", "ikings", "1stkings"]),
    ("2Kgs", &["2ki", "2kgs", "2kings", "iikings", "2ndkings"]),
    (
        "1Chr",
        &["1ch", "1chr", "1chron", "1chronicles", "ichronicles"],
    ),
    (
        "2Chr",
        &["2ch", "2chr", "2chron", "2chronicles", "iichronicles"],
    ),
    ("Ezra", &["ezr", "ezra"]),
    ("Neh", &["ne", "neh", "nehemiah"]),
    ("Esth", &["es", "est", "esth", "esther"]),
    ("Job", &["jb", "job"]),
    ("Ps", &["ps", "psa", "pss", "psalm", "psalms", "psm"]),
    ("Prov", &["pr", "pro", "prv", "prov", "proverbs"]),
    (
        "Eccl",
        &["ec", "ecc", "eccl", "eccles", "ecclesiastes", "qoh"],
    ),
    (
        "Song",
        &[
            "so",
            "sos",
            "song",
            "songofsongs",
            "songofsolomon",
            "cant",
            "canticles",
        ],
    ),
    ("Isa", &["is", "isa", "isaiah"]),
    ("Jer", &["je", "jer", "jeremiah"]),
    ("Lam", &["la", "lam", "lamentations"]),
    ("Ezek", &["eze", "ezek", "ezk", "ezekiel"]),
    ("Dan", &["da", "dn", "dan", "daniel"]),
    ("Hos", &["ho", "hos", "hosea"]),
    ("Joel", &["jl", "joel"]),
    ("Amos", &["am", "amos"]),
    ("Obad", &["ob", "oba", "obad", "obadiah"]),
    ("Jonah", &["jon", "jnh", "jonah"]),
    ("Mic", &["mi", "mic", "micah"]),
    ("Nah", &["na", "nah", "nahum"]),
    ("Hab", &["hab", "habakkuk"]),
    ("Zeph", &["zep", "zeph", "zephaniah"]),
    ("Hag", &["hag", "haggai"]),
    ("Zech", &["zec", "zech", "zechariah"]),
    ("Mal", &["mal", "malachi"]),
    ("Matt", &["mt", "mat", "matt", "matthew"]),
    ("Mark", &["mk", "mar", "mrk", "mark"]),
    ("Luke", &["lk", "luk", "luke"]),
    ("John", &["jn", "jhn", "joh", "john"]),
    ("Acts", &["ac", "act", "acts"]),
    ("Rom", &["ro", "rm", "rom", "romans"]),
    (
        "1Cor",
        &["1co", "1cor", "1corinthians", "icor", "icorinthians"],
    ),
    (
        "2Cor",
        &["2co", "2cor", "2corinthians", "iicor", "iicorinthians"],
    ),
    ("Gal", &["ga", "gal", "galatians"]),
    ("Eph", &["eph", "ephes", "ephesians"]),
    ("Phil", &["php", "phil", "philippians"]),
    ("Col", &["col", "colossians"]),
    (
        "1Thess",
        &["1th", "1thes", "1thess", "1thessalonians", "ithessalonians"],
    ),
    (
        "2Thess",
        &[
            "2th",
            "2thes",
            "2thess",
            "2thessalonians",
            "iithessalonians",
        ],
    ),
    ("1Tim", &["1ti", "1tim", "1timothy", "itimothy"]),
    ("2Tim", &["2ti", "2tim", "2timothy", "iitimothy"]),
    ("Titus", &["tit", "titus"]),
    ("Phlm", &["phm", "phlm", "philem", "philemon"]),
    ("Heb", &["heb", "hebrews"]),
    ("Jas", &["jas", "jam", "jms", "james"]),
    ("1Pet", &["1pe", "1pet", "1peter", "ipeter"]),
    ("2Pet", &["2pe", "2pet", "2peter", "iipeter"]),
    ("1John", &["1jn", "1jo", "1joh", "1john", "ijohn"]),
    ("2John", &["2jn", "2jo", "2joh", "2john", "iijohn"]),
    ("3John", &["3jn", "3jo", "3joh", "3john", "iiijohn"]),
    ("Jude", &["jud", "jude"]),
    ("Rev", &["re", "rev", "revelation", "apoc", "apocalypse"]),
    ("Tob", &["tob", "tobit"]),
    ("Jdt", &["jdt", "judith"]),
    ("Wis", &["wis", "wisdom", "wisdomofsolomon"]),
    ("Sir", &["sir", "sirach", "ecclus", "ecclesiasticus"]),
    ("Bar", &["bar", "baruch"]),
    ("1Macc", &["1mac", "1macc", "1maccabees"]),
    ("2Macc", &["2mac", "2macc", "2maccabees"]),
    ("1Esd", &["1esd", "1esdras"]),
    ("2Esd", &["2esd", "2esdras"]),
    ("PrMan", &["prman", "manasseh", "prayerofmanasseh"]),
];

static NAMES: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for sys in ["KJVA", "KJV"] {
        for b in &v11n::system(sys).books {
            m.insert(norm(&b.osis), b.osis.clone());
            m.insert(norm(&b.name), b.osis.clone());
            m.insert(norm(&b.abbr), b.osis.clone());
        }
    }
    for (osis, alts) in EXTRA {
        for a in *alts {
            m.insert(a.to_string(), osis.to_string());
        }
    }
    m
});

/// Map any book name or abbreviation to its OSIS id.
pub fn canonical_book(name: &str) -> Option<String> {
    let n = norm(name);
    let n = n
        .replace("first", "1")
        .replace("second", "2")
        .replace("third", "3");
    NAMES.get(&n).cloned()
}

static REF_RE: Lazy<Regex> = Lazy::new(|| {
    // book (optionally numbered), chapter, optional :verse and range
    Regex::new(
        r"(?x)
        \b(?P<book>(?:[123]|I{1,3})\s?[A-Za-z]{2,}\.?|Song\sof\s(?:Songs|Solomon)|[A-Z][a-z]{1,}\.?)
        \s?(?P<ch>\d{1,3})
        (?:[:.](?P<v>\d{1,3})(?:\s?[-–—]\s?(?P<ve>\d{1,3}))?)?
        \b",
    )
    .unwrap()
});

/// A reference found in free text, with its byte range.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub start: usize,
    pub end: usize,
    pub r: Ref,
}

/// Find Bible references in text ("see Rom. 5:8 and 1 Jn 4:9–10").
pub fn find_refs(text: &str) -> Vec<Found> {
    let mut out = Vec::new();
    for c in REF_RE.captures_iter(text) {
        let book = c.name("book").unwrap().as_str();
        let Some(osis) = canonical_book(book) else {
            continue;
        };
        let ch: u16 = c["ch"].parse().unwrap_or(0);
        let Some(b) = v11n::system("KJVA").book(&osis) else {
            continue;
        };
        if ch == 0 || ch > b.chapters() {
            continue;
        }
        // bare "Is 5" style hits are too noisy without a verse
        let v: u16 = c
            .name("v")
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);
        if v == 0 && book.len() <= 3 {
            continue;
        }
        let ve: u16 = c
            .name("ve")
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);
        let m = c.get(0).unwrap();
        out.push(Found {
            start: m.start(),
            end: m.end(),
            r: Ref {
                book: osis,
                chapter: ch,
                verse: v,
                verse_end: ve,
            },
        });
    }
    out
}

/// Parse a typed reference ("john 3:16", "Ps 23"). Used by the "go to" box.
/// A quick "go to" typed while reading `book` `chapter`:
/// "16" is a verse here, "3:16" or "3:" a chapter of this book,
/// anything else a full reference ("Rom 8:28", "ps 23").
pub fn parse_relative(text: &str, book: &str, chapter: u16) -> Option<Ref> {
    let t = text.trim().trim_start_matches(['v', 'V']).trim();
    if t.is_empty() {
        return None;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        let v: u16 = t.parse().ok()?;
        return Some(Ref::new(book, chapter.max(1), v.max(1)));
    }
    if let Some((c, v)) = t.split_once([':', '.']) {
        if c.chars().all(|x| x.is_ascii_digit())
            && v.chars().all(|x| x.is_ascii_digit())
            && !c.is_empty()
        {
            let c: u16 = c.parse().ok()?;
            let v: u16 = v.parse().unwrap_or(0);
            return Some(Ref::new(book, c.max(1), v));
        }
    }
    parse(text)
}

pub fn parse(text: &str) -> Option<Ref> {
    let t = text.trim();
    let mut cap = String::new();
    for (i, ch) in t.char_indices() {
        if i == 0
            || t[..i].ends_with(|c: char| c.is_ascii_digit() || c == ' ')
                && t[..i].trim().chars().all(|c| c.is_ascii_digit())
        {
            cap.extend(ch.to_uppercase());
        } else {
            cap.push(ch);
        }
    }
    let t = cap.as_str();
    if let Some(f) = find_refs(t).into_iter().next() {
        return Some(f.r);
    }
    // "John 3" / "Genesis" without verse
    let re = Regex::new(r"^(?P<book>.+?)\s*(?P<ch>\d+)?$").ok()?;
    let c = re.captures(t)?;
    let osis = canonical_book(c.name("book")?.as_str())?;
    let ch = c
        .name("ch")
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(1);
    Some(Ref::new(&osis, ch, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_forms() {
        assert_eq!(parse("John 3:16"), Some(Ref::new("John", 3, 16)));
        assert_eq!(parse("jn 3:16").unwrap().book, "John");
        assert_eq!(parse("1 Cor 13").unwrap(), Ref::new("1Cor", 13, 0));
        assert_eq!(parse("Psalm 23"), Some(Ref::new("Ps", 23, 0)));
        let r = parse("Rom. 8:28–30").unwrap();
        assert_eq!(
            (r.book.as_str(), r.chapter, r.verse, r.verse_end),
            ("Rom", 8, 28, 30)
        );
        assert_eq!(parse_osis("John.3.16-John.3.18").unwrap().verse_end, 18);
    }

    #[test]
    fn finds_refs_in_prose() {
        let f = find_refs("Compare Rom 5:8, 1 Jn 4:9–10 and Eph. 2:8. Is it true?");
        let got: Vec<String> = f.iter().map(|x| x.r.to_string()).collect();
        assert_eq!(got, vec!["Romans 5:8", "1 John 4:9–10", "Ephesians 2:8"]);
    }

    #[test]
    fn uri_round_trip() {
        let r = Ref {
            book: "John".into(),
            chapter: 3,
            verse: 16,
            verse_end: 18,
        };
        assert_eq!(r.uri(), "bible://John.3.16-18");
        assert_eq!(Ref::from_uri(&r.uri()).unwrap().verse_end, 18);
    }
}

#[cfg(test)]
mod relative_tests {
    use super::*;

    #[test]
    fn quick_goto() {
        let r = parse_relative("16", "John", 3).unwrap();
        assert_eq!((r.book.as_str(), r.chapter, r.verse), ("John", 3, 16));
        let r = parse_relative("4:7", "John", 3).unwrap();
        assert_eq!((r.chapter, r.verse), (4, 7));
        let r = parse_relative("5:", "John", 3).unwrap();
        assert_eq!((r.chapter, r.verse), (5, 0));
        let r = parse_relative("Rom 8:28", "John", 3).unwrap();
        assert_eq!((r.book.as_str(), r.chapter, r.verse), ("Rom", 8, 28));
        assert!(parse_relative("", "John", 3).is_none());
    }
}
