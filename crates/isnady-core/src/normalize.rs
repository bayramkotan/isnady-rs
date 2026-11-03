//! Text normalisation for search — the same rules as the Python isnady (`isnady/core/normalize.py`).
//!
//! Arabic: diacritics (harakat, tanwin, shadda, sukun, Quranic marks), tatweel and superscript alef are removed;
//! alef forms become bare alef; alef maqsura and hamza-on-ya become ya; hamza-on-waw becomes waw; ta marbuta becomes
//! ha; Persian kaf/ya become Arabic ones; Arabic-Indic digits become 0-9.
//!
//! Latin scripts (Turkish, English, French ...): case-folded, accents removed, dotless ı folded to i, apostrophes and
//! ʿayn/hamza signs removed ("Mu'adh" = "Muadh", "Müslim" = "muslim").
//!
//! `NORMALIZER_VERSION` must be the Python one: the search index the Python isnady built is read as it is.

use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

pub const NORMALIZER_VERSION: u32 = 1;

fn removed(c: char) -> bool {
    let u = c as u32;
    matches!(u, 0x0610..=0x061A | 0x064B..=0x065F | 0x06D6..=0x06DC | 0x06DF..=0x06E8 | 0x06EA..=0x06ED)
        || matches!(
            c,
            '\u{0640}'
                | '\u{0670}'
                | '\''
                | '`'
                | '\u{2018}'
                | '\u{2019}'
                | '\u{02bf}'
                | '\u{02be}'
                | '\u{02bc}'
        )
}

fn mapped(c: char) -> Option<char> {
    Some(match c {
        '\u{0623}' | '\u{0625}' | '\u{0622}' | '\u{0671}' => '\u{0627}', // أ إ آ ٱ -> ا
        '\u{0649}' | '\u{0626}' | '\u{06cc}' => '\u{064a}',              // ى ئ ی -> ي
        '\u{0624}' => '\u{0648}',                                        // ؤ -> و
        '\u{0629}' => '\u{0647}',                                        // ة -> ه
        '\u{06a9}' => '\u{0643}',                                        // ک -> ك
        '\u{0131}' => 'i',                                               // ı -> i
        '\u{0660}'..='\u{0669}' => char::from_digit(c as u32 - 0x0660, 10)?, // ٠..٩
        '\u{06F0}'..='\u{06F9}' => char::from_digit(c as u32 - 0x06F0, 10)?, // ۰..۹
        _ => return None,
    })
}

/// What one character becomes (none, one or several characters), pushed onto `out`.
fn fold_char(c: char, out: &mut String) {
    if removed(c) {
        return;
    }
    if let Some(m) = mapped(c) {
        out.push(m);
        return;
    }
    let u = c as u32;
    if !(0x0600..=0x06FF).contains(&u) {
        let base: String = std::iter::once(c)
            .nfkd()
            .filter(|d| canonical_combining_class(*d) == 0)
            .collect();
        for f in caseless::default_case_fold_str(&base).chars() {
            out.push(if f == '\u{0131}' { 'i' } else { f });
        }
        return;
    }
    out.push(c);
}

/// The normalised text.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        fold_char(c, &mut out);
    }
    out
}

/// The normalised text and, for every character of it, the index (in characters) of the original character it came
/// from — to mark the words found in the original text.
pub fn normalize_with_map(text: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut map = Vec::with_capacity(text.len());
    let mut piece = String::new();
    for (i, c) in text.chars().enumerate() {
        piece.clear();
        fold_char(c, &mut piece);
        for f in piece.chars() {
            out.push(f);
            map.push(i);
        }
    }
    (out, map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_and_latin() {
        assert_eq!(normalize("الأَعْمَالُ"), "الاعمال");
        assert_eq!(normalize("Müslim"), "muslim");
        assert_eq!(normalize("Mu'adh"), "muadh");
        assert_eq!(normalize("ISTANBUL ı"), "istanbul i");
        assert_eq!(normalize("٤٥"), "45");
        assert_eq!(normalize("مدينة"), "مدينه");
    }

    #[test]
    fn map_points_back() {
        let (n, m) = normalize_with_map("أَب");
        assert_eq!(n, "اب");
        assert_eq!(m, vec![0, 2]);
    }
}
