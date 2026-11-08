//! Genealogies (GEN1): the family trees isnady draws — the Python isnady's `core/genealogy.py`.
//!
//! Four trees: the prophets from Adam to Muhammad ﷺ and Quraysh (written for isnady, data/genealogy.json from the
//! Python isnady's assets/genealogy/trees.json); a narrator's family, read from the narrators' own names ("X b. Y b. Z"
//! is the son of "Y b. Z" when exactly one narrator of the same tradition has that name and his generation is not
//! later); a narrator's line of learning — his teachers above him, his students below, from the imported chains.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::narrators::{display_name, Langs};
use crate::normalize::normalize;
use crate::{books, cache, i18n, lives, people};

#[derive(Serialize, Clone, Debug)]
pub struct Node {
    pub id: String,
    /// en, tr, ar
    pub names: HashMap<String, String>,
    /// prophet, caliph, imam, person, narrator
    pub kind: String,
    pub parent: Option<String>,
    /// the generations between it and its parent not known (drawn dashed)
    pub gap: bool,
    /// a note: the reports differ
    pub disputed: Option<HashMap<String, String>>,
    pub note: Option<HashMap<String, String>>,
    /// a narrator in the database
    pub person_id: Option<i64>,
    /// how often (teacher / student links)
    pub count: Option<i64>,
    pub children: Vec<String>,
}

impl Node {
    pub fn name(&self, lang: &str) -> String {
        let get = |k: &str| self.names.get(k).filter(|v| !v.is_empty()).cloned();
        if lang == "tr" {
            return get("tr").or_else(|| get("en")).unwrap_or_default();
        }
        if ["ar", "fa", "prs", "ps", "ur", "ug"].contains(&lang) {
            return get("ar").or_else(|| get("en")).unwrap_or_default();
        }
        get("en").or_else(|| get("tr")).unwrap_or_default()
    }

    /// The note or the disputed report in a language, else the English.
    pub fn text(&self, which: &str, lang: &str) -> String {
        let d = if which == "note" {
            &self.note
        } else {
            &self.disputed
        };
        d.as_ref()
            .and_then(|d| {
                d.get(lang)
                    .filter(|v| !v.is_empty())
                    .or_else(|| d.get("en"))
            })
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Tree {
    pub key: String,
    pub title: String,
    pub root: String,
    pub nodes: HashMap<String, Node>,
    /// the order the nodes were added in (Python keeps a dict's order)
    pub order: Vec<String>,
    /// for a line of learning: the root of the teachers' side (drawn above)
    pub up: Option<String>,
}

impl Tree {
    /// (node, depth) in order, the root first.
    pub fn walk(&self, start: Option<&str>) -> Vec<(&Node, usize)> {
        let mut out = vec![];
        fn go<'a>(t: &'a Tree, id: &str, d: usize, out: &mut Vec<(&'a Node, usize)>) {
            let n = &t.nodes[id];
            out.push((n, d));
            for c in &n.children {
                go(t, c, d + 1, out);
            }
        }
        go(self, start.unwrap_or(&self.root), 0, &mut out);
        out
    }

    fn add(&mut self, node: Node) {
        self.order.push(node.id.clone());
        self.nodes.insert(node.id.clone(), node);
    }
}

#[derive(Deserialize)]
struct RawNode {
    id: String,
    parent: Option<String>,
    en: String,
    tr: String,
    ar: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    gap: Option<bool>,
    #[serde(default)]
    disputed: Option<HashMap<String, String>>,
    #[serde(default)]
    note: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct RawTree {
    title: HashMap<String, String>,
    root: String,
    nodes: Vec<RawNode>,
}

#[derive(Deserialize)]
struct Data {
    sources: String,
    trees: HashMap<String, RawTree>,
    written: Vec<String>,
    prophet_words: Vec<(String, String, Vec<String>)>,
    honoured: Vec<String>,
    road_b_only_own: Vec<String>,
    not_him: String,
    name_head: Vec<String>,
    life_nodes: Vec<(String, String, String)>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/genealogy.json")).expect("data/genealogy.json")
    })
}

/// The written trees: prophets, quraysh.
pub fn written_keys() -> &'static [String] {
    &data().written
}

pub fn is_written(key: &str) -> bool {
    data().written.iter().any(|k| k == key)
}

