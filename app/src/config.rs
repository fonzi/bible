//! User settings, stored as JSON in ~/.config/bible-app/config.json.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceCfg {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    #[serde(default)]
    pub custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HighlightColour {
    pub id: String,
    pub name: String,
    pub hex: String,
}

pub fn default_palette() -> Vec<HighlightColour> {
    [
        ("promises", "Promises", "#f5c211"),
        ("christ", "Christ", "#3584e4"),
        ("sin", "Sin and judgement", "#e01b24"),
        ("commands", "Commands", "#2ec27e"),
        ("prayer", "Prayer", "#9141ac"),
        ("history", "History", "#e66100"),
    ]
    .iter()
    .map(|(id, n, h)| HighlightColour {
        id: id.to_string(),
        name: n.to_string(),
        hex: h.to_string(),
    })
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// "system", "light" or "dark"
    pub style: String,
    pub text_size: f64,
    pub red_letters: bool,
    pub verse_numbers: bool,
    pub show_apocrypha: bool,
    pub feature_tips: bool,
    pub notes_dir: PathBuf,
    pub obsidian_links: bool,
    pub tradition: String,
    pub legal_notice: bool,
    /// look for a new version once a day
    pub check_updates: bool,
    /// the day updates were last checked (YYYY-MM-DD)
    pub update_checked: String,
    pub sources: Vec<SourceCfg>,
    pub palette: Vec<HighlightColour>,
    pub bible: String,
    pub parallel: String,
    pub commentary: String,
    pub book: String,
    pub chapter: u16,
    pub dismissed_tips: Vec<String>,
    /// "devotional" or "off"
    pub daily_card: String,
    pub devotional: String,
    /// built-in passage of the day on Today
    pub daily_passage: bool,
    /// fasting calendar on Today (opt in)
    pub fasting: bool,
    /// classic typography: decorated initials, old-style type, on the normal page (opt in)
    pub vintage: bool,
    /// splash screen at start (opt in) and its style
    pub splash: bool,
    pub splash_style: String,
    /// loading animation: an id from loader::STYLES
    pub loader: String,
    /// a short hymn when the app opens (opt in): an id from sounds::SOUNDS
    pub startup_sound: String,
    /// bumped when a default must be applied again to existing configs
    pub defaults_version: u32,
    /// the reading plan being followed
    pub plan: Option<bible_core::plans::PlanState>,
    /// bookmarked verses, "John.3.16", with their tags
    pub bookmarks: Vec<Bookmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Bookmark {
    pub osis: String,
    pub tags: Vec<String>,
    /// "2026-11-16"
    pub added: String,
}

impl Default for Config {
    fn default() -> Self {
        let notes = dirs::document_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Bible Notes");
        Config {
            style: "system".into(),
            text_size: 15.0,
            red_letters: true,
            verse_numbers: true,
            show_apocrypha: false,
            feature_tips: true,
            notes_dir: notes,
            obsidian_links: true,
            tradition: "Western".into(),
            legal_notice: true,
            check_updates: true,
            update_checked: String::new(),
            sources: bible_sword::install::default_sources()
                .into_iter()
                .map(|s| SourceCfg {
                    name: s.name,
                    url: s.url,
                    enabled: s.enabled,
                    custom: false,
                })
                .collect(),
            palette: default_palette(),
            bible: String::new(),
            parallel: String::new(),
            commentary: String::new(),
            book: "John".into(),
            chapter: 1,
            dismissed_tips: vec![],
            daily_card: "devotional".into(),
            plan: None,
            daily_passage: true,
            fasting: false,
            vintage: false,
            splash: false,
            splash_style: "minimal".into(),
            loader: "cross".into(),
            startup_sound: "off".into(),
            defaults_version: DEFAULTS_VERSION,
            bookmarks: vec![],
            devotional: String::new(),
        }
    }
}

const DEFAULTS_VERSION: u32 = 1;

fn path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bible-app")
        .join("config.json")
}

impl Config {
    pub fn load() -> Config {
        let mut c: Config = std::fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        if c.defaults_version < 1 {
            // the legal notice is opt-out: earlier versions turned it off with one click
            c.legal_notice = true;
        }
        c.defaults_version = DEFAULTS_VERSION;
        c
    }

    pub fn save(&self) {
        let p = path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    }

    pub fn colour(&self, id: &str) -> Option<&HighlightColour> {
        self.palette.iter().find(|c| c.id == id)
    }

    pub fn sources(&self) -> Vec<bible_sword::Source> {
        self.sources
            .iter()
            .filter(|s| s.enabled)
            .map(|s| bible_sword::Source {
                name: s.name.clone(),
                url: s.url.clone(),
                enabled: true,
            })
            .collect()
    }
}
