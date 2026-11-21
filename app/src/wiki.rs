//! Full Wikipedia articles for the book introductions: text, links and the
//! article's freely licensed pictures, fetched once on the user's computer and
//! kept for offline reading. Text is CC BY-SA 4.0; each picture keeps its own
//! author and licence from Wikimedia Commons.

use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

const API: &str = "https://en.wikipedia.org/w/api.php";
const SITE: &str = "https://en.wikipedia.org";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Block {
    Heading { level: u8, text: String },
    /// Pango markup, links kept (they open Wikipedia)
    Para { markup: String },
    Image {
        /// file inside the book's folder
        file: String,
        caption: String,
        artist: String,
        license: String,
        /// the picture's page on Wikimedia Commons
        page: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Article {
    pub title: String,
    pub url: String,
    pub fetched: String,
    pub blocks: Vec<Block>,
}

pub fn dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("bible-app/wikipedia")
}

fn book_dir(osis: &str) -> PathBuf {
    dir().join(osis)
}

/// The saved article for a book, if it has been downloaded.
pub fn load(osis: &str) -> Option<(Article, PathBuf)> {
    let d = book_dir(osis);
    let text = std::fs::read_to_string(d.join("article.json")).ok()?;
    Some((serde_json::from_str(&text).ok()?, d))
}

/// Fetch with one retry after a pause when Wikipedia is busy or rate-limits.
fn get_retry(url: &str) -> io::Result<Vec<u8>> {
    let mut last = None;
    for attempt in 0..3 {
        if attempt > 0 {
            // rate limited: back off 5 s, then 15 s
            std::thread::sleep(std::time::Duration::from_secs([0, 5, 15][attempt as usize]));
        }
        match bible_sword::install::get_bytes(url) {
            Ok(Some(b)) => return Ok(b),
            Ok(None) => return Err(io::Error::new(io::ErrorKind::NotFound, "page not found on Wikipedia")),
            Err(e) => {
                let busy = ["429", "503", "502", "timed out", "reset"].iter().any(|k| e.to_string().contains(k));
                last = Some(e);
                if !busy {
                    break;
                }
            }
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("download failed")))
}

fn get_json(url: &str) -> io::Result<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_slice(&get_retry(url)?).map_err(io::Error::other)?;
    if let Some(info) = v["error"]["info"].as_str() {
        return Err(io::Error::other(format!("Wikipedia says: {info}")));
    }
    Ok(v)
}

/// A short, readable reason for a failed download.
pub fn reason(e: &io::Error) -> String {
    let t = e.to_string();
    if t.contains("429") {
        "Wikipedia asked us to slow down; try again in a minute".into()
    } else if t.contains("403") {
        "Wikipedia refused the request (403)".into()
    } else if t.contains("dns") || t.contains("resolve") || t.contains("Connection refused") || t.contains("connect") {
        "no internet connection".into()
    } else if t.contains("timed out") {
        "the connection timed out".into()
    } else {
        t.chars().take(120).collect()
    }
}

/// How many books have their article saved.
pub fn count() -> usize {
    std::fs::read_dir(dir())
        .map(|r| r.flatten().filter(|e| e.path().join("article.json").exists()).count())
        .unwrap_or(0)
}

/// Delete a saved article.
pub fn remove(osis: &str) {
    let _ = std::fs::remove_dir_all(book_dir(osis));
}

/// Sections that are lists of sources rather than reading.
const SKIP: &[&str] = &[
    "See also",
    "References",
    "Notes",
    "Citations",
    "Sources",
    "Bibliography",
    "Further reading",
    "External links",
    "Works cited",
    "Footnotes",
    "Explanatory notes",
    "Commentaries",
    "Cited works",
];

fn esc(t: &str) -> String {
    gtk::glib::markup_escape_text(t).to_string()
}

fn abs_link(href: &str) -> Option<String> {
    if href.starts_with("/wiki/") {
        Some(format!("{SITE}{href}"))
    } else if href.starts_with("//") {
        Some(format!("https:{href}"))
    } else if href.starts_with("http") {
        Some(href.to_string())
    } else {
        None
    }
}

