//! Turn module markup (OSIS, GBF, ThML) into a flat list of styled spans
//! that the interface can render however it likes.

use bible_core::refs::{canonical_book, find_refs, parse, parse_osis, Ref};

/// A footnote or cross-reference attached to the text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Note {
    pub xref: bool,
    pub text: String,
    pub refs: Vec<Ref>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Span {
    /// set on marker spans that stand for a footnote or cross-reference
    pub note: Option<Note>,
    pub text: String,
    pub red: bool,
    pub italic: bool,
    pub bold: bool,
    pub title: bool,
    pub small_caps: bool,
    pub strong: Option<String>,
    pub link: Option<Ref>,
    /// paragraph / line break before this span
    pub br: bool,
}

#[derive(Default, Clone)]
struct State {
    /// inside <reference type="annotateRef"> (the "3:16" label of a footnote)
    annotate: bool,
    red: bool,
    italic: u32,
    bold: u32,
    title: u32,
    small_caps: u32,
    skip: u32,
    strong: Option<String>,
    link: Option<Ref>,
    br: bool,
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(';').filter(|e| *e < 10);
        match end {
            Some(e) => {
                let ent = &rest[1..e];
                let ch = match ent {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some('\u{a0}'),
                    _ if ent.starts_with("#x") => u32::from_str_radix(&ent[2..], 16)
                        .ok()
                        .and_then(char::from_u32),
                    _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
                    _ => None,
                };
                match ch {
                    Some(c) => {
                        out.push(c);
                        rest = &rest[e + 1..];
                    }
                    None => {
                        out.push('&');
                        rest = &rest[1..];
                    }
                }
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=");
    let mut search = tag;
    loop {
        let i = search.find(&key)?;
        let before_ok = i == 0 || search.as_bytes()[i - 1].is_ascii_whitespace();
        let after = &search[i + key.len()..];
        if before_ok {
            let q = after.chars().next()?;
            if q == '"' || q == '\'' {
                let end = after[1..].find(q)?;
                return Some(&after[1..1 + end]);
            }
            let end = after
                .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                .unwrap_or(after.len());
            return Some(&after[..end]);
        }
        search = after;
    }
}

fn strong_from_lemma(lemma: &str) -> Option<String> {
    lemma.split_whitespace().find_map(|l| {
        let v = l.strip_prefix("strong:").unwrap_or(l);
        let v = v.trim_start_matches("x-Strongs:");
        let c = v.chars().next()?;
        if (c == 'G' || c == 'H')
            && v[1..]
                .chars()
                .next()
                .map(|d| d.is_ascii_digit())
                .unwrap_or(false)
        {
            Some(
                v.trim_end_matches(|c: char| !c.is_alphanumeric())
                    .to_string(),
            )
        } else {
            None
        }
    })
}

struct Builder {
    spans: Vec<Span>,
    st: State,
    stack: Vec<(String, State)>,
    /// note being collected: (is cross-reference, text, refs)
    note: Option<(bool, String, Vec<Ref>)>,
    note_depth: u32,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            spans: Vec::new(),
            st: State::default(),
            stack: Vec::new(),
            note: None,
            note_depth: 0,
        }
    }
    fn begin_note(&mut self, xref: bool) {
        self.note_depth += 1;
        if self.note_depth == 1 {
            self.note = Some((xref, String::new(), Vec::new()));
        }
    }
    fn note_ref(&mut self, r: Option<Ref>) {
        if let (Some(n), Some(r)) = (self.note.as_mut(), r) {
            n.2.push(r);
        }
    }
    fn end_note(&mut self) {
        self.note_depth = self.note_depth.saturating_sub(1);
        if self.note_depth > 0 {
            return;
        }
        if let Some((xref, text, mut refs)) = self.note.take() {
            let text = collapse_ws(text.trim());
            for f in find_refs(&text) {
                if !refs.contains(&f.r) {
                    refs.push(f.r);
                }
            }
            if text.is_empty() && refs.is_empty() {
                return;
            }
            let xref = xref
                || (!refs.is_empty()
                    && text.chars().filter(|c| c.is_alphabetic()).count() < 40
                    && text.split_whitespace().count() <= refs.len() * 3);
            self.spans.push(Span {
                text: if xref { "†".into() } else { "*".into() },
                note: Some(Note { xref, text, refs }),
                ..Default::default()
            });
        }
    }
    fn text(&mut self, raw: &str) {
        if let Some(n) = self.note.as_mut() {
            // inside a note: the text belongs to the note, not the verse
            if !self.st.annotate {
                n.1.push_str(&decode_entities(raw));
            }
            return;
        }
        if self.st.skip > 0 || raw.is_empty() {
            return;
        }
        let t = decode_entities(raw).replace('¶', "");
        let span = Span {
            note: None,
            text: t,
            red: self.st.red,
            italic: self.st.italic > 0,
            bold: self.st.bold > 0,
            title: self.st.title > 0,
            small_caps: self.st.small_caps > 0,
            strong: self.st.strong.clone(),
            link: self.st.link.clone(),
            br: std::mem::take(&mut self.st.br),
        };
        // merge with previous span when the styling is identical
        if let Some(last) = self.spans.last_mut() {
            if !span.br
                && last.note.is_none()
                && last.red == span.red
                && last.italic == span.italic
                && last.bold == span.bold
                && last.title == span.title
                && last.small_caps == span.small_caps
                && last.strong == span.strong
                && last.link == span.link
            {
                last.text.push_str(&span.text);
                return;
            }
        }
        self.spans.push(span);
    }
    fn push(&mut self, name: &str) {
        self.stack.push((name.to_string(), self.st.clone()));
    }
    fn pop(&mut self, name: &str) {
        if let Some(pos) = self.stack.iter().rposition(|(n, _)| n == name) {
            let (_, saved) = self.stack.remove(pos);
            let br = self.st.br;
            let red = self.st.red;
            self.st = saved;
            self.st.br = br;
            // milestone-style red-letter quotes are not restored by element nesting
            if name != "q" {
                self.st.red = red && self.st.red;
            }
        }
    }
    fn finish(mut self) -> Vec<Span> {
        for s in &mut self.spans {
            s.text = collapse_ws(&s.text);
        }
        // drop leading whitespace-only spans, then trim the first real one
        while self
            .spans
            .first()
            .map(|f| f.text.trim().is_empty())
            .unwrap_or(false)
        {
            self.spans.remove(0);
        }
        if let Some(f) = self.spans.first_mut() {
            f.text = f.text.trim_start().to_string();
            f.br = false;
        }
        self.spans.retain(|s| !s.text.is_empty());
        self.spans
    }
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut sp = false;
    for c in s.chars() {
        if c.is_whitespace() && c != '\u{a0}' {
            if !sp {
                out.push(' ');
            }
            sp = true;
        } else {
            out.push(c);
            sp = false;
        }
    }
    out
}

