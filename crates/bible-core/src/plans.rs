//! Reading plans: each day is a list of chapters.

use crate::timeline::timeline;
use crate::v11n;
use serde::{Deserialize, Serialize};

pub struct Plan {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

pub const PLANS: &[Plan] = &[
    Plan { id: "bible-year", title: "The whole Bible in a year", description: "Genesis to Revelation, about three or four chapters a day." },
    Plan { id: "chronological", title: "Chronological, in a year", description: "The books in the order the events happened, using the timeline’s dates: Job with the patriarchs, the prophets with their kings, and the letters in the order they were written." },
    Plan { id: "nt-90", title: "New Testament in 90 days", description: "Matthew to Revelation, about three chapters a day." },
    Plan { id: "gospels-30", title: "The Gospels in 30 days", description: "Matthew, Mark, Luke and John, three chapters a day." },
    Plan { id: "psalms-proverbs", title: "Psalms and Proverbs in a month", description: "A chapter of Proverbs and five psalms each day for 31 days." },
];

pub fn plan(id: &str) -> Option<&'static Plan> {
    PLANS.iter().find(|p| p.id == id)
}

/// What the user is following: kept in the config file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanState {
    pub id: String,
    /// "2026-11-16"
    pub start: String,
    /// days finished (0-based)
    #[serde(default)]
    pub done: Vec<u16>,
}

impl PlanState {
    pub fn start_date(&self) -> Option<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(&self.start, "%Y-%m-%d").ok()
    }

    /// Today's day number (0-based), clamped to the plan.
    pub fn today(&self, now: chrono::NaiveDate, len: usize) -> usize {
        let d = self
            .start_date()
            .map(|s| (now - s).num_days())
            .unwrap_or(0)
            .max(0) as usize;
        d.min(len.saturating_sub(1))
    }

    /// The first unfinished day, the one to read next.
    pub fn next(&self, len: usize) -> Option<usize> {
        (0..len).find(|d| !self.done.contains(&(*d as u16)))
    }

    /// How many days due by today are not done.
    pub fn behind(&self, now: chrono::NaiveDate, len: usize) -> usize {
        let t = self.today(now, len);
        (0..=t)
            .filter(|d| !self.done.contains(&(*d as u16)))
            .count()
            .saturating_sub(1)
    }
}

fn chapters_of(books: &[&str]) -> Vec<(String, u16)> {
    let sys = v11n::kjv();
    let mut out = vec![];
    for b in books {
        if let Some(info) = sys.book(b) {
            for c in 1..=info.chapters() {
                out.push((b.to_string(), c));
            }
        }
    }
    out
}

/// Split chapters as evenly as possible across `days`.
fn spread(chapters: Vec<(String, u16)>, days: usize) -> Vec<Vec<(String, u16)>> {
    let n = chapters.len();
    let mut out = Vec::with_capacity(days);
    let mut it = chapters.into_iter();
    for d in 0..days {
        let take = (n * (d + 1)) / days - (n * d) / days;
        out.push(it.by_ref().take(take).collect());
    }
    out
}

/// The 66 books in Bible order (OSIS names).
pub fn books() -> Vec<&'static str> {
    OT.iter().chain(NT.iter()).copied().collect()
}

const OT: [&str; 39] = [
    "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sam", "2Sam", "1Kgs", "2Kgs",
    "1Chr", "2Chr", "Ezra", "Neh", "Esth", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer",
    "Lam", "Ezek", "Dan", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph",
    "Hag", "Zech", "Mal",
];
const NT: [&str; 27] = [
    "Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col",
    "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas", "1Pet", "2Pet", "1John",
    "2John", "3John", "Jude", "Rev",
];

