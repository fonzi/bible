//! Turning module spans into Pango markup for labels.

use bible_sword::Span;
use gtk::glib;

pub struct Style<'a> {
    pub red_letters: bool,
    pub highlight: Option<&'a str>, // hex colour for the whole verse
    pub dark: bool,
    /// part-verse highlights: (from, to, hex) in plain-text character offsets
    pub ranges: &'a [(usize, usize, String)],
    /// module name, used in note links ("note:<module>:<index>")
    pub module: &'a str,
}

fn esc(s: &str) -> String {
    glib::markup_escape_text(s).to_string()
}

fn bg(hex: &str, dark: bool) -> String {
    format!(
        "<span background=\"{hex}\" bgalpha=\"{}%\">",
        if dark { 40 } else { 32 }
    )
}

/// Markup for body text, the titles (shown separately as headings), and the plain text
/// the markup displays (offsets in `Style::ranges` refer to it).
pub fn spans_to_markup(spans: &[Span], st: &Style) -> (String, Vec<String>, String) {
    let mut body = String::new();
    let mut plain = String::new();
    let mut titles = Vec::new();
    let red = if st.dark { "#ff7b63" } else { "#c01c28" };
    let body_spans: Vec<&Span> = spans.iter().filter(|s| !s.title).collect();
    let mut note_i = 0;
    let mut after_note = false;
    for s in spans.iter().filter(|s| s.title) {
        let t = s.text.trim();
        if !t.is_empty() {
            titles.push(t.to_string());
        }
    }
    let last = body_spans.len().saturating_sub(1);
    for (i, s) in body_spans.iter().enumerate() {
        if s.note.is_some() {
            // footnote / cross-reference marker: a small superscript link, not part of the plain text
            // thin spaces around the mark give it a bigger click target
            body.push_str(&format!("<span rise=\"4pt\" weight=\"bold\"><a href=\"note:{}:{note_i}\">\u{2009}{}\u{2009}</a></span>", st.module, esc(&s.text)));
            note_i += 1;
            after_note = true;
            continue;
        }
        let mut text: &str = &s.text;
        if plain.is_empty() {
            text = text.trim_start();
        }
        if i == last {
            text = text.trim_end();
        }
        if text.is_empty() {
            continue;
        }
        let joins_word = after_note
            && plain
                .chars()
                .last()
                .map(|c| c.is_alphanumeric())
                .unwrap_or(false)
            && text
                .chars()
                .next()
                .map(|c| c.is_alphanumeric())
                .unwrap_or(false);
        if (s.br || joins_word)
            && !plain.is_empty()
            && !plain.ends_with(' ')
            && !text.starts_with(' ')
        {
            plain.push(' ');
            body.push(' ');
        }
        after_note = false;
        // split this span at highlight boundaries
        let start = plain.chars().count();
        let chars: Vec<char> = text.chars().collect();
        let mut inner = String::new();
        let mut k = 0;
        while k < chars.len() {
            let pos = start + k;
            let cur = st.ranges.iter().find(|(f, t, _)| *f <= pos && pos < *t);
            // run until the highlight state changes
            let mut j = k + 1;
            while j < chars.len() {
                let p2 = start + j;
                let c2 = st.ranges.iter().find(|(f, t, _)| *f <= p2 && p2 < *t);
                if c2.map(|x| (x.0, x.1)) != cur.map(|x| (x.0, x.1)) {
                    break;
                }
                j += 1;
            }
            let piece: String = chars[k..j].iter().collect();
            match cur {
                Some((_, _, hex)) => {
                    inner.push_str(&format!("{}{}</span>", bg(hex, st.dark), esc(&piece)))
                }
                None => inner.push_str(&esc(&piece)),
            }
            k = j;
        }
        plain.push_str(text);
        let mut t = inner;
        if s.small_caps {
            t = format!("<span variant=\"smallcaps\">{t}</span>");
        }
        if s.italic {
            t = format!("<i>{t}</i>");
        }
        if s.bold {
            t = format!("<b>{t}</b>");
        }
        if s.red && st.red_letters {
            t = format!("<span foreground=\"{red}\">{t}</span>");
        }
        if let Some(r) = &s.link {
            t = format!("<a href=\"{}\">{t}</a>", esc(&r.uri()));
        }
        body.push_str(&t);
    }
    let body = match st.highlight {
        Some(hex) => format!("{}{body}</span>", bg(hex, st.dark)),
        None => body,
    };
    (body, titles, plain)
}

/// Commentary and other long text: keeps paragraph breaks.
pub fn long_markup(spans: &[Span], dark: bool) -> String {
    let mut out = String::new();
    let red = if dark { "#ff7b63" } else { "#c01c28" };
    for s in spans.iter().filter(|s| s.note.is_none()) {
        if s.br && !out.is_empty() {
            out.push_str(if s.title { "\n\n" } else { "\n" });
        }
        let mut t = esc(&s.text);
        if s.title {
            t = format!("<b>{t}</b>");
        }
        if s.italic {
            t = format!("<i>{t}</i>");
        }
        if s.bold {
            t = format!("<b>{t}</b>");
        }
        if s.red {
            t = format!("<span foreground=\"{red}\">{t}</span>");
        }
        if let Some(r) = &s.link {
            t = format!("<a href=\"{}\">{t}</a>", esc(&r.uri()));
        }
        out.push_str(&t);
    }
    // collapse runs of blank lines
    let mut cleaned = String::new();
    let mut blank = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank <= 1 {
                cleaned.push('\n');
            }
        } else {
            blank = 0;
            cleaned.push_str(line.trim());
            cleaned.push('\n');
        }
    }
    cleaned.trim().to_string()
}

