//! Installed SWORD modules: finding them on disk and reading verse-keyed text
//! (zText, zText4, zCom, zCom4, RawText, RawText4, RawCom, RawCom4).
//!
//! This is an independent implementation of the published SWORD file formats.

use crate::conf::{decode, Conf};
use crate::markup::{self, Span};
use bible_core::refs::Ref;
use bible_core::v11n::{self, System};
use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Bible,
    Commentary,
    Lexicon,
    Devotional,
    Book,
    Other,
}

impl Kind {
    pub fn from_conf(c: &Conf) -> Kind {
        let drv = c.get_or("ModDrv", "").to_ascii_lowercase();
        let cat = c.get_or("Category", "");
        if c.all("Feature").contains(&"DailyDevotion") || cat == "Daily Devotional" {
            return Kind::Devotional;
        }
        // some dictionaries and glossaries are stored like Bibles; their
        // category says what they are
        let c_low = cat.to_ascii_lowercase();
        if c_low.contains("lexicon") || c_low.contains("dictionar") || c_low.contains("glossar") {
            return Kind::Lexicon;
        }
        if drv.contains("text") {
            Kind::Bible
        } else if drv.contains("com") {
            Kind::Commentary
        } else if drv.contains("ld") {
            Kind::Lexicon
        } else if drv.contains("genbook") {
            Kind::Book
        } else {
            Kind::Other
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Kind::Bible => "Bibles",
            Kind::Commentary => "Commentaries",
            Kind::Lexicon => "Dictionaries and lexicons",
            Kind::Devotional => "Devotionals",
            Kind::Book => "Books",
            Kind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Driver {
    ZText { size4: bool },
    Raw { size4: bool },
    Ld(crate::ld::LdKind),
    GenBook,
    Unsupported,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    block: u32,
    offset: u32,
    size: u32,
}

#[derive(Default)]
struct Testament {
    loaded: bool,
    entries: Vec<Entry>,
    blocks: Vec<(u32, u32, u32)>, // offset, compressed size, uncompressed size
}

pub struct Module {
    pub name: String,
    pub conf: Conf,
    pub kind: Kind,
    /// SWORD library root that holds mods.d/ and modules/
    pub root: PathBuf,
    pub user_installed: bool,
    driver: Driver,
    data_dir: PathBuf,
    v11n: &'static System,
    /// first letter of BlockType: zText files are named ot.bz?, ot.cz?, ot.vz?
    block_letter: char,
    testaments: Mutex<[Testament; 2]>,
    cache: Mutex<HashMap<(u8, u32), Arc<Vec<u8>>>>,
    ld: std::sync::OnceLock<Option<Arc<crate::ld::Ld>>>,
    book: std::sync::OnceLock<Option<Arc<crate::genbook::GenBook>>>,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module({})", self.name)
    }
}

impl Module {
    pub fn open(conf: Conf, root: &Path, user_installed: bool) -> Module {
        let drv = conf.get_or("ModDrv", "").to_ascii_lowercase();
        let compress_ok = conf
            .get_or("CompressType", "ZIP")
            .eq_ignore_ascii_case("ZIP");
        let locked = conf.get("CipherKey").is_some();
        let driver = match drv.as_str() {
            _ if locked => Driver::Unsupported,
            "ztext" | "zcom" if compress_ok => Driver::ZText { size4: false },
            "ztext4" | "zcom4" if compress_ok => Driver::ZText { size4: true },
            "rawtext" | "rawcom" => Driver::Raw { size4: false },
            "rawtext4" | "rawcom4" => Driver::Raw { size4: true },
            "rawld" => Driver::Ld(crate::ld::LdKind::Raw { size4: false }),
            "rawld4" => Driver::Ld(crate::ld::LdKind::Raw { size4: true }),
            "zld" if compress_ok => Driver::Ld(crate::ld::LdKind::Z),
            "rawgenbook" => Driver::GenBook,
            _ => Driver::Unsupported,
        };
        let dp = conf
            .get_or("DataPath", "")
            .trim_start_matches("./")
            .trim_end_matches('/')
            .to_string();
        let data_dir = root.join(dp);
        let v11n = v11n::system(conf.get_or("Versification", "KJV"));
        let block_letter = block_letter(&conf);
        Module {
            name: conf.name.clone(),
            kind: Kind::from_conf(&conf),
            conf,
            root: root.to_path_buf(),
            user_installed,
            driver,
            data_dir,
            v11n,
            block_letter,
            testaments: Mutex::new([Testament::default(), Testament::default()]),
            cache: Mutex::new(HashMap::new()),
            ld: std::sync::OnceLock::new(),
            book: std::sync::OnceLock::new(),
        }
    }

    /// Dictionary / lexicon / devotional data, for RawLD and zLD modules.
    pub fn ld(&self) -> Option<Arc<crate::ld::Ld>> {
        self.ld
            .get_or_init(|| match self.driver {
                Driver::Ld(k) => crate::ld::Ld::open(&self.data_dir, k).ok().map(Arc::new),
                _ => None,
            })
            .clone()
    }

    /// Table of contents and text, for general books.
    pub fn genbook(&self) -> Option<Arc<crate::genbook::GenBook>> {
        self.book
            .get_or_init(|| match self.driver {
                Driver::GenBook => crate::genbook::GenBook::open(&self.data_dir)
                    .ok()
                    .map(Arc::new),
                _ => None,
            })
            .clone()
    }

    /// Lexicon for Greek or Hebrew Strong's numbers?
    pub fn strongs_lang(&self) -> Option<char> {
        let f = self.conf.all("Feature");
        if f.contains(&"GreekDef") {
            Some('G')
        } else if f.contains(&"HebrewDef") {
            Some('H')
        } else {
            None
        }
    }

    pub fn description(&self) -> String {
        self.conf
            .get("Description")
            .unwrap_or(&self.name)
            .to_string()
    }
    pub fn abbreviation(&self) -> String {
        self.conf
            .get("Abbreviation")
            .unwrap_or(&self.name)
            .to_string()
    }
    pub fn lang(&self) -> String {
        self.conf.get_or("Lang", "en").to_string()
    }
    pub fn source_type(&self) -> &str {
        self.conf.get_or("SourceType", "Plaintext")
    }
    pub fn versification(&self) -> &'static System {
        self.v11n
    }
    pub fn is_readable(&self) -> bool {
        match self.driver {
            Driver::Unsupported => false,
            Driver::ZText { .. } | Driver::Raw { .. } => {
                matches!(self.kind, Kind::Bible | Kind::Commentary)
            }
            Driver::Ld(_) | Driver::GenBook => true,
        }
    }
    pub fn is_locked(&self) -> bool {
        self.conf.get("CipherKey").is_some()
    }
    pub fn license(&self) -> String {
        self.conf
            .get("DistributionLicense")
            .or(self.conf.get("ShortCopyright"))
            .or(self.conf.get("Copyright"))
            .unwrap_or("See module information")
            .to_string()
    }

    /// Number of index entries a testament file should have for this versification.
    fn expected_entries(&self, t: u8) -> usize {
        let books = &self.v11n.books;
        let range = if t == 1 {
            0..self.v11n.ot_books
        } else {
            self.v11n.ot_books..books.len()
        };
        if range.is_empty() {
            return 0;
        }
        2 + books[range]
            .iter()
            .map(|b| 1 + b.verses.iter().map(|v| 1 + *v as usize).sum::<usize>())
            .sum::<usize>()
    }

    fn load_testament(&self, t: u8) -> std::io::Result<()> {
        let mut ts = self.testaments.lock().unwrap();
        let slot = &mut ts[(t - 1) as usize];
        if slot.loaded {
            return Ok(());
        }
        slot.loaded = true;
        let pre = if t == 1 { "ot" } else { "nt" };
        match self.driver {
            Driver::ZText { size4 } => {
                let l = self.block_letter;
                let bzv = fs::read(self.data_dir.join(format!("{pre}.{l}zv")))?;
                // trust the file over the driver name: some modules labelled zCom4 use 2-byte sizes
                let expected = self.expected_entries(t);
                let width = if expected > 0 && bzv.len() == expected * 12 {
                    12
                } else if expected > 0 && bzv.len() == expected * 10 {
                    10
                } else if size4 {
                    12
                } else {
                    10
                };
                let size4 = width == 12;
                slot.entries = bzv
                    .chunks_exact(width)
                    .map(|c| Entry {
                        block: u32::from_le_bytes(c[0..4].try_into().unwrap()),
                        offset: u32::from_le_bytes(c[4..8].try_into().unwrap()),
                        size: if size4 {
                            u32::from_le_bytes(c[8..12].try_into().unwrap())
                        } else {
                            u16::from_le_bytes(c[8..10].try_into().unwrap()) as u32
                        },
                    })
                    .collect();
                let bzs = fs::read(self.data_dir.join(format!("{pre}.{l}zs")))?;
                slot.blocks = bzs
                    .chunks_exact(12)
                    .map(|c| {
                        (
                            u32::from_le_bytes(c[0..4].try_into().unwrap()),
                            u32::from_le_bytes(c[4..8].try_into().unwrap()),
                            u32::from_le_bytes(c[8..12].try_into().unwrap()),
                        )
                    })
                    .collect();
            }
            Driver::Raw { size4 } => {
                let vss = fs::read(self.data_dir.join(format!("{pre}.vss")))?;
                let width = if size4 { 8 } else { 6 };
                slot.entries = vss
                    .chunks_exact(width)
                    .map(|c| Entry {
                        block: 0,
                        offset: u32::from_le_bytes(c[0..4].try_into().unwrap()),
                        size: if size4 {
                            u32::from_le_bytes(c[4..8].try_into().unwrap())
                        } else {
                            u16::from_le_bytes(c[4..6].try_into().unwrap()) as u32
                        },
                    })
                    .collect();
            }
            _ => {}
        }
        Ok(())
    }

    fn block(&self, t: u8, b: u32) -> Option<Arc<Vec<u8>>> {
        if let Some(v) = self.cache.lock().unwrap().get(&(t, b)) {
            return Some(v.clone());
        }
        let (off, csize, _usize) = {
            let ts = self.testaments.lock().unwrap();
            *ts[(t - 1) as usize].blocks.get(b as usize)?
        };
        let pre = if t == 1 { "ot" } else { "nt" };
        let mut f =
            File::open(self.data_dir.join(format!("{pre}.{}zz", self.block_letter))).ok()?;
        f.seek(SeekFrom::Start(off as u64)).ok()?;
        let mut comp = vec![0u8; csize as usize];
        f.read_exact(&mut comp).ok()?;
        let mut out = Vec::new();
        ZlibDecoder::new(&comp[..]).read_to_end(&mut out).ok()?;
        let arc = Arc::new(out);
        let mut cache = self.cache.lock().unwrap();
        if cache.len() > 24 {
            cache.clear();
        }
        cache.insert((t, b), arc.clone());
        Some(arc)
    }

    /// Raw markup for a verse, or None if the module has no text there.
    pub fn raw_entry(&self, book: &str, chapter: u16, verse: u16) -> Option<String> {
        let slot = self.v11n.slot_for(book, chapter, verse)?;
        self.load_testament(slot.testament).ok()?;
        let e = {
            let ts = self.testaments.lock().unwrap();
            *ts[(slot.testament - 1) as usize].entries.get(slot.index)?
        };
        if e.size == 0 {
            return None;
        }
        let bytes = match self.driver {
            Driver::ZText { .. } => {
                let blk = self.block(slot.testament, e.block)?;
                blk.get(e.offset as usize..(e.offset + e.size) as usize)?
                    .to_vec()
            }
            Driver::Raw { .. } => {
                let pre = if slot.testament == 1 { "ot" } else { "nt" };
                let mut f = File::open(self.data_dir.join(pre)).ok()?;
                f.seek(SeekFrom::Start(e.offset as u64)).ok()?;
                let mut buf = vec![0u8; e.size as usize];
                f.read_exact(&mut buf).ok()?;
                buf
            }
            _ => return None,
        };
        let s = decode(&bytes);
        let s = s.trim_end_matches(['\0', '\r', '\n']).to_string();
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }

    pub fn render(&self, raw: &str) -> Vec<Span> {
        markup::render(self.source_type(), raw)
    }

    pub fn verse_spans(&self, book: &str, chapter: u16, verse: u16) -> Option<Vec<Span>> {
        self.raw_entry(book, chapter, verse)
            .map(|r| self.render(&r))
    }

    pub fn verse_plain(&self, book: &str, chapter: u16, verse: u16) -> Option<String> {
        self.raw_entry(book, chapter, verse)
            .map(|r| markup::plain(self.source_type(), &r))
    }

    /// All verses of a chapter that have text: (verse number, spans).
    pub fn chapter(&self, book: &str, chapter: u16) -> Vec<(u16, Vec<Span>)> {
        let Some(b) = self.v11n.book(book) else {
            return vec![];
        };
        (1..=b.verse_count(chapter))
            .filter_map(|v| self.verse_spans(book, chapter, v).map(|s| (v, s)))
            .collect()
    }

    /// Commentary entry for a verse; walks back to the nearest preceding entry when a
    /// commentary covers a range of verses with one note.
    pub fn commentary_for(&self, book: &str, chapter: u16, verse: u16) -> Option<(u16, Vec<Span>)> {
        let mut v = verse.max(1);
        loop {
            if let Some(s) = self.verse_spans(book, chapter, v) {
                return Some((v, s));
            }
            if v <= 1 || verse - v >= 12 {
                return None;
            }
            v -= 1;
        }
    }

    /// Books this module actually contains, in canonical order.
    pub fn books(&self) -> Vec<String> {
        self.v11n
            .books
            .iter()
            .filter(|b| {
                (1..=b.chapters().min(3)).any(|c| {
                    (1..=b.verse_count(c).min(4)).any(|v| self.raw_entry(&b.osis, c, v).is_some())
                })
            })
            .map(|b| b.osis.clone())
            .collect()
    }

    /// Case-insensitive search through every verse. Strong's numbers ("G25", "H430")
    /// match tagged words. Calls `progress` with a 0..1 fraction now and then.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        mut progress: impl FnMut(f64),
    ) -> Vec<(Ref, String)> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return vec![];
        }
        let strong = {
            let u = query.trim().to_uppercase();
            let mut c = u.chars();
            matches!(c.next(), Some('G') | Some('H'))
                && c.clone().count() > 0
                && c.all(|d| d.is_ascii_digit())
        }
        .then(|| query.trim().to_uppercase());
        let n = self.v11n.books.len();
        let mut out = Vec::new();
        for (i, b) in self.v11n.books.iter().enumerate() {
            progress(i as f64 / n as f64);
            for c in 1..=b.chapters() {
                for v in 1..=b.verse_count(c) {
                    let Some(raw) = self.raw_entry(&b.osis, c, v) else {
                        continue;
                    };
                    let hit = match &strong {
                        Some(s) => self.render(&raw).iter().any(|sp| {
                            sp.strong
                                .as_deref()
                                .map(|x| {
                                    x.trim_start_matches(['G', 'H']).trim_start_matches('0')
                                        == s[1..].trim_start_matches('0')
                                        && x.starts_with(&s[..1])
                                })
                                .unwrap_or(false)
                        }),
                        None => markup::plain(self.source_type(), &raw)
                            .to_lowercase()
                            .contains(&q),
                    };
                    if hit {
                        out.push((
                            Ref::new(&b.osis, c, v),
                            markup::plain(self.source_type(), &raw),
                        ));
                        if out.len() >= limit {
                            return out;
                        }
                    }
                }
            }
        }
        progress(1.0);
        out
    }
}

