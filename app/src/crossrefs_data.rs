//! Cross-references as data: which links a Bible has (its own, or
//! OpenBible.info's), the saved copies, and the orderings and groupings the
//! views draw them in.

use bible_sword::Module;
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn chapters(m: &Module) -> Vec<(String, u16, u16)> {
    let sys = m.versification();
    let mut out = vec![];
    for b in sys
        .books
        .iter()
        .filter(|b| !bible_core::v11n::is_apocrypha(&b.osis))
    {
        for c in 1..=b.chapters() {
            out.push((b.osis.clone(), c, b.verse_count(c)));
        }
    }
    out
}

pub(crate) fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("bible-app/crossrefs")
}

/// The saved links for one Bible, keyed by its name and version, so an
/// updated module is worked out again.
pub(crate) fn cache_path(m: &Module) -> PathBuf {
    let ver: String = m
        .conf
        .get_or("Version", "0")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '_' })
        .collect();
    cache_dir().join(format!("{}-{ver}-links-v6.tsv", m.name))
}

/// Remove saved links from older formats, and this Bible's other versions.
pub(crate) fn prune_cache(keep: &std::path::Path, module: &str) {
    let Ok(rd) = std::fs::read_dir(cache_dir()) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        let old_format = !name.ends_with("-links-v6.tsv");
        let other_version = name.starts_with(&format!("{module}-")) && p != keep;
        if old_format || other_version {
            let _ = std::fs::remove_file(&p);
        }
    }
}

/// One verse-to-verse cross-reference: [from chapter index, from verse,
/// to chapter index, to verse].
pub(crate) type VLink = [u16; 4];

/// Verse-level links, placed on this Bible's chapters, from the built-in data.
pub(crate) fn compute(m: &Module, progress: &dyn Fn(f64)) -> (Vec<VLink>, String) {
    let chs = chapters(m);
    let index: HashMap<(String, u16), u16> = chs
        .iter()
        .enumerate()
        .map(|(i, (b, c, _))| ((b.clone(), *c), i as u16))
        .collect();
    // the Bible's own cross-references, when it has them
    if m.conf.features().contains(&bible_sword::conf::Feature::CrossRefs) {
        let mut own = Vec::new();
        for (i, (book, c, _)) in chs.iter().enumerate() {
            for (v, spans) in m.chapter(book, *c) {
                // references from every note: many Bibles (ISV among them) put
                // them in footnotes rather than separate cross-reference notes
                for n in spans.iter().filter_map(|s| s.note.as_ref()) {
                    for r in &n.refs {
                        if let Some(&y) = index.get(&(r.book.clone(), r.chapter)) {
                            own.push([i as u16, v.max(1), y, r.verse.max(1)]);
                        }
                    }
                }
            }
            if i % 50 == 0 {
                progress(i as f64 / chs.len().max(1) as f64);
            }
        }
        // the Bible's own links come first, even when there are few of them
        if !own.is_empty() {
            return (own, format!("Cross-references from {} itself", m.abbreviation()));
        }
    }
    let all = bible_core::crossrefs::all();
    let n = all.len().max(1);
    let mut out = Vec::with_capacity(n);
    for (k, (from, to, _votes)) in all.iter().enumerate() {
        let (Some(a), Some(b)) = (
            bible_core::refs::parse_osis(from),
            bible_core::refs::parse_osis(to.split('-').next().unwrap_or(to)),
        ) else {
            continue;
        };
        if let (Some(&x), Some(&y)) = (
            index.get(&(a.book, a.chapter)),
            index.get(&(b.book, b.chapter)),
        ) {
            out.push([x, a.verse.max(1), y, b.verse.max(1)]);
        }
        if k % 20_000 == 0 {
            progress(k as f64 / n as f64);
        }
    }
    (
        out,
        format!(
            "{} has no cross-references of its own, so these are OpenBible.info’s (CC BY 4.0, from the Treasury of Scripture Knowledge), built into the app and placed on {}’s chapters",
            m.abbreviation(),
            m.abbreviation()
        ),
    )
}

/// Chapter-to-chapter totals, for drawing the whole Bible quickly.
pub(crate) fn by_chapter(v: &[VLink]) -> Vec<(u16, u16, u16)> {
    let mut links: HashMap<(u16, u16), u32> = HashMap::new();
    for l in v {
        if l[0] != l[2] {
            *links.entry((l[0].min(l[2]), l[0].max(l[2]))).or_default() += 1;
        }
    }
    let mut out: Vec<(u16, u16, u16)> = links
        .into_iter()
        .map(|((a, b), c)| (a, b, c.min(u16::MAX as u32) as u16))
        .collect();
    out.sort();
    out
}

