//! Notes and highlights, stored as plain Markdown files the user owns:
//!
//! ```text
//! <root>/Bible/43 John/John 03.md
//! ---
//! book: John
//! chapter: 3
//! highlights:
//!   - {verse: 16, color: promises}
//! ---
//!
//! ## 3:16
//! The gift precedes the response — compare [Eph 2:8](bible://Eph.2.8).
//! ```

use crate::refs::{find_refs, Ref};
use crate::v11n;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A highlight on part of a verse: character offsets into the verse's plain text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RangeHighlight {
    pub verse: u16,
    pub from: usize,
    pub to: usize,
    pub color: String,
    /// the highlighted words, kept so the Markdown file reads well in Obsidian
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChapterNotes {
    /// whole-verse highlights
    pub highlights: BTreeMap<u16, String>,
    /// highlights on part of a verse
    pub ranges: Vec<RangeHighlight>,
    pub notes: BTreeMap<u16, String>,
    /// Text before the first "## c:v" heading that isn't ours: kept verbatim.
    pub preamble: String,
}

impl ChapterNotes {
    pub fn is_empty(&self) -> bool {
        self.highlights.is_empty()
            && self.ranges.is_empty()
            && self.notes.is_empty()
            && self.preamble.trim().is_empty()
    }

    pub fn ranges_for(&self, verse: u16) -> impl Iterator<Item = &RangeHighlight> {
        self.ranges.iter().filter(move |r| r.verse == verse)
    }

    /// Add a part-verse highlight, replacing any overlap in the same verse.
    pub fn add_range(&mut self, r: RangeHighlight) {
        self.remove_range(r.verse, r.from, r.to);
        self.ranges.push(r);
        self.ranges.sort_by_key(|x| (x.verse, x.from));
    }

    /// Remove part-verse highlights overlapping [from, to).
    pub fn remove_range(&mut self, verse: u16, from: usize, to: usize) {
        self.ranges
            .retain(|x| !(x.verse == verse && x.from < to && from < x.to));
    }
}

#[derive(Debug, Clone)]
pub struct NotesStore {
    pub root: PathBuf,
    pub obsidian_links: bool,
}

impl NotesStore {
    pub fn new(root: impl Into<PathBuf>) -> NotesStore {
        NotesStore {
            root: root.into(),
            obsidian_links: true,
        }
    }

    /// Bible/43 John/John 03.md, numbered in canonical order.
    pub fn chapter_path(&self, book: &str, chapter: u16) -> PathBuf {
        let sys = v11n::system("KJVA");
        let kjv = v11n::kjv();
        let (num, name) = match kjv.book_index(book) {
            Some(i) => (i + 1, kjv.books[i].name.clone()),
            None => (
                sys.book_index(book).map(|i| i + 1).unwrap_or(99),
                Ref::new(book, 1, 0).book_name(),
            ),
        };
        let width = if kjv.book(book).map(|b| b.chapters()).unwrap_or(0) >= 100 {
            3
        } else {
            2
        };
        self.root
            .join("Bible")
            .join(format!("{num:02} {name}"))
            .join(format!("{name} {:0width$}.md", chapter, width = width))
    }

    pub fn load(&self, book: &str, chapter: u16) -> ChapterNotes {
        match fs::read_to_string(self.chapter_path(book, chapter)) {
            Ok(s) => parse(&s, chapter),
            Err(_) => ChapterNotes::default(),
        }
    }

    pub fn save(&self, book: &str, chapter: u16, n: &ChapterNotes) -> io::Result<PathBuf> {
        let path = self.chapter_path(book, chapter);
        if n.is_empty() {
            let _ = fs::remove_file(&path);
            return Ok(path);
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("md.tmp");
        fs::write(&tmp, render(book, chapter, n, self.obsidian_links))?;
        fs::rename(&tmp, &path)?;
        Ok(path)
    }

    /// Every Markdown file under the notes folder, newest first.
    pub fn all_files(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
        out
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if !p
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                walk(&p, out);
            }
        } else if p.extension().map(|x| x == "md").unwrap_or(false) {
            out.push(p);
        }
    }
}

/// Turn plain references in a note into links, so Obsidian and this app can follow them.
pub fn linkify(text: &str) -> String {
    let mut out = String::new();
    let mut last = 0;
    for f in find_refs(text) {
        // skip refs already inside a [label](link)
        let before = &text[..f.start];
        if before.ends_with('[')
            || before
                .rfind("](")
                .map(|i| !before[i..].contains(')'))
                .unwrap_or(false)
        {
            continue;
        }
        out.push_str(&text[last..f.start]);
        out.push_str(&format!("[{}]({})", &text[f.start..f.end], f.r.uri()));
        last = f.end;
    }
    out.push_str(&text[last..]);
    out
}

