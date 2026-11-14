//! A work of the OpenITI corpus as a readable book (`openiti_work.py`, LIB1): the sira, the tabaqat. Its headings,
//! entries and text as the corpus marks them; when the written lives quote its entries (al-Dhahabi's Siyar and
//! Tadhkira), they are also kept apart (`core/source_entries.py`).

use rusqlite::{params, Connection};
use std::collections::HashMap;

use super::{ImportError, ImportReport, Options, SourceInfo};
use crate::pre;
use crate::py::thousands;
use crate::reader::pyre::{self, Ps};
use crate::sources::{now_iso, read_bytes_with, tier_for_license, ResourceError};

const LONG: usize = 6000;

fn markup() -> &'static pyre::Re {
    pre!(r"(?m)PageV\d+P\d+|\bms\d+\b|^~~|^#+ ?\|*|^# ")
}

fn clean(chunk: &str) -> String {
    let chunk = pre!(r"\n~~").sub(chunk, " ");
    let chunk = pre!(r"\n# ").sub(&chunk, "\n");
    let text = markup().sub(&chunk, "");
    let text = pre!(r"[ \t]+").sub(&text, " ");
    let text = pre!(r"\n").sub(&text, "\n\n");
    pyre::strip_ws(&pre!(r"\n\s*\n+").sub(&text, "\n\n")).to_string()
}

fn heading(text: &str) -> String {
    let t = pre!(r"\s+").sub(&markup().sub(text, ""), " ");
    pyre::strip(&t, " *:.").to_string()
}

#[derive(Debug, Clone)]
pub struct Item {
    pub entry: bool,
    pub level: usize,
    pub label: Option<String>,
    pub title: String,
    pub text: String,
}

/// The work as a flat list of headings and entries, in the order of the book (`read_items`).
pub fn read_items(text: &str) -> Vec<Item> {
    let body = text
        .split_once("#META#Header#End#")
        .map(|(_, b)| b)
        .unwrap_or(text);
    let mut items: Vec<Item> = vec![];
    let mut current: Option<usize> = None;
    let mut buf: Vec<&str> = vec![];
    let close = |items: &mut Vec<Item>, current: Option<usize>, buf: &[&str]| match current {
        Some(i) => items[i].text = clean(&buf.join("\n")),
        None => {
            if !pyre::strip_ws(&buf.concat()).is_empty() {
                let front = clean(&buf.join("\n"));
                if !front.is_empty() {
                    items.insert(
                        0,
                        Item {
                            entry: false,
                            level: 1,
                            label: None,
                            title: "Introduction".into(),
                            text: front,
                        },
                    );
                }
            }
        }
    };
    for line in body.split('\n') {
        let Some(m) = pre!(r"^### (\|+|\$+) ?(.*)$").match_(line) else {
            buf.push(line);
            continue;
        };
        close(&mut items, current, &buf);
        buf.clear();
        let (marks, rest) = (m.g(1).to_string(), m.g(2).to_string());
        let item = if marks.starts_with('|') {
            let t = heading(&rest);
            Item {
                entry: false,
                level: marks.len(),
                label: None,
                title: if t.is_empty() { "—".into() } else { t },
                text: String::new(),
            }
        } else {
            let (label, title) =
                match pre!(r"^(\d+)\s*[-–ـ:.]?\s*(.*)$").match_(pyre::strip_ws(&rest)) {
                    Some(n) => (Some(n.g(1).to_string()), n.g(2).to_string()),
                    None => (None, rest.clone()),
                };
            Item {
                entry: true,
                level: 0,
                label,
                title: heading(&title),
                text: String::new(),
            }
        };
        items.push(item);
        current = Some(items.len() - 1);
    }
    close(&mut items, current, &buf);
    items
}

fn parts(text: &str) -> Vec<String> {
    if pyre::len(text) <= LONG {
        return vec![text.to_string()];
    }
    let mut out = vec![];
    let mut part = String::new();
    for para in text.split("\n\n") {
        if !part.is_empty() && pyre::len(&part) + pyre::len(para) > LONG {
            out.push(std::mem::take(&mut part));
            part = para.to_string();
        } else if part.is_empty() {
            part = para.to_string();
        } else {
            part = format!("{part}\n\n{para}");
        }
    }
    if !part.is_empty() {
        out.push(part);
    }
    out
}