/// Ways to lay the books out along the drawing.
pub(crate) const ORDERS: [&str; 12] = [
    "Bible order",
    "Chronological",
    "Age written",
    "Writers",
    "Language",
    "Culture and setting",
    "Genre",
    "Hebrew Bible order",
    "Most referenced",
    "Densest",
    "Longest first",
    "Literary form",
];

/// What each order means, in a sentence.
pub(crate) const ORDER_HELP: [&str; 12] = [
    "The books as printed in most English Bibles, Genesis to Revelation.",
    "Books placed by when their events happen, so Job sits with the patriarchs and the prophets sit inside the kings.",
    "Books placed by when they were written, using traditional dates, in six eras from Moses to the apostles.",
    "Books grouped by who traditionally wrote them: Moses, David, Solomon, the prophets, Luke, John, Paul and others.",
    "Books grouped by the language they were first written in: Hebrew, Hebrew with Aramaic chapters, and Greek.",
    "Books grouped by the world they come from: Egypt and the wilderness, Canaan, Assyria, Babylon and Persia, Roman Judea, the Greco-Roman cities.",
    "Books grouped by kind: law, history, poetry and wisdom, prophets, apocalyptic, gospels, letters.",
    "The Jewish order of the Hebrew Bible (Torah, Prophets, Writings), then the New Testament.",
    "The books with the most cross-references in total come first.",
    "Books ranked by how densely they are cross-referenced for their length, so short, much-quoted books rise to the top.",
    "The longest books (by verses) first.",
    "Every chapter sorted by what kind of writing it is: songs and poems, parables, prayers, sermons, genealogies, law, visions, wisdom, prophecy, letters and narrative.",
];

/// What each view means.
pub(crate) const MODE_HELP: [&str; 6] = [
    "Rainbow: each arc joins two passages that refer to each other, the colour showing how far apart they are.",
    "Circle: the books run clockwise from the top, each chord a cross-reference; dense bundles show books that lean on each other.",
    "Matrix: a dot for every cross-reference, from across and to down. The diagonal is passages quoting their neighbours; squares and stripes are groups of books that echo each other, repeating at every zoom level like a fractal.",
    "Cube (3D): the matrix with depth. Across is where a reference starts, down is where it lands, and depth is how far back in time it reaches: references from late books to early ones sit at the back, ones to later writings at the front. Drag to turn it.",
    "Globe (3D): the Old Testament spirals over the northern half and the New Testament over the southern half, in the order you pick. Links within a Testament stay near the surface; links between the Testaments cut through the middle. Drag to turn it.",
    "Möbius strip (3D): a cross-reference joins two places, and with the Bible closed into a loop (Revelation hands back to Genesis) every such pair is one point on a Möbius strip. Round the strip is where a link is centred; its single edge holds links between neighbouring passages, its centre line links half the Bible apart. Drag to turn it.",
];

