//! Church history articles for offline reading. The events on Today are one
//! line each, with no article of their own; when the person asks, each event
//! is looked up with Wikipedia's own search and the closest article's summary
//! is saved. The match is Wikipedia's, and the app says so.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Article {
    pub title: String,
    pub extract: String,
}

fn path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("bible-app/history.json")
}

/// The key an event is saved under: its date and year.
pub fn key(month: u32, day: u32, year: i32) -> String {
    format!("{month}-{day}-{year}")
}

pub fn load() -> BTreeMap<String, Article> {
    std::fs::read(path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn is_downloaded() -> bool {
    path().exists()
}

fn get_json(url: &str) -> std::io::Result<serde_json::Value> {
    let b = bible_sword::install::get_bytes(url)?
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Wikipedia sent nothing"))?;
    serde_json::from_slice(&b).map_err(std::io::Error::other)
}

/// Look up every event and save the closest article's summary. Events
/// Wikipedia finds nothing for are left out.
pub fn download(progress: &mut dyn FnMut(usize, usize)) -> std::io::Result<usize> {
    let events: Vec<_> = bible_core::onthisday::events().collect();
    let mut out = BTreeMap::new();
    for (i, e) in events.iter().enumerate() {
        progress(i, events.len());
        let q = urlencoding::encode(e.text.trim_end_matches('.'));
        let search = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srlimit=1&format=json&srsearch={q}"
        );
        let Ok(found) = get_json(&search) else { continue };
        let Some(title) = found["query"]["search"][0]["title"].as_str() else { continue };
        let summary = format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
            urlencoding::encode(&title.replace(' ', "_"))
        );
        let Ok(s) = get_json(&summary) else { continue };
        if let Some(extract) = s["extract"].as_str().filter(|t| !t.is_empty()) {
            out.insert(
                key(e.month, e.day, e.year),
                Article { title: title.to_string(), extract: extract.to_string() },
            );
        }
    }
    progress(events.len(), events.len());
    if out.is_empty() {
        return Err(std::io::Error::other("Wikipedia couldn’t be reached"));
    }
    if let Some(d) = path().parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path(), serde_json::to_vec(&out).map_err(std::io::Error::other)?)?;
    Ok(out.len())
}

#[cfg(test)]
mod url_checks {
    /// One event looked up the way the download does: Wikipedia's search
    /// finds an article, and its summary has text.
    #[test]
    #[ignore = "needs the internet; run with --ignored download_urls"]
    fn download_urls_history_lookup() {
        let q = urlencoding::encode("The Council of Trent decrees the canon of Scripture");
        let found = super::get_json(&format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srlimit=1&format=json&srsearch={q}"
        ))
        .unwrap();
        let title = found["query"]["search"][0]["title"].as_str().expect("a search result");
        let s = super::get_json(&format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
            urlencoding::encode(&title.replace(' ', "_"))
        ))
        .unwrap();
        assert!(s["extract"].as_str().is_some_and(|t| t.len() > 50), "{title} has a summary");
    }
}
