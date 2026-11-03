//! Small things Python does with a string, done the same way here — so a name is cut, trimmed and capitalised
//! exactly as the Python isnady does it.

use unicode_normalization::char::is_combining_mark;

/// `s.split()`: the words between runs of white space.
pub fn words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

/// `s.strip(chars)`.
pub fn strip<'a>(s: &'a str, chars: &str) -> &'a str {
    s.trim_matches(|c| chars.contains(c))
}

/// `s[:1].upper() + s[1:]`.
pub fn upper_first(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        Some(c) => c.to_uppercase().collect::<String>() + it.as_str(),
        None => String::new(),
    }
}

/// `s[:1].lower() + s[1:]`.
pub fn lower_first(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        Some(c) => c.to_lowercase().collect::<String>() + it.as_str(),
        None => String::new(),
    }
}

/// `c.isalpha()`: a letter, not a mark.
pub fn isalpha(c: char) -> bool {
    c.is_alphabetic() && !is_combining_mark(c) && !c.is_numeric()
}

/// `s[:n]` in characters.
pub fn take(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// The number of characters (`len(s)`).
pub fn len(s: &str) -> usize {
    s.chars().count()
}

/// `f"{n:,}"`.
pub fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// The character index of a byte offset in `s`.
pub fn char_index(s: &str, byte: usize) -> usize {
    s[..byte].chars().count()
}

/// `str(x)` of an optional integer: "None" when it is missing.
pub fn opt(v: Option<i64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "None".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_python() {
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(12), "12");
        assert_eq!(strip(" ،-ab ،", " ،,-"), "ab");
        assert_eq!(upper_first("şam"), "Şam");
        assert!(!isalpha('\u{0307}'));
        assert!(isalpha('ş'));
    }
}
