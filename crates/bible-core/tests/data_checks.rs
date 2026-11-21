//! Checks over the built-in data: every reference parses, every plan covers
//! what it promises, the calendars agree with known dates.

use bible_core::churchyear::{easter_orthodox, easter_western, Tradition};
use bible_core::refs::{parse, parse_osis};
use chrono::NaiveDate;
use std::collections::HashSet;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn easter_dates_match_published_tables() {
    for (y, w, o) in [
        (2024, d(2024, 3, 31), d(2024, 5, 5)),
        (2025, d(2025, 4, 20), d(2025, 4, 20)),
        (2026, d(2026, 4, 5), d(2026, 4, 12)),
        (2027, d(2027, 3, 28), d(2027, 5, 2)),
        (2030, d(2030, 4, 21), d(2030, 4, 28)),
    ] {
        assert_eq!(easter_western(y), w, "western {y}");
        assert_eq!(easter_orthodox(y), o, "orthodox {y}");
    }
}

#[test]
fn good_friday_is_a_fast_everywhere_that_keeps_one() {
    let gf = easter_western(2027) - chrono::Duration::days(2);
    for t in [Tradition::Catholic, Tradition::Anglican] {
        assert!(bible_core::fasting::fast_day(gf, t).is_some(), "{t:?}");
    }
    let ogf = easter_orthodox(2027) - chrono::Duration::days(2);
    assert!(bible_core::fasting::fast_day(ogf, Tradition::Orthodox).is_some());
    // Easter Sunday itself is never a fast
    for t in [Tradition::Catholic, Tradition::Anglican, Tradition::Orthodox] {
        let e = if t == Tradition::Orthodox { easter_orthodox(2027) } else { easter_western(2027) };
        let f = bible_core::fasting::fast_day(e, t);
        assert!(f.is_none() || format!("{:?}", f.unwrap().kind).contains("FastFree"), "{t:?}");
    }
}

#[test]
fn reference_parsing_covers_common_forms() {
    for (text, osis, c, v) in [
        ("John 3:16", "John", 3, 16),
        ("jn 3:16", "John", 3, 16),
        ("1 Cor 13", "1Cor", 13, 0),
        ("1 Corinthians 13:4", "1Cor", 13, 4),
        ("Ps 23", "Ps", 23, 0),
        ("Psalm 119:105", "Ps", 119, 105),
        ("Song of Solomon 2:1", "Song", 2, 1),
        ("Rev 22:21", "Rev", 22, 21),
        ("Tobit 1:1", "Tob", 1, 1),
        ("Sirach 2", "Sir", 2, 0),
        ("1 Maccabees 4:36", "1Macc", 4, 36),
    ] {
        let r = parse(text).unwrap_or_else(|| panic!("{text}"));
        assert_eq!((r.book.as_str(), r.chapter, r.verse), (osis, c, v), "{text}");
    }
    assert!(parse("not a reference").is_none());
}

#[test]
fn osis_round_trips() {
    for s in ["Gen.1.1", "John.3.16", "Rev.22.21", "Ps.119.176", "Tob.1.1"] {
        let r = parse_osis(s).unwrap();
        assert_eq!(format!("{}.{}.{}", r.book, r.chapter, r.verse), s);
    }
}

#[test]
fn bible_in_a_year_reads_every_chapter_once() {
    let v = bible_core::v11n::kjv();
    let mut want = HashSet::new();
    for b in v.books.iter().filter(|b| !bible_core::v11n::is_apocrypha(&b.osis)) {
        for c in 1..=b.chapters() {
            want.insert((b.osis.clone(), c));
        }
    }
    let days = bible_core::plans::days("bible-year");
    assert_eq!(days.len(), 365);
    let mut seen = HashSet::new();
    for day in &days {
        for (b, c) in day {
            assert!(seen.insert((b.clone(), *c)), "{b} {c} read twice");
        }
    }
    assert_eq!(seen, want, "missing or extra chapters");
}

