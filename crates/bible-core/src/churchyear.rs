//! The church year, computed offline: Easter and everything that moves with it,
//! fixed feasts, seasons and their liturgical colours.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tradition {
    Western,
    Lutheran,
    Reformed,
    Anglican,
    Catholic,
    Orthodox,
}

impl Tradition {
    pub fn all() -> &'static [(Tradition, &'static str)] {
        &[
            (Tradition::Western, "Western"),
            (Tradition::Lutheran, "Lutheran"),
            (Tradition::Reformed, "Reformed"),
            (Tradition::Anglican, "Anglican"),
            (Tradition::Catholic, "Catholic"),
            (Tradition::Orthodox, "Orthodox"),
        ]
    }
    pub fn from_name(s: &str) -> Tradition {
        Tradition::all()
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case(s))
            .map(|(t, _)| *t)
            .unwrap_or(Tradition::Western)
    }
    pub fn name(&self) -> &'static str {
        Tradition::all()
            .iter()
            .find(|(t, _)| t == self)
            .map(|(_, n)| *n)
            .unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    Purple,
    White,
    Green,
    Red,
    Black,
}

impl Colour {
    pub fn name(&self) -> &'static str {
        match self {
            Colour::Purple => "purple",
            Colour::White => "white",
            Colour::Green => "green",
            Colour::Red => "red",
            Colour::Black => "black",
        }
    }
    /// GNOME palette swatch for the colour
    pub fn hex(&self) -> &'static str {
        match self {
            Colour::Purple => "#9141ac",
            Colour::White => "#deddda",
            Colour::Green => "#2ec27e",
            Colour::Red => "#e01b24",
            Colour::Black => "#3d3846",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DayInfo {
    pub date: NaiveDate,
    /// e.g. "Ash Wednesday" or "25th week after Pentecost"
    pub title: String,
    /// e.g. "Lent", "Ordinary Time"
    pub season: String,
    pub colour: Colour,
    /// upcoming feasts within the next ~6 weeks: (name, date)
    pub upcoming: Vec<(String, NaiveDate)>,
}

/// Gregorian Easter (anonymous Gregorian algorithm).
pub fn easter_western(y: i32) -> NaiveDate {
    let a = y % 19;
    let b = y / 100;
    let c = y % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    NaiveDate::from_ymd_opt(y, month as u32, day as u32).unwrap()
}

/// Orthodox Easter: Julian computus, converted to the Gregorian calendar.
pub fn easter_orthodox(y: i32) -> NaiveDate {
    let a = y % 4;
    let b = y % 7;
    let c = y % 19;
    let d = (19 * c + 15) % 30;
    let e = (2 * a + 4 * b - d + 34) % 7;
    let month = (d + e + 114) / 31;
    let day = ((d + e + 114) % 31) + 1;
    let julian = NaiveDate::from_ymd_opt(y, month as u32, day as u32).unwrap();
    let diff = y / 100 - y / 400 - 2; // Julian–Gregorian offset (13 days in 1900–2099)
    julian + Duration::days(diff as i64)
}

fn ordinal(n: i64) -> String {
    let suf = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

/// First Sunday of Advent: the fourth Sunday before Christmas.
pub fn advent_start(y: i32) -> NaiveDate {
    let xmas = NaiveDate::from_ymd_opt(y, 12, 25).unwrap();
    let back = xmas.weekday().num_days_from_sunday() as i64;
    let sunday_before = if back == 0 {
        xmas - Duration::days(7)
    } else {
        xmas - Duration::days(back)
    };
    sunday_before - Duration::days(21)
}

fn feasts(y: i32, t: Tradition) -> Vec<(String, NaiveDate)> {
    let easter = if t == Tradition::Orthodox {
        easter_orthodox(y)
    } else {
        easter_western(y)
    };
    let d = |m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
    if t == Tradition::Orthodox {
        // the Great Feasts and the start of the fasts (fixed dates on the Revised Julian calendar)
        let mut v: Vec<(String, NaiveDate)> = vec![
            ("Theophany".to_string(), d(1, 6)),
            ("Meeting of the Lord".into(), d(2, 2)),
            ("Annunciation".into(), d(3, 25)),
            ("Clean Monday".into(), easter - Duration::days(48)),
            ("Palm Sunday".into(), easter - Duration::days(7)),
            ("Great and Holy Friday".into(), easter - Duration::days(2)),
            ("Pascha".into(), easter),
            ("Ascension".into(), easter + Duration::days(39)),
            ("Pentecost".into(), easter + Duration::days(49)),
            ("All Saints".into(), easter + Duration::days(56)),
            ("Transfiguration".into(), d(8, 6)),
            ("Dormition of the Theotokos".into(), d(8, 15)),
            ("Nativity of the Theotokos".into(), d(9, 8)),
            ("Exaltation of the Cross".into(), d(9, 14)),
            ("Entry of the Theotokos".into(), d(11, 21)),
            ("Nativity of Christ".into(), d(12, 25)),
        ];
        v.sort_by_key(|(_, d)| *d);
        return v;
    }
    let mut v = vec![
        ("Epiphany".to_string(), d(1, 6)),
        ("Ash Wednesday".into(), easter - Duration::days(46)),
        ("Palm Sunday".into(), easter - Duration::days(7)),
        ("Maundy Thursday".into(), easter - Duration::days(3)),
        ("Good Friday".into(), easter - Duration::days(2)),
        ("Easter Day".into(), easter),
        ("Ascension".into(), easter + Duration::days(39)),
        ("Pentecost".into(), easter + Duration::days(49)),
        ("Trinity Sunday".into(), easter + Duration::days(56)),
        ("All Saints’ Day".into(), d(11, 1)),
        (
            "Christ the King".into(),
            advent_start(y) - Duration::days(7),
        ),
        ("First Sunday of Advent".into(), advent_start(y)),
        ("Christmas Day".into(), d(12, 25)),
    ];
    if matches!(
        t,
        Tradition::Lutheran | Tradition::Reformed | Tradition::Western
    ) {
        v.push(("Reformation Day".into(), d(10, 31)));
    }
    v.sort_by_key(|(_, d)| *d);
    v
}

pub fn day_info(date: NaiveDate, t: Tradition) -> DayInfo {
    let y = date.year();
    let easter = if t == Tradition::Orthodox {
        easter_orthodox(y)
    } else {
        easter_western(y)
    };
    let ash = easter - Duration::days(46);
    let palm = easter - Duration::days(7);
    let pentecost = easter + Duration::days(49);
    let advent = advent_start(y);
    let epiphany = NaiveDate::from_ymd_opt(y, 1, 6).unwrap();

    let (mut title, season, mut colour);
    let orthodox = t == Tradition::Orthodox;
    let ash = if orthodox {
        easter - Duration::days(48)
    } else {
        ash
    };
    let nativity_fast = NaiveDate::from_ymd_opt(y, 11, 15).unwrap();
    if orthodox && date >= nativity_fast && date < NaiveDate::from_ymd_opt(y, 12, 25).unwrap() {
        season = "Nativity Fast".into();
        title = format!(
            "{} week of the Nativity Fast",
            ordinal((date - nativity_fast).num_days() / 7 + 1)
        );
        colour = Colour::Purple;
    } else if date >= NaiveDate::from_ymd_opt(y, 12, 25).unwrap() || date < epiphany {
        season = "Christmastide".to_string();
        title = "Christmas".to_string();
        colour = Colour::White;
    } else if date >= advent && !orthodox {
        let week = (date - advent).num_days() / 7 + 1;
        season = "Advent".into();
        title = format!("{} week of Advent", ordinal(week));
        colour = Colour::Purple;
    } else if date >= epiphany && date < ash {
        let week = (date - epiphany).num_days() / 7 + 1;
        season = if orthodox {
            "After Theophany".into()
        } else {
            "Epiphany".into()
        };
        title = format!(
            "{} week after {}",
            ordinal(week),
            if orthodox { "Theophany" } else { "Epiphany" }
        );
        colour = if date == epiphany {
            Colour::White
        } else {
            Colour::Green
        };
    } else if date >= ash && date < palm {
        season = if orthodox {
            "Great Lent".into()
        } else {
            "Lent".into()
        };
        let week = (date - ash).num_days() / 7;
        title = if orthodox {
            format!("{} week of Great Lent", ordinal(week + 1))
        } else if week == 0 {
            "Week of Ash Wednesday".into()
        } else {
            format!("{} week of Lent", ordinal(week))
        };
        colour = Colour::Purple;
    } else if date >= palm && date < easter {
        season = "Holy Week".into();
        title = "Holy Week".into();
        colour = Colour::Red;
    } else if date >= easter && date < pentecost {
        season = if orthodox {
            "Paschal season".into()
        } else {
            "Eastertide".into()
        };
        let week = (date - easter).num_days() / 7 + 1;
        title = format!(
            "{} week of {}",
            ordinal(week),
            if orthodox { "Pascha" } else { "Easter" }
        );
        colour = Colour::White;
    } else {
        season = if orthodox {
            "After Pentecost".into()
        } else {
            "Ordinary Time".into()
        };
        let week = (date - pentecost).num_days() / 7;
        title = if week == 0 {
            "Week of Pentecost".into()
        } else {
            format!("{} week after Pentecost", ordinal(week))
        };
        colour = Colour::Green;
    }
    // named days override the generic title
    for (name, d) in feasts(y, t) {
        if d == date {
            title = name.clone();
            colour = match name.as_str() {
                "Good Friday" | "Great and Holy Friday" => Colour::Black,
                "Pentecost" | "Palm Sunday" | "Reformation Day" => Colour::Red,
                "Ash Wednesday" => Colour::Purple,
                _ if season == "Ordinary Time" && name == "Trinity Sunday" => Colour::White,
                "Easter Day" | "Pascha" | "Christmas Day" | "Nativity of Christ" | "Theophany"
                | "Epiphany" | "Ascension" | "All Saints’ Day" | "Christ the King" => {
                    Colour::White
                }
                _ => colour,
            };
        }
    }
    let mut upcoming: Vec<(String, NaiveDate)> = feasts(y, t)
        .into_iter()
        .chain(feasts(y + 1, t))
        .filter(|(_, d)| *d > date && (*d - date).num_days() <= 45)
        .collect();
    upcoming.truncate(3);
    DayInfo {
        date,
        title,
        season,
        colour,
        upcoming,
    }
}

/// "Sunday, November 22" style label for upcoming feasts.
pub fn short_date(d: NaiveDate) -> String {
    let wd = match d.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };
    format!("{wd}, {}", d.format("%b %-d"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easter_dates() {
        assert_eq!(
            easter_western(2026),
            NaiveDate::from_ymd_opt(2026, 4, 5).unwrap()
        );
        assert_eq!(
            easter_western(2024),
            NaiveDate::from_ymd_opt(2024, 3, 31).unwrap()
        );
        assert_eq!(
            easter_orthodox(2024),
            NaiveDate::from_ymd_opt(2024, 5, 5).unwrap()
        );
        assert_eq!(
            easter_orthodox(2026),
            NaiveDate::from_ymd_opt(2026, 4, 12).unwrap()
        );
    }

    #[test]
    fn november_2026() {
        let d = day_info(
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            Tradition::Western,
        );
        assert_eq!(d.season, "Ordinary Time");
        assert_eq!(d.title, "25th week after Pentecost");
        assert_eq!(
            advent_start(2026),
            NaiveDate::from_ymd_opt(2026, 11, 29).unwrap()
        );
        assert_eq!(d.upcoming[0].0, "Christ the King");
    }

    #[test]
    fn ash_wednesday_2026() {
        let d = day_info(
            NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            Tradition::Western,
        );
        assert_eq!(d.title, "Ash Wednesday");
        assert_eq!(d.colour, Colour::Purple);
    }
}