/// Split raw markup into text and tags: Some(tag body) for tags, None-wrapped text otherwise.
fn tokens(src: &str) -> Vec<(bool, &str)> {
    let mut out = Vec::new();
    let mut rest = src;
    while !rest.is_empty() {
        match rest.find('<') {
            Some(0) => match rest.find('>') {
                Some(e) => {
                    out.push((true, &rest[1..e]));
                    rest = &rest[e + 1..];
                }
                None => {
                    out.push((false, rest));
                    break;
                }
            },
            Some(i) => {
                out.push((false, &rest[..i]));
                rest = &rest[i..];
            }
            None => {
                out.push((false, rest));
                break;
            }
        }
    }
    out
}

fn tag_name(t: &str) -> (String, bool, bool) {
    let closing = t.starts_with('/');
    let body = t.trim_start_matches('/');
    let name: String = body
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '/')
        .collect();
    let self_closing = t.ends_with('/');
    (name, closing, self_closing)
}

pub fn osis(src: &str) -> Vec<Span> {
    let mut b = Builder::new();
    for (is_tag, tok) in tokens(src) {
        if !is_tag {
            b.text(tok);
            continue;
        }
        let (name, closing, selfc) = tag_name(tok);
        match (name.as_str(), closing) {
            ("note", false) if !selfc => {
                let t = attr(tok, "type").unwrap_or("");
                b.begin_note(t == "crossReference" || t == "x-cross-ref");
            }
            ("note", true) => b.end_note(),
            ("title", false) if !selfc => {
                b.push("title");
                b.st.title += 1;
                b.st.br = true;
            }
            ("title", true) => {
                b.pop("title");
                b.st.br = true;
            }
            ("w", false) if !selfc => {
                b.push("w");
                b.st.strong = attr(tok, "lemma").and_then(strong_from_lemma);
            }
            ("w", true) => b.pop("w"),
            ("q", false) => {
                let jesus = attr(tok, "who")
                    .map(|w| w.eq_ignore_ascii_case("jesus"))
                    .unwrap_or(false);
                if attr(tok, "eID").is_some() {
                    b.st.red = false;
                } else if jesus {
                    if !selfc {
                        b.push("q");
                    }
                    b.st.red = true;
                } else if !selfc {
                    b.push("q");
                }
            }
            ("q", true) => b.pop("q"),
            ("transChange", false) if !selfc => {
                b.push("transChange");
                if attr(tok, "type").map(|t| t == "added").unwrap_or(true) {
                    b.st.italic += 1;
                }
            }
            ("transChange", true) => b.pop("transChange"),
            ("divineName", false) if !selfc => {
                b.push("divineName");
                b.st.small_caps += 1;
            }
            ("divineName", true) => b.pop("divineName"),
            ("hi", false) if !selfc => {
                b.push("hi");
                match attr(tok, "type") {
                    Some("bold") => b.st.bold += 1,
                    Some("small-caps") => b.st.small_caps += 1,
                    _ => b.st.italic += 1,
                }
            }
            ("hi", true) => b.pop("hi"),
            ("reference", false) if !selfc => {
                b.push("reference");
                let r = attr(tok, "osisRef")
                    .and_then(|r| parse_osis(r.split(':').next_back().unwrap_or(r)));
                if b.note.is_some() {
                    if attr(tok, "type") == Some("annotateRef") {
                        b.st.annotate = true;
                    } else {
                        b.note_ref(r);
                    }
                } else {
                    b.st.link = r;
                }
            }
            ("reference", true) => b.pop("reference"),
            ("reference", false) => {
                let r = attr(tok, "osisRef")
                    .and_then(|r| parse_osis(r.split(':').next_back().unwrap_or(r)));
                b.note_ref(r);
            }
            ("lb", _) | ("p", _) | ("l", true) | ("lg", _) | ("div", _) | ("br", _) => {
                b.st.br = true
            }
            ("milestone", _) => {
                if attr(tok, "type")
                    .map(|t| t == "x-p" || t == "line")
                    .unwrap_or(false)
                {
                    b.st.br = true;
                }
            }
            ("catchWord", _) | ("rdg", _) => {}
            _ => {}
        }
    }
    b.finish()
}

