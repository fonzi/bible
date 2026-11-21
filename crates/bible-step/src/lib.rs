//! Interlinear Greek and Hebrew with glosses, grammar and a brief lexicon, from
//! STEP Bible's open data (Tyndale House, Cambridge; CC BY 4.0, www.STEPBible.org).
//!
//! The raw files are downloaded on demand from GitHub and converted once into a
//! compact local copy, so everything works offline afterwards.

pub mod morph;

use bible_core::refs::canonical_book;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const CREDIT: &str = "Interlinear and lexicon data: STEP Bible (www.STEPBible.org), CC BY 4.0";
const BASE: &str = "https://raw.githubusercontent.com/STEPBible/STEPBible-Data/master/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pack {
    GreekNt,
    HebrewOt,
}

impl Pack {
    pub fn all() -> [Pack; 2] {
        [Pack::GreekNt, Pack::HebrewOt]
    }
    pub fn id(&self) -> &'static str {
        match self {
            Pack::GreekNt => "nt",
            Pack::HebrewOt => "ot",
        }
    }
    pub fn title(&self) -> &'static str {
        match self {
            Pack::GreekNt => "Greek New Testament, interlinear",
            Pack::HebrewOt => "Hebrew Old Testament, interlinear",
        }
    }
    pub fn description(&self) -> &'static str {
        match self {
            Pack::GreekNt => "Every word with its English gloss, grammar and Strong’s number, plus a brief Greek lexicon (STEP Bible TAGNT and TBESG, CC BY 4.0). About 35 MB to download.",
            Pack::HebrewOt => "The Leningrad Codex word by word with glosses, grammar and Strong’s numbers, plus a brief Hebrew lexicon (STEP Bible TAHOT and TBESH, CC BY 4.0). About 75 MB to download.",
        }
    }
    fn sources(&self) -> Vec<&'static str> {
        match self {
            Pack::GreekNt => vec![
                "Translators Amalgamated OT+NT/TAGNT Mat-Jhn - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt",
                "Translators Amalgamated OT+NT/TAGNT Act-Rev - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt",
            ],
            Pack::HebrewOt => vec![
                "Translators Amalgamated OT+NT/TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
                "Translators Amalgamated OT+NT/TAHOT Jos-Est - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
                "Translators Amalgamated OT+NT/TAHOT Job-Sng - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
                "Translators Amalgamated OT+NT/TAHOT Isa-Mal - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
            ],
        }
    }
    fn lexicon(&self) -> &'static str {
        match self {
            Pack::GreekNt => "Lexicons/TBESG - Translators Brief lexicon of Extended Strongs for Greek - STEPBible.org CC BY.txt",
            Pack::HebrewOt => "Lexicons/TBESH - Translators Brief lexicon of Extended Strongs for Hebrew - STEPBible.org CC BY.txt",
        }
    }
    pub fn for_book(osis: &str) -> Option<Pack> {
        let sys = bible_core::v11n::kjv();
        let i = sys.book_index(osis)?;
        Some(if i < sys.ot_books {
            Pack::HebrewOt
        } else {
            Pack::GreekNt
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    /// original-language text as printed (Greek or pointed Hebrew)
    pub text: String,
    pub translit: String,
    /// English gloss in context
    pub gloss: String,
    /// extended Strong's, e.g. "G0025" or "H7225G"
    pub strong: String,
    /// grammar code, e.g. "V-AAI-3S" or "HVqp3ms"
    pub morph: String,
    /// dictionary form
    pub lemma: String,
    /// dictionary gloss
    pub lemma_gloss: String,
    /// not in the main critical text (NA28) — e.g. a Textus Receptus reading
    pub variant: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexEntry {
    pub strong: String,
    pub lemma: String,
    pub translit: String,
    pub morph: String,
    pub gloss: String,
    /// definition converted to Pango markup, with bible:// links
    pub definition: String,
}

/// STEP's three-letter book codes to OSIS ids.
fn step_book(code: &str) -> Option<String> {
    let osis = match code {
        "Gen" => "Gen",
        "Exo" => "Exod",
        "Lev" => "Lev",
        "Num" => "Num",
        "Deu" => "Deut",
        "Jos" => "Josh",
        "Jdg" => "Judg",
        "Rut" => "Ruth",
        "1Sa" => "1Sam",
        "2Sa" => "2Sam",
        "1Ki" => "1Kgs",
        "2Ki" => "2Kgs",
        "1Ch" => "1Chr",
        "2Ch" => "2Chr",
        "Ezr" => "Ezra",
        "Neh" => "Neh",
        "Est" => "Esth",
        "Job" => "Job",
        "Psa" => "Ps",
        "Pro" => "Prov",
        "Ecc" => "Eccl",
        "Sng" => "Song",
        "Isa" => "Isa",
        "Jer" => "Jer",
        "Lam" => "Lam",
        "Ezk" => "Ezek",
        "Dan" => "Dan",
        "Hos" => "Hos",
        "Jol" => "Joel",
        "Amo" => "Amos",
        "Oba" => "Obad",
        "Jon" => "Jonah",
        "Mic" => "Mic",
        "Nam" => "Nah",
        "Hab" => "Hab",
        "Zep" => "Zeph",
        "Hag" => "Hag",
        "Zec" => "Zech",
        "Mal" => "Mal",
        "Mat" => "Matt",
        "Mrk" => "Mark",
        "Luk" => "Luke",
        "Jhn" => "John",
        "Act" => "Acts",
        "Rom" => "Rom",
        "1Co" => "1Cor",
        "2Co" => "2Cor",
        "Gal" => "Gal",
        "Eph" => "Eph",
        "Php" => "Phil",
        "Col" => "Col",
        "1Th" => "1Thess",
        "2Th" => "2Thess",
        "1Ti" => "1Tim",
        "2Ti" => "2Tim",
        "Tit" => "Titus",
        "Phm" => "Phlm",
        "Heb" => "Heb",
        "Jas" => "Jas",
        "1Pe" => "1Pet",
        "2Pe" => "2Pet",
        "1Jn" => "1John",
        "2Jn" => "2John",
        "3Jn" => "3John",
        "Jud" => "Jude",
        "Rev" => "Rev",
        other => return canonical_book(other),
    };
    Some(osis.to_string())
}

/// "Jhn.3.16#01=NKO" or "Gen.31.55(32.1)#01=L" -> "John.3.16" (English numbering)
fn parse_ref(field: &str) -> Option<String> {
    let r = field.split('#').next()?;
    let r = r.split('(').next()?;
    let mut parts = r.split('.');
    let book = step_book(parts.next()?)?;
    let ch: u16 = parts.next()?.parse().ok()?;
    let v: u16 = parts.next()?.parse().ok()?;
    Some(format!("{book}.{ch}.{v}"))
}

fn split_paren(s: &str) -> (String, String) {
    match s.rfind(" (") {
        Some(i) if s.ends_with(')') => (
            s[..i].trim().to_string(),
            s[i + 2..s.len() - 1].trim().to_string(),
        ),
        _ => (s.trim().to_string(), String::new()),
    }
}

fn clean_tsv(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// Convert one TAGNT/TAHOT line into our compact record: ref \t fields...
fn convert_line(line: &str, pack: Pack) -> Option<String> {
    let c: Vec<&str> = line.split('\t').collect();
    if c.len() < 6 {
        return None;
    }
    let r = parse_ref(c[0])?;
    let w = match pack {
        Pack::GreekNt => {
            let (text, translit) = split_paren(c[1]);
            let (strong, morph) = c[3].split_once('=').unwrap_or((c[3], ""));
            let (lemma, lgloss) = c[4].split_once('=').unwrap_or((c[4], ""));
            let variant = !c[5].contains("NA28");
            Word {
                text,
                translit,
                gloss: c[2].trim().to_string(),
                strong: strong.trim().to_string(),
                morph: morph.trim().to_string(),
                lemma: lemma.trim().to_string(),
                lemma_gloss: lgloss.trim().to_string(),
                variant,
            }
        }
        Pack::HebrewOt => {
            // main Strong's is in braces: H9003/{H7225G}
            let strongs = c[4];
            let strong = strongs
                .split('{')
                .nth(1)
                .and_then(|x| x.split('}').next())
                .unwrap_or(strongs)
                .to_string();
            let expanded = c.get(11).copied().unwrap_or("");
            let main = expanded
                .split('{')
                .nth(1)
                .and_then(|x| x.split('}').next())
                .unwrap_or("");
            let mut mp = main.split('=');
            let _ = mp.next();
            let lemma = mp.next().unwrap_or("").to_string();
            let lgloss = mp
                .next()
                .unwrap_or("")
                .split('»')
                .next()
                .unwrap_or("")
                .trim_start_matches(':')
                .trim()
                .to_string();
            Word {
                text: c[1].replace(['/', '\\'], ""),
                translit: c[2].replace('/', ""),
                gloss: c[3].replace("/ ", " ").replace('/', " ").trim().to_string(),
                strong,
                morph: c[5].trim().to_string(),
                lemma,
                lemma_gloss: lgloss,
                variant: false,
            }
        }
    };
    if w.text.is_empty() {
        return None;
    }
    Some(format!(
        "{r}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        clean_tsv(&w.text),
        clean_tsv(&w.translit),
        clean_tsv(&w.gloss),
        w.strong,
        w.morph,
        clean_tsv(&w.lemma),
        clean_tsv(&w.lemma_gloss),
        if w.variant { "v" } else { "" }
    ))
}

/// STEP definition HTML -> Pango markup with bible:// links. Tags are always balanced.
pub fn definition_markup(html: &str) -> String {
    let mut out = String::new();
    let mut stack: Vec<&'static str> = Vec::new(); // open b / i
    let mut link_open = false;
    let mut ref_depth: u32 = 0;
    let mut rest = html;
    let close_all = |out: &mut String, stack: &mut Vec<&'static str>| {
        while let Some(t) = stack.pop() {
            out.push_str(if t == "b" { "</b>" } else { "</i>" });
        }
    };
    while !rest.is_empty() {
        let Some(i) = rest.find('<') else {
            push_text(&mut out, rest);
            break;
        };
        push_text(&mut out, &rest[..i]);
        let Some(e) = rest[i..].find('>') else {
            push_text(&mut out, &rest[i..]);
            break;
        };
        let tag = &rest[i + 1..i + e];
        let lower = tag.trim().to_ascii_lowercase();
        if lower.starts_with("br") {
            out.push('\n');
        } else if (lower == "b" || lower == "i") && !link_open {
            let t: &'static str = if lower == "b" { "b" } else { "i" };
            out.push_str(if t == "b" { "<b>" } else { "<i>" });
            stack.push(t);
        } else if (lower == "/b" || lower == "/i") && !link_open {
            let t = if lower == "/b" { "b" } else { "i" };
            if stack.last() == Some(&t) {
                stack.pop();
                out.push_str(if t == "b" { "</b>" } else { "</i>" });
            }
        } else if lower.starts_with("ref=") || lower.starts_with("ref ") {
            if ref_depth == 0 {
                let target = tag.split(['\'', '"']).nth(1).unwrap_or("");
                let first = target.split([';', ',', '-']).next().unwrap_or("").trim();
                if let [b, c, v, ..] = first.split('.').collect::<Vec<_>>().as_slice() {
                    if let Some(osis) = step_book(b) {
                        close_all(&mut out, &mut stack);
                        out.push_str(&format!("<a href=\"bible://{}.{}.{}\">", osis, c, v.trim()));
                        link_open = true;
                    }
                }
            }
            ref_depth += 1;
        } else if lower == "/ref" {
            ref_depth = ref_depth.saturating_sub(1);
            if ref_depth == 0 && link_open {
                out.push_str("</a>");
                link_open = false;
            }
        }
        rest = &rest[i + e + 1..];
    }
    if link_open {
        out.push_str("</a>");
    }
    close_all(&mut out, &mut stack);
    // tidy blank lines but keep line breaks; tags never span lines now except b/i, so join safely
    let lines: Vec<&str> = out
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

fn push_text(out: &mut String, t: &str) {
    for ch in t.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

fn convert_lex_line(line: &str) -> Option<String> {
    let c: Vec<&str> = line.split('\t').collect();
    if c.len() < 8
        || !(c[0].starts_with('G') || c[0].starts_with('H'))
        || !c[0][1..].starts_with(|ch: char| ch.is_ascii_digit())
    {
        return None;
    }
    let key = c[1].split_whitespace().next().unwrap_or(c[0]);
    Some(format!(
        "{key}\t{}\t{}\t{}\t{}\t{}\t{}",
        c[0],
        clean_tsv(c[3]),
        clean_tsv(c[4]),
        clean_tsv(c[5]),
        clean_tsv(c[6]),
        clean_tsv(&definition_markup(c[7]).replace('\n', "\\n"))
    ))
}

/// Where packs live: ~/.local/share/bible-app/step
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bible-app")
        .join("step")
}

fn words_file(dir: &Path, p: Pack) -> PathBuf {
    dir.join(format!("{}-words.tsv.gz", p.id()))
}
fn lex_file(dir: &Path, p: Pack) -> PathBuf {
    dir.join(format!("{}-lexicon.tsv.gz", p.id()))
}

pub fn is_installed(p: Pack) -> bool {
    is_installed_in(&data_dir(), p)
}
pub fn is_installed_in(dir: &Path, p: Pack) -> bool {
    words_file(dir, p).exists() && lex_file(dir, p).exists()
}

pub fn remove(p: Pack) -> io::Result<()> {
    let d = data_dir();
    let _ = fs::remove_file(words_file(&d, p));
    let _ = fs::remove_file(lex_file(&d, p));
    CACHE.lock().unwrap().remove(&p);
    Ok(())
}

fn fetch(path: &str) -> io::Result<Box<dyn Read + Send>> {
    let url = format!("{BASE}{}", path.replace(' ', "%20").replace('+', "%2B"));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(300))
        .try_proxy_from_env(true)
        .build();
    match agent.get(&url).call() {
        Ok(r) => Ok(Box::new(r.into_reader())),
        Err(e) => Err(io::Error::other(format!("{url}: {e}"))),
    }
}

/// Convert raw STEP files (read from `open`) into the pack's compact files.
pub fn build_pack(
    dir: &Path,
    p: Pack,
    open: &mut dyn FnMut(&str) -> io::Result<Box<dyn Read + Send>>,
    progress: &mut dyn FnMut(usize, usize),
) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let total = p.sources().len() + 1;
    let tmp = words_file(dir, p).with_extension("tmp");
    {
        let mut gz = GzEncoder::new(fs::File::create(&tmp)?, Compression::default());
        for (i, src) in p.sources().iter().enumerate() {
            progress(i, total);
            let rd = BufReader::new(open(src)?);
            for line in rd.lines() {
                let line = line?;
                if let Some(rec) = convert_line(&line, p) {
                    gz.write_all(rec.as_bytes())?;
                    gz.write_all(b"\n")?;
                }
            }
        }
        gz.finish()?;
    }
    progress(total - 1, total);
    let ltmp = lex_file(dir, p).with_extension("tmp");
    {
        let mut gz = GzEncoder::new(fs::File::create(&ltmp)?, Compression::default());
        for line in BufReader::new(open(p.lexicon())?).lines() {
            if let Some(rec) = convert_lex_line(&line?) {
                gz.write_all(rec.as_bytes())?;
                gz.write_all(b"\n")?;
            }
        }
        gz.finish()?;
    }
    fs::rename(&tmp, words_file(dir, p))?;
    fs::rename(&ltmp, lex_file(dir, p))?;
    CACHE.lock().unwrap().remove(&p);
    progress(total, total);
    Ok(())
}

/// Download from GitHub and build the pack.
pub fn download(p: Pack, progress: &mut dyn FnMut(usize, usize)) -> io::Result<()> {
    build_pack(&data_dir(), p, &mut |path| fetch(path), progress)
}

/// One use of a word: where, the original form, and its gloss there.
#[derive(Debug, Clone)]
pub struct Occurrence {
    pub osis: String,
    pub text: String,
    pub gloss: String,
}

/// "G0025", "G25", "g25", "H7225G" -> "G25" / "H7225"
pub fn plain_strong(s: &str) -> String {
    let s = s.trim();
    let mut it = s.chars();
    let Some(p) = it.next() else {
        return String::new();
    };
    let digits: String = it.take_while(|c| c.is_ascii_digit()).collect();
    let n = digits.trim_start_matches('0');
    format!(
        "{}{}",
        p.to_ascii_uppercase(),
        if n.is_empty() { "0" } else { n }
    )
}

pub struct Interlinear {
    words: HashMap<String, Vec<Word>>,
    lexicon: HashMap<String, LexEntry>,
}

static CACHE: Lazy<Mutex<HashMap<Pack, Arc<Interlinear>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

impl Interlinear {
    pub fn load_from(dir: &Path, p: Pack) -> io::Result<Interlinear> {
        let mut words: HashMap<String, Vec<Word>> = HashMap::new();
        for line in BufReader::new(GzDecoder::new(fs::File::open(words_file(dir, p))?)).lines() {
            let line = line?;
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 9 {
                continue;
            }
            words.entry(c[0].to_string()).or_default().push(Word {
                text: c[1].replace('\\', ""),
                translit: c[2].into(),
                gloss: c[3].into(),
                strong: c[4].into(),
                morph: c[5].into(),
                lemma: c[6].into(),
                lemma_gloss: c[7].into(),
                variant: c[8] == "v",
            });
        }
        let mut lexicon = HashMap::new();
        for line in BufReader::new(GzDecoder::new(fs::File::open(lex_file(dir, p))?)).lines() {
            let line = line?;
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 7 {
                continue;
            }
            let e = LexEntry {
                strong: c[1].into(),
                lemma: c[2].into(),
                translit: c[3].into(),
                morph: c[4].into(),
                gloss: c[5].into(),
                definition: c[6].replace("\\n", "\n"),
            };
            lexicon.entry(c[1].to_string()).or_insert_with(|| e.clone());
            lexicon.insert(c[0].to_string(), e);
        }
        Ok(Interlinear { words, lexicon })
    }

    /// Cached load from the user's data directory.
    pub fn get(p: Pack) -> Option<Arc<Interlinear>> {
        if let Some(i) = CACHE.lock().unwrap().get(&p) {
            return Some(i.clone());
        }
        let i = Arc::new(Interlinear::load_from(&data_dir(), p).ok()?);
        CACHE.lock().unwrap().insert(p, i.clone());
        Some(i)
    }

    pub fn lexicon_entries(&self) -> impl Iterator<Item = &LexEntry> {
        self.lexicon.values()
    }

    pub fn verse(&self, book: &str, chapter: u16, verse: u16) -> &[Word] {
        self.words
            .get(&format!("{book}.{chapter}.{verse}"))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Every place a Strong's number is used, in Bible order.
    pub fn occurrences(&self, strong: &str) -> Vec<Occurrence> {
        let want = plain_strong(strong);
        let mut out: Vec<Occurrence> = vec![];
        for (key, words) in &self.words {
            for w in words
                .iter()
                .filter(|w| !w.variant && plain_strong(&w.strong) == want)
            {
                out.push(Occurrence {
                    osis: key.clone(),
                    text: w.text.clone(),
                    gloss: w.gloss.clone(),
                });
            }
        }
        let sys = bible_core::v11n::kjv();
        let order = |k: &str| {
            let mut p = k.split('.');
            let b = p.next().unwrap_or("");
            let c: u16 = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
            let v: u16 = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
            (sys.book_index(b).unwrap_or(999), c, v)
        };
        out.sort_by_key(|o| order(&o.osis));
        out
    }

    /// Look up an extended Strong's number, falling back to the plain number.
    pub fn lookup(&self, strong: &str) -> Option<&LexEntry> {
        let s = strong.trim();
        self.lexicon.get(s).or_else(|| {
            // "H7225G" -> "H7225"
            let plain: String = s
                .chars()
                .take(1)
                .chain(s.chars().skip(1).take_while(|c| c.is_ascii_digit()))
                .collect();
            self.lexicon.get(&plain)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEK: &str = "Jhn.3.16#03=NKO\tἠγάπησεν (ēgapēsen)\tloved\tG0025=V-AAI-3S\tἀγαπάω=to love\tNA28+NA27+Tyn+SBL+WH+Treg+TR+Byz\t\t\tamó\tto love\t#03\tG0025";
    const HEB: &str = "Gen.1.1#01=L\tבְּ/רֵאשִׁ֖ית\tbe./re.Shit\tin/ beginning\tH9003/{H7225G}\tHR/Ncfsa\t\t\tH7225G\t\t\tH9003=ב=in/{H7225G=רֵאשִׁית=: beginning»first:1_beginning}";

    #[test]
    fn converts_lines() {
        let g = convert_line(GREEK, Pack::GreekNt).unwrap();
        assert!(g.starts_with(
            "John.3.16\tἠγάπησεν\tēgapēsen\tloved\tG0025\tV-AAI-3S\tἀγαπάω\tto love\t"
        ));
        let h = convert_line(HEB, Pack::HebrewOt).unwrap();
        assert!(
            h.starts_with(
                "Gen.1.1\tבְּרֵאשִׁ֖ית\tbe.re.Shit\tin beginning\tH7225G\tHR/Ncfsa\tרֵאשִׁית\tbeginning"
            ),
            "{h}"
        );
        assert_eq!(parse_ref("Gen.31.55(32.1)#01=L").unwrap(), "Gen.31.55");
    }

    #[test]
    fn definition_links() {
        let m = definition_markup(
            "<b>ἀγαπάω</b>, -ῶ, <BR /><b>to love</b>: <ref='Mat.5.43'>Mat.5:43;</ref> & more",
        );
        assert!(m.contains("<b>to love</b>"));
        assert!(
            m.contains("<a href=\"bible://Matt.5.43\">Mat.5:43;</a>"),
            "{m}"
        );
        assert!(m.contains("&amp; more"));
    }

    #[test]
    fn builds_from_local_files() {
        let src = std::env::var("STEP_DATA").unwrap_or_else(|_| "/home/claude/dl/step".into());
        if !Path::new(&src).join("Lexicons").exists() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("step-test-{}", std::process::id()));
        let mut open = |p: &str| -> io::Result<Box<dyn Read + Send>> {
            Ok(Box::new(fs::File::open(Path::new(&src).join(p))?))
        };
        build_pack(&dir, Pack::GreekNt, &mut open, &mut |_, _| {}).unwrap();
        let i = Interlinear::load_from(&dir, Pack::GreekNt).unwrap();
        let v = i.verse("John", 3, 16);
        assert!(v.len() >= 25, "{}", v.len());
        assert_eq!(v[2].strong, "G0025");
        assert_eq!(v[2].translit, "ēgapēsen");
        assert_eq!(v[2].gloss, "loved");
        let e = i.lookup(&v[2].strong).unwrap();
        assert_eq!(e.translit, "agapaō");
        assert!(e.definition.contains("to love"));
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod live {
    use super::*;
    /// STEP_LIVE=1 downloads the Greek pack from GitHub into a temp dir.
    #[test]
    fn download_greek() {
        if std::env::var("STEP_LIVE").is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("step-live-{}", std::process::id()));
        build_pack(&dir, Pack::GreekNt, &mut |p| fetch(p), &mut |d, t| {
            eprintln!("{d}/{t}")
        })
        .unwrap();
        let i = Interlinear::load_from(&dir, Pack::GreekNt).unwrap();
        assert!(i.verse("Rev", 22, 21).len() > 5);
        // "grace" (G5485) is in the last verse of the Bible
        let occ = i.occurrences("G5485");
        assert!(occ.iter().any(|o| o.osis == "Rev.22.21"), "{}", occ.len());
        assert_eq!(super::plain_strong("G0025"), "G25");
        assert_eq!(super::plain_strong("H7225G"), "H7225");
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod balance {
    #[test]
    fn nested_refs_and_tags_balance() {
        let m = super::definition_markup(
            "<ref='2Ti.4.8, 10'>II <ref='Tim.4.8'>Tim 4:8</ref>, 10</ref> <b>bold <i>x</b> tail",
        );
        assert_eq!(m.matches("<a ").count(), m.matches("</a>").count());
        assert_eq!(m.matches("<b>").count(), m.matches("</b>").count());
        assert_eq!(m.matches("<i>").count(), m.matches("</i>").count());
    }
}
