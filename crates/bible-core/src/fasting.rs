//! Fast and abstinence days as commonly kept in each tradition. Shown only
//! when the user turns the fasting calendar on. Practice varies by country,
//! jurisdiction and parish, so every entry is a summary, not a rule.

use crate::churchyear::{easter_orthodox, easter_western, Tradition};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// a full fast day (e.g. Good Friday)
    Strict,
    /// a fast day (reduced food, or no meat and dairy)
    Fast,
    /// abstinence from meat only
    Abstinence,
    /// a day when fasting is set aside (feast weeks)
    FastFree,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastDay {
    pub kind: Kind,
    pub title: String,
    pub detail: String,
}

fn day(kind: Kind, title: &str, detail: &str) -> Option<FastDay> {
    Some(FastDay {
        kind,
        title: title.into(),
        detail: detail.into(),
    })
}

/// Which family of practice a tradition follows here.
pub fn family(t: Tradition) -> &'static str {
    match t {
        Tradition::Catholic => "Catholic",
        Tradition::Orthodox => "Orthodox",
        Tradition::Anglican => "Anglican",
        _ => "Protestant",
    }
}

pub fn note(t: Tradition) -> &'static str {
    match t {
        Tradition::Catholic => "Latin-rite norms: fasting binds ages 18–59 (one full meal and two smaller ones), abstinence from meat age 14 and up. Bishops’ conferences set Friday practice outside Lent.",
        Tradition::Orthodox => "The usual monastic rule, simplified. Fixed dates follow the Revised Julian (New) Calendar; Old Calendar churches keep them 13 days later. Ask your priest about your own rule.",
        Tradition::Anglican => "From the Book of Common Prayer’s days of fasting and abstinence. Practice varies widely between provinces and parishes.",
        _ => "Most Protestant churches have no required fast days. These are the days many Christians choose to keep.",
    }
}

pub fn fast_day(date: NaiveDate, t: Tradition) -> Option<FastDay> {
    match family(t) {
        "Catholic" => catholic(date),
        "Orthodox" => orthodox(date),
        "Anglican" => anglican(date),
        _ => protestant(date),
    }
}

fn lent_west(date: NaiveDate) -> (i64, NaiveDate) {
    let easter = easter_western(date.year());
    ((date - easter).num_days(), easter)
}

fn catholic(date: NaiveDate) -> Option<FastDay> {
    let (off, _) = lent_west(date);
    match off {
        -46 => {
            return day(
                Kind::Fast,
                "Ash Wednesday",
                "Fast and abstinence: one full meal, two smaller meals, no meat.",
            )
        }
        -2 => {
            return day(
                Kind::Fast,
                "Good Friday",
                "Fast and abstinence: one full meal, two smaller meals, no meat.",
            )
        }
        -45..=-3 if date.weekday() == Weekday::Fri => {
            return day(Kind::Abstinence, "Friday of Lent", "Abstinence from meat.")
        }
        _ => {}
    }
    // Christmas and solemnities on a Friday lift the Friday penance
    if date.weekday() == Weekday::Fri
        && !(date.month() == 12 && date.day() == 25)
        && !(0..=6).contains(&off)
    {
        return day(
            Kind::Abstinence,
            "Friday penance",
            "Abstinence from meat, or another penance where the bishops allow it.",
        );
    }
    None
}

fn anglican(date: NaiveDate) -> Option<FastDay> {
    let (off, _) = lent_west(date);
    match off {
        -46 => day(Kind::Fast, "Ash Wednesday", "A day of fasting."),
        -2 => day(Kind::Fast, "Good Friday", "A day of fasting."),
        -45..=-3 if date.weekday() != Weekday::Sun => day(
            Kind::Abstinence,
            "Weekday in Lent",
            "A day of special discipline and self-denial.",
        ),
        _ if date.weekday() == Weekday::Fri
            && !(0..=6).contains(&off)
            && !(date.month() == 12 && date.day() == 25) =>
        {
            day(
                Kind::Abstinence,
                "Friday",
                "Fridays are days of special devotion and self-denial.",
            )
        }
        _ => None,
    }
}

fn protestant(date: NaiveDate) -> Option<FastDay> {
    let (off, _) = lent_west(date);
    match off {
        -46 => day(
            Kind::Fast,
            "Ash Wednesday",
            "Many observe a fast at the start of Lent.",
        ),
        -2 => day(
            Kind::Fast,
            "Good Friday",
            "Many observe a fast in remembrance of the crucifixion.",
        ),
        -45..=-3 if date.weekday() != Weekday::Sun => day(
            Kind::Abstinence,
            "Lent",
            "A season many keep with a personal fast or discipline.",
        ),
        _ => None,
    }
}