/// The work's tree (replacing the earlier one): (work id, chapters, leaves) (`openiti_work.write_work`).
#[allow(clippy::too_many_arguments)]
pub fn write_work(
    conn: &Connection,
    items: &[Item],
    source_id: i64,
    key: &str,
    kind: &str,
    title: &str,
    title_ar: Option<&str>,
    stamp: &str,
) -> rusqlite::Result<(i64, i64, i64)> {
    conn.execute("DELETE FROM works WHERE key = ?", [key])?;
    conn.execute(
        "INSERT INTO works (key, kind, title, title_ar, source_id, stamp) VALUES (?, ?, ?, ?, ?, ?)",
        params![key, kind, title, title_ar, source_id, stamp],
    )?;
    let wid = conn.last_insert_rowid();
    let mut ordinals: HashMap<Option<i64>, i64> = HashMap::new();
    let mut counts = [0i64, 0i64];
    let mut st = conn.prepare(
        "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, title, text) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )?;
    let mut add = |parent: Option<i64>,
                   node_kind: &str,
                   label: Option<&str>,
                   t: Option<&str>,
                   text: Option<&str>|
     -> rusqlite::Result<i64> {
        let o = ordinals.entry(parent).or_insert(0);
        *o += 1;
        counts[if ["kitab", "bab", "section"].contains(&node_kind) {
            0
        } else {
            1
        }] += 1;
        st.execute(params![wid, parent, *o, node_kind, label, t, text])?;
        Ok(conn.last_insert_rowid())
    };
    let mut stack: Vec<(usize, i64)> = vec![];
    for (i, item) in items.iter().enumerate() {
        if item.entry {
            if stack.is_empty() {
                let id = add(None, "kitab", None, Some("Entries"), None)?;
                stack.push((1, id));
            }
            let parent = stack.last().map(|s| s.1);
            add(
                parent,
                "entry",
                item.label.as_deref(),
                Some(&item.title),
                Some(&item.text),
            )?;
            continue;
        }
        let level = item.level;
        let nxt = items.get(i + 1);
        let chapter =
            level == 1 || stack.is_empty() || nxt.is_some_and(|n| n.entry || n.level > level);
        while stack.last().is_some_and(|(l, _)| *l >= level) {
            stack.pop();
        }
        if chapter {
            let parent = stack.last().map(|s| s.1);
            let node = add(
                parent,
                if stack.is_empty() { "kitab" } else { "bab" },
                None,
                Some(&item.title),
                None,
            )?;
            stack.push((level, node));
            if !item.text.is_empty() {
                for p in parts(&item.text) {
                    add(Some(node), "paragraph", None, None, Some(&p))?;
                }
            }
        } else {
            let parent = stack.last().map(|s| s.1);
            for (k, p) in parts(&item.text).iter().enumerate() {
                add(
                    parent,
                    "paragraph",
                    None,
                    if k == 0 { Some(&item.title) } else { None },
                    Some(p),
                )?;
            }
        }
    }
    Ok((wid, counts[0], counts[1]))
}

fn opt<'a>(options: &'a Options, k: &str) -> Option<&'a str> {
    options
        .get(k)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
}

/// `OpenITIWorkImporter.run`.
pub fn run(
    conn: &Connection,
    source: &SourceInfo,
    options: &Options,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    let Some(key) = opt(options, "book")
        .or_else(|| opt(options, "key"))
        .map(str::to_string)
    else {
        return Err(ImportError::Resource(
            "A work needs its key (the catalog's book).".into(),
        ));
    };
    let lang = crate::i18n::ui_language();
    say(&crate::i18n::t(
        &lang,
        "Reading {title}",
        &[("title", opt(options, "title").unwrap_or(&key).to_string())],
    ));
    let bytes = read_bytes_with(&source.location, &source.auth, Some(900))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| ResourceError(format!("Not UTF-8 text: {}", source.location)))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
    if !text.contains("#META#Header#End") && !text.contains("######OpenITI#") {
        return Err(ResourceError(format!("Not an OpenITI text: {}", source.location)).into());
    }
    let items = read_items(&text);
    if items.len() < 10 {
        return Err(ResourceError(format!(
            "Only {} headings found in {}",
            items.len(),
            source.location
        ))
        .into());
    }
    let name = source
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .or_else(|| opt(options, "title").map(str::to_string))
        .unwrap_or_else(|| key.clone());
    let tier = source
        .tier
        .clone()
        .unwrap_or_else(|| tier_for_license(source.license.as_deref()));
    let mut report = ImportReport {
        source_key: key.clone(),
        ..Default::default()
    };
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO sources (key, name, format, location, license, tier, origin, auth_type, imported_at)
         VALUES (?, ?, 'openiti-work', ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET name = excluded.name, location = excluded.location,
             license = excluded.license, tier = excluded.tier, origin = excluded.origin,
             auth_type = excluded.auth_type, imported_at = excluded.imported_at",
        params![key, name, source.location, source.license, tier, source.origin, source.auth.describe(), now_iso()],
    )?;
    let source_id: i64 =
        tx.query_row("SELECT id FROM sources WHERE key = ?", [&key], |r| r.get(0))?;
    let (_wid, chapters, leaves) = write_work(
        &tx,
        &items,
        source_id,
        &key,
        opt(options, "kind").unwrap_or("other"),
        opt(options, "title").unwrap_or(&name),
        opt(options, "title_ar"),
        &format!("{key}|{}|{}", items.len(), pyre::len(&text)),
    )?;
    tx.commit()?;
    if let Some(entries) = opt(options, "entries") {
        if WORKS.contains(&entries) {
            say(&crate::i18n::t(
                &lang,
                "Keeping its entries for the written lives",
                &[],
            ));
            store(entries, &text, &source.location)
                .map_err(|e| ImportError::Resource(e.to_string()))?;
        }
    }
    report.editions.push(key);
    report.texts = leaves;
    // a summary, said once: not a warning (iy printed it again as one)
    say(&format!(
        "{} chapters, {} entries and sections",
        thousands(chapters),
        thousands(leaves)
    ));
    Ok(report)
}