pub fn gbf(src: &str) -> Vec<Span> {
    let mut b = Builder::new();
    for (is_tag, tok) in tokens(src) {
        if !is_tag {
            b.text(tok);
            continue;
        }
        let t = tok.trim();
        match t {
            "FR" => b.st.red = true,
            "Fr" => b.st.red = false,
            "FI" => b.st.italic += 1,
            "Fi" => b.st.italic = b.st.italic.saturating_sub(1),
            "FB" => b.st.bold += 1,
            "Fb" => b.st.bold = b.st.bold.saturating_sub(1),
            "FS" | "FU" => {}
            "TS" => {
                b.st.title += 1;
                b.st.br = true;
            }
            "Ts" => {
                b.st.title = b.st.title.saturating_sub(1);
                b.st.br = true;
            }
            "RF" => b.begin_note(false),
            "RX" => b.begin_note(true),
            "Rf" | "Rx" => b.end_note(),
            "CM" | "CL" | "CG" => b.st.br = true,
            _ if t.starts_with("WG") || t.starts_with("WH") => {
                // Strong's number follows the word it belongs to
                let num = format!("{}{}", &t[1..2], &t[2..]);
                if let Some(last) = b.spans.last_mut() {
                    if last.strong.is_none() {
                        last.strong = Some(num);
                    }
                }
            }
            _ => {}
        }
    }
    b.finish()
}