/// What the written trees are drawn from (`genealogy.sources`).
pub fn sources() -> &'static str {
    &data().sources
}

pub fn written(key: &str, lang: &str) -> Option<Tree> {
    let d = data().trees.get(key)?;
    let mut tree = Tree {
        key: key.to_string(),
        title: d
            .title
            .get(lang)
            .or_else(|| d.title.get("en"))
            .cloned()
            .unwrap_or_default(),
        root: d.root.clone(),
        nodes: HashMap::new(),
        order: vec![],
        up: None,
    };
    for x in &d.nodes {
        tree.add(Node {
            id: x.id.clone(),
            names: [("en", &x.en), ("tr", &x.tr), ("ar", &x.ar)]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            kind: x.kind.clone().unwrap_or_else(|| "person".into()),
            parent: x.parent.clone(),
            gap: x.gap.unwrap_or(false),
            disputed: x.disputed.clone(),
            note: x.note.clone(),
            person_id: None,
            count: None,
            children: vec![],
        });
    }
    for id in tree.order.clone() {
        if let Some(p) = tree.nodes[&id].parent.clone() {
            if let Some(parent) = tree.nodes.get_mut(&p) {
                parent.children.push(id);
            }
        }
    }
    Some(tree)
}

/// The line from the root down to a node.
pub fn path_to<'a>(tree: &'a Tree, node_id: &str) -> Vec<&'a Node> {
    let mut out = vec![];
    let mut current = tree.nodes.get(node_id);
    while let Some(n) = current {
        out.insert(0, n);
        current = n.parent.as_ref().and_then(|p| tree.nodes.get(p));
    }
    out
}

/// The boxes whose names hold every word, the closest name first ("Musa" is Moses before "'Imran, father of Musa").
pub fn find<'a>(tree: &'a Tree, text: &str) -> Vec<&'a Node> {
    let words: Vec<String> = normalize(text)
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let mut out: Vec<(usize, usize, &Node)> = vec![];
    for (i, id) in tree.order.iter().enumerate() {
        let n = &tree.nodes[id];
        let names: Vec<&str> = ["en", "tr", "ar"]
            .iter()
            .filter_map(|k| n.names.get(*k).map(String::as_str))
            .collect();
        let hay = normalize(&names.join(" ")).to_lowercase();
        if !words.is_empty() && words.iter().all(|w| hay.contains(w.as_str())) {
            out.push((hay.chars().count(), i, n));
        }
    }
    out.sort_by_key(|x| (x.0, x.1));
    out.into_iter().map(|x| x.2).collect()
}

// ---------------------------------------------------------------------- narrators' families, from their names

/// The names of a nasab: "يحيى بن سعيد بن قيس الأنصاري" → ["يحيى", "سعيد", "قيس"] (normalised; nisbas dropped).
pub fn segments(name: &str) -> Vec<String> {
    static SPLIT: OnceLock<Regex> = OnceLock::new();
    let re = SPLIT.get_or_init(|| Regex::new(r"\s+(?:بن|ابن|بنت)\s+").expect("split"));
    let shown = display_name(name);
    let mut out = vec![];
    for p in re.split(shown.trim()) {
        let words: Vec<&str> = p.split_whitespace().collect();
        if words.is_empty() {
            break;
        }
        let head = &data().name_head;
        let take = if words.len() > 1 && (head.iter().any(|h| h == words[0]) || words[1] == "الله")
        {
            2
        } else {
            1
        };
        out.push(normalize(&words[..take].join(" ")));
    }
    out
}

type Edges = (HashMap<i64, i64>, HashMap<i64, Vec<i64>>);

