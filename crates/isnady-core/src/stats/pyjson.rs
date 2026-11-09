//! The statistics' saved results as the Python isnady writes them (`json.dumps(result, ensure_ascii=False)`), and read
//! back whichever program wrote them: the two isnady keep one folder of results (data_dir()/derived/).

use serde_json::Value;

fn string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else {
                out.push_str(&crate::py::float_repr(n.as_f64().unwrap_or(0.0)));
            }
        }
        Value::String(s) => string(out, s),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write(out, x);
            }
            out.push(']');
        }
        Value::Object(o) => {
            out.push('{');
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                string(out, k);
                out.push_str(": ");
                write(out, x);
            }
            out.push('}');
        }
    }
}

/// `json.dumps(v, ensure_ascii=False)`.
pub fn dumps(v: &Value) -> String {
    let mut out = String::new();
    write(&mut out, v);
    out
}

/// `json.loads`, also of what Python writes for a value that is not a finite number (NaN, Infinity): read as null.
pub fn loads(text: &str) -> Option<Value> {
    if let Ok(v) = serde_json::from_str(text) {
        return Some(v);
    }
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            rest = &rest[c.len_utf8()..];
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            rest = &rest[1..];
            continue;
        }
        let mut hit = false;
        for word in ["-Infinity", "Infinity", "NaN"] {
            if let Some(after) = rest.strip_prefix(word) {
                out.push_str("null");
                rest = after;
                hit = true;
                break;
            }
        }
        if !hit {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    serde_json::from_str(&out).ok()
}

fn write_indented(out: &mut String, v: &Value, indent: usize, level: usize) {
    let pad = |n: usize| " ".repeat(indent * n);
    match v {
        Value::Array(a) if !a.is_empty() => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&pad(level + 1));
                write_indented(out, x, indent, level + 1);
            }
            out.push('\n');
            out.push_str(&pad(level));
            out.push(']');
        }
        Value::Object(o) if !o.is_empty() => {
            out.push_str("{\n");
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&pad(level + 1));
                string(out, k);
                out.push_str(": ");
                write_indented(out, x, indent, level + 1);
            }
            out.push('\n');
            out.push_str(&pad(level));
            out.push('}');
        }
        other => write(out, other),
    }
}

/// `json.dumps(v, ensure_ascii=False, indent=n)`.
pub fn dumps_indent(v: &Value, indent: usize) -> String {
    let mut out = String::new();
    write_indented(&mut out, v, indent, 0);
    out
}