pub fn thml(src: &str) -> Vec<Span> {
    let mut b = Builder::new();
    for (is_tag, tok) in tokens(src) {
        if !is_tag {
            b.text(tok);
            continue;
        }
        let (name, closing, selfc) = tag_name(tok);
        match (name.to_ascii_lowercase().as_str(), closing) {
            ("note", false) if !selfc => {
                let t = attr(tok, "type").unwrap_or("");
                b.begin_note(t == "crossReference" || t == "x-cross-ref");
            }
            ("note", true) => b.end_note(),
            ("scripref", false) if !selfc => {
                b.push("scripRef");
                let r = attr(tok, "passage").and_then(|p| parse(p.split(';').next().unwrap_or(p)));
                if b.note.is_some() {
                    b.note_ref(r);
                } else {
                    b.st.link = r;
                }
            }
            ("scripref", true) => b.pop("scripRef"),
            ("font", false) if !selfc => {
                b.push("font");
                if attr(tok, "color")
                    .map(|c| c.eq_ignore_ascii_case("red"))
                    .unwrap_or(false)
                {
                    b.st.red = true;
                }
            }
            ("font", true) => b.pop("font"),
            ("i", false) | ("em", false) => {
                b.push("i");
                b.st.italic += 1;
            }
            ("i", true) | ("em", true) => b.pop("i"),
            ("b", false) | ("strong", false) => {
                b.push("b");
                b.st.bold += 1;
            }
            ("b", true) | ("strong", true) => b.pop("b"),
            ("sync", _) => {
                if attr(tok, "type") == Some("Strongs") {
                    if let (Some(v), Some(last)) = (attr(tok, "value"), b.spans.last_mut()) {
                        if last.strong.is_none() {
                            last.strong = Some(v.to_string());
                        }
                    }
                }
            }
            ("br", _) | ("p", _) | ("div", _) | ("h1", _) | ("h2", _) | ("h3", _) | ("h4", _) => {
                b.st.br = true
            }
            _ => {}
        }
    }
    b.finish()
}

/// Render according to the module's SourceType.
pub fn render(source_type: &str, src: &str) -> Vec<Span> {
    let spans = match source_type.to_ascii_uppercase().as_str() {
        "OSIS" => osis(src),
        "GBF" => gbf(src),
        "THML" => thml(src),
        _ if src.contains("<w ") || src.contains("osisRef") => osis(src),
        _ => thml(src),
    };
    add_text_links(spans)
}

/// Find plain-text references ("compare Rom 5:8") and turn them into links.
pub fn add_text_links(spans: Vec<Span>) -> Vec<Span> {
    let mut out = Vec::with_capacity(spans.len());
    for s in spans {
        if s.link.is_some() || s.title || s.note.is_some() {
            out.push(s);
            continue;
        }
        let found = find_refs(&s.text);
        if found.is_empty() {
            out.push(s);
            continue;
        }
        let mut last = 0;
        let mut first = true;
        for f in found {
            if f.start > last {
                out.push(Span {
                    text: s.text[last..f.start].to_string(),
                    br: s.br && first,
                    link: None,
                    ..s.clone()
                });
                first = false;
            }
            out.push(Span {
                text: s.text[f.start..f.end].to_string(),
                link: Some(f.r),
                br: s.br && first,
                ..s.clone()
            });
            first = false;
            last = f.end;
        }
        if last < s.text.len() {
            out.push(Span {
                text: s.text[last..].to_string(),
                br: false,
                link: None,
                ..s.clone()
            });
        }
    }
    out
}