/// (father of, sons of): read from every narrator's name once per change of the database and kept.
fn family_edges(conn: &Connection) -> rusqlite::Result<Edges> {
    cache::kept(conn, "family-edges", || {
        let mut st =
            conn.prepare("SELECT id, name_ar, COALESCE(tradition, 'sunni'), tabaqa FROM persons")?;
        let people: Vec<(i64, String, String, Option<i64>)> = st
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    r.get(2)?,
                    r.get(3)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let segs: HashMap<i64, Vec<String>> = people
            .iter()
            .map(|(pid, n, _, _)| (*pid, segments(n)))
            .collect();
        let info: HashMap<i64, (String, Option<i64>)> = people
            .iter()
            .map(|(pid, _, t, tb)| (*pid, (t.clone(), *tb)))
            .collect();
        let mut index: HashMap<(String, Vec<String>), Vec<i64>> = HashMap::new();
        for (pid, _, _, _) in &people {
            let s = &segs[pid];
            for k in [2usize, 3] {
                if s.len() >= k {
                    index
                        .entry((info[pid].0.clone(), s[..k].to_vec()))
                        .or_default()
                        .push(*pid);
                }
            }
        }
        let truthy = |x: Option<i64>| x.filter(|v| *v != 0);
        let mut father: HashMap<i64, i64> = HashMap::new();
        let mut order: Vec<i64> = vec![];
        for (pid, _, _, _) in &people {
            let s = &segs[pid];
            if s.len() < 3 {
                continue;
            }
            let want = if s.len() >= 4 {
                s[1..4].to_vec()
            } else {
                s[1..3].to_vec()
            };
            let ts = truthy(info[pid].1);
            let mut found: Vec<i64> = index
                .get(&(info[pid].0.clone(), want))
                .map(|v| v.iter().copied().filter(|c| c != pid).collect())
                .unwrap_or_default();
            found.retain(|c| !matches!((truthy(info[c].1), ts), (Some(a), Some(b)) if a > b));
            if found.len() > 1 {
                if let Some(ts) = ts {
                    let earlier: Vec<i64> = found
                        .iter()
                        .copied()
                        .filter(|c| truthy(info[c].1).is_some_and(|t| t < ts))
                        .collect();
                    if earlier.len() == 1 {
                        found = earlier;
                    }
                }
            }
            if found.len() != 1 {
                continue;
            }
            father.insert(*pid, found[0]);
            order.push(*pid);
        }
        let mut sons: HashMap<i64, Vec<i64>> = HashMap::new();
        for s in order {
            sons.entry(father[&s]).or_default().push(s);
        }
        Ok((father, sons))
    })
}

pub fn family_count(conn: &Connection) -> usize {
    family_edges(conn).map(|e| e.0.len()).unwrap_or(0)
}