/// The literary form of a chapter, most specific first.
pub(crate) fn form(book: &str, ch: u16) -> &'static str {
    let any = |list: &[(&str, u16, u16)]| list.iter().any(|(b, a, z)| *b == book && (*a..=*z).contains(&ch));
    const GENEALOGY: &[(&str, u16, u16)] = &[("Gen", 5, 5), ("Gen", 10, 11), ("Gen", 36, 36), ("Gen", 46, 46), ("Num", 1, 1), ("Num", 26, 26), ("1Chr", 1, 9), ("Ezra", 2, 2), ("Neh", 7, 7), ("Matt", 1, 1), ("Luke", 3, 3)];
    const PARABLE: &[(&str, u16, u16)] = &[("Judg", 9, 9), ("2Sam", 12, 12), ("Ezek", 17, 17), ("Matt", 13, 13), ("Matt", 18, 18), ("Matt", 20, 22), ("Matt", 25, 25), ("Mark", 4, 4), ("Mark", 12, 12), ("Luke", 8, 8), ("Luke", 10, 10), ("Luke", 12, 16), ("Luke", 18, 20), ("John", 10, 10), ("John", 15, 15)];
    const SONG: &[(&str, u16, u16)] = &[("Exod", 15, 15), ("Deut", 32, 33), ("Judg", 5, 5), ("1Sam", 2, 2), ("2Sam", 1, 1), ("2Sam", 22, 23), ("1Chr", 16, 16), ("Ps", 1, 150), ("Song", 1, 8), ("Lam", 1, 5), ("Isa", 5, 5), ("Isa", 12, 12), ("Isa", 26, 26), ("Isa", 38, 38), ("Jonah", 2, 2), ("Hab", 3, 3), ("Luke", 1, 1), ("Phil", 2, 2), ("Col", 1, 1), ("Rev", 5, 5), ("Rev", 15, 15), ("Rev", 19, 19), ("Job", 3, 41)];
    const PRAYER: &[(&str, u16, u16)] = &[("1Kgs", 8, 8), ("2Chr", 6, 6), ("Ezra", 9, 9), ("Neh", 1, 1), ("Neh", 9, 9), ("Dan", 9, 9), ("John", 17, 17)];
    const LAW: &[(&str, u16, u16)] = &[("Exod", 20, 23), ("Exod", 25, 31), ("Exod", 34, 40), ("Lev", 1, 27), ("Num", 5, 6), ("Num", 15, 15), ("Num", 18, 19), ("Num", 28, 30), ("Deut", 12, 26)];
    const VISION: &[(&str, u16, u16)] = &[("Isa", 6, 6), ("Ezek", 1, 1), ("Ezek", 37, 37), ("Ezek", 40, 48), ("Dan", 7, 12), ("Amos", 7, 9), ("Zech", 1, 6), ("Rev", 1, 22)];
    const SERMON: &[(&str, u16, u16)] = &[("Deut", 1, 11), ("Deut", 27, 31), ("Josh", 23, 24), ("1Sam", 12, 12), ("Matt", 5, 7), ("Matt", 10, 10), ("Matt", 23, 24), ("Mark", 13, 13), ("Luke", 6, 6), ("Luke", 21, 21), ("John", 6, 6), ("John", 13, 16), ("Acts", 2, 2), ("Acts", 7, 7), ("Acts", 13, 13), ("Acts", 17, 17), ("Acts", 20, 20), ("Acts", 22, 22), ("Acts", 26, 26)];
    if any(GENEALOGY) {
        return "Genealogies";
    }
    if any(PARABLE) {
        return "Parables";
    }
    if any(SONG) {
        return "Songs and poems";
    }
    if any(PRAYER) {
        return "Prayers";
    }
    if any(LAW) {
        return "Law";
    }
    if any(VISION) {
        return "Visions";
    }
    if any(SERMON) {
        return "Sermons and speeches";
    }
    match book {
        "Prov" | "Eccl" => "Wisdom sayings",
        "Isa" | "Jer" | "Ezek" | "Dan" | "Hos" | "Joel" | "Amos" | "Obad" | "Mic" | "Nah" | "Hab" | "Zeph" | "Hag" | "Zech" | "Mal" => "Prophecy",
        "Rom" | "1Cor" | "2Cor" | "Gal" | "Eph" | "Phil" | "Col" | "1Thess" | "2Thess" | "1Tim" | "2Tim" | "Titus" | "Phlm" | "Heb" | "Jas" | "1Pet" | "2Pet" | "1John" | "2John" | "3John" | "Jude" => "Letters",
        _ => "Narrative",
    }
}

pub(crate) const FORMS: [&str; 11] = [
    "Songs and poems",
    "Parables",
    "Prayers",
    "Sermons and speeches",
    "Wisdom sayings",
    "Prophecy",
    "Visions",
    "Law",
    "Genealogies",
    "Letters",
    "Narrative",
];

pub(crate) type Buckets = &'static [(&'static str, &'static [&'static str])];

/// Traditional (conservative) dates of writing.
pub(crate) const AGE: Buckets = &[
    ("Before 1200 BC", &["Job", "Gen", "Exod", "Lev", "Num", "Deut", "Josh"]),
    ("United kingdom, 1050–930 BC", &["Judg", "Ruth", "1Sam", "2Sam", "Ps", "Song", "Prov", "Eccl"]),
    ("Divided kingdom, 930–586 BC", &["Obad", "Joel", "Jonah", "Amos", "Hos", "Mic", "Isa", "Nah", "Zeph", "Hab", "Jer"]),
    ("Exile, 586–538 BC", &["Lam", "1Kgs", "2Kgs", "Ezek", "Dan"]),
    ("Persian period, 538–400 BC", &["Hag", "Zech", "Esth", "1Chr", "2Chr", "Ezra", "Neh", "Mal"]),
    ("Roman era, AD 45–95", &["Jas", "Gal", "1Thess", "2Thess", "1Cor", "2Cor", "Rom", "Mark", "Matt", "Luke", "Eph", "Col", "Phlm", "Phil", "Acts", "1Tim", "Titus", "1Pet", "2Tim", "2Pet", "Heb", "Jude", "John", "1John", "2John", "3John", "Rev"]),
];