/// Books in the order their events happened (or, for letters, were written).
/// Books the timeline can't date are placed by tradition.
pub fn chronological_books() -> Vec<&'static str> {
    let tl = timeline();
    let fixed = |b: &str| -> Option<i32> {
        Some(match b {
            "Job" => -2000,  // with the patriarchs
            "Ps" => -1000,   // mostly David's time
            "Joel" => -835,  // early, in Joash's reign
            "Obad" => -586,  // after Jerusalem's fall
            "1Chr" => -1011, // alongside Samuel
            _ => return None,
        })
    };
    let mut all: Vec<(&str, i32, usize)> = OT
        .iter()
        .chain(NT.iter())
        .enumerate()
        .map(|(i, b)| {
            // Genesis starts at creation
            let y = if *b == "Gen" {
                -9999
            } else {
                fixed(b)
                    .or_else(|| tl.when(b, 1).map(|w| w.0))
                    .or_else(|| tl.when(b, 12).map(|w| w.0))
                    .unwrap_or(0)
            };
            (*b, y, i)
        })
        .collect();
    all.sort_by_key(|(_, y, i)| (*y, *i));
    all.into_iter().map(|(b, _, _)| b).collect()
}

/// Each day's chapters for a plan.
pub fn days(id: &str) -> Vec<Vec<(String, u16)>> {
    match id {
        "bible-year" => {
            let books: Vec<&str> = OT.iter().chain(NT.iter()).copied().collect();
            spread(chapters_of(&books), 365)
        }
        "chronological" => spread(chapters_of(&chronological_books()), 365),
        "nt-90" => spread(chapters_of(&NT), 90),
        "gospels-30" => spread(chapters_of(&NT[..4]), 30),
        "psalms-proverbs" => {
            let ps: Vec<(String, u16)> = chapters_of(&["Ps"]);
            let ps = spread(ps, 31);
            (0..31)
                .map(|d| {
                    std::iter::once(("Prov".to_string(), d as u16 + 1))
                        .chain(ps[d].iter().cloned())
                        .collect()
                })
                .collect()
        }
        _ => vec![],
    }
}

/// "Genesis 1–3", "Proverbs 4 · Psalms 21–25"
pub fn describe(day: &[(String, u16)]) -> String {
    let mut parts: Vec<String> = vec![];
    let mut i = 0;
    while i < day.len() {
        let (b, c) = &day[i];
        let mut j = i;
        while j + 1 < day.len() && day[j + 1].0 == *b && day[j + 1].1 == day[j].1 + 1 {
            j += 1;
        }
        let name = crate::Ref::new(b, 1, 0).book_name();
        parts.push(if j > i {
            format!("{name} {c}–{}", day[j].1)
        } else {
            format!("{name} {c}")
        });
        i = j + 1;
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_cover_everything() {
        let y = days("bible-year");
        assert_eq!(y.len(), 365);
        assert_eq!(y.iter().map(|d| d.len()).sum::<usize>(), 1189);
        let c = days("chronological");
        assert_eq!(c.iter().map(|d| d.len()).sum::<usize>(), 1189);
        assert_eq!(days("nt-90").iter().map(|d| d.len()).sum::<usize>(), 260);
        assert_eq!(
            days("gospels-30").iter().map(|d| d.len()).sum::<usize>(),
            89
        );
        let pp = days("psalms-proverbs");
        assert_eq!(pp.len(), 31);
        assert_eq!(pp.iter().map(|d| d.len()).sum::<usize>(), 181);
        assert_eq!(describe(&y[0]), "Genesis 1–3");
        let order = chronological_books();
        let pos = |b| order.iter().position(|x| *x == b).unwrap();
        assert!(pos("Job") < pos("Exod"));
        assert!(pos("Gal") < pos("Rom") && pos("Acts") < pos("Rom"));
        assert!(pos("Isa") < pos("Jer") && pos("Mal") > pos("Neh"));
    }

    #[test]
    fn progress() {
        let s = PlanState {
            id: "gospels-30".into(),
            start: "2026-11-01".into(),
            done: vec![0, 1, 2],
        };
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert_eq!(s.today(now, 30), 5);
        assert_eq!(s.next(30), Some(3));
        assert_eq!(s.behind(now, 30), 2);
    }
}