fn person_node(
    conn: &Connection,
    pid: i64,
    parent: Option<&str>,
    count: Option<i64>,
    langs: Langs,
) -> Node {
    let name: String = conn
        .query_row("SELECT name_ar FROM persons WHERE id = ?", [pid], |r| {
            r.get::<_, Option<String>>(0)
        })
        .ok()
        .flatten()
        .unwrap_or_default();
    let (reading, arabic) = lives::reading(conn, pid, &name, langs);
    let shown = if reading.is_empty() {
        arabic.clone()
    } else {
        reading
    };
    Node {
        id: format!("p{pid}"),
        names: [("en", shown.clone()), ("tr", shown), ("ar", arabic)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        kind: "narrator".into(),
        parent: parent.map(String::from),
        gap: false,
        disputed: None,
        note: None,
        person_id: Some(pid),
        count,
        children: vec![],
    }
}

/// The name a narrator's box shows.
pub fn person_name(conn: &Connection, pid: i64, langs: Langs) -> String {
    person_node(conn, pid, None, None, langs).name(langs.words)
}

/// A narrator's family as his and his kin's names give it: his forefathers (as far as they are narrators) and every
/// descendant of the eldest of them. None when the names show no kin.
pub fn family(conn: &Connection, person_id: i64, langs: Langs) -> rusqlite::Result<Option<Tree>> {
    let (father, sons) = family_edges(conn)?;
    let mut top = person_id;
    let mut seen = HashSet::from([top]);
    while let Some(f) = father.get(&top) {
        if seen.contains(f) {
            break;
        }
        top = *f;
        seen.insert(top);
    }
    if top == person_id && !sons.contains_key(&person_id) {
        return Ok(None);
    }
    let mut tree = Tree {
        key: "family".into(),
        title: i18n::t(langs.words, "Family", &[]),
        root: format!("p{top}"),
        nodes: HashMap::new(),
        order: vec![],
        up: None,
    };
    fn add(
        conn: &Connection,
        tree: &mut Tree,
        sons: &HashMap<i64, Vec<i64>>,
        pid: i64,
        parent: Option<&str>,
        langs: Langs,
    ) {
        let node = person_node(conn, pid, parent, None, langs);
        let id = node.id.clone();
        tree.add(node);
        if let Some(p) = parent {
            tree.nodes
                .get_mut(p)
                .expect("parent")
                .children
                .push(id.clone());
        }
        let mut kids = sons.get(&pid).cloned().unwrap_or_default();
        kids.sort();
        for s in kids {
            if !tree.nodes.contains_key(&format!("p{s}")) {
                add(conn, tree, sons, s, Some(&id), langs);
            }
        }
    }
    add(conn, &mut tree, &sons, top, None, langs);
    Ok(Some(tree))
}

/// His teachers (and theirs) above him, his students (and theirs) below — the most frequent `width` at each step,
/// from the imported chains. Two trees joined at him: `up` is the teachers' side.
pub fn learning(
    conn: &Connection,
    person_id: i64,
    depth: usize,
    width: usize,
    langs: Langs,
) -> rusqlite::Result<Tree> {
    let root = person_node(conn, person_id, None, None, langs);
    let mut tree = Tree {
        key: "learning".into(),
        title: i18n::t(langs.words, "Line of learning", &[]),
        root: root.id.clone(),
        nodes: HashMap::new(),
        order: vec![],
        up: Some("up".into()),
    };
    let up = Node {
        id: "up".into(),
        kind: "narrator".into(),
        person_id: Some(person_id),
        ..root.clone()
    };
    tree.add(root.clone());
    tree.add(up);
    let mut used = HashSet::from([person_id]);
    #[allow(clippy::too_many_arguments)]
    fn grow(
        conn: &Connection,
        tree: &mut Tree,
        used: &mut HashSet<i64>,
        pid: i64,
        node_id: &str,
        teachers: bool,
        level: usize,
        prefix: &str,
        depth: usize,
        width: usize,
        langs: Langs,
    ) -> rusqlite::Result<()> {
        if level >= depth {
            return Ok(());
        }
        let rel = people::relations(conn, pid, (width * 3) as i64)?;
        let side = if teachers { rel.teachers } else { rel.students };
        let picked: Vec<_> = side
            .into_iter()
            .filter(|r| !used.contains(&r.id))
            .take(width)
            .collect();
        for r in picked {
            used.insert(r.id);
            let mut child = person_node(conn, r.id, Some(node_id), Some(r.count), langs);
            child.id = format!("{prefix}{}", r.id);
            let cid = child.id.clone();
            tree.add(child);
            tree.nodes
                .get_mut(node_id)
                .expect("node")
                .children
                .push(cid.clone());
            grow(
                conn,
                tree,
                used,
                r.id,
                &cid,
                teachers,
                level + 1,
                prefix,
                depth,
                width,
                langs,
            )?;
        }
        Ok(())
    }
    grow(
        conn, &mut tree, &mut used, person_id, "up", true, 0, "t", depth, width, langs,
    )?;
    let rid = tree.root.clone();
    grow(
        conn, &mut tree, &mut used, person_id, &rid, false, 0, "s", depth, width, langs,
    )?;
    Ok(tree)
}

// ---------------------------------------------------------------------- text (the command line)

fn line(n: &Node, lang: &str, words: &str) -> String {
    let mut text = n.name(lang);
    if let Some(ar) = n.names.get("ar").filter(|a| !a.is_empty()) {
        if lang != "ar" && *ar != text {
            text += &format!("  {ar}");
        }
    }
    if let Some(c) = n.count.filter(|c| *c != 0) {
        text += &format!("  ({c})");
    }
    if n.gap {
        text += &format!(
            "   [{}]",
            i18n::t(words, "generations between not known", &[])
        );
    }
    if n.disputed.is_some() {
        text += &format!("   [{}]", i18n::t(words, "disputed", &[]));
    }
    text
}

/// The tree as lines of text (`genealogy.as_text`); `words`: the interface's language for its own words.
pub fn as_text(tree: &Tree, lang: &str, words: &str) -> String {
    let mut lines = vec![tree.title.clone()];
    if let Some(up) = &tree.up {
        lines.push(format!("  {}", i18n::t(words, "teachers (above):", &[])));
        for (n, d) in tree.walk(Some(up)).into_iter().skip(1) {
            lines.push(format!(
                "    {}↑ {}",
                "  ".repeat(d - 1),
                line(n, lang, words)
            ));
        }
        lines.push(format!("  {}", i18n::t(words, "students (below):", &[])));
    }
    for (n, d) in tree.walk(None) {
        lines.push(format!(
            "  {}{}{}",
            "  ".repeat(d),
            if d > 0 { "└ " } else { "" },
            line(n, lang, words)
        ));
    }
    lines.join("\n")
}

// ------------------------------------------------------------------ the prophets in the hadith (GEN1-H)

fn words_of(node_id: &str) -> Option<&'static (String, String, Vec<String>)> {
    data().prophet_words.iter().find(|(k, _, _)| k == node_id)
}