/// Traditional authors.
pub(crate) const WRITERS: Buckets = &[
    ("Moses", &["Gen", "Exod", "Lev", "Num", "Deut"]),
    ("Joshua", &["Josh"]),
    ("Samuel", &["Judg", "Ruth", "1Sam"]),
    ("Nathan and Gad", &["2Sam"]),
    ("Jeremiah", &["1Kgs", "2Kgs", "Jer", "Lam"]),
    ("Ezra", &["1Chr", "2Chr", "Ezra"]),
    ("Nehemiah", &["Neh"]),
    ("David and others", &["Ps"]),
    ("Solomon", &["Prov", "Eccl", "Song"]),
    ("Isaiah", &["Isa"]),
    ("Ezekiel", &["Ezek"]),
    ("Daniel", &["Dan"]),
    ("The Twelve", &["Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal"]),
    ("Unknown", &["Esth", "Job"]),
    ("Matthew", &["Matt"]),
    ("Mark", &["Mark"]),
    ("Peter", &["1Pet", "2Pet"]),
    ("Luke", &["Luke", "Acts"]),
    ("John", &["John", "1John", "2John", "3John", "Rev"]),
    ("Paul", &["Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm"]),
    ("Hebrews (unknown)", &["Heb"]),
    ("James", &["Jas"]),
    ("Jude", &["Jude"]),
];

pub(crate) const LANGUAGE: Buckets = &[
    ("Hebrew", &["Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sam", "2Sam", "1Kgs", "2Kgs", "1Chr", "2Chr", "Neh", "Esth", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer", "Lam", "Ezek", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal"]),
    ("Hebrew and Aramaic", &["Ezra", "Dan"]),
    ("Koine Greek", &["Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas", "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev"]),
];

/// Where and among whom the books are set.
pub(crate) const CULTURE: Buckets = &[
    ("Ancient Near East", &["Gen", "Job"]),
    ("Egypt and the wilderness", &["Exod", "Lev", "Num", "Deut"]),
    ("Israel among the Canaanites", &["Josh", "Judg", "Ruth", "1Sam", "2Sam", "Ps", "Prov", "Eccl", "Song"]),
    ("Israel and Judah under Assyria", &["1Kgs", "2Kgs", "Isa", "Hos", "Joel", "Amos", "Jonah", "Mic", "Nah"]),
    ("Judah under Babylon", &["Hab", "Zeph", "Jer", "Lam", "Obad"]),
    ("Exiles in Babylon and Persia", &["Ezek", "Dan", "Esth"]),
    ("Persian Judea", &["1Chr", "2Chr", "Ezra", "Neh", "Hag", "Zech", "Mal"]),
    ("Roman Judea and Galilee", &["Matt", "Mark", "Luke", "John", "Jas", "Heb"]),
    ("The Greco-Roman world", &["Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev"]),
];

pub(crate) const GENRE: Buckets = &[
    ("Law", &["Gen", "Exod", "Lev", "Num", "Deut"]),
    ("History", &["Josh", "Judg", "Ruth", "1Sam", "2Sam", "1Kgs", "2Kgs", "1Chr", "2Chr", "Ezra", "Neh", "Esth"]),
    ("Poetry and wisdom", &["Job", "Ps", "Prov", "Eccl", "Song"]),
    ("Major prophets", &["Isa", "Jer", "Lam", "Ezek"]),
    ("Minor prophets", &["Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal"]),
    ("Apocalyptic", &["Dan", "Rev"]),
    ("Gospels", &["Matt", "Mark", "Luke", "John"]),
    ("Church history", &["Acts"]),
    ("Paul’s letters", &["Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm"]),
    ("General letters", &["Heb", "Jas", "1Pet", "2Pet", "1John", "2John", "3John", "Jude"]),
];

pub(crate) const TANAKH: Buckets = &[
    ("Torah", &["Gen", "Exod", "Lev", "Num", "Deut"]),
    ("Nevi’im (Prophets)", &["Josh", "Judg", "1Sam", "2Sam", "1Kgs", "2Kgs", "Isa", "Jer", "Ezek", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal"]),
    ("Ketuvim (Writings)", &["Ps", "Prov", "Job", "Song", "Ruth", "Lam", "Eccl", "Esth", "Dan", "Ezra", "Neh", "1Chr", "2Chr"]),
    ("New Testament", &["Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas", "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev"]),
];

pub(crate) fn buckets(kind: u32) -> Option<Buckets> {
    Some(match kind {
        2 => AGE,
        3 => WRITERS,
        4 => LANGUAGE,
        5 => CULTURE,
        6 => GENRE,
        7 => TANAKH,
        _ => return None,
    })
}

