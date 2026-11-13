//! Python's `re` on Python's strings, for the chain reader: the patterns are the Python isnady's own (written to
//! data/isnad_parser.json by tools/gen_tables.py) and read as Python reads them — `\w`, `\s` and `$` as Python
//! means them — and every position is a character index, as in Python, not a byte index.

use fancy_regex::{Regex, RegexBuilder};

/// A text with the byte offset of each of its characters: positions are characters, as Python counts them.
pub struct Ps {
    pub s: String,
    b: Vec<usize>,
}

impl Ps {
    pub fn new(s: &str) -> Ps {
        let mut b: Vec<usize> = s.char_indices().map(|(i, _)| i).collect();
        b.push(s.len());
        Ps {
            s: s.to_string(),
            b,
        }
    }
    pub fn len(&self) -> usize {
        self.b.len() - 1
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn byte(&self, i: usize) -> usize {
        self.b[i.min(self.len())]
    }
    pub fn char_of(&self, byte: usize) -> usize {
        self.b.partition_point(|&x| x < byte)
    }
    /// `s[a:b]` with Python's clamping.
    pub fn sl(&self, a: usize, b: usize) -> &str {
        let a = a.min(self.len());
        let b = b.min(self.len()).max(a);
        &self.s[self.b[a]..self.b[b]]
    }
    pub fn from(&self, a: usize) -> &str {
        self.sl(a, self.len())
    }
    pub fn at(&self, i: usize) -> Option<char> {
        if i < self.len() {
            self.s[self.b[i]..].chars().next()
        } else {
            None
        }
    }
    /// `s.find(sub, start)`, in characters; -1 as None.
    pub fn find(&self, sub: &str, start: usize) -> Option<usize> {
        if start > self.len() {
            return None;
        }
        self.s[self.b[start]..]
            .find(sub)
            .map(|x| self.char_of(self.b[start] + x))
    }
}

/// A match, its positions in characters.
#[derive(Clone, Debug)]
pub struct M {
    spans: Vec<Option<(usize, usize)>>,
    texts: Vec<Option<String>>,
}

impl M {
    pub fn start(&self) -> usize {
        self.spans[0].map(|x| x.0).unwrap_or(0)
    }
    pub fn end(&self) -> usize {
        self.spans[0].map(|x| x.1).unwrap_or(0)
    }
    /// `m.group(i)`, "" when the group took part in nothing (test with `has`).
    pub fn g(&self, i: usize) -> &str {
        self.texts.get(i).and_then(|x| x.as_deref()).unwrap_or("")
    }
    pub fn has(&self, i: usize) -> bool {
        self.texts.get(i).is_some_and(|x| x.is_some())
    }
    pub fn group0(&self) -> &str {
        self.g(0)
    }
    pub fn start_g(&self, i: usize) -> usize {
        self.spans[i].map(|x| x.0).unwrap_or(usize::MAX)
    }
    pub fn end_g(&self, i: usize) -> usize {
        self.spans[i].map(|x| x.1).unwrap_or(usize::MAX)
    }
}

/// A Python pattern as fancy-regex reads it.
pub fn translate(p: &str) -> String {
    let w = r"\p{L}\p{N}_";
    let s = r"\s\x1c-\x1f";
    let cs: Vec<char> = p.chars().collect();
    let mut out = String::new();
    let mut in_class = false;
    let mut class_start = false;
    let mut in_quant = false;
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '\\' && i + 1 < cs.len() {
            let d = cs[i + 1];
            i += 2;
            match d {
                'w' => out.push_str(&if in_class {
                    w.to_string()
                } else {
                    format!("[{w}]")
                }),
                'W' => {
                    assert!(!in_class, "\\W in a class");
                    out.push_str(&format!("[^{w}]"));
                }
                's' => out.push_str(&if in_class {
                    s.to_string()
                } else {
                    format!("[{s}]")
                }),
                'S' => {
                    assert!(!in_class, "\\S in a class");
                    out.push_str(&format!("[^{s}]"));
                }
                'Z' => out.push_str(r"\z"),
                'u' => {
                    let hex: String = cs[i..i + 4].iter().collect();
                    out.push_str(&format!("\\x{{{hex}}}"));
                    i += 4;
                }
                'x' => {
                    let hex: String = cs[i..i + 2].iter().collect();
                    out.push_str(&format!("\\x{{{hex}}}"));
                    i += 2;
                }
                d if !d.is_ascii() => out.push(d),
                d => {
                    out.push('\\');
                    out.push(d);
                }
            }
            class_start = false;
            continue;
        }
        if in_class {
            match c {
                ']' if !class_start => {
                    in_class = false;
                    out.push(']');
                }
                '[' => out.push_str(r"\["),
                '&' => out.push_str(r"\&"),
                '~' => out.push_str(r"\~"),
                '^' if class_start => {
                    out.push('^');
                    i += 1;
                    continue;
                }
                _ => out.push(c),
            }
            class_start = false;
            i += 1;
            continue;
        }
        match c {
            '[' => {
                in_class = true;
                class_start = true;
                out.push('[');
            }
            '$' => out.push_str(r"(?:\z|(?=\n\z))"),
            '{' => {
                // a quantifier, or a literal brace as Python takes it
                let rest: String = cs[i..].iter().take(12).collect();
                let quant = regex::Regex::new(r"^\{\d*(,\d*)?\}").unwrap();
                if quant.is_match(&rest)
                    && rest
                        .as_bytes()
                        .get(1)
                        .is_some_and(|b| b.is_ascii_digit() || *b == b',')
                {
                    out.push('{');
                    in_quant = true;
                } else {
                    out.push_str(r"\{");
                }
            }
            '}' => {
                // closes a quantifier only when one was opened
                out.push_str(if in_quant { "}" } else { r"\}" });
                in_quant = false;
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

pub struct Re {
    pub pattern: String,
    search: Regex,
    anchored: Regex,
    full: Regex,
}

fn build(p: &str) -> Regex {
    RegexBuilder::new(p)
        .backtrack_limit(100_000_000)
        .build()
        .unwrap_or_else(|e| panic!("pattern {p}: {e}"))
}

impl Re {
    pub fn new(python: &str) -> Re {
        let t = translate(python);
        Re {
            pattern: python.to_string(),
            search: build(&t),
            anchored: build(&format!(r"\G(?:{t})")),
            full: build(&format!(r"\G(?:{t})\z")),
        }
    }

    fn run(&self, re: &Regex, t: &Ps, pos: usize, endpos: usize) -> Option<M> {
        let endpos = endpos.min(t.len());
        if pos > endpos {
            return None;
        }
        let hay = &t.s[..t.byte(endpos)];
        let caps = re.captures_from_pos(hay, t.byte(pos)).ok().flatten()?;
        let mut spans = vec![];
        let mut texts = vec![];
        for i in 0..caps.len() {
            match caps.get(i) {
                Some(m) => {
                    spans.push(Some((t.char_of(m.start()), t.char_of(m.end()))));
                    texts.push(Some(m.as_str().to_string()));
                }
                None => {
                    spans.push(None);
                    texts.push(None);
                }
            }
        }
        Some(M { spans, texts })
    }

    /// `pattern.search(t, pos, endpos)`.
    pub fn search_in(&self, t: &Ps, pos: usize, endpos: usize) -> Option<M> {
        self.run(&self.search, t, pos, endpos)
    }
    pub fn search_at(&self, t: &Ps, pos: usize) -> Option<M> {
        self.run(&self.search, t, pos, usize::MAX)
    }
    pub fn search(&self, s: &str) -> Option<M> {
        self.search_at(&Ps::new(s), 0)
    }
    pub fn is_search(&self, s: &str) -> bool {
        self.search.is_match(s).unwrap_or(false)
    }
    /// `pattern.match(t, pos, endpos)`.
    pub fn match_in(&self, t: &Ps, pos: usize, endpos: usize) -> Option<M> {
        self.run(&self.anchored, t, pos, endpos)
    }
    pub fn match_at(&self, t: &Ps, pos: usize) -> Option<M> {
        self.run(&self.anchored, t, pos, usize::MAX)
    }
    pub fn match_(&self, s: &str) -> Option<M> {
        self.match_at(&Ps::new(s), 0)
    }
    pub fn is_match(&self, s: &str) -> bool {
        self.anchored.is_match(s).unwrap_or(false)
    }
    pub fn fullmatch(&self, s: &str) -> bool {
        self.full.is_match(s).unwrap_or(false)
    }
    /// `pattern.finditer(t, pos, endpos)`.
    pub fn finditer_in(&self, t: &Ps, pos: usize, endpos: usize) -> Vec<M> {
        let endpos = endpos.min(t.len());
        let mut out = vec![];
        let mut at = pos;
        while at <= endpos {
            let Some(m) = self.run(&self.search, t, at, endpos) else {
                break;
            };
            at = if m.end() == m.start() {
                m.end() + 1
            } else {
                m.end()
            };
            out.push(m);
        }
        out
    }
    pub fn finditer(&self, s: &str) -> Vec<M> {
        let t = Ps::new(s);
        self.finditer_in(&t, 0, t.len())
    }
    /// `pattern.sub(f, s)`.
    pub fn sub_with(&self, s: &str, mut f: impl FnMut(&M) -> String) -> String {
        let t = Ps::new(s);
        let mut out = String::new();
        let mut last = 0;
        for m in self.finditer_in(&t, 0, t.len()) {
            out.push_str(t.sl(last, m.start()));
            out.push_str(&f(&m));
            last = m.end();
        }
        out.push_str(t.from(last));
        out
    }
    /// `pattern.sub(text, s)` with a plain replacement (no group references).
    pub fn sub(&self, s: &str, with: &str) -> String {
        self.sub_with(s, |_| with.to_string())
    }
    /// `pattern.sub(text, s, count=1)`.
    pub fn sub1_with(&self, s: &str, mut f: impl FnMut(&M) -> String) -> String {
        let t = Ps::new(s);
        match self.search_at(&t, 0) {
            Some(m) => format!("{}{}{}", t.sl(0, m.start()), f(&m), t.from(m.end())),
            None => s.to_string(),
        }
    }
    /// `pattern.split(s)` (no groups in the pattern).
    pub fn split(&self, s: &str) -> Vec<String> {
        let t = Ps::new(s);
        let mut out = vec![];
        let mut last = 0;
        for m in self.finditer_in(&t, 0, t.len()) {
            if m.end() == m.start() && m.start() == last && !out.is_empty() {
                continue;
            }
            out.push(t.sl(last, m.start()).to_string());
            last = m.end();
        }
        out.push(t.from(last).to_string());
        out
    }
}

/// A pattern written in the code of the Python isnady (not one of its module's patterns), compiled once.
#[macro_export]
macro_rules! pre {
    ($p:expr) => {{
        static R: std::sync::OnceLock<$crate::reader::pyre::Re> = std::sync::OnceLock::new();
        R.get_or_init(|| $crate::reader::pyre::Re::new($p))
    }};
}

/// Python's `str.isalnum()` for one character.
pub fn isalnum(c: char) -> bool {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let r = R.get_or_init(|| regex::Regex::new(r"^[\p{L}\p{N}]$").unwrap());
    let mut b = [0u8; 4];
    r.is_match(c.encode_utf8(&mut b))
}

/// `s.split()`.
pub fn split(s: &str) -> Vec<&str> {
    s.split(|c: char| c.is_whitespace() || ('\x1c'..='\x1f').contains(&c))
        .filter(|w| !w.is_empty())
        .collect()
}

/// `s.lstrip(chars)`.
pub fn lstrip<'a>(s: &'a str, chars: &str) -> &'a str {
    s.trim_start_matches(|c: char| chars.contains(c))
}

/// `s.strip(chars)`.
pub fn strip<'a>(s: &'a str, chars: &str) -> &'a str {
    s.trim_matches(|c| chars.contains(c))
}

