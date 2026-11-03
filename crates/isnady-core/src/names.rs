//! Narrators' names in Latin letters, Turkish and English — the Python isnady's `isnady/core/names.py`, from the same
//! hand-written word list (`data/tables.json`). A name is read from the start up to the first note ("ويقال", "مولاهم"
//! …). A word the list does not know is spelled letter by letter (`spell`), its short vowels a guess.

use crate::normalize::normalize;
use crate::py;
use crate::tables;

const SHADDA: char = '\u{0651}';
const DANGLING: [&str; 9] = [
    "b.", "bint", "Ebû", "Abu", "Abi", "Ümmü", "Umm", "Ebâ", "Aba",
];

fn punctuation_to_space(name: &str) -> String {
    name.chars()
        .map(|c| if "،,؛;".contains(c) { ' ' } else { c })
        .collect()
}

/// The word list's key for a word: "أبي" followed by "بن" is Ubayy, the given name.
fn key(word: &str, after: &str, before: &str) -> String {
    let words = &tables::get().names.words;
    let k = normalize(word);
    let with = format!("{k}{SHADDA}");
    if words.contains_key(&with)
        && (word.contains(SHADDA)
            || matches!(normalize(after).as_str(), "بن" | "ابن")
            || matches!(normalize(before).as_str(), "ابو" | "ابا" | "ام"))
    {
        return with;
    }
    k
}

fn pick(pair: &(String, String), tr: bool) -> String {
    if tr {
        pair.0.clone()
    } else {
        pair.1.clone()
    }
}

/// The word at i of the name: (the Latin word, how many Arabic words it took), or None for an unknown word; Err
/// for a stop word (the name ends there).
fn word_at(
    parts: &[&str],
    i: usize,
    tr: bool,
    out: &[String],
) -> Result<Option<(String, usize)>, ()> {
    let t = tables::get();
    let after = parts.get(i + 1).copied().unwrap_or("");
    let before = if i > 0 { parts[i - 1] } else { "" };
    let k = key(parts[i], after, before);
    if t.names.stops.contains(&k) {
        return Err(());
    }
    if i + 1 < parts.len() {
        let pair = format!("{} {}", normalize(parts[i]), normalize(parts[i + 1]));
        if let Some(v) = t.names.compounds.get(&pair) {
            let mut word = pick(v, tr);
            if let Some(rest) = word.strip_prefix("Ebû ") {
                if out.last().is_some_and(|w| w == "b." || w == "bint") {
                    word = format!("Ebî {rest}");
                }
            }
            return Ok(Some((word, 2)));
        }
    }
    if let Some(v) = t.names.words.get(&k) {
        let mut word = pick(v, tr);
        if word == "Ebû" && out.last().is_some_and(|w| w == "b." || w == "bint") {
            word = "Ebî".into();
        }
        return Ok(Some((word, 1)));
    }
    Ok(None)
}

fn finish(mut out: Vec<String>) -> String {
    while out.last().is_some_and(|w| DANGLING.contains(&w.as_str())) {
        out.pop();
    }
    py::upper_first(&out.join(" "))
}

/// The 'tr' or 'en' form of an Arabic name from the word list, or None when its first words are not all known.
pub fn latin(name: &str, language: &str, max_words: usize) -> Option<String> {
    let tr = language == "tr";
    let cleaned = punctuation_to_space(name);
    let parts = py::words(&cleaned);
    let (mut out, mut i, mut read) = (Vec::<String>::new(), 0usize, 0usize);
    while i < parts.len() && read < max_words {
        match word_at(&parts, i, tr, &out) {
            Err(()) => break,
            Ok(Some((word, n))) => {
                out.push(word);
                i += n;
                read += n;
            }
            Ok(None) => break,
        }
    }
    if out.is_empty() {
        return None;
    }
    let needs_father = parts.len() > 2 && normalize(parts[1]) == "بن";
    if needs_father && read < 3 {
        return None;
    }
    let text = finish(out);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// (Latin form, spelled): the name word by word from the word list, any word it does not know spelled letter by
/// letter; spelled is true when a word was. ("", false) for a name with nothing to read.
pub fn reading(name: &str, language: &str, max_words: usize) -> (String, bool) {
    let name = punctuation_to_space(name);
    let known = latin(&name, language, max_words);
    let parts = py::words(&name);
    if let Some(k) = &known {
        if py::words(k).len() >= parts.len() {
            return (k.clone(), false);
        }
    }
    let tr = language == "tr";
    let (mut out, mut spelled, mut i) = (Vec::<String>::new(), false, 0usize);
    while i < parts.len() && out.len() < max_words {
        match word_at(&parts, i, tr, &out) {
            Err(()) => break,
            Ok(Some((word, n))) => {
                out.push(word);
                i += n;
            }
            Ok(None) => {
                let word = spell(parts[i], language);
                if !word.is_empty() {
                    out.push(word);
                    spelled = true;
                }
                i += 1;
            }
        }
    }
    (finish(out), spelled)
}

fn en_letter(c: char) -> Option<&'static str> {
    Some(match c {
        'ء' | 'أ' | 'إ' | 'آ' | 'ؤ' | 'ئ' | 'ع' => "'",
        'ب' => "b",
        'ت' | 'ط' => "t",
        'ث' => "th",
        'ج' => "j",
        'ح' | 'ه' => "h",
        'خ' => "kh",
        'د' | 'ض' => "d",
        'ذ' => "dh",
        'ر' => "r",
        'ز' | 'ظ' => "z",
        'س' | 'ص' => "s",
        'ش' => "sh",
        'غ' => "gh",
        'ف' => "f",
        'ق' => "q",
        'ك' => "k",
        'ل' => "l",
        'م' => "m",
        'ن' => "n",
        'و' => "w",
        'ي' => "y",
        _ => return None,
    })
}

