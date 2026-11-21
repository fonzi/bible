//! Plain-English descriptions of grammar codes: Robinson codes for Greek
//! (V-AAI-3S) and the OpenScriptures/STEP codes for Hebrew (HVqp3ms).

fn greek_case(c: char) -> Option<&'static str> {
    Some(match c {
        'N' => "nominative",
        'G' => "genitive",
        'D' => "dative",
        'A' => "accusative",
        'V' => "vocative",
        _ => return None,
    })
}
fn number(c: char) -> Option<&'static str> {
    Some(match c {
        'S' | 's' => "singular",
        'P' | 'p' => "plural",
        'd' => "dual",
        _ => return None,
    })
}
fn gender(c: char) -> Option<&'static str> {
    Some(match c {
        'M' | 'm' => "masculine",
        'F' | 'f' => "feminine",
        'N' => "neuter",
        'c' => "common",
        _ => return None,
    })
}
fn person(c: char) -> Option<&'static str> {
    Some(match c {
        '1' => "1st person",
        '2' => "2nd person",
        '3' => "3rd person",
        _ => return None,
    })
}

fn cng(s: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let ch: Vec<char> = s.chars().collect();
    if let Some(&c) = ch.first() {
        if let Some(x) = greek_case(c) {
            out.push(x);
        }
    }
    if let Some(&c) = ch.get(1) {
        if let Some(x) = number(c) {
            out.push(x);
        }
    }
    if let Some(&c) = ch.get(2) {
        if let Some(x) = gender(c) {
            out.push(x);
        }
    }
    out
}

pub fn greek(code: &str) -> String {
    let parts: Vec<&str> = code.split('-').collect();
    let head = parts.first().copied().unwrap_or("");
    let name = match head {
        "N" => "noun",
        "A" => "adjective",
        "T" => "article",
        "V" => "verb",
        "P" => "personal pronoun",
        "R" => "relative pronoun",
        "C" => "reciprocal pronoun",
        "D" => "demonstrative pronoun",
        "K" => "correlative pronoun",
        "I" => "interrogative pronoun",
        "X" => "indefinite pronoun",
        "Q" => "correlative or interrogative pronoun",
        "F" => "reflexive pronoun",
        "S" => "possessive pronoun",
        "ADV" => "adverb",
        "CONJ" => "conjunction",
        "COND" => "conditional particle",
        "PRT" => "particle",
        "PREP" => "preposition",
        "INJ" => "interjection",
        "ARAM" => "Aramaic word",
        "HEB" => "Hebrew word",
        _ => return code.to_string(),
    };
    let mut out = vec![name.to_string()];
    if head == "V" {
        if let Some(tvm) = parts.get(1) {
            let t: Vec<char> = tvm.chars().collect();
            let tense = match t.first() {
                Some('P') => "present",
                Some('I') => "imperfect",
                Some('F') => "future",
                Some('A') => "aorist",
                Some('R') => "perfect",
                Some('L') => "pluperfect",
                Some('2') => "second aorist",
                _ => "",
            };
            let skip = if t.first() == Some(&'2') { 2 } else { 1 };
            let voice = match t.get(skip) {
                Some('A') => "active",
                Some('M') => "middle",
                Some('P') => "passive",
                Some('E') | Some('D') => "middle or passive",
                Some('O') => "passive deponent",
                Some('N') => "middle deponent",
                _ => "",
            };
            let mood = match t.get(skip + 1) {
                Some('I') => "indicative",
                Some('S') => "subjunctive",
                Some('O') => "optative",
                Some('M') => "imperative",
                Some('N') => "infinitive",
                Some('P') => "participle",
                _ => "",
            };
            let tvm_s: Vec<&str> = [tense, voice, mood]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect();
            if !tvm_s.is_empty() {
                out.push(tvm_s.join(" "));
            }
        }
        if let Some(rest) = parts.get(2) {
            let r: Vec<char> = rest.chars().collect();
            if r.first().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                let mut x = Vec::new();
                if let Some(p) = r.first().and_then(|c| person(*c)) {
                    x.push(p);
                }
                if let Some(n) = r.get(1).and_then(|c| number(*c)) {
                    x.push(n);
                }
                out.push(x.join(" "));
            } else {
                out.push(cng(rest).join(" "));
            }
        }
    } else if let Some(rest) = parts.get(1) {
        if head == "P" || head == "S" || head == "F" {
            let r: Vec<char> = rest.chars().collect();
            if let Some(p) = r.first().and_then(|c| person(*c)) {
                out.push(p.to_string());
                out.push(cng(&rest[1..]).join(" "));
            } else {
                out.push(cng(rest).join(" "));
            }
        } else {
            let d = cng(rest).join(" ");
            if !d.is_empty() {
                out.push(d);
            }
        }
    }
    out.retain(|s| !s.is_empty());
    out.join(", ")
}