/// Plain text without markup, for search and copying.
pub fn plain(source_type: &str, src: &str) -> String {
    let spans = match source_type.to_ascii_uppercase().as_str() {
        "GBF" => gbf(src),
        "THML" => thml(src),
        _ => osis(src),
    };
    let mut out = String::new();
    let mut after_note = false;
    for s in spans.iter().filter(|s| !s.title) {
        if s.note.is_some() {
            after_note = true;
            continue;
        }
        // a footnote between two words stood in for the space
        if after_note
            && !out.ends_with(char::is_whitespace)
            && !s
                .text
                .starts_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            && !out.is_empty()
        {
            out.push(' ');
        }
        after_note = false;
        out.push_str(&s.text);
    }
    out.trim().to_string()
}

/// Is `name` a recognised book? (re-exported for callers that only have this crate)
pub fn is_book(name: &str) -> bool {
    canonical_book(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osis_words_of_christ_and_strongs() {
        let s = osis(
            r#"<w lemma="strong:G3004">Jesus said</w>, <q who="Jesus" marker=""><w lemma="strong:G1473">I</w> am</q><note type="x-study">skip me</note> the <transChange type="added">way</transChange>."#,
        );
        let text: String = s
            .iter()
            .filter(|x| x.note.is_none())
            .map(|x| x.text.as_str())
            .collect();
        assert_eq!(text, "Jesus said, I am the way.");
        assert_eq!(s.iter().filter(|x| x.note.is_some()).count(), 1);
        assert!(s.iter().any(|x| x.red && x.text == "I"));
        assert_eq!(s[0].strong.as_deref(), Some("G3004"));
        assert!(s.iter().any(|x| x.italic && x.text == "way"));
    }

    #[test]
    fn footnotes_and_cross_references() {
        let s = osis(
            r#"only born<note placement="foot"><reference type="annotateRef">3:16 </reference>The phrase “only born” is from the Greek.</note> Son<note type="crossReference"><reference osisRef="Rom.5.8">Rom 5:8</reference>; <reference osisRef="1John.4.9">1 Jn 4:9</reference></note>."#,
        );
        let notes: Vec<&Note> = s.iter().filter_map(|x| x.note.as_ref()).collect();
        assert_eq!(notes.len(), 2);
        assert!(!notes[0].xref);
        assert!(
            notes[0].text.starts_with("The phrase"),
            "{:?}",
            notes[0].text
        );
        assert!(notes[1].xref);
        assert_eq!(notes[1].refs.len(), 2);
        assert_eq!(
            plain("OSIS", r#"only born<note placement="foot">x</note> Son"#),
            "only born Son"
        );
    }

    #[test]
    fn gbf_red_and_italic() {
        let s = gbf("Jesus said <FR>Follow me<Fr> <FI>now<Fi><RF>note<Rf>.");
        assert!(s.iter().any(|x| x.red && x.text.contains("Follow me")));
        assert!(s.iter().any(|x| x.italic && x.text == "now"));
        assert!(!s
            .iter()
            .any(|x| x.note.is_none() && x.text.contains("note")));
    }

    #[test]
    fn plain_refs_become_links() {
        let s = render(
            "OSIS",
            "See Rom 5:8 and <reference osisRef=\"John.1.14\">the Word</reference>.",
        );
        let links: Vec<String> = s
            .iter()
            .filter_map(|x| x.link.as_ref().map(|r| r.to_string()))
            .collect();
        assert_eq!(links, vec!["Romans 5:8", "John 1:14"]);
    }
}
