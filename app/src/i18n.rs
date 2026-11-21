//! Translations. Strings are marked with `t("…")` and translated from
//! gettext `.po` files in `po/`, built into the app so it works offline.
//! The language follows the desktop (LANGUAGE, LC_ALL, LC_MESSAGES, LANG)
//! unless one is chosen in Preferences.
//!
//! To add a language: copy `po/es.po` to `po/xx.po`, translate the
//! `msgstr` lines, and add it to `CATALOGS` below.

use std::collections::HashMap;
use std::sync::OnceLock;

/// (code, name in that language, catalog)
pub const CATALOGS: &[(&str, &str, &str)] = &[("es", "Español", include_str!("../../po/es.po"))];

static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();

/// The language to use: the Preferences choice ("" or "system" means follow
/// the desktop), else the environment.
pub fn pick(choice: &str) -> String {
    if !choice.is_empty() && choice != "system" {
        return choice.to_string();
    }
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            let first = v.split(':').next().unwrap_or("");
            let code = first.split(['_', '.', '@']).next().unwrap_or("");
            if !code.is_empty() && code != "C" && code != "POSIX" {
                return code.to_lowercase();
            }
        }
    }
    "en".into()
}

/// Load the catalog for a language (call once at startup).
pub fn init(lang: &str) {
    let table = CATALOGS
        .iter()
        .find(|c| c.0 == lang)
        .map(|c| parse(c.2))
        .unwrap_or_default();
    let _ = TABLE.set(table);
}

/// Translate a string; untranslated strings come back as they are.
pub fn t(s: &str) -> String {
    TABLE
        .get()
        .and_then(|m| m.get(s))
        .filter(|x| !x.is_empty())
        .cloned()
        .unwrap_or_else(|| s.to_string())
}

fn unquote(line: &str) -> String {
    let l = line.trim();
    let l = l.strip_prefix('"').unwrap_or(l);
    let inner = l.strip_suffix('"').unwrap_or(l);
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A small reader for gettext .po files: msgid/msgstr pairs, multi-line
/// strings, comments. (No plurals or contexts yet.)
pub fn parse(po: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let (mut id, mut st) = (String::new(), String::new());
    #[derive(PartialEq)]
    enum In {
        None,
        Id,
        Str,
    }
    let mut cur = In::None;
    let mut flush = |id: &mut String, st: &mut String| {
        if !id.is_empty() {
            map.insert(std::mem::take(id), std::mem::take(st));
        } else {
            st.clear();
        }
    };
    for line in po.lines() {
        let l = line.trim();
        if l.starts_with('#') || l.is_empty() {
            continue;
        }
        if let Some(rest) = l.strip_prefix("msgid ") {
            if cur == In::Str {
                flush(&mut id, &mut st);
            }
            id = unquote(rest);
            cur = In::Id;
        } else if let Some(rest) = l.strip_prefix("msgstr ") {
            st = unquote(rest);
            cur = In::Str;
        } else if l.starts_with('"') {
            match cur {
                In::Id => id.push_str(&unquote(l)),
                In::Str => st.push_str(&unquote(l)),
                In::None => {}
            }
        }
    }
    if cur == In::Str {
        flush(&mut id, &mut st);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_po() {
        let m = parse("# c\nmsgid \"\"\nmsgstr \"Content-Type: text/plain\\n\"\n\nmsgid \"Today\"\nmsgstr \"Hoy\"\n\nmsgid \"\"\n\"Two \"\n\"lines\"\nmsgstr \"Dos \\\"líneas\\\"\"\n");
        assert_eq!(m.get("Today").map(String::as_str), Some("Hoy"));
        assert_eq!(m.get("Two lines").map(String::as_str), Some("Dos \"líneas\""));
        assert!(!m.contains_key(""));
    }

    #[test]
    fn spanish_catalog_is_complete_and_sane() {
        let m = parse(CATALOGS[0].2);
        assert!(m.len() > 40, "{} strings", m.len());
        for (k, v) in &m {
            assert!(!v.is_empty(), "untranslated: {k}");
            // placeholders must survive translation
            assert_eq!(k.matches("{}").count(), v.matches("{}").count(), "{k}");
        }
    }

    #[test]
    fn picks_language_from_environment_shape() {
        assert_eq!(pick("es"), "es");
    }
}