fn hebrew_one(seg: &str) -> String {
    let c: Vec<char> = seg.chars().collect();
    if c.is_empty() {
        return String::new();
    }
    let rest = &c[1..];
    match c[0] {
        'V' => {
            let stem = match rest.first() {
                Some('q') => "Qal",
                Some('N') => "Niphal",
                Some('p') => "Piel",
                Some('P') => "Pual",
                Some('h') => "Hiphil",
                Some('H') => "Hophal",
                Some('t') => "Hithpael",
                Some('o') => "Polel",
                Some('O') => "Polal",
                Some('r') => "Hithpolel",
                Some('Q') => "Qal passive",
                _ => "",
            };
            let form = match rest.get(1) {
                Some('p') => "perfect",
                Some('q') => "sequential perfect",
                Some('i') => "imperfect",
                Some('w') => "sequential imperfect",
                Some('h') => "cohortative",
                Some('j') => "jussive",
                Some('v') => "imperative",
                Some('r') => "participle (active)",
                Some('s') => "participle (passive)",
                Some('a') => "infinitive absolute",
                Some('c') => "infinitive construct",
                _ => "",
            };
            let mut x = vec!["verb".to_string()];
            let sf: Vec<&str> = [stem, form].into_iter().filter(|s| !s.is_empty()).collect();
            if !sf.is_empty() {
                x.push(sf.join(" "));
            }
            let png: Vec<&str> = rest
                .iter()
                .skip(2)
                .filter_map(|ch| person(*ch).or_else(|| gender(*ch)).or_else(|| number(*ch)))
                .collect();
            if !png.is_empty() {
                x.push(png.join(" "));
            }
            x.join(", ")
        }
        'N' | 'A' => {
            let kind = match (c[0], rest.first()) {
                ('N', Some('p')) => "proper noun",
                ('N', Some('g')) => "gentilic noun",
                ('N', _) => "noun",
                ('A', Some('o')) => "ordinal number",
                ('A', Some('c')) => "cardinal number",
                _ => "adjective",
            };
            let mut x = vec![kind.to_string()];
            let gn: Vec<&str> = rest
                .iter()
                .skip(1)
                .filter_map(|ch| gender(*ch).or_else(|| number(*ch)))
                .collect();
            if !gn.is_empty() {
                x.push(gn.join(" "));
            }
            match rest.last() {
                Some('c') if rest.len() >= 3 => x.push("construct".into()),
                Some('a') if rest.len() >= 3 => x.push("absolute".into()),
                Some('d') if rest.len() >= 3 => x.push("determined".into()),
                _ => {}
            }
            x.join(", ")
        }
        'R' => "preposition".into(),
        'C' => "conjunction".into(),
        'D' => "adverb".into(),
        'T' => match rest.first() {
            Some('d') => "definite article".into(),
            Some('o') => "direct object marker".into(),
            Some('n') => "negative particle".into(),
            Some('i') => "interrogative particle".into(),
            Some('r') => "relative particle".into(),
            _ => "particle".into(),
        },
        'P' => "pronoun".into(),
        'S' => "suffix".into(),
        _ => seg.to_string(),
    }
}

pub fn hebrew(code: &str) -> String {
    // "HR/Ncfsa": language letter, then morphemes separated by '/'
    let lang = if code.starts_with('A') {
        "Aramaic"
    } else {
        "Hebrew"
    };
    let body = code.get(1..).unwrap_or("");
    let parts: Vec<String> = body
        .split('/')
        .map(hebrew_one)
        .filter(|s| !s.is_empty())
        .collect();
    if lang == "Aramaic" {
        format!("{} (Aramaic)", parts.join(" + "))
    } else {
        parts.join(" + ")
    }
}

/// Describe any grammar code.
pub fn describe(code: &str) -> String {
    // Hebrew ("HVqp3ms") and Aramaic ("AVqp3ms") codes share one scheme
    let second_upper = code.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase());
    let semitic =
        (code.starts_with('H') && code.len() > 1) || (code.starts_with('A') && code.len() > 2);
    if semitic && second_upper && !code.contains('-') {
        hebrew(code)
    } else {
        greek(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greek_codes() {
        assert_eq!(
            greek("V-AAI-3S"),
            "verb, aorist active indicative, 3rd person singular"
        );
        assert_eq!(greek("N-NSM-T"), "noun, nominative singular masculine");
        assert_eq!(
            greek("V-PAP-NSM"),
            "verb, present active participle, nominative singular masculine"
        );
        assert_eq!(greek("ADV"), "adverb");
        assert_eq!(
            greek("P-1NS"),
            "personal pronoun, 1st person, nominative singular"
        );
    }

    #[test]
    fn hebrew_codes() {
        assert_eq!(
            describe("HVqp3ms"),
            "verb, Qal perfect, 3rd person masculine singular"
        );
        assert_eq!(
            describe("HR/Ncfsa"),
            "preposition + noun, feminine singular, absolute"
        );
        assert_eq!(describe("HTo"), "direct object marker");
    }
}