/// Would this label markup parse? Pango itself doesn't know GTK's <a href> links,
/// so swap them for spans before checking.
pub fn markup_ok(m: &str) -> bool {
    let mut t = String::with_capacity(m.len());
    let mut rest = m;
    while let Some(i) = rest.find("<a ") {
        t.push_str(&rest[..i]);
        match rest[i..].find('>') {
            Some(e) => {
                t.push_str("<span>");
                rest = &rest[i + e + 1..];
            }
            None => {
                rest = &rest[i..];
                break;
            }
        }
    }
    t.push_str(rest);
    let t = t.replace("</a>", "</span>");
    gtk::pango::parse_markup(&t, '\0').is_ok()
}

/// Classic typography: take the first letter out of a verse's markup so it can
/// be drawn as a decorated initial. Returns the letter and the rest.
pub fn split_initial(markup: &str) -> Option<(char, String)> {
    let mut out = String::with_capacity(markup.len());
    let (mut in_tag, mut in_entity) = (false, false);
    let mut found = None;
    for ch in markup.chars() {
        if found.is_some() {
            out.push(ch);
            continue;
        }
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            '&' if !in_tag => in_entity = true,
            ';' if in_entity => in_entity = false,
            c if !in_tag && !in_entity && c.is_alphanumeric() => {
                found = Some(c);
                continue;
            }
            _ => {}
        }
        out.push(ch);
    }
    found.map(|c| (c, out))
}

pub fn verse_number(n: u16) -> String {
    format!("<span size=\"small\" weight=\"bold\" alpha=\"55%\" rise=\"3pt\">{n}</span>  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_initial() {
        let (c, rest) =
            split_initial("<span foreground=\"red\">&#8220;In the beginning</span>").unwrap();
        assert_eq!(c, 'I');
        assert_eq!(
            rest,
            "<span foreground=\"red\">&#8220;n the beginning</span>"
        );
    }

    #[test]
    fn part_verse_highlight() {
        let spans = vec![
            Span {
                text: "For God so ".into(),
                ..Default::default()
            },
            Span {
                text: "loved the world".into(),
                italic: true,
                ..Default::default()
            },
        ];
        let ranges = vec![(8, 20, "#3584e4".to_string())];
        let (m, _, plain) = spans_to_markup(
            &spans,
            &Style {
                red_letters: true,
                highlight: None,
                dark: false,
                ranges: &ranges,
                module: "T",
            },
        );
        assert_eq!(
            &plain.chars().skip(8).take(12).collect::<String>(),
            "so loved the"
        );
        assert!(m.starts_with("For God <span background=\"#3584e4\" bgalpha=\"32%\">so </span><i><span background"), "{m}");
        assert!(markup_ok(&m));
    }

    #[test]
    fn step_definitions_are_valid_markup() {
        // set STEP_DATA to a folder of STEP Bible files to run this test
        let Ok(src) = std::env::var("STEP_DATA") else { return };
        let src = std::path::Path::new(&src);
        if !src.join("Lexicons").exists() {
            return;
        }
        for pack in bible_step::Pack::all() {
            let dir = std::env::temp_dir().join(format!(
                "step-markup-{}-{}",
                pack.id(),
                std::process::id()
            ));
            let mut open = |p: &str| -> std::io::Result<Box<dyn std::io::Read + Send>> {
                Ok(Box::new(std::fs::File::open(src.join(p))?))
            };
            bible_step::build_pack(&dir, pack, &mut open, &mut |_, _| {}).unwrap();
            let il = bible_step::Interlinear::load_from(&dir, pack).unwrap();
            let bad: Vec<String> = il
                .lexicon_entries()
                .filter(|e| !markup_ok(&e.definition))
                .map(|e| e.strong.clone())
                .collect();
            let total = il.lexicon_entries().count();
            assert!(
                bad.len() * 1000 < total.max(1),
                "{} of {total} invalid, e.g. {:?}",
                bad.len(),
                &bad[..bad.len().min(5)]
            );
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn markup_escapes_and_styles() {
        let spans = vec![
            Span {
                text: "Title".into(),
                title: true,
                ..Default::default()
            },
            Span {
                text: "A & B ".into(),
                ..Default::default()
            },
            Span {
                text: "I am".into(),
                red: true,
                ..Default::default()
            },
        ];
        let (m, t, plain) = spans_to_markup(
            &spans,
            &Style {
                red_letters: true,
                highlight: Some("#f5c211"),
                dark: false,
                ranges: &[],
                module: "T",
            },
        );
        assert_eq!(plain, "A & B I am");
        assert_eq!(t, vec!["Title"]);
        assert!(m.contains("A &amp; B"));
        assert!(m.contains("foreground=\"#c01c28\""));
        assert!(m.starts_with("<span background=\"#f5c211\""));
    }
}
