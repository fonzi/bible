//! SWORD module .conf files: `[Name]` followed by `Key=Value` lines.
//! Keys may repeat (e.g. GlobalOptionFilter) and values may continue with a trailing backslash.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Conf {
    pub name: String,
    pub entries: BTreeMap<String, Vec<String>>,
    pub raw: String,
}

impl Conf {
    pub fn parse(text: &str) -> Option<Conf> {
        let text = decode(text.as_bytes());
        let mut conf = Conf {
            raw: text.clone(),
            ..Default::default()
        };
        let lines = text.lines().peekable();
        let mut pending: Option<(String, String)> = None;
        for line in lines {
            let line = line.trim_end_matches('\r');
            if let Some((k, mut v)) = pending.take() {
                // continuation of the previous value
                let cont = line.ends_with('\\');
                v.push('\n');
                v.push_str(line.trim_end_matches('\\'));
                if cont {
                    pending = Some((k, v));
                } else {
                    conf.entries.entry(k).or_default().push(v);
                }
                continue;
            }
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            if l.starts_with('[') && l.ends_with(']') && conf.name.is_empty() {
                conf.name = l[1..l.len() - 1].trim().to_string();
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                let k = k.trim().to_string();
                let v = v.trim();
                if v.ends_with('\\') {
                    pending = Some((k, v.trim_end_matches('\\').to_string()));
                } else {
                    conf.entries.entry(k).or_default().push(v.to_string());
                }
            }
        }
        if let Some((k, v)) = pending {
            conf.entries.entry(k).or_default().push(v);
        }
        if conf.name.is_empty() {
            None
        } else {
            Some(conf)
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .get(key)
            .and_then(|v| v.first())
            .map(|s| s.as_str())
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    pub fn all(&self, key: &str) -> Vec<&str> {
        self.entries
            .get(key)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// What a Bible's markup makes possible in the app, from its conf.
    pub fn features(&self) -> Vec<Feature> {
        let filters = self.all("GlobalOptionFilter");
        let feats = self.all("Feature");
        let has = |names: &[&str]| {
            filters
                .iter()
                .chain(feats.iter())
                .any(|f| names.iter().any(|n| f.eq_ignore_ascii_case(n)))
        };
        let mut out = vec![];
        if has(&["OSISStrongs", "GBFStrongs", "ThMLStrongs", "StrongsNumbers"]) {
            out.push(Feature::Strongs);
        }
        if has(&["OSISMorph", "GBFMorph", "ThMLMorph"]) {
            out.push(Feature::Grammar);
        }
        if has(&["OSISFootnotes", "GBFFootnotes", "ThMLFootnotes"]) {
            out.push(Feature::Footnotes);
        }
        if has(&["OSISScripref", "ThMLScripref"]) {
            out.push(Feature::CrossRefs);
        }
        if has(&["OSISRedLetterWords", "GBFRedLetterWords"]) {
            out.push(Feature::RedLetters);
        }
        if has(&["OSISHeadings", "ThMLHeadings", "GBFHeadings"]) {
            out.push(Feature::Headings);
        }
        out
    }

    /// Higher is richer: used to suggest the Bible that turns on the most.
    pub fn feature_score(&self) -> u32 {
        self.features().iter().map(|f| f.weight()).sum()
    }

    /// Description with RTF-ish escapes and \par removed.
    pub fn about(&self) -> String {
        let a = self.get("About").unwrap_or("");
        a.replace("\\par", "\n")
            .replace("\\pard", "")
            .replace("\\qc", "")
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Strongs,
    Grammar,
    Footnotes,
    CrossRefs,
    RedLetters,
    Headings,
}

impl Feature {
    pub fn label(self) -> &'static str {
        match self {
            Feature::Strongs => "Strong’s word study",
            Feature::Grammar => "Grammar",
            Feature::Footnotes => "Footnotes",
            Feature::CrossRefs => "Cross-references",
            Feature::RedLetters => "Words of Jesus in red",
            Feature::Headings => "Section headings",
        }
    }

    fn weight(self) -> u32 {
        match self {
            Feature::Strongs => 4,
            Feature::Grammar => 2,
            _ => 1,
        }
    }
}

/// UTF-8 if it is valid, otherwise Latin-1 (older modules).
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeats_and_continuations() {
        let c = Conf::parse("[KJV]\nDataPath=./modules/texts/ztext/kjv/\nGlobalOptionFilter=OSISStrongs\nGlobalOptionFilter=OSISFootnotes\nAbout=Line one\\\nline two\nLang=en\n").unwrap();
        assert_eq!(c.name, "KJV");
        assert_eq!(c.all("GlobalOptionFilter").len(), 2);
        assert_eq!(c.get("About"), Some("Line one\nline two"));
        assert_eq!(c.get("Lang"), Some("en"));
    }
}

#[cfg(test)]
mod feature_tests {
    use super::*;

    #[test]
    fn reads_features() {
        let c = Conf::parse("[KJV]\nModDrv=zText\nFeature=StrongsNumbers\nGlobalOptionFilter=OSISStrongs\nGlobalOptionFilter=OSISMorph\nGlobalOptionFilter=OSISFootnotes\nGlobalOptionFilter=OSISRedLetterWords\n").unwrap();
        assert_eq!(
            c.features(),
            vec![
                Feature::Strongs,
                Feature::Grammar,
                Feature::Footnotes,
                Feature::RedLetters
            ]
        );
        assert_eq!(c.feature_score(), 8);
    }
}
