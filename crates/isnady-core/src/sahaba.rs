//! Companions and the Household (AM1, EB1): the ten promised Paradise, the people of the cloak, the Mothers of the
//! Believers — and, on the Shia road and apart from the Sunni one (SH1), the Fourteen held infallible: the Python
//! isnady's `core/sahaba.py`.
//!
//! Each group stands as the sources give it: the hadith that names it (shown from the books imported here), what the
//! scholars of its own road read in it, and its members with what the imported books hold of them — how many chain
//! links and hadith each is in (the Sunni road), or how many chains end at each (the Shia road). The groups are
//! written for isnady (data/sahaba.json, from the Python isnady by tools/gen_tables.py).

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{lives, people};

#[derive(Deserialize, Clone, Debug)]
pub struct Group {
    pub id: String,
    pub road: String,
    pub title: HashMap<String, String>,
    pub lead: HashMap<String, String>,
    #[serde(default)]
    pub hadith: Vec<(String, String)>,
    #[serde(default)]
    pub lives: Vec<String>,
    #[serde(default)]
    pub famous: Option<String>,
    #[serde(default)]
    pub nodes: Vec<(String, HashMap<String, String>)>,
}

#[derive(Deserialize)]
struct Node {
    names: HashMap<String, String>,
    note: Value,
}

#[derive(Deserialize)]
struct Data {
    groups: Vec<Group>,
    shared_kunyas: Vec<String>,
    quraysh: HashMap<String, Node>,
    life_nodes: HashMap<String, (String, String)>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/sahaba.json")).expect("data/sahaba.json")
    })
}

fn lang_of(lang: &str) -> &str {
    if ["en", "tr", "ar"].contains(&lang) {
        lang
    } else {
        "en"
    }
}

/// A text written for isnady in a language: its own, else the English (`sahaba.text`).
pub fn text(values: &HashMap<String, String>, lang: &str) -> String {
    values
        .get(lang_of(lang))
        .filter(|v| !v.is_empty())
        .or_else(|| values.get("en"))
        .cloned()
        .unwrap_or_default()
}

/// The groups of a road (both when None).
pub fn groups(road: Option<&str>) -> Vec<&'static Group> {
    data()
        .groups
        .iter()
        .filter(|g| road.map_or(true, |r| g.road == r))
        .collect()
}

pub fn find(group_id: &str) -> Option<&'static Group> {
    data().groups.iter().find(|g| g.id == group_id)
}

/// (book, number, hadith id) of the hadith that name the group, those of them imported here.
pub fn hadith(conn: &Connection, group: &Group) -> Vec<(String, String, i64)> {
    group
        .hadith
        .iter()
        .filter_map(|(book, number)| {
            conn.query_row(
                "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ? AND h.number = ?",
                [book, number],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|id| (book.clone(), number.clone(), id))
        })
        .collect()
}

