//! Audio Bibles: where each chapter's recording comes from and where it is kept.
//!
//! Audio is downloaded a chapter at a time into
//! ~/.local/share/bible-app/audio/<source>/<Book>/<chapter>.mp3

use std::path::{Path, PathBuf};

pub struct AudioSource {
    pub id: &'static str,
    pub title: &'static str,
    pub narrator: &'static str,
    /// Text versions this recording reads (SWORD module names, any case).
    pub versions: &'static [&'static str],
    pub licence: &'static str,
    pub credit: &'static str,
    url: fn(&str, u16) -> Option<String>,
}

impl AudioSource {
    pub fn url(&self, book: &str, chapter: u16) -> Option<String> {
        if chapter == 0 {
            return None;
        }
        let url = (self.url)(book, chapter)?;
        // for testing against a local mirror
        match std::env::var("BIBLE_APP_AUDIO_MIRROR") {
            Ok(m) if !m.is_empty() => Some(format!(
                "{}/{}",
                m.trim_end_matches('/'),
                url.rsplit('/').next().unwrap_or("")
            )),
            _ => Some(url),
        }
    }

    /// Whether this recording reads the given text version.
    pub fn reads(&self, module: &str) -> bool {
        let m = module.to_lowercase();
        self.versions
            .iter()
            .any(|v| m == v.to_lowercase() || m.starts_with(&v.to_lowercase()))
    }
}

/// OpenBible.com's book numbers and codes for the 66 books.
const OPENBIBLE: [(&str, &str); 66] = [
    ("Gen", "Gen"),
    ("Exod", "Exo"),
    ("Lev", "Lev"),
    ("Num", "Num"),
    ("Deut", "Deu"),
    ("Josh", "Jos"),
    ("Judg", "Jdg"),
    ("Ruth", "Rut"),
    ("1Sam", "1Sa"),
    ("2Sam", "2Sa"),
    ("1Kgs", "1Ki"),
    ("2Kgs", "2Ki"),
    ("1Chr", "1Ch"),
    ("2Chr", "2Ch"),
    ("Ezra", "Ezr"),
    ("Neh", "Neh"),
    ("Esth", "Est"),
    ("Job", "Job"),
    ("Ps", "Psa"),
    ("Prov", "Pro"),
    ("Eccl", "Ecc"),
    ("Song", "Sng"),
    ("Isa", "Isa"),
    ("Jer", "Jer"),
    ("Lam", "Lam"),
    ("Ezek", "Ezk"),
    ("Dan", "Dan"),
    ("Hos", "Hos"),
    ("Joel", "Jol"),
    ("Amos", "Amo"),
    ("Obad", "Oba"),
    ("Jonah", "Jon"),
    ("Mic", "Mic"),
    ("Nah", "Nam"),
    ("Hab", "Hab"),
    ("Zeph", "Zep"),
    ("Hag", "Hag"),
    ("Zech", "Zec"),
    ("Mal", "Mal"),
    ("Matt", "Mat"),
    ("Mark", "Mrk"),
    ("Luke", "Luk"),
    ("John", "Jhn"),
    ("Acts", "Act"),
    ("Rom", "Rom"),
    ("1Cor", "1Co"),
    ("2Cor", "2Co"),
    ("Gal", "Gal"),
    ("Eph", "Eph"),
    ("Phil", "Php"),
    ("Col", "Col"),
    ("1Thess", "1Th"),
    ("2Thess", "2Th"),
    ("1Tim", "1Ti"),
    ("2Tim", "2Ti"),
    ("Titus", "Tts"),
    ("Phlm", "Phm"),
    ("Heb", "Heb"),
    ("Jas", "Jas"),
    ("1Pet", "1Pe"),
    ("2Pet", "2Pe"),
    ("1John", "1Jn"),
    ("2John", "2Jn"),
    ("3John", "3Jn"),
    ("Jude", "Jud"),
    ("Rev", "Rev"),
];

fn bsb_souer(book: &str, chapter: u16) -> Option<String> {
    let i = OPENBIBLE.iter().position(|(b, _)| *b == book)?;
    Some(format!(
        "https://openbible.com/audio/souer/BSB_{:02}_{}_{:03}.mp3",
        i + 1,
        OPENBIBLE[i].1,
        chapter
    ))
}

pub const SOURCES: &[AudioSource] = &[AudioSource {
    id: "bsb-souer",
    title: "Berean Standard Bible",
    narrator: "Bob Souer",
    versions: &["BSB", "engBSB"],
    licence: "Public domain",
    credit: "Berean Standard Bible audio narrated by Bob Souer, dedicated to the public domain. From OpenBible.com.",
    url: bsb_souer,
}];

