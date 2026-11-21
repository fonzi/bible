//! Cross-references for the whole Bible, built in: OpenBible.info's set
//! (CC BY 4.0), largely from the public-domain Treasury of Scripture
//! Knowledge, with community votes. About 340,000 links.

use std::io::Read;

pub const CREDIT: &str = "Cross-references: OpenBible.info (CC BY 4.0), based on the Treasury of Scripture Knowledge.";

/// Every link as (from OSIS, to OSIS or range, votes).
pub fn all() -> Vec<(String, String, i32)> {
    let mut s = String::new();
    let _ = flate2::read::GzDecoder::new(&include_bytes!("../data/crossrefs.tsv.gz")[..]).read_to_string(&mut s);
    s.lines()
        .filter_map(|l| {
            let mut p = l.split('\t');
            Some((p.next()?.to_string(), p.next()?.to_string(), p.next()?.parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn loads() {
        let a = super::all();
        assert!(a.len() > 300_000, "{}", a.len());
        assert!(a.iter().any(|(f, t, _)| f == "Gen.1.1" && t.starts_with("John.1.1")));
    }
}