/// Whether the hadith can show who speaks of this prophet.
pub fn has_prophet_words(node_id: &str) -> bool {
    words_of(node_id).is_some()
}

/// The phrases (normalized) that name this prophet in a hadith text; [] for one the hadith do not name.
pub fn prophet_phrases(node_id: &str) -> Vec<String> {
    let Some((_, name, extra)) = words_of(node_id) else {
        return vec![];
    };
    let mut all: Vec<String> = data()
        .honoured
        .iter()
        .map(|h| format!("{name} {h}"))
        .collect();
    all.push(format!("نبي الله {name}"));
    all.extend(extra.iter().cloned());
    let mut seen = HashSet::new();
    let mut out = vec![];
    for p in all {
        let n = normalize(&p)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if n.chars().count() >= 3 && seen.insert(n.clone()) {
            out.push(n);
        }
    }
    out
}

fn really_him(conn: &Connection, hadith_id: i64, phrases: &[String], name: &str) -> bool {
    static NOT_HIM: OnceLock<Regex> = OnceLock::new();
    let not_him = NOT_HIM.get_or_init(|| Regex::new(&data().not_him).expect("not_him"));
    if phrases.is_empty() {
        return false;
    }
    let Ok(mut st) = conn.prepare(
        "SELECT n.norm FROM text_norm n JOIN texts t ON t.id = n.text_id WHERE t.hadith_id = ?",
    ) else {
        return false;
    };
    let Ok(rows) = st.query_map([hadith_id], |r| r.get::<_, String>(0)) else {
        return false;
    };
    for norm in rows.flatten() {
        let chars: Vec<char> = norm.chars().collect();
        for p in phrases {
            let pc: Vec<char> = p.chars().collect();
            let mut start = 0;
            while start + pc.len() <= chars.len() {
                let Some(at) =
                    (start..=chars.len() - pc.len()).find(|i| chars[*i..*i + pc.len()] == pc[..])
                else {
                    break;
                };
                let before: String = chars[at.saturating_sub(12)..at].iter().collect();
                if !p.starts_with(name) || !not_him.is_match(&before) {
                    return true;
                }
                start = at + 1;
            }
        }
    }
    false
}

/// One hadith that names a prophet.
#[derive(Serialize, Clone, Debug)]
pub struct Named {
    pub hadith_id: i64,
    pub book: String,
    pub number: String,
}

/// The hadith whose texts name a prophet (`genealogy.prophet_hadith`): the two roads apart (SH1), in the books'
/// reading order.
#[derive(Serialize, Clone, Debug, Default)]
pub struct ProphetHadith {
    pub total: usize,
    pub sunni: Vec<Named>,
    pub shia: Vec<Named>,
    pub count_sunni: usize,
    pub count_shia: usize,
}