// ------------------------------------------------------------------ the entries the written lives quote
/// The works whose entries the written lives quote (`source_entries.WORKS`).
pub const WORKS: [&str; 2] = ["siyar", "tadhkira"];

/// (volume, number, heading, text) of every entry "### $ N - heading" (`source_entries.split`).
pub fn split(text: &str, whole: bool) -> Vec<(i64, i64, String, String)> {
    let body = text
        .split_once("#META#Header#End#")
        .map(|(_, b)| b)
        .unwrap_or(text);
    let b = Ps::new(body);
    let heads = pre!(r"(?m)^### \$+ (\d+)\s*-?\s*(.*)").finditer_in(&b, 0, b.len());
    let mut out = vec![];
    for (i, m) in heads.iter().enumerate() {
        let end = heads.get(i + 1).map(|h| h.start()).unwrap_or(b.len());
        let chunk = b.sl(m.end(), end);
        let volume = if whole {
            0
        } else {
            pre!(r"PageV(\d+)P(\d+)")
                .search_in(&b, m.start(), b.len().min(m.start() + 200000))
                .and_then(|p| p.g(1).parse().ok())
                .unwrap_or(0)
        };
        let chunk = pre!(r"\n~~").sub(chunk, " ");
        let chunk = pre!(r"\n# ").sub(&chunk, "\n");
        let c = pre!(r"(?m)PageV\d+P\d+|ms\d+|^~~|^#+ ?\|*|^# ").sub(&chunk, "");
        let c = pre!(r"[ \t]+").sub(&c, " ");
        let c = pyre::strip_ws(&pre!(r"\n\s*\n+").sub(&c, "\n\n")).to_string();
        let h = pre!(r"\s+").sub(
            &pre!(r"(?m)PageV\d+P\d+|ms\d+|^~~|^#+ ?\|*|^# ").sub(m.g(2), ""),
            " ",
        );
        out.push((
            volume,
            m.g(1).parse().unwrap_or(0),
            pyre::strip(&h, " *").to_string(),
            c,
        ));
    }
    out
}

/// Keep the entries of a work's text; how many (`source_entries.store`).
pub fn store(work: &str, text: &str, location: &str) -> rusqlite::Result<usize> {
    let entries = split(text, work == "tadhkira");
    let path = crate::paths::data_dir()
        .join("sources")
        .join(format!("{work}.db"));
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        let conn = Connection::open(&tmp)?;
        conn.execute_batch(
            "CREATE TABLE entries (volume INTEGER, number INTEGER, heading TEXT, text TEXT, PRIMARY KEY (volume, number));
             CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);",
        )?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut st = tx.prepare("INSERT OR IGNORE INTO entries VALUES (?, ?, ?, ?)")?;
            for (v, n, h, t) in &entries {
                st.execute(params![v, n, h, t])?;
            }
        }
        tx.execute("INSERT INTO meta VALUES ('source', ?)", [location])?;
        tx.commit()?;
    }
    let _ = std::fs::rename(&tmp, &path);
    Ok(entries.len())
}