pub(crate) fn block_letter(conf: &Conf) -> char {
    match conf
        .get_or("BlockType", "BOOK")
        .to_ascii_uppercase()
        .chars()
        .next()
    {
        Some('C') => 'c',
        Some('V') => 'v',
        _ => 'b',
    }
}

/// All SWORD library roots we look in, user-writable one first.
pub fn default_roots() -> Vec<(PathBuf, bool)> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("SWORD_PATH") {
        // the path separator is ; on Windows, : elsewhere
        let sep = if cfg!(windows) { ';' } else { ':' };
        for part in p.split(sep).filter(|s| !s.is_empty()) {
            v.push((PathBuf::from(part), false));
        }
    }
    v.push((user_root(), true));
    if cfg!(unix) {
        v.push((PathBuf::from("/usr/share/sword"), false));
        v.push((PathBuf::from("/usr/local/share/sword"), false));
    }
    v
}

/// The directory new modules are installed into: ~/.sword on Linux and
/// macOS (shared with other SWORD apps), %APPDATA%\Sword on Windows (where
/// Xiphos and other Windows SWORD apps keep theirs).
pub fn user_root() -> PathBuf {
    if cfg!(windows) {
        if let Some(a) = std::env::var_os("APPDATA") {
            return PathBuf::from(a).join("Sword");
        }
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sword")
}

#[derive(Default)]
pub struct Library {
    pub modules: Vec<Arc<Module>>,
}

impl Library {
    pub fn scan() -> Library {
        Library::scan_roots(&default_roots())
    }

    pub fn scan_roots(roots: &[(PathBuf, bool)]) -> Library {
        let mut seen = std::collections::HashSet::new();
        let mut modules = Vec::new();
        for (root, user) in roots {
            let Ok(rd) = fs::read_dir(root.join("mods.d")) else {
                continue;
            };
            let mut files: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "conf").unwrap_or(false))
                .collect();
            files.sort();
            for f in files {
                let Ok(bytes) = fs::read(&f) else { continue };
                let Some(conf) = Conf::parse(&decode(&bytes)) else {
                    continue;
                };
                if conf.name.eq_ignore_ascii_case("globals")
                    || !seen.insert(conf.name.to_lowercase())
                {
                    continue;
                }
                modules.push(Arc::new(Module::open(conf, root, *user)));
            }
        }
        modules.sort_by_key(|a| a.description().to_lowercase());
        Library { modules }
    }

    pub fn get(&self, name: &str) -> Option<Arc<Module>> {
        self.modules
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    pub fn of_kind(&self, k: Kind) -> Vec<Arc<Module>> {
        self.modules
            .iter()
            .filter(|m| m.kind == k)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> Library {
        Library::scan_roots(&[(PathBuf::from("/usr/share/sword"), false)])
    }

    #[test]
    fn reads_kjv_john_3_16() {
        let l = lib();
        let Some(m) = l.get("engKJV2006eb") else {
            eprintln!("KJV module not installed; skipping");
            return;
        };
        let t = m.verse_plain("John", 3, 16).unwrap();
        assert!(t.starts_with("For God so loved the world"), "{t}");
        assert!(t.contains("everlasting life"));
        let g = m.verse_plain("Gen", 1, 1).unwrap();
        assert!(g.starts_with("In the beginning God created"), "{g}");
        let r = m.verse_plain("Rev", 22, 21).unwrap();
        assert!(r.contains("grace"), "{r}");
        let spans = m.verse_spans("John", 3, 16).unwrap();
        assert!(spans.iter().any(|s| s.strong.is_some()));
        assert_eq!(m.chapter("John", 3).len(), 36);
    }

    #[test]
    fn red_letters_and_commentary() {
        let l = lib();
        if let Some(m) = l.get("engKJV2006eb") {
            let spans = m.verse_spans("John", 3, 3).unwrap();
            assert!(spans.iter().any(|s| s.red), "{spans:?}");
        }
        if let Some(c) = l.get("MHCC") {
            let (_, spans) = c.commentary_for("John", 3, 16).unwrap();
            let text: String = spans.iter().map(|s| s.text.as_str()).collect();
            assert!(text.len() > 50, "{text}");
        }
    }

    #[test]
    fn other_formats_and_versifications() {
        let l = lib();
        if let Some(m) = l.get("GerLut1545") {
            let t = m.verse_plain("John", 3, 16).unwrap();
            assert!(t.contains("Gott"), "{t}");
        }
        if let Some(m) = l.get("TR") {
            let t = m.verse_plain("John", 3, 16).unwrap();
            assert!(
                t.contains("θεος")
                    || t.contains("θεὸς")
                    || t.contains("Θεός")
                    || t.contains("θεός"),
                "{t}"
            );
            assert!(!m.books().contains(&"Gen".to_string()));
        }
        if let Some(m) = l.get("engWEB2015eb") {
            assert_eq!(m.versification().name, "NRSVA");
            assert!(m.verse_plain("Tob", 1, 1).is_some());
            assert!(m
                .verse_plain("John", 3, 16)
                .unwrap()
                .contains("God so loved"));
        }
    }

    #[test]
    fn dictionaries_devotionals_books() {
        let roots = [
            (
                PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata")),
                false,
            ),
            (PathBuf::from("/usr/share/sword"), false),
        ];
        let l = Library::scan_roots(&roots);
        let dev = l.get("TestDev").expect("test devotional");
        assert_eq!(dev.kind, Kind::Devotional);
        let (_, t) = dev.ld().unwrap().for_date(11, 16).unwrap();
        assert!(t.contains("November the sixteenth"), "{t}");
        let lex = l.get("TestLex").unwrap();
        assert_eq!(lex.strongs_lang(), Some('G'));
        let (k, t) = lex.ld().unwrap().get("G25").unwrap();
        assert_eq!(k, "G0025");
        assert!(t.contains("to love"));
        assert!(lex.ld().unwrap().get("H430").unwrap().1.contains("elohim"));
        let book = l.get("TestBook").unwrap();
        let gb = book.genbook().unwrap();
        let paths: Vec<&str> = gb.nodes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "",
                "/Book I",
                "/Book I/Chapter 1",
                "/Book I/Chapter 2",
                "/Book II",
                "/Book II/Chapter 1"
            ]
        );
        let i = gb.find("/Book I/Chapter 1").unwrap();
        assert!(gb.text(i).unwrap().contains("knowledge of God"));
        assert_eq!(gb.next_with_text(i, true), Some(i + 1));
        if let Some(sg) = l.get("StrongsGreek") {
            let (k, t) = sg.ld().unwrap().get("G25").unwrap();
            assert_eq!(k, "00025");
            assert!(t.to_lowercase().contains("love"), "{t}");
        }
    }

    #[test]
    fn search_finds_phrase_and_strongs() {
        let l = lib();
        let Some(m) = l.get("engKJV2006eb") else {
            return;
        };
        let hits = m.search("only begotten", 50, |_| {});
        let refs: Vec<String> = hits.iter().map(|(r, _)| r.to_string()).collect();
        assert!(refs.contains(&"John 3:16".to_string()), "{refs:?}");
        assert_eq!(refs.len(), 6, "{refs:?}");
    }
}
