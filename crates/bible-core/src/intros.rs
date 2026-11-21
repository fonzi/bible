//! Introductions to each book of the Bible, built in so they work offline.
//! Text from Wikipedia (CC BY-SA 4.0), plus a short line giving the traditional
//! author and date. Built by tools/mkintros.py.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Intro {
    /// Wikipedia article title
    pub article: String,
    pub text: String,
    /// traditional author and date
    pub traditional: String,
}

impl Intro {
    pub fn url(&self) -> String {
        format!(
            "https://en.wikipedia.org/wiki/{}",
            self.article.replace(' ', "_")
        )
    }

    pub fn credit(&self) -> String {
        format!(
            "From the Wikipedia article “{}”, CC BY-SA 4.0.",
            self.article
        )
    }
}

pub fn intro(book: &str) -> Option<&'static Intro> {
    static I: Lazy<HashMap<String, Intro>> = Lazy::new(|| {
        serde_json::from_str(include_str!("../data/intros.json")).expect("intros.json")
    });
    I.get(book)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_book() {
        for b in crate::v11n::kjv().books.iter().take(66) {
            let i = super::intro(&b.osis).unwrap_or_else(|| panic!("{}", b.osis));
            assert!(i.text.len() > 100 && !i.traditional.is_empty());
        }
    }
}