/// Inline HTML to Pango markup, keeping links, bold and italics, dropping
/// footnote markers and edit links.
fn inline(el: ElementRef, out: &mut String) {
    for child in el.children() {
        match child.value() {
            Node::Text(t) => out.push_str(&esc(t)),
            Node::Element(e) => {
                let Some(c) = ElementRef::wrap(child) else { continue };
                let classes: Vec<&str> = e.classes().collect();
                let name = e.name();
                if matches!(name, "sup" | "style" | "script" | "link" | "meta")
                    && (name != "sup" || classes.iter().any(|c| *c == "reference" || *c == "noprint"))
                {
                    continue;
                }
                if classes.iter().any(|c| matches!(*c, "mw-editsection" | "noprint" | "reference" | "mw-ref" | "error")) {
                    continue;
                }
                match name {
                    "b" | "strong" => {
                        out.push_str("<b>");
                        inline(c, out);
                        out.push_str("</b>");
                    }
                    "i" | "em" | "cite" => {
                        out.push_str("<i>");
                        inline(c, out);
                        out.push_str("</i>");
                    }
                    "a" => {
                        let href = e.attr("href").and_then(abs_link);
                        match href {
                            Some(h) if !h.contains("action=edit") => {
                                out.push_str(&format!("<a href=\"{}\">", esc(&h)));
                                inline(c, out);
                                out.push_str("</a>");
                            }
                            _ => inline(c, out),
                        }
                    }
                    "br" => out.push('\n'),
                    "sub" => {
                        out.push_str("<sub>");
                        inline(c, out);
                        out.push_str("</sub>");
                    }
                    "sup" => {
                        out.push_str("<sup>");
                        inline(c, out);
                        out.push_str("</sup>");
                    }
                    _ => inline(c, out),
                }
            }
            _ => {}
        }
    }
}

fn markup_of(el: ElementRef) -> String {
    let mut s = String::new();
    inline(el, &mut s);
    // tidy whitespace
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.replace(" ,", ",").replace(" .", ".").trim().to_string()
}