pub fn prophet_hadith(
    conn: &Connection,
    node_id: &str,
    limit: usize,
) -> rusqlite::Result<ProphetHadith> {
    let phrases = prophet_phrases(node_id);
    let mut out = ProphetHadith::default();
    if phrases.is_empty() {
        return Ok(out);
    }
    let has_fts = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE name = 'texts_fts'",
            [],
            |_| Ok(()),
        )
        .is_ok();
    type Row = (i64, String, String, f64, String);
    let mut rows: Vec<Row> = vec![];
    if has_fts {
        let matched: Vec<String> = phrases
            .iter()
            .map(|p| format!("\"{}\"", p.replace('"', "")))
            .collect();
        let mut st = conn.prepare(
            "SELECT DISTINCT h.id, c.key, h.number, h.number_sort, COALESCE(c.tradition, 'sunni') \
             FROM texts_fts f JOIN texts t ON t.id = f.rowid JOIN hadiths h ON h.id = t.hadith_id \
             JOIN collections c ON c.id = h.collection_id WHERE texts_fts MATCH ?",
        )?;
        for r in st.query_map([matched.join(" OR ")], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                r.get(4)?,
            ))
        })? {
            rows.push(r?);
        }
    } else {
        let mut st = conn.prepare(
            "SELECT h.id, c.key, h.number, h.number_sort, COALESCE(c.tradition, 'sunni'), n.norm \
             FROM text_norm n JOIN texts t ON t.id = n.text_id JOIN hadiths h ON h.id = t.hadith_id \
             JOIN collections c ON c.id = h.collection_id",
        )?;
        let mut q = st.query([])?;
        while let Some(r) = q.next()? {
            let norm: String = r.get(5)?;
            if phrases.iter().any(|p| norm.contains(p.as_str())) {
                rows.push((
                    r.get(0)?,
                    r.get(1)?,
                    r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                    r.get(4)?,
                ));
            }
        }
    }
    let (_, name_raw, _) = words_of(node_id).expect("words");
    let name = normalize(name_raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let honoured: HashSet<String> = data()
        .honoured
        .iter()
        .map(|h| {
            normalize(&format!("{name_raw} {h}"))
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let only_own = data().road_b_only_own.iter().any(|k| k == node_id);
    let mut found: HashMap<i64, (String, String, f64, &'static str)> = HashMap::new();
    for (hid, key, number, sort, trad) in rows {
        if found.contains_key(&hid) {
            continue;
        }
        let own: Vec<String> = if trad == "shia" && only_own {
            phrases
                .iter()
                .filter(|p| !honoured.contains(*p))
                .cloned()
                .collect()
        } else {
            phrases.clone()
        };
        if !really_him(conn, hid, &own, &name) {
            continue;
        }
        found.insert(
            hid,
            (
                key,
                number,
                sort,
                if trad == "shia" { "shia" } else { "sunni" },
            ),
        );
    }
    let order: HashMap<String, usize> = books::all_books(conn, true)?
        .into_iter()
        .enumerate()
        .map(|(i, b)| (b.key, i))
        .collect();
    let off = books::disabled();
    let mut items: Vec<(&str, usize, f64, i64, String, String)> = found
        .into_iter()
        .filter(|(_, v)| !off.contains(&v.0))
        .map(|(hid, v)| {
            (
                v.3,
                order.get(&v.0).copied().unwrap_or(999),
                v.2,
                hid,
                v.0,
                v.1,
            )
        })
        .collect();
    items.sort_by(|a, b| {
        (a.0, a.1)
            .cmp(&(b.0, b.1))
            .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.3.cmp(&b.3))
    });
    out.total = items.len();
    for (road, _, _, hid, key, number) in items {
        let (list, count) = if road == "shia" {
            (&mut out.shia, &mut out.count_shia)
        } else {
            (&mut out.sunni, &mut out.count_sunni)
        };
        *count += 1;
        if list.len() < limit {
            list.push(Named {
                hadith_id: hid,
                book: key,
                number,
            });
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ the written lives in the written trees (GEN1-L)

/// (tree, box) of a written life, when he stands in a written tree.
pub fn life_node(life_id: &str) -> Option<(&'static str, &'static str)> {
    data()
        .life_nodes
        .iter()
        .find(|(l, _, _)| l == life_id)
        .map(|(_, t, n)| (t.as_str(), n.as_str()))
}

/// The written life of the person in this box of this tree.
pub fn node_life(tree_key: &str, node_id: &str) -> Option<&'static str> {
    data()
        .life_nodes
        .iter()
        .find(|(_, t, n)| t == tree_key && n == node_id)
        .map(|(l, _, _)| l.as_str())
}

/// The written life by its id (`lives.by_id`).
pub fn life_by_id(life_id: &str) -> Option<&'static Value> {
    lives::written()
        .iter()
        .find(|l| l["id"].as_str() == Some(life_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_written_trees() {
        let t = written("prophets", "en").expect("prophets");
        assert_eq!(t.root, "adam");
        let line = path_to(&t, "muhammad");
        assert_eq!(line.first().map(|n| n.id.as_str()), Some("adam"));
        let hits = find(&t, "Musa");
        assert_eq!(hits.first().map(|n| n.id.as_str()), Some("musa"));
        assert!(!prophet_phrases("isa").is_empty());
        assert_eq!(segments("يحيى بن سعيد بن قيس الأنصاري").len(), 3);
        assert_eq!(node_life("quraysh", "ali"), Some("ali"));
    }
}