fn tr_letter(c: char) -> Option<&'static str> {
    Some(match c {
        'ث' => "s",
        'ج' => "c",
        'خ' => "h",
        'ذ' => "z",
        'ش' => "ş",
        'غ' => "g",
        'ق' => "k",
        'و' => "v",
        _ => return en_letter(c),
    })
}

fn harakat(c: char) -> bool {
    matches!(c as u32, 0x064B..=0x0652 | 0x0670 | 0x0640)
}

/// One Arabic word in Latin letters, letter by letter: رومان → "Ruman" / "Rûmân", الكوفي → "al-Kufi".
pub fn spell(word: &str, language: &str) -> String {
    let tr = language == "tr";
    let table = if tr { tr_letter } else { en_letter };
    let mut w: Vec<char> = word
        .chars()
        .filter(|c| !harakat(*c))
        .map(|c| if c == 'ى' { 'ا' } else { c })
        .collect();
    let mut article = String::new();
    if w.len() > 3 && w[0] == 'ا' && w[1] == 'ل' {
        w.drain(..2);
        article = if tr {
            let sun = "تثدذرزسشصضطظلن".contains(w[0]);
            format!(
                "e{}-",
                if sun {
                    tr_letter(w[0]).unwrap_or("l")
                } else {
                    "l"
                }
            )
        } else {
            "al-".into()
        };
    }
    let mut out = String::new();
    let mut run = 0;
    let n = w.len();
    for i in 0..n {
        let ch = w[i];
        let prev_cons = i == 1 || (i > 1 && !"اوي".contains(w[i - 1]));
        let nxt = if i + 1 < n { Some(w[i + 1]) } else { None };
        let mut vowel = "";
        if ch == 'ا' || (ch == 'آ' && i > 0) {
            vowel = if tr { "â" } else { "a" };
        } else if "أإآ".contains(ch) && i == 0 {
            vowel = if ch == 'إ' { "i" } else { "a" };
        } else if ch == 'ة' {
            vowel = if tr { "e" } else { "a" };
        } else if "وي".contains(ch)
            && i > 0
            && prev_cons
            && nxt.map(|x| !"اوي".contains(x)).unwrap_or(true)
        {
            vowel = match (ch == 'و', tr) {
                (true, true) => "û",
                (false, true) => "î",
                (true, false) => "u",
                (false, false) => "i",
            };
        }
        if !vowel.is_empty() {
            out.push_str(vowel);
            run = 0;
            continue;
        }
        let Some(mut c) = table(ch) else { continue };
        if c == "'" && i == 0 {
            c = "";
        }
        if run > 0 && (i == 1 || run >= 2) {
            out.push('a');
            run = 0;
        }
        out.push_str(c);
        run += 1;
    }
    let text = py::strip(&out, "'");
    if text.is_empty() {
        return String::new();
    }
    format!("{article}{}", py::upper_first(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_and_spelled() {
        assert_eq!(
            reading("عبد الله بن عمر", "en", 8),
            ("Abdullah b. Umar".to_string(), false)
        );
        assert_eq!(spell("الكوفي", "en"), "al-Kufi");
        assert_eq!(spell("رومان", "tr"), "Rûmân");
    }
}