fn plain(el: ElementRef) -> String {
    el.text().collect::<Vec<_>>().join("").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A picture found in the article: (File: title, caption markup).
struct Found {
    file: String,
    caption: String,
}

/// Turn the article HTML into blocks; pictures are recorded by file name and
/// filled in after their licences are checked.
fn parse(html: &str) -> (Vec<Block>, Vec<(usize, Found)>) {
    let doc = Html::parse_fragment(html);
    let root_sel = Selector::parse(".mw-parser-output").unwrap();
    let root = doc
        .select(&root_sel)
        .next()
        .unwrap_or_else(|| doc.root_element());
    let img_link = Selector::parse("a[href^=\"/wiki/File:\"]").unwrap();
    let caption_sel = Selector::parse("figcaption, .thumbcaption, .infobox-caption").unwrap();
    let li_sel = Selector::parse(":scope > li").unwrap();
    let mut blocks: Vec<Block> = vec![];
    let mut pics: Vec<(usize, Found)> = vec![];
    let mut skipping = false;
    let mut seen = std::collections::HashSet::new();
    let mut add_pic = |blocks: &mut Vec<Block>, pics: &mut Vec<(usize, Found)>, el: ElementRef| {
        if let Some(a) = el.select(&img_link).next() {
            let file = a.value().attr("href").unwrap_or("").trim_start_matches("/wiki/").to_string();
            let file = urlencoding::decode(&file).map(|s| s.into_owned()).unwrap_or(file);
            if seen.insert(file.clone()) {
                let caption = el.select(&caption_sel).next().map(markup_of).unwrap_or_default();
                pics.push((blocks.len(), Found { file, caption }));
            }
        }
    };
    for child in root.children() {
        let Some(el) = ElementRef::wrap(child) else { continue };
        let name = el.value().name();
        let classes: Vec<&str> = el.value().classes().collect();
        // headings: <h2> or <div class="mw-heading"><h2>
        let heading = if matches!(name, "h2" | "h3" | "h4") {
            Some((name[1..].parse::<u8>().unwrap_or(2), plain(el)))
        } else if classes.contains(&"mw-heading") {
            el.children()
                .filter_map(ElementRef::wrap)
                .find(|h| matches!(h.value().name(), "h2" | "h3" | "h4"))
                .map(|h| (h.value().name()[1..].parse::<u8>().unwrap_or(2), plain(h)))
        } else {
            None
        };
        if let Some((level, text)) = heading {
            if level == 2 {
                skipping = SKIP.iter().any(|s| text.eq_ignore_ascii_case(s));
            }
            if !skipping {
                blocks.push(Block::Heading { level, text });
            }
            continue;
        }
        if skipping {
            continue;
        }
        match name {
            "p" => {
                let m = markup_of(el);
                if !m.is_empty() {
                    blocks.push(Block::Para { markup: m });
                }
            }
            "ul" | "ol" => {
                if classes.iter().any(|c| matches!(*c, "gallery")) {
                    for li in el.select(&li_sel) {
                        add_pic(&mut blocks, &mut pics, li);
                    }
                    continue;
                }
                for (i, li) in el.select(&li_sel).enumerate() {
                    let m = markup_of(li);
                    if !m.is_empty() {
                        let bullet = if name == "ol" { format!("{}.", i + 1) } else { "•".into() };
                        blocks.push(Block::Para { markup: format!("{bullet} {m}") });
                    }
                }
            }
            "blockquote" => {
                let m = markup_of(el);
                if !m.is_empty() {
                    blocks.push(Block::Para { markup: format!("<i>{m}</i>") });
                }
            }
            "figure" => add_pic(&mut blocks, &mut pics, el),
            "table" if classes.iter().any(|c| c.starts_with("infobox")) => {
                add_pic(&mut blocks, &mut pics, el)
            }
            "div" if classes.iter().any(|c| matches!(*c, "thumb" | "tmulti" | "tright" | "tleft")) => {
                add_pic(&mut blocks, &mut pics, el)
            }
            _ => {}
        }
    }
    (blocks, pics)
}

fn strip_tags(html: &str) -> String {
    let frag = Html::parse_fragment(html);
    frag.root_element()
        .text()
        .collect::<Vec<_>>()
        .join("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fetch and save the article for one book. `article` is the Wikipedia title.
pub fn fetch(osis: &str, article: &str) -> io::Result<Article> {
    let title = urlencoding::encode(article);
    let j = get_json(&format!(
        "{API}?action=parse&format=json&formatversion=2&redirects=1&prop=text&disableeditsection=1&disabletoc=1&page={title}"
    ))?;
    let html = j["parse"]["text"]
        .as_str()
        .ok_or_else(|| io::Error::other("no article text"))?;
    let real_title = j["parse"]["title"].as_str().unwrap_or(article).to_string();
    let (mut blocks, pics) = parse(html);
    let d = book_dir(osis);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d)?;
    // check every picture's licence: only freely licensed Commons files
    let names: Vec<&str> = pics.iter().map(|(_, f)| f.file.as_str()).take(40).collect();
    let mut info: std::collections::HashMap<String, serde_json::Value> = Default::default();
    for chunk in names.chunks(20) {
        let titles = urlencoding::encode(&chunk.join("|")).into_owned();
        if let Ok(j) = get_json(&format!(
            "{API}?action=query&format=json&formatversion=2&prop=imageinfo&iiprop=url|extmetadata|mime&iiurlwidth=960&titles={titles}"
        )) {
            for p in j["query"]["pages"].as_array().into_iter().flatten() {
                let t = p["title"].as_str().unwrap_or("").to_string();
                if let Some(ii) = p["imageinfo"].get(0) {
                    info.insert(t, ii.clone());
                }
            }
            // normalised titles (underscores to spaces)
            for n in j["query"]["normalized"].as_array().into_iter().flatten() {
                if let (Some(from), Some(to)) = (n["from"].as_str(), n["to"].as_str()) {
                    if let Some(v) = info.get(to).cloned() {
                        info.insert(from.to_string(), v);
                    }
                }
            }
        }
    }
    // insert pictures from the end so earlier positions stay right
    let mut kept = 0;
    for (pos, f) in pics.into_iter().rev() {
        let Some(ii) = info.get(&f.file).or_else(|| info.get(&f.file.replace('_', " "))) else {
            continue;
        };
        let url = ii["url"].as_str().unwrap_or("");
        // non-free files live on en.wikipedia, not on Commons: leave them out
        if !url.contains("/wikipedia/commons/") {
            continue;
        }
        let mime = ii["mime"].as_str().unwrap_or("");
        let Some(thumb) = ii["thumburl"].as_str() else { continue };
        let w = ii["thumbwidth"].as_u64().unwrap_or(0);
        if w < 200 || !(mime.starts_with("image/jpeg") || mime.starts_with("image/png") || mime.starts_with("image/svg") || mime.starts_with("image/tiff") || mime.starts_with("image/gif")) {
            continue;
        }
        let meta = &ii["extmetadata"];
        let license = meta["LicenseShortName"]["value"].as_str().unwrap_or("").to_string();
        if license.is_empty() || license.to_lowercase().contains("fair use") {
            continue;
        }
        let artist = strip_tags(meta["Artist"]["value"].as_str().unwrap_or(""));
        let ext = if thumb.ends_with(".png") || mime.contains("svg") || mime.contains("png") { "png" } else { "jpg" };
        let file = format!("{pos}-{kept}.{ext}");
        // pictures come from a stricter server: one at a time, gently
        std::thread::sleep(std::time::Duration::from_millis(600));
        let Ok(bytes) = get_retry(thumb) else { continue };
        if std::fs::write(d.join(&file), bytes).is_err() {
            continue;
        }
        kept += 1;
        let caption = if f.caption.is_empty() {
            esc(&strip_tags(meta["ImageDescription"]["value"].as_str().unwrap_or("")).chars().take(200).collect::<String>())
        } else {
            f.caption
        };
        blocks.insert(
            pos.min(blocks.len()),
            Block::Image {
                file,
                caption,
                artist,
                license,
                page: ii["descriptionurl"].as_str().unwrap_or("").to_string(),
            },
        );
    }
    let a = Article {
        url: format!("{SITE}/wiki/{}", urlencoding::encode(&real_title.replace(' ', "_"))),
        title: real_title,
        fetched: chrono::Local::now().format("%Y-%m-%d").to_string(),
        blocks,
    };
    std::fs::write(d.join("article.json"), serde_json::to_string(&a)?)?;
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"<div class="mw-content-ltr mw-parser-output" lang="en" dir="ltr">
<div class="shortdescription nomobile noexcerpt noprint searchaux" style="display:none">Book of the Bible</div>
<table class="infobox"><tbody><tr><td><span typeof="mw:File"><a href="/wiki/File:Papyrus_66.jpg" class="mw-file-description"><img src="//upload.wikimedia.org/x.jpg"></a></span><div class="infobox-caption">Papyrus 66</div></td></tr></tbody></table>
<p>The <b>Gospel of John</b> is the fourth of the <a href="/wiki/Gospel" title="Gospel">gospels</a>.<sup id="cite_ref-1" class="reference"><a href="#cite_note-1">[1]</a></sup></p>
<div class="mw-heading mw-heading2"><h2 id="Composition">Composition</h2></div>
<figure class="mw-default-size" typeof="mw:File/Thumb"><a href="/wiki/File:John_Rylands.jpg" class="mw-file-description"><img src="//upload.wikimedia.org/y.jpg"></a><figcaption>The <a href="/wiki/Rylands_Library_Papyrus_P52">Rylands fragment</a></figcaption></figure>
<p>Written <i>c.</i> AD 90&ndash;110 &amp; later.</p>
<ul><li>First point</li><li>Second <a href="https://example.org">point</a></li></ul>
<div class="mw-heading mw-heading2"><h2 id="References">References</h2></div>
<p>Should be skipped.</p>
</div>"##;

    #[test]
    fn parses_article() {
        let (blocks, pics) = parse(SAMPLE);
        let paras: Vec<String> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Para { markup } => Some(markup.clone()),
                _ => None,
            })
            .collect();
        assert!(paras[0].contains("<b>Gospel of John</b>"));
        assert!(paras[0].contains("<a href=\"https://en.wikipedia.org/wiki/Gospel\">gospels</a>"));
        assert!(!paras[0].contains("[1]"), "{}", paras[0]);
        assert!(paras.iter().any(|p| p.contains("&amp; later")));
        assert!(paras.iter().any(|p| p.starts_with("• Second")));
        assert!(!paras.iter().any(|p| p.contains("skipped")));
        assert!(blocks.iter().any(|b| matches!(b, Block::Heading { text, .. } if text == "Composition")));
        assert_eq!(pics.len(), 2);
        assert_eq!(pics[0].1.file, "File:Papyrus_66.jpg");
        assert!(pics[1].1.caption.contains("Rylands fragment"));
        // every paragraph is valid markup (GTK labels add <a> on top of Pango)
        for p in &paras {
            let mut q = p.replace("</a>", "");
            while let Some(i) = q.find("<a href=") {
                let j = q[i..].find('>').map(|j| i + j + 1).unwrap_or(q.len());
                q.replace_range(i..j, "");
            }
            assert!(gtk::pango::parse_markup(&q, '\0').is_ok(), "{p}");
        }
    }
}