pub fn source(id: &str) -> Option<&'static AudioSource> {
    SOURCES.iter().find(|s| s.id == id)
}

/// The recording that reads this version, if any.
pub fn for_version(module: &str) -> Option<&'static AudioSource> {
    SOURCES.iter().find(|s| s.reads(module))
}

pub fn root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bible-app/audio")
}

pub fn path(src: &AudioSource, book: &str, chapter: u16) -> PathBuf {
    path_in(&root(), src, book, chapter)
}

fn path_in(root: &Path, src: &AudioSource, book: &str, chapter: u16) -> PathBuf {
    root.join(src.id)
        .join(book)
        .join(format!("{chapter:03}.mp3"))
}

pub fn is_downloaded(src: &AudioSource, book: &str, chapter: u16) -> bool {
    path(src, book, chapter).is_file()
}

/// A downloaded chapter.
#[derive(Debug, Clone)]
pub struct Downloaded {
    pub book: String,
    pub chapter: u16,
    pub bytes: u64,
    pub path: PathBuf,
}

/// Every downloaded chapter of a source, in Bible order.
pub fn downloaded(src: &AudioSource) -> Vec<Downloaded> {
    downloaded_in(&root(), src)
}

fn downloaded_in(root: &Path, src: &AudioSource) -> Vec<Downloaded> {
    let mut out = vec![];
    let Ok(books) = std::fs::read_dir(root.join(src.id)) else {
        return out;
    };
    for b in books.flatten() {
        let book = b.file_name().to_string_lossy().to_string();
        let Ok(files) = std::fs::read_dir(b.path()) else {
            continue;
        };
        for f in files.flatten() {
            let name = f.file_name().to_string_lossy().to_string();
            let Some(ch) = name
                .strip_suffix(".mp3")
                .and_then(|n| n.parse::<u16>().ok())
            else {
                continue;
            };
            let bytes = f.metadata().map(|m| m.len()).unwrap_or(0);
            out.push(Downloaded {
                book: book.clone(),
                chapter: ch,
                bytes,
                path: f.path(),
            });
        }
    }
    let order = |b: &str| OPENBIBLE.iter().position(|(x, _)| *x == b).unwrap_or(999);
    out.sort_by(|a, b| {
        order(&a.book)
            .cmp(&order(&b.book))
            .then(a.chapter.cmp(&b.chapter))
    });
    out
}

pub fn remove(src: &AudioSource, book: &str, chapter: Option<u16>) -> std::io::Result<()> {
    match chapter {
        Some(c) => std::fs::remove_file(path(src, book, c)),
        None => std::fs::remove_dir_all(root().join(src.id).join(book)),
    }
}

/// "4.2 MB"
pub fn size(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1e9)
    } else {
        format!("{:.1} MB", bytes as f64 / 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        let s = source("bsb-souer").unwrap();
        assert_eq!(
            s.url("Gen", 1).unwrap(),
            "https://openbible.com/audio/souer/BSB_01_Gen_001.mp3"
        );
        assert_eq!(
            s.url("Ps", 150).unwrap(),
            "https://openbible.com/audio/souer/BSB_19_Psa_150.mp3"
        );
        assert_eq!(
            s.url("John", 3).unwrap(),
            "https://openbible.com/audio/souer/BSB_43_Jhn_003.mp3"
        );
        assert_eq!(
            s.url("Titus", 2).unwrap(),
            "https://openbible.com/audio/souer/BSB_56_Tts_002.mp3"
        );
        assert_eq!(
            s.url("Rev", 22).unwrap(),
            "https://openbible.com/audio/souer/BSB_66_Rev_022.mp3"
        );
        assert!(s.url("Tob", 1).is_none());
        assert!(s.reads("BSB") && !s.reads("KJV"));
    }

    #[test]
    fn lists_downloads() {
        let dir = std::env::temp_dir().join(format!("audio-test-{}", std::process::id()));
        let s = source("bsb-souer").unwrap();
        for (b, c) in [("John", 3), ("Gen", 1), ("John", 1)] {
            let p = path_in(&dir, s, b, c);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"x").unwrap();
        }
        let d: Vec<_> = downloaded_in(&dir, s)
            .iter()
            .map(|d| format!("{} {}", d.book, d.chapter))
            .collect();
        assert_eq!(d, vec!["Gen 1", "John 1", "John 3"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