fn orthodox(date: NaiveDate) -> Option<FastDay> {
    let pascha = easter_orthodox(date.year());
    let off = (date - pascha).num_days();
    let (m, d) = (date.month(), date.day());
    let wd = date.weekday();
    // fast-free periods first
    if (0..=6).contains(&off) {
        return day(
            Kind::FastFree,
            "Bright Week",
            "No fasting, even on Wednesday and Friday.",
        );
    }
    if (50..=56).contains(&off) {
        return day(Kind::FastFree, "Week after Pentecost", "No fasting.");
    }
    if (-69..=-63).contains(&off) {
        return day(
            Kind::FastFree,
            "Week of the Publican and the Pharisee",
            "No fasting.",
        );
    }
    if (m == 12 && d >= 25) || (m == 1 && d <= 4) {
        return day(
            Kind::FastFree,
            "Christmastide",
            "No fasting from Nativity to the eve of Theophany.",
        );
    }
    // Great Lent and Holy Week
    match off {
        -2 => {
            return day(
                Kind::Strict,
                "Great and Holy Friday",
                "A strict fast; many take no food until after the evening service.",
            )
        }
        -1 => {
            return day(
                Kind::Fast,
                "Great and Holy Saturday",
                "No meat, dairy or oil; wine is allowed.",
            )
        }
        -6..=-3 => {
            return day(
                Kind::Fast,
                "Holy Week",
                "No meat, dairy, fish, wine or oil.",
            )
        }
        -7 => return day(Kind::Fast, "Palm Sunday", "Fish, wine and oil allowed."),
        -48..=-8 => {
            if m == 3 && d == 25 {
                return day(
                    Kind::Fast,
                    "Annunciation in Lent",
                    "Fish, wine and oil allowed.",
                );
            }
            return match wd {
                Weekday::Sat | Weekday::Sun => day(
                    Kind::Fast,
                    "Great Lent",
                    "No meat or dairy; wine and oil allowed at the weekend.",
                ),
                _ => day(
                    Kind::Fast,
                    "Great Lent",
                    "No meat, dairy, fish, wine or oil.",
                ),
            };
        }
        -55..=-49 => {
            return day(
                Kind::Abstinence,
                "Cheesefare Week",
                "No meat; dairy, eggs and fish allowed, even on Wednesday and Friday.",
            )
        }
        _ => {}
    }
    // one-day strict fasts
    match (m, d) {
        (8, 29) => {
            return day(
                Kind::Fast,
                "Beheading of St John the Baptist",
                "A strict fast day.",
            )
        }
        (9, 14) => {
            return day(
                Kind::Fast,
                "Exaltation of the Holy Cross",
                "A strict fast day.",
            )
        }
        (1, 5) => return day(Kind::Fast, "Eve of Theophany", "A strict fast day."),
        _ => {}
    }
    // the Apostles' Fast: Monday after All Saints (Pascha + 57) to June 28
    let apostles = pascha + Duration::days(57);
    let june28 = NaiveDate::from_ymd_opt(date.year(), 6, 28)?;
    if date >= apostles && date <= june28 {
        return day(
            Kind::Fast,
            "Apostles’ Fast",
            "No meat or dairy; fish, wine and oil on most days.",
        );
    }
    if m == 8 && (1..=14).contains(&d) {
        if d == 6 {
            return day(
                Kind::Fast,
                "Dormition Fast · Transfiguration",
                "Fish, wine and oil allowed.",
            );
        }
        return day(
            Kind::Fast,
            "Dormition Fast",
            "No meat, dairy or fish; wine and oil at the weekend.",
        );
    }
    if (m == 11 && d >= 15) || (m == 12 && d <= 24) {
        let detail = if m == 12 && d >= 20 {
            "No meat, dairy or fish in the last days before Nativity."
        } else {
            "No meat or dairy; fish, wine and oil on many days."
        };
        return day(Kind::Fast, "Nativity Fast", detail);
    }
    if matches!(wd, Weekday::Wed | Weekday::Fri) {
        return day(
            Kind::Fast,
            if wd == Weekday::Wed {
                "Wednesday"
            } else {
                "Friday"
            },
            "No meat or dairy; often no fish, wine or oil.",
        );
    }
    None
}

/// Every fast or fast-free day in a month.
pub fn month(year: i32, month: u32, t: Tradition) -> Vec<(NaiveDate, FastDay)> {
    let mut out = vec![];
    let Some(mut d) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return out;
    };
    while d.month() == month {
        if let Some(f) = fast_day(d, t) {
            out.push((d, f));
        }
        d += Duration::days(1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn western_days() {
        // 2026: Easter April 5, Ash Wednesday February 18
        assert_eq!(
            fast_day(ymd(2026, 2, 18), Tradition::Catholic)
                .unwrap()
                .title,
            "Ash Wednesday"
        );
        assert_eq!(
            fast_day(ymd(2026, 4, 3), Tradition::Catholic)
                .unwrap()
                .title,
            "Good Friday"
        );
        assert_eq!(
            fast_day(ymd(2026, 3, 6), Tradition::Catholic).unwrap().kind,
            Kind::Abstinence
        );
        assert!(
            fast_day(ymd(2026, 4, 10), Tradition::Catholic).is_none(),
            "Friday in Easter week"
        );
        assert!(fast_day(ymd(2026, 3, 4), Tradition::Lutheran).is_some());
        assert!(fast_day(ymd(2026, 7, 1), Tradition::Reformed).is_none());
    }

    #[test]
    fn orthodox_days() {
        // 2026: Orthodox Pascha April 12
        assert_eq!(
            fast_day(ymd(2026, 4, 10), Tradition::Orthodox)
                .unwrap()
                .kind,
            Kind::Strict
        );
        assert_eq!(
            fast_day(ymd(2026, 4, 15), Tradition::Orthodox)
                .unwrap()
                .kind,
            Kind::FastFree
        );
        assert_eq!(
            fast_day(ymd(2026, 11, 18), Tradition::Orthodox)
                .unwrap()
                .title,
            "Nativity Fast"
        );
        assert_eq!(
            fast_day(ymd(2026, 8, 3), Tradition::Orthodox)
                .unwrap()
                .title,
            "Dormition Fast"
        );
        assert_eq!(
            fast_day(ymd(2026, 12, 30), Tradition::Orthodox)
                .unwrap()
                .kind,
            Kind::FastFree
        );
        // an ordinary Wednesday
        assert_eq!(
            fast_day(ymd(2026, 10, 7), Tradition::Orthodox)
                .unwrap()
                .title,
            "Wednesday"
        );
        assert!(fast_day(ymd(2026, 10, 8), Tradition::Orthodox).is_none());
        assert!(!month(2026, 3, Tradition::Orthodox).is_empty());
    }
}