/// Show stored notes without the link syntax: "[Eph 2:8](bible://Eph.2.8)" -> "Eph 2:8".
pub fn unlink(text: &str) -> String {
    let re = regex::Regex::new(r"\[([^\]]+)\]\(bible://[^)]+\)").unwrap();
    re.replace_all(text, "$1").into_owned()
}

fn render(book: &str, chapter: u16, n: &ChapterNotes, links: bool) -> String {
    let mut s = String::from("---\n");
    s.push_str(&format!("book: {book}\nchapter: {chapter}\n"));
    if !n.highlights.is_empty() || !n.ranges.is_empty() {
        s.push_str("highlights:\n");
        for (v, c) in &n.highlights {
            s.push_str(&format!("  - {{verse: {v}, color: {c}}}\n"));
        }
        for r in &n.ranges {
            let t = r.text.replace('"', "'");
            s.push_str(&format!(
                "  - {{verse: {}, color: {}, from: {}, to: {}, text: \"{}\"}}\n",
                r.verse, r.color, r.from, r.to, t
            ));
        }
    }
    s.push_str("---\n");
    if !n.preamble.trim().is_empty() {
        s.push('\n');
        s.push_str(n.preamble.trim_end());
        s.push('\n');
    }
    for (v, text) in &n.notes {
        if text.trim().is_empty() {
            continue;
        }
        let body = if links {
            linkify(text.trim())
        } else {
            text.trim().to_string()
        };
        s.push_str(&format!("\n## {chapter}:{v}\n{body}\n"));
    }
    s
}

fn parse(src: &str, chapter: u16) -> ChapterNotes {
    let mut n = ChapterNotes::default();
    let mut body = src;
    if let Some(rest) = src.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let fm = &rest[..end];
            body = rest[end + 4..].trim_start_matches('\n');
            for line in fm.lines() {
                let l = line.trim();
                if let Some(inner) = l.strip_prefix("- {").and_then(|x| x.strip_suffix('}')) {
                    // text: "..." may contain commas, so take it out first
                    let (fields, text) = match inner.find("text: \"") {
                        Some(i) => (
                            &inner[..i],
                            inner[i + 7..].trim_end_matches('"').to_string(),
                        ),
                        None => (inner, String::new()),
                    };
                    let mut verse = None;
                    let mut color = None;
                    let mut from = None;
                    let mut to = None;
                    for part in fields.split(',') {
                        let (k, v) = part.split_once(':').unwrap_or(("", ""));
                        match k.trim() {
                            "verse" => verse = v.trim().parse::<u16>().ok(),
                            "color" => color = Some(v.trim().to_string()),
                            "from" => from = v.trim().parse::<usize>().ok(),
                            "to" => to = v.trim().parse::<usize>().ok(),
                            _ => {}
                        }
                    }
                    match (verse, color, from, to) {
                        (Some(v), Some(c), Some(f), Some(t)) if t > f => {
                            n.ranges.push(RangeHighlight {
                                verse: v,
                                from: f,
                                to: t,
                                color: c,
                                text,
                            })
                        }
                        (Some(v), Some(c), _, _) => {
                            n.highlights.insert(v, c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    let mut cur: Option<u16> = None;
    let mut buf = String::new();
    let flush = |cur: Option<u16>, buf: &mut String, n: &mut ChapterNotes| {
        match cur {
            Some(v) => {
                n.notes.insert(v, unlink(buf.trim()));
            }
            None => n.preamble = buf.trim().to_string(),
        }
        buf.clear();
    };
    for line in body.lines() {
        let heading = line
            .strip_prefix("## ")
            .and_then(|h| h.trim().split_once(':'))
            .filter(|(c, _)| c.trim().parse::<u16>().ok() == Some(chapter))
            .and_then(|(_, v)| v.trim().parse::<u16>().ok());
        if let Some(v) = heading {
            flush(cur, &mut buf, &mut n);
            cur = Some(v);
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    flush(cur, &mut buf, &mut n);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join(format!("bible-notes-test-{}", std::process::id()));
        let store = NotesStore::new(&dir);
        let mut n = ChapterNotes::default();
        n.highlights.insert(16, "promises".into());
        n.highlights.insert(17, "christ".into());
        n.notes.insert(
            16,
            "The gift precedes the response — compare Eph 2:8.".into(),
        );
        n.add_range(RangeHighlight {
            verse: 3,
            from: 24,
            to: 52,
            color: "christ".into(),
            text: "Verily, verily, I say unto thee".into(),
        });
        let path = store.save("John", 3, &n).unwrap();
        assert!(path.ends_with("Bible/43 John/John 03.md"));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[Eph 2:8](bible://Eph.2.8)"));
        assert!(text.contains("  - {verse: 16, color: promises}"));
        let back = store.load("John", 3);
        assert_eq!(back, n);
        assert_eq!(
            store.chapter_path("Ps", 23).file_name().unwrap(),
            "Psalms 023.md"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
