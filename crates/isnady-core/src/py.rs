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

/// `textwrap.fill(text, width, initial_indent=indent, subsequent_indent=indent)` with Python's defaults
/// (break_long_words, break_on_hyphens, drop_whitespace).
pub fn fill(text: &str, width: usize, indent: &str) -> String {
    fill_with(text, width, indent, indent)
}

/// `textwrap.fill(text, width, initial_indent=first, subsequent_indent=rest)`.
pub fn fill_with(text: &str, width: usize, first: &str, rest: &str) -> String {
    use std::sync::OnceLock;
    static SEP: OnceLock<fancy_regex::Regex> = OnceLock::new();
    // Python's TextWrapper.wordsep_re, word for word
    let sep = SEP.get_or_init(|| {
        let wp = r#"[\w!"'&.,?]"#;
        let lt = r"[^\d\W]";
        let ws = r"[\t\n\x0b\x0c\r ]";
        let nws = r"[^\t\n\x0b\x0c\r ]";
        let p = format!(
            r"({ws}+|(?<={wp})-{{2,}}(?=\w)|{nws}+?(?:-(?:(?<={lt}{{2}}-)|(?<={lt}-{lt}-))(?={lt}-?{lt})|(?={ws}|\z)|(?<={wp})(?=-{{2,}}\w)))"
        );
        fancy_regex::Regex::new(&p).expect("wordsep")
    });
    let munged: String = text
        .chars()
        .map(|c| if "\t\n\x0b\x0c\r".contains(c) { ' ' } else { c })
        .collect();
    let mut chunks: Vec<String> = sep
        .find_iter(&munged)
        .filter_map(|m| m.ok())
        .map(|m| m.as_str().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    chunks.reverse();
    let width_of = |s: &str| s.chars().count();
    let mut lines: Vec<String> = Vec::new();
    while !chunks.is_empty() {
        let mut cur: Vec<String> = Vec::new();
        let mut cur_len = 0usize;
        let indent = if lines.is_empty() { first } else { rest };
        let w = width.saturating_sub(width_of(indent));
        if chunks.last().is_some_and(|c| c.trim().is_empty()) && !lines.is_empty() {
            chunks.pop();
        }
        while let Some(c) = chunks.last() {
            let l = width_of(c);
            if cur_len + l <= w {
                cur_len += l;
                cur.push(chunks.pop().expect("chunk"));
            } else {
                break;
            }
        }
        if chunks.last().is_some_and(|c| width_of(c) > w) {
            let space_left = if w < 1 { 1 } else { w - cur_len };
            let chunk: Vec<char> = chunks.last().expect("chunk").chars().collect();
            let mut end = space_left;
            if chunk.len() > space_left {
                if let Some(h) = chunk[..space_left].iter().rposition(|c| *c == '-') {
                    if h > 0 && chunk[..h].iter().any(|c| *c != '-') {
                        end = h + 1;
                    }
                }
            }
            let end = end.min(chunk.len());
            cur.push(chunk[..end].iter().collect());
            let rest: String = chunk[end..].iter().collect();
            *chunks.last_mut().expect("chunk") = rest;
            if chunks.last().is_some_and(|c| c.is_empty()) {
                chunks.pop();
            }
        }
        if cur.last().is_some_and(|c| c.trim().is_empty()) {
            cur.pop();
        }
        if !cur.is_empty() {
            lines.push(format!("{indent}{}", cur.concat()));
        }
    }
    lines.join("\n")
}

/// `repr(x)` of a float: the shortest text that reads back as x, with ".0" on a whole number and Python's exponent
/// form below 1e-4 and from 1e16.
pub fn float_repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        // Rust writes 1e-5 and 1.5e16; Python writes 1e-05 and 1.5e+16
        let s = format!("{x:e}");
        let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
        let e: i32 = e.parse().unwrap_or(0);
        return format!("{m}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs());
    }
    let s = format!("{x}");
    if s.contains('.') {
        s
    } else {
        format!("{s}.0")
    }
}

/// `float(text)`: a number written as Python reads one ("22", " 1.5 ", "1e3", "inf").
pub fn parse_float(text: &str) -> Option<f64> {
    let t = text.trim().replace('_', "");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

/// `format(x, ".{p}g")`: p significant digits, trailing zeros dropped, the exponent form when the exponent is below
/// -4 or not below p.
pub fn fmt_g(x: f64, p: usize) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let p = p.max(1);
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    // the exponent after rounding to p digits
    let sci = format!("{:.*e}", p - 1, x);
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let strip = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if exp < -4 || exp >= p as i32 {
        format!(
            "{}e{}{:02}",
            strip(mant),
            if exp < 0 { '-' } else { '+' },
            exp.abs()
        )
    } else {
        let decimals = (p as i32 - 1 - exp).max(0) as usize;
        strip(&format!("{:.*}", decimals, x))
    }
}

/// `time.strftime(fmt)`: the local time now.
pub fn now_local(fmt: &str) -> String {
    chrono::Local::now().format(fmt).to_string()
}

/// `time.strftime(fmt, time.localtime(t))` of a file's time.
pub fn local_time(t: std::time::SystemTime, fmt: &str) -> String {
    chrono::DateTime::<chrono::Local>::from(t)
        .format(fmt)
        .to_string()
}

/// `round(x)`: to the nearest whole number, halves to the even one.
pub fn round_even(x: f64) -> f64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
        r - x.signum()
    } else {
        r
    }
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
        assert_eq!(fill("aaa bbb ccc", 9, "  "), "  aaa bbb\n  ccc");
        assert_eq!(
            fill_with("aaa bbb ccc", 10, "  x: ", "    "),
            "  x: aaa\n    bbb\n    ccc"
        );
    }

    #[test]
    fn python_numbers() {
        assert_eq!(float_repr(20.0), "20.0");
        assert_eq!(float_repr(0.8), "0.8");
        assert_eq!(float_repr(1e-5), "1e-05");
        assert_eq!(fmt_g(0.8, 6), "0.8");
        assert_eq!(fmt_g(48.0, 6), "48");
        assert_eq!(fmt_g(3.14159e-5, 2), "3.1e-05");
        assert_eq!(fmt_g(0.000123, 2), "0.00012");
        assert_eq!(fmt_g(123456.0, 2), "1.2e+05");
        assert_eq!(round_even(2.5), 2.0);
        assert_eq!(round_even(3.5), 4.0);
    }
}