#[test]
fn every_plan_has_days_and_valid_chapters() {
    let v = bible_core::v11n::kjv();
    for id in ["bible-year", "chronological", "nt-90", "gospels-30", "psalms-proverbs"] {
        let days = bible_core::plans::days(id);
        assert!(!days.is_empty(), "{id}");
        for day in &days {
            assert!(!day.is_empty(), "{id} has an empty day");
            for (b, c) in day {
                let book = v.book(b).unwrap_or_else(|| panic!("{id}: {b}"));
                assert!(*c >= 1 && *c <= book.chapters(), "{id}: {b} {c}");
            }
            assert!(!bible_core::plans::describe(day).is_empty());
        }
    }
}

#[test]
fn chronological_order_holds_all_66_books() {
    let books: HashSet<_> = bible_core::plans::chronological_books().into_iter().collect();
    let canon: HashSet<_> = bible_core::plans::books().into_iter().collect();
    assert_eq!(books, canon);
    assert_eq!(canon.len(), 66);
}

#[test]
fn daily_passage_every_day_of_a_year_is_real() {
    let v = bible_core::v11n::kjv();
    let mut day = d(2026, 1, 1);
    let mut seen = HashSet::new();
    while day.year_ce().1 == 2026 {
        for t in [Tradition::Western, Tradition::Orthodox] {
            let (_, osis) = bible_core::daily::passage(day, t);
            let (b, c, v1, v2) = bible_core::daily::verses(osis).unwrap_or_else(|| panic!("{osis}"));
            let book = v.book(&b).unwrap_or_else(|| panic!("{osis}"));
            assert!(c >= 1 && c <= book.chapters(), "{osis}");
            assert!(v1 >= 1 && v2 >= v1 && v2 <= book.verse_count(c), "{osis}");
            seen.insert(osis);
        }
        day = day.succ_opt().unwrap();
    }
    assert!(seen.len() > 300, "passages repeat too often: {}", seen.len());
}

#[test]
fn church_history_has_something_near_every_date() {
    let mut day = d(2027, 1, 1);
    while day.year_ce().1 == 2027 {
        let (_, events) = bible_core::onthisday::for_date(day);
        assert!(!events.is_empty(), "nothing near {day}");
        day = day.succ_opt().unwrap();
    }
}

#[test]
fn church_day_is_named_for_every_tradition() {
    for (t, _) in Tradition::all() {
        let mut day = d(2026, 1, 1);
        for _ in 0..366 {
            let info = bible_core::churchyear::day_info(day, *t);
            assert!(!info.season.is_empty(), "{t:?} {day}");
            day = day.succ_opt().unwrap();
        }
    }
}

#[test]
fn intros_cover_canon_and_deuterocanon() {
    for b in bible_core::plans::books() {
        assert!(bible_core::intros::intro(b).is_some(), "{b}");
    }
    for b in [
        "Tob", "Jdt", "AddEsth", "Wis", "Sir", "Bar", "EpJer", "PrAzar", "Sus", "Bel", "1Macc",
        "2Macc", "3Macc", "4Macc", "1Esd", "2Esd", "PrMan", "Ps151",
    ] {
        let i = bible_core::intros::intro(b).unwrap_or_else(|| panic!("{b}"));
        assert!(i.text.len() > 150, "{b} intro too short");
        assert!(!i.traditional.is_empty() && !i.article.is_empty(), "{b}");
        assert!(i.url().starts_with("https://en.wikipedia.org/wiki/"));
    }
}

#[test]
fn bundled_cross_references_are_complete_and_parse() {
    let all = bible_core::crossrefs::all();
    assert!(all.len() > 340_000, "{}", all.len());
    let bad = all
        .iter()
        .filter(|(f, t, _)| parse_osis(f).is_none() || parse_osis(t.split('-').next().unwrap()).is_none())
        .count();
    assert_eq!(bad, 0, "{bad} references don't parse");
    assert!(all.iter().all(|(_, _, votes)| *votes > 0));
}

trait YearCe {
    fn year_ce(&self) -> (bool, i32);
}
impl YearCe for NaiveDate {
    fn year_ce(&self) -> (bool, i32) {
        use chrono::Datelike;
        (true, self.year())
    }
}