fn counts(conn: &Connection, person_id: i64) -> (i64, i64) {
    let links = conn
        .query_row(
            "SELECT COUNT(*) FROM isnad_links WHERE person_id = ?",
            [person_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let hadith = conn
        .query_row(
            "SELECT COUNT(DISTINCT i.hadith_id) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id WHERE l.person_id = ?",
            [person_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    (links, hadith)
}

/// One member of a group (`sahaba.members`).
#[derive(Serialize, Clone, Debug, Default)]
pub struct Member {
    pub name: String,
    pub arabic: String,
    pub died: String,
    /// the narrator's row id, when his rijal work is imported here
    pub person: Option<i64>,
    /// the id of his written life
    pub life: Option<String>,
    /// the first lines of his life
    pub summary: String,
    /// his box in the tree of Quraysh
    pub node: Option<String>,
    pub links: Option<i64>,
    pub hadith: Option<i64>,
    /// the Shia road: how many chains end at him
    pub ends: Option<i64>,
}

/// Each member, with what the imported books hold of him.
pub fn members(conn: Option<&Connection>, group: &Group, lang: &str) -> Vec<Member> {
    let lang = lang_of(lang);
    let mut out = vec![];
    if !group.lives.is_empty() {
        for life_id in &group.lives {
            let Some(life) = lives::written()
                .iter()
                .find(|l| l["id"].as_str() == Some(life_id.as_str()))
            else {
                continue;
            };
            let view = lives::view(life, lang);
            let pid = conn.and_then(|c| lives::person_of(c, life));
            let died = view
                .fact_keys
                .iter()
                .zip(view.facts.iter())
                .find(|(k, _)| *k == "died")
                .map(|(_, (_, v))| v.clone())
                .unwrap_or_default();
            let node = data()
                .life_nodes
                .get(life_id)
                .filter(|(tree, _)| tree == "quraysh")
                .map(|(_, n)| n.clone());
            let mut m = Member {
                name: view.title.clone(),
                arabic: life["name"]["ar"].as_str().unwrap_or("").to_string(),
                died,
                person: pid,
                life: Some(life_id.clone()),
                summary: view.paragraphs.first().cloned().unwrap_or_default(),
                node,
                ..Default::default()
            };
            if let (Some(p), Some(c)) = (pid, conn) {
                let (links, hadith) = counts(c, p);
                m.links = Some(links);
                m.hadith = Some(hadith);
            }
            out.push(m);
        }
    } else if let Some(famous) = &group.famous {
        let Some(g) = people::famous_groups(Some(&group.road))
            .into_iter()
            .find(|g| &g.id == famous)
        else {
            return out;
        };
        for key in &g.keys {
            let arabic = people::famous_person(key)
                .map(|p| p[3].clone())
                .unwrap_or_default();
            let mut m = Member {
                name: people::famous_title(key, lang),
                arabic,
                ..Default::default()
            };
            if let Some(c) = conn {
                let found: Option<(i64, Option<i64>)> = c
                    .query_row(
                        "SELECT id, death_year_ah FROM persons WHERE key = ?",
                        [key],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .ok()
                    .flatten();
                if let Some((pid, death)) = found {
                    m.person = Some(pid);
                    if let Some(y) = death.filter(|y| *y != 0) {
                        let form = match lang {
                            "tr" => "v. h. {y}",
                            "ar" => "ت {y}هـ",
                            _ => "d. {y} AH",
                        };
                        m.died = form.replace("{y}", &y.to_string());
                    }
                    let (links, hadith) = counts(c, pid);
                    m.links = Some(links);
                    m.hadith = Some(hadith);
                    if let Some(life) = lives::of_person(c, pid) {
                        m.life = life["id"].as_str().map(String::from);
                        m.summary = lives::view(life, lang)
                            .paragraphs
                            .first()
                            .cloned()
                            .unwrap_or_default();
                    }
                }
            }
            out.push(m);
        }
        out.sort_by_key(|m| -m.hadith.unwrap_or(0));
    } else if !group.nodes.is_empty() {
        let mut ends: HashMap<String, i64> = HashMap::new();
        if let Some(c) = conn {
            if let Ok(mut st) = c.prepare(
                "SELECT i.ends_with, COUNT(*) FROM isnads i JOIN hadiths h ON h.id = i.hadith_id \
                 JOIN collections c ON c.id = h.collection_id WHERE c.tradition = 'shia' AND i.ends_with IS NOT NULL \
                 GROUP BY i.ends_with",
            ) {
                if let Ok(rows) = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))) {
                    ends.extend(rows.flatten());
                }
            }
        }
        for (node_id, years) in &group.nodes {
            let Some(node) = data().quraysh.get(node_id) else {
                continue;
            };
            let note = match &node.note {
                Value::Object(o) => o
                    .get(lang)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .or_else(|| o.get("en").and_then(Value::as_str))
                    .unwrap_or("")
                    .to_string(),
                Value::String(s) => s.clone(),
                _ => String::new(),
            };
            let end = if node_id == "muhammad" {
                "prophet"
            } else {
                node_id.as_str()
            };
            out.push(Member {
                name: node
                    .names
                    .get(lang)
                    .filter(|s| !s.is_empty())
                    .or_else(|| node.names.get("en"))
                    .cloned()
                    .unwrap_or_default(),
                arabic: node.names.get("ar").cloned().unwrap_or_default(),
                died: text(years, lang),
                summary: note,
                node: Some(node_id.clone()),
                ends: Some(ends.get(end).copied().unwrap_or(0)),
                ..Default::default()
            });
        }
    }
    out
}

/// Chains of the Shia books ending at an Imam not named (al-Imam, Abu al-Hasan said of several, one of the two).
pub fn shared_ends(conn: &Connection) -> i64 {
    let kunyas = &data().shared_kunyas;
    let marks = vec!["?"; kunyas.len()].join(",");
    conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM isnads i JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id \
             WHERE c.tradition = 'shia' AND i.ends_with IN ({marks})"
        ),
        rusqlite::params_from_iter(kunyas.iter()),
        |r| r.get(0),
    )
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_groups() {
        assert_eq!(groups(None).len(), 4);
        assert_eq!(groups(Some("shia")).len(), 1);
        let g = find("masumin").expect("masumin");
        let m = members(None, g, "en");
        assert_eq!(m.len(), 14);
        assert_eq!(m[0].died, "d. 11 AH");
        assert_eq!(
            text(&find("ten").expect("ten").title, "fa"),
            "The ten promised Paradise"
        );
    }
}