/// `s.strip()`.
pub fn strip_ws(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || ('\x1c'..='\x1f').contains(&c))
}

pub fn len(s: &str) -> usize {
    s.chars().count()
}

/// `s[a:]` in characters.
pub fn from(s: &str, a: usize) -> &str {
    match s.char_indices().nth(a) {
        Some((b, _)) => &s[b..],
        None => "",
    }
}

/// `s[:a]` in characters.
pub fn upto(s: &str, a: usize) -> &str {
    match s.char_indices().nth(a) {
        Some((b, _)) => &s[..b],
        None => s,
    }
}

/// `s[-k:]` in characters.
pub fn last_chars(s: &str, k: usize) -> &str {
    let n = len(s);
    from(s, n.saturating_sub(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_python() {
        let r = Re::new(r"(?<!\w)قال(?!\w)");
        assert!(r.search("ثم قال له").is_some());
        assert_eq!(r.search("ثم قال له").unwrap().start(), 3);
        let a = Re::new(r"\s*ب");
        let t = Ps::new("اب ب");
        assert!(a.match_at(&t, 0).is_none());
        assert_eq!(a.match_at(&t, 2).unwrap().end(), 4);
        assert_eq!(Re::new(r"ا$").sub("بابا", ""), "باب");
        assert!(Re::new(r"[{}]").is_search("{"));
        assert!(Re::new(r"‏").is_search("\u{200f}"));
        assert!(Re::new(r"a{2}").is_search("aa"));
        assert_eq!(Re::new(r"\sو(?=\S)").split("ا وب وج"), vec!["ا", "ب", "ج"]);
    }
}
