//! Road A: the Sunni chains (`_parse_sunni` and the roads around it in core/isnad.py), line for line.

use super::pyre::{self, split, strip, strip_ws, Ps};
use super::*;

fn sub(norm: &Ps, a: usize, b: usize) -> String {
    norm.sl(a, b).to_string()
}

fn joined_clean(raw: &str) -> String {
    clean_name(&p("_JOINED_RE").sub(raw, ""))
}

fn renumber(links: &[Link]) -> Vec<Link> {
    links
        .iter()
        .enumerate()
        .map(|(i, l)| Link::new(i + 1, &l.term, &l.raw_name, l.fresh))
        .collect()
}

pub fn parse_all_sunni(text: &str, depth: usize) -> Vec<Parsed> {
    let first = parse_sunni(text, 0);
    let roads: Vec<Parsed> = if first.parallel_at >= 0 {
        split_parallel(text, &first, depth)
    } else if first.problem != problem("tahwil") {
        expand(&first)
    } else if let Some(r) = split_tahwil(text, depth) {
        r
    } else if let Some(r) = tahwil_roads(text, depth) {
        r
    } else {
        let mut first = first;
        if !first.links.is_empty() {
            first.problem = String::new();
        }
        vec![first]
    };
    let mut out = vec![];
    for mut road in roads {
        let again = cut_repeat(&mut road);
        out.push(road);
        if let Some(a) = again {
            out.push(a);
        }
    }
    for road in out.iter_mut() {
        road.links = renumber(&road.links);
    }
    out
}

fn first_tahwil(norm: &Ps) -> Option<usize> {
    let n = norm.len();
    p("_TAHWIL_RE")
        .finditer_in(norm, 0, n.min(1500))
        .into_iter()
        .find(|m| m.end() >= n || !pyre::isalnum(norm.at(m.end()).unwrap()))
        .map(|m| m.start())
}

fn split_tahwil(text: &str, depth: usize) -> Option<Vec<Parsed>> {
    let tx = Ps::new(text);
    let (norm, index_map) = norm_map(text);
    let n = norm.len();
    let at = first_tahwil(&norm)?;
    if depth > 4 {
        return None;
    }
    let cut = if at < n { index_map[at] } else { tx.len() };
    let road_a = parse_sunni(tx.sl(0, cut), 0);
    if road_a.links.is_empty() || !road_a.problem.is_empty() {
        return None;
    }
    let rest_start = if at + 1 < n {
        index_map[at + 1]
    } else {
        tx.len()
    };
    let mut rest = tx.from(rest_start).to_string();
    let mut rest_n = norm_name(&rest);
    if let Some(lead) = p("_COMPILER_SAYS_RE").match_(&rest_n) {
        if lead.end() > 0 {
            rest = pyre::from(&rest, orig_len(&rest, lead.end())).to_string();
            rest_n = norm_name(&rest);
        }
    }
    let mut said_link = None;
    if let Some(said) = p("_SAID_LINK_RE").match_(&rest_n) {
        said_link = Some(strip_ws(said.g(1)).to_string()).filter(|x| !x.is_empty());
        rest = pyre::from(&rest, orig_len(&rest, said.end())).to_string();
    }
    let mut roads = parse_all_sunni(&rest, depth + 1);
    let last_a = road_a.links.last().unwrap().raw_name.clone();
    if roads.is_empty()
        || roads[0].links.is_empty()
        || (!roads[0].problem.is_empty()
            && roads[0].problem != problem("second_chain")
            && !roads[0].links.iter().any(|l| shared(&l.raw_name, &last_a))
            && roads[0].links.len() <= road_a.links.len())
    {
        return None;
    }
    if let Some(s) = &said_link {
        roads[0]
            .links
            .insert(0, Link::new(0, "qala", &clean_name(s), false));
        for (i, link) in roads[0].links.iter_mut().enumerate() {
            link.position = i + 1;
        }
    }
    let road_b = &roads[0];
    let mut meet = road_b
        .links
        .iter()
        .position(|l| shared(&l.raw_name, &last_a));
    if meet.is_none() {
        for j in 1..road_b.links.len() {
            let k = name_key(&strip_said(&road_b.links[j].raw_name));
            if k == "ابي" || k == "ابيه" {
                let son = name_key(&strip_said(&road_b.links[j - 1].raw_name));
                if let Some(father) = pre!(r"(?<!\w)بن (.+)$").search(&son) {
                    let f = father.g(1).split(" بن ").next().unwrap_or("").to_string();
                    if same_narrator(&f, &strip_said(&last_a)) {
                        meet = Some(j);
                        break;
                    }
                }
            }
        }
    }
    if meet.is_none() {
        meet = meeting(road_b, &rest);
    }
    let meet = meet?;
    let tail: Vec<Link> = road_b.links[meet + 1..].to_vec();
    let head_len = tx.len() - pyre::len(&rest);
    let raw = clean_name(&format!("{}{}", tx.sl(0, head_len), road_b.raw_isnad));
    let mut first = Parsed {
        raw_isnad: raw.clone(),
        links: road_a
            .links
            .iter()
            .chain(tail.iter())
            .enumerate()
            .map(|(i, l)| Link::new(i + 1, &l.term, &joined_clean(&l.raw_name), l.fresh))
            .collect(),
        reaches_prophet: road_b.reaches_prophet,
        ends_with: road_b.ends_with.clone(),
        raised: road_b.raised,
        ..Default::default()
    };
    for road in roads.iter_mut() {
        road.raw_isnad = raw.clone();
        for link in road.links.iter_mut() {
            link.raw_name = joined_clean(&link.raw_name);
        }
    }
    first.alternates = road_a
        .alternates
        .iter()
        .filter(|(i, _)| *i < road_a.links.len())
        .map(|(i, l)| {
            (
                *i,
                Link::new(i + 1, &l.term, &joined_clean(&l.raw_name), false),
            )
        })
        .collect();
    let mut out = expand(&first);
    out.extend(roads);
    Some(out)
}

fn tahwil_roads(text: &str, depth: usize) -> Option<Vec<Parsed>> {
    let tx = Ps::new(text);
    let (norm, index_map) = norm_map(text);
    let at = first_tahwil(&norm)?;
    if depth > 4 {
        return None;
    }
    let mut road_a = parse_sunni(tx.sl(0, index_map[at]), 0);
    if road_a.links.is_empty() {
        return None;
    }
    road_a.problem = String::new();
    let start = if at + 1 < norm.len() {
        index_map[at + 1]
    } else {
        tx.len()
    };
    let mut rest = tx.from(start).to_string();
    if let Some(lead) = p("_COMPILER_SAYS_RE").match_(&norm_name(&rest)) {
        if lead.end() > 0 {
            rest = pyre::from(&rest, orig_len(&rest, lead.end())).to_string();
        }
    }
    let roads: Vec<Parsed> = parse_all_sunni(&rest, depth + 1)
        .into_iter()
        .filter(|r| !r.links.is_empty())
        .collect();
    if roads.is_empty() {
        return None;
    }
    let mut every = vec![road_a];
    every.extend(roads);
    for r in 0..every.len() {
        let skip = {
            let road = &every[r];
            road.reaches_prophet
                || !road.ends_with.is_empty()
                || is_companion(road.links.last().unwrap())
        };
        if !skip {
            let last = every[r].links.last().unwrap().raw_name.clone();
            for o in 0..every.len() {
                if o == r {
                    continue;
                }
                let k = every[o]
                    .links
                    .iter()
                    .position(|l| shared(&l.raw_name, &last));
                if let Some(k) = k {
                    if k + 1 < every[o].links.len() {
                        let more: Vec<Link> = every[o].links[k + 1..]
                            .iter()
                            .map(|l| Link::new(0, &l.term, &l.raw_name, l.fresh))
                            .collect();
                        let (rp, ew) = (every[o].reaches_prophet, every[o].ends_with.clone());
                        let road = &mut every[r];
                        road.links.extend(more);
                        road.reaches_prophet = rp;
                        road.ends_with = ew;
                        break;
                    }
                }
            }
        }
        if skip {
            continue;
        }
        let road = &mut every[r];
        if road.problem == problem("tahwil") || road.problem == problem("second_chain") {
            road.problem = String::new();
        }
    }
    let raw = clean_name(text);
    for road in every.iter_mut() {
        if road.raw_isnad.is_empty() {
            road.raw_isnad = raw.clone();
        }
    }
    Some(every)
}

fn begun_again(links: &[Link], j: usize, i: usize) -> bool {
    if i + 1 < links.len()
        && same_narrator(
            &strip_said(&links[j + 1].raw_name),
            &strip_said(&links[i + 1].raw_name),
        )
    {
        return true;
    }
    if links[j + 1].fresh {
        return true;
    }
    links[j..i]
        .iter()
        .any(|l| pre!(r"\sو(?:\S|$)").is_search(strip_ws(&norm_name(&l.raw_name))))
}

fn cut_repeat(road: &mut Parsed) -> Option<Parsed> {
    for _ in 0..4 {
        let mut keys: Vec<(usize, String)> = vec![];
        let mut cut = false;
        for i in 0..road.links.len() {
            let key = name_key(&strip_said(&road.links[i].raw_name));
            if key.is_empty()
                || p("_RELATIVE_RE").is_match(&key)
                || pyre::len(&key) <= 2
                || key == "عبد الله"
            {
                continue;
            }
            let j = keys
                .iter()
                .find(|(j, earlier)| {
                    *earlier == key && i > j + 1 && begun_again(&road.links, *j, i)
                })
                .map(|x| x.0);
            if j == Some(0) {
                let mut again = Parsed {
                    raw_isnad: road.raw_isnad.clone(),
                    links: road.links[i..].to_vec(),
                    reaches_prophet: road.reaches_prophet,
                    ends_with: road.ends_with.clone(),
                    raised: road.raised,
                    problem: road.problem.clone(),
                    ..Default::default()
                };
                road.links.truncate(i);
                road.reaches_prophet = false;
                road.ends_with = String::new();
                let _ = cut_repeat(&mut again);
                return Some(again);
            }
            if let Some(j) = j {
                let mut kept = road.links[..j + 1].to_vec();
                kept.extend_from_slice(&road.links[i + 1..]);
                road.links = kept;
                cut = true;
                break;
            }
            keys.push((i, key));
        }
        if !cut {
            return None;
        }
    }
    None
}

/// `_attach`: (the second road written whole, the first completed from it — None: the first unchanged).
fn attach(
    a: &[Link],
    road: &Parsed,
    kind: &str,
    joined_said: bool,
) -> (Option<Vec<Link>>, Option<Vec<Link>>) {
    let b = &road.links;
    if b.is_empty() {
        return (None, None);
    }
    let mut pairs = vec![];
    for k in 0..a.len() {
        for j in 0..b.len() {
            if shared(&a[k].raw_name, &b[j].raw_name) {
                pairs.push((k, j));
            }
        }
    }
    let cat = |x: &[Link], y: &[Link]| -> Vec<Link> {
        let mut v = x.to_vec();
        v.extend_from_slice(y);
        v
    };
    if !pairs.is_empty() {
        let (k, j) = *pairs
            .iter()
            .min_by_key(|(k, j)| (*j as i64, -(*k as i64)))
            .unwrap();
        let whole = if k as i64 - j as i64 >= 1 {
            cat(&a[..k - j], b)
        } else if k == j || kind == "first" || kind == "said" {
            b.clone()
        } else {
            return (None, None);
        };
        let last: Vec<usize> = pairs
            .iter()
            .filter(|(kk, _)| *kk == a.len() - 1)
            .map(|x| x.1)
            .collect();
        let done = last.iter().max().map(|m| cat(a, &b[m + 1..]));
        return (Some(whole), done);
    }
    let last_companion = is_companion(a.last().unwrap());
    if kind == "an" {
        if last_companion && is_companion(b.last().unwrap()) && a.len() > b.len() {
            return (Some(cat(&a[..a.len() - b.len()], b)), None);
        }
        if !last_companion {
            let done = if b.len() > 1
                && !p("_RELATIVE_RE").is_match(&name_key(&b[1].raw_name))
                && !shared(&b[1].raw_name, &a.last().unwrap().raw_name)
            {
                Some(cat(a, &b[1..]))
            } else {
                None
            };
            return (Some(cat(&a[..a.len() - 1], b)), done);
        }
        return (None, None);
    }
    if last_companion {
        return (None, None);
    }
    if kind == "said" {
        return (Some(cat(a, b)), Some(cat(a, b)));
    }
    let joined =
        b.len() > 1 && (p("_JOINED_RE").is_search(&norm_name(&b[0].raw_name)) || joined_said);
    if a.len() == 1 && !joined {
        return (Some(cat(a, b)), Some(cat(a, b)));
    }
    (
        Some(cat(&a[..a.len() - 1], b)),
        if joined { Some(cat(a, &b[1..])) } else { None },
    )
}

fn split_parallel(text: &str, first: &Parsed, depth: usize) -> Vec<Parsed> {
    let tx = Ps::new(text);
    let rest = tx.from(first.parallel_at as usize).to_string();
    let read = if first.parallel_kind == "co" {
        format!("عن {}", pyre::from(&rest, 1))
    } else {
        rest.clone()
    };
    let roads: Vec<Parsed> = if depth < 6 {
        parse_all_sunni(&read, depth + 1)
    } else {
        vec![]
    }
    .into_iter()
    .filter(|r| !r.links.is_empty())
    .collect();
    let a = first.links.clone();
    let mut out_first = Parsed {
        raw_isnad: first.raw_isnad.clone(),
        links: a.clone(),
        reaches_prophet: first.reaches_prophet,
        ends_with: first.ends_with.clone(),
        raised: first.raised,
        alternates: first.alternates.clone(),
        ..Default::default()
    };
    let mut others: Vec<Parsed> = vec![];
    let mut completed = false;
    let kind = if first.parallel_kind == "co" {
        "an"
    } else {
        first.parallel_kind.as_str()
    };
    let joined_said = pre!(r"[^،,]{0,90}?(?<!\w)(?:قالا|قالوا|كلاهما|جميعا)(?!\w)")
        .is_match(pyre::from(&norm_name(&read), 3));
    for road in &roads {
        let (whole, done) = attach(&a, road, kind, joined_said);
        let Some(whole) = whole else {
            continue;
        };
        if done.is_some() && !completed {
            out_first.links = done.clone().unwrap();
            out_first.reaches_prophet = out_first.reaches_prophet || road.reaches_prophet;
            if !road.ends_with.is_empty() {
                out_first.ends_with = road.ends_with.clone();
            }
            completed = true;
        }
        if whole == *done.as_ref().unwrap_or(&a) {
            continue;
        }
        others.push(Parsed {
            links: whole,
            reaches_prophet: road.reaches_prophet,
            ends_with: road.ends_with.clone(),
            raised: road.raised,
            problem: road.problem.clone(),
            ..Default::default()
        });
    }
    if !out_first.reaches_prophet
        && !out_first.links.is_empty()
        && is_companion(out_first.links.last().unwrap())
        && roads.iter().any(|r| r.reaches_prophet)
    {
        out_first.reaches_prophet = true;
        out_first.ends_with = "prophet".into();
    }
    let head_len = tx.len() - pyre::len(&rest);
    let raw = clean_name(&format!(
        "{}{}",
        tx.sl(0, head_len),
        roads.last().map(|r| r.raw_isnad.as_str()).unwrap_or("")
    ));
    let fix = |p: &mut Parsed| {
        p.raw_isnad = raw.clone();
        p.links = p
            .links
            .iter()
            .enumerate()
            .map(|(i, l)| Link::new(i + 1, &l.term, &joined_clean(&l.raw_name), l.fresh))
            .collect();
    };
    fix(&mut out_first);
    let mut expanded = expand(&out_first);
    for mut o in others {
        fix(&mut o);
        expanded.push(o);
    }
    expanded
}

pub(crate) fn expand(p: &Parsed) -> Vec<Parsed> {
    let mut out = vec![p.clone()];
    for (index, other) in &p.alternates {
        if *index >= p.links.len() {
            continue;
        }
        let mut links = p.links.clone();
        links[*index] = Link::new(
            index + 1,
            &links[*index].term.clone(),
            &other.raw_name,
            links[*index].fresh,
        );
        out.push(Parsed {
            raw_isnad: p.raw_isnad.clone(),
            links,
            reaches_prophet: p.reaches_prophet,
            problem: p.problem.clone(),
            ends_with: p.ends_with.clone(),
            raised: p.raised,
            ..Default::default()
        });
    }
    out
}

fn meeting(road: &Parsed, rest: &str) -> Option<usize> {
    let norm = Ps::new(&norm_name(rest));
    let mut found: Vec<(usize, String)> = vec![];
    let mut pos = 0;
    for (j, link) in road.links.iter().enumerate() {
        let key = norm_name(&link.raw_name);
        if let Some(joined) = p("_JOINED_RE").search(&key) {
            found.push((j, joined.group0().to_string()));
            continue;
        }
        let at = if key.is_empty() {
            None
        } else {
            norm.find(&key, pos)
        };
        let Some(at) = at else {
            continue;
        };
        pos = at + pyre::len(&key);
        let gap = norm.sl(pos, pos + 25).to_string();
        let nxt = p("_TERM_RE").search(&gap);
        let part = match &nxt {
            Some(m) => pyre::upto(&gap, m.start()).to_string(),
            None => gap.clone(),
        };
        if let Some(joined) = p("_JOINED_RE").search(&part) {
            found.push((j, joined.group0().to_string()));
        }
    }
    let own = |j: usize, word: &str| -> bool {
        let pair = road.alternates.iter().filter(|(i, _)| *i == j).count();
        pair >= if ["قالوا", "كلهم", "جميعهم"].contains(&word) {
            2
        } else {
            1
        }
    };
    for (j, word) in &found {
        if ["كلهم", "كلاهما", "جميعا", "جميعهم"].contains(&word.as_str()) && !own(*j, word)
        {
            return Some(*j);
        }
    }
    for (j, word) in &found {
        if ["قالا", "قالوا"].contains(&word.as_str()) && !own(*j, word) {
            return Some(*j);
        }
    }
    found.last().map(|x| x.0)
}

fn co_ok(co_name: &str) -> bool {
    !(p("_TERM_RE").is_match(co_name)
        || p("_TERM_RE").is_search(co_name)
        || split(co_name).len() > 10
        || p("_PROPHET_RE").is_search(co_name)
        || p("_NARRATES_RE").is_search(co_name)
        || pre!(r"(?:عمن|من|هو|هي|كان|غيره|غيرهما|غير واحد|ذكر)(?!\w)").is_match(co_name))
}

fn wording_of(name: &str, named: &[String]) -> bool {
    let n = name_key(&strip_said(name));
    if n.is_empty() || split(&n).len() > 5 {
        return false;
    }
    if tables().others.contains(&n) {
        return true;
    }
    for other in named {
        let o = name_key(&strip_said(other));
        if o.is_empty() {
            continue;
        }
        if same_narrator(&n, &o)
            || o.starts_with(&format!("{n} "))
            || o.ends_with(&format!(" {n}"))
            || (n.starts_with("ابن ")
                && format!(" {o}").contains(&format!(" بن {}", pyre::from(&n, 4))))
        {
            return true;
        }
    }
    false
}

/// `_said_name`: (the name, where the term begins, where the name begins).
fn said_name(
    norm: &Ps,
    mut pos: usize,
    allow_an: bool,
    depth: usize,
) -> Option<(String, usize, usize)> {
    let lead = pre!(r"[\s،,]*(?:لي|لنا|كان|و?ذكر)\s+").match_(norm.sl(pos, pos + 12));
    let mut strong = allow_an || lead.is_some();
    if let Some(l) = &lead {
        pos += l.end();
    }
    let term = p("_TERM_RE").search_in(norm, pos, pos + 70);
    let qala = pre!(r"(?<!\w)قال(?!\w)").search_in(norm, pos, pos + 70);
    let mut end_name = None;
    let end;
    if let Some(q) = qala
        .as_ref()
        .filter(|q| term.as_ref().map_or(true, |t| q.start() < t.start()) && depth < 4)
    {
        let mut after = q.end();
        while after < norm.len() && " ،,:".contains(norm.at(after).unwrap()) {
            after += 1;
        }
        let nxt = p("_TERM_RE").match_at(norm, after);
        let ok = nxt
            .as_ref()
            .is_some_and(|m| m.g(1) != "ان" && m.g(1) != "زعم")
            || said_name(norm, after, true, depth + 1).is_some();
        if !ok {
            return None;
        }
        end = q.start();
    } else {
        let t = term.as_ref()?;
        if t.g(1) == "ان" || t.g(1) == "زعم" {
            return None;
        }
        end = t.start();
        if let Some(verb) =
            pre!(r"[\s،,]+(?:يحدث|يقول|يخبر|يذكر)[\s،,]*$").search(norm.sl(pos, end))
        {
            strong = true;
            end_name = Some(pos + verb.start());
        }
        if t.g(1) == "عن"
            && !strong
            && !pre!(r"عن\s+[^،,.:]{2,40}?[\s،,]+(?:و\S+[^،,.:]{0,30}?[\s،,]+)?(?:عن|حدثنا|حدثني|اخبرنا|اخبرني|ان|سمعت)(?!\w)")
                .is_match(norm.sl(t.start(), t.start() + 120))
        {
            return None;
        }
    }
    let name = norm.sl(pos, end_name.unwrap_or(end)).to_string();
    let name = pre!(r"(?:\s+و)+\s*$").sub(&name, "");
    let bare = strip(&name, " ،,").to_string();
    let ws = split(&bare);
    if p("_JUNK_NAME_RE").is_match(&bare)
        || pre!(r"(?<!\w)(?:وسمعته|سمعته|وانا)(?!\w)").is_search(&bare)
    {
        return None;
    }
    if ws.is_empty()
        || ws.len() > 6
        || pre!(r"[.:]").is_search(&bare)
        || p("_ASKED_RE").is_match(&format!("{bare} "))
        || pre!(r"(?<!\w)(?:ممن|سل|اذهب|انظر|اسال|ايت)(?!\w)").is_search(&bare)
        || pre!(r"(?:قيل|ذكر|ذكره|ذكرت|بلغني|بلغنا|اتي|جاء|كتب|اخذ|راي|رايت|شهدت|وجدت|قرات|لما|ولما)(?!\w)").is_match(&bare)
        || p("_PROPHET_RE").is_search(&bare)
        || p("_PROPHET_NAME_RE").is_search(&bare)
        || p("_SENTENCE_START_RE").is_match(&bare)
        || p("_NOT_A_NAME_RE").is_match(&bare)
        || pre!(r"(?<!\w)(?:قال|قالت|يقول|كان|كنا|كنت|لنا|لي|له|لها|في|ما|لا|اذا|حين|يوم|سمعت|حدث\w*)(?!\w)").is_search(&bare)
    {
        return None;
    }
    Some((name.trim_end_matches([' ', '،', ',']).to_string(), end, pos))
}

fn asked_name(norm: &Ps, pos: usize) -> Option<(usize, usize)> {
    let stop = p("_ASKED_STOP_RE").search_in(norm, pos, pos + 80);
    let mut end = match &stop {
        Some(s) => s.start(),
        None => norm.len().min(pos + 40),
    };
    if let Some(honor) = p("_HONORIFIC_RE").search_in(norm, pos, end) {
        end = honor.start();
    }
    while end > pos && " ،,".contains(norm.at(end - 1).unwrap()) {
        end -= 1;
    }
    let seg = norm.sl(pos, end);
    let ws = split(seg);
    if ws.is_empty()
        || ws.len() > 6
        || p("_SENTENCE_START_RE").is_match(seg)
        || p("_GENERIC_SUBJECT_RE").is_match(seg)
        || p("_LONE_GENERIC_RE").is_match(seg)
    {
        return None;
    }
    Some((pos, end))
}

fn resume(
    text: &Ps,
    norm: &Ps,
    index_map: &[usize],
    start: usize,
    stop: usize,
    depth: usize,
) -> Option<(usize, Parsed)> {
    if depth >= 8 {
        return None;
    }
    for again in p("_RESUME_RE").finditer_in(norm, start, stop) {
        let mut rest = parse_sunni(text.from(index_map[again.start()]), depth + 1);
        let mut good = 0;
        for link in &rest.links {
            if !looks_named(&link.raw_name, 8) {
                break;
            }
            good += 1;
        }
        if good == 0 {
            continue;
        }
        if good < rest.links.len() {
            rest.links.truncate(good);
            rest.reaches_prophet = false;
            rest.ends_with = String::new();
            rest.problem = String::new();
        }
        return Some((again.start(), rest));
    }
    None
}

fn min_comma(norm: &Ps, from: usize) -> usize {
    [norm.find("،", from), norm.find(",", from)]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(norm.len())
}

const TERMS_HEARD: [&str; 8] = [
    "'an",
    "haddathana",
    "haddathani",
    "akhbarana",
    "akhbarani",
    "anba'ana",
    "anba'ani",
    "sami'tu",
];

pub fn parse_sunni(text: &str, depth: usize) -> Parsed {
    let tx = Ps::new(text);
    let (norm, index_map) = norm_map(text);
    let n = norm.len();
    let orig = |a: usize, b: usize| -> String {
        let s = if a < n { index_map[a] } else { tx.len() };
        let e = if b < n { index_map[b] } else { tx.len() };
        tx.sl(s, e).to_string()
    };
    let term_re = p("_TERM_RE");
    let mut result = Parsed::default();
    let head_len = n.min(1200);
    if strip_ws(&norm.s).is_empty() {
        result.problem = problem("no_term").into();
        return result;
    }
    let preamble = p("_SUNNI_PREAMBLE_RE").match_at(&norm, 0);
    let pre_end = preamble.as_ref().map(|m| m.end()).unwrap_or(0);
    let mut pos = pre_end;
    let mut first = if pos < head_len {
        term_re.match_in(&norm, pos, 1200)
    } else {
        None
    };
    let mut links: Vec<Link> = vec![];
    let mut ends: Vec<(usize, usize)> = vec![];
    let mut end_of_chain = 0usize;
    if first.is_none() {
        if let Some(ordinal) =
            pre!(r#"[\s\d{\[().،,\"'‏‎\-]*ال\w+(?:\s+\S+){0,2}?\s*:\s*"#).match_at(&norm, 0)
        {
            if term_re.match_in(&norm, ordinal.end(), 1200).is_some() {
                pos = ordinal.end();
                first = term_re.match_in(&norm, pos, 1200);
            }
        }
    }
    if first.is_none() {
        let opening =
            pre!(r#"[\s\d{\[().،,\"'‏‎\-]*(?:و|ثم\s+)?(?:قال|قالت|زاد|رواه)\s+"#).match_at(&norm, 0);
        let cited = p("_CITED_RE").match_at(&norm, 0).is_some();
        let mut said = opening
            .as_ref()
            .and_then(|o| said_name(&norm, o.end(), true, 0));
        if said.is_none() && opening.is_none() && !cited {
            said = said_name(&norm, pre_end, false, 0);
        }
        if said.is_none() && cited {
            result.problem = problem("no_chain").into();
            return result;
        }
        if said.is_none() && depth < 8 {
            if let Some(later) = p("_LATER_CHAIN_RE").search_in(&norm, 0, n.min(400)) {
                let mut again = parse_sunni(tx.from(index_map[later.start()]), depth + 1);
                if !again.links.is_empty() {
                    again.raw_isnad = clean_name(&format!(
                        "{}{}",
                        tx.sl(0, index_map[later.start()]),
                        again.raw_isnad
                    ));
                    return again;
                }
            }
        }
        let Some((name_text, next_term_at, start)) = said else {
            result.problem = problem("no_term").into();
            return result;
        };
        let l = pyre::len(&name_text);
        links.push(Link::new(
            1,
            "qala",
            &clean_name(&orig(start, start + l)),
            false,
        ));
        ends.push((0, start + l));
        end_of_chain = start + l;
        pos = next_term_at;
    }
    let mut skip_same = false;
    let mut skipped_to = 0usize;
    while pos < n {
        if !links.is_empty() {
            if let Some(wording) = p("_WORDING_RE").match_at(&norm, pos) {
                let mut named: Vec<String> = links.iter().map(|l| l.raw_name.clone()).collect();
                named.extend(result.alternates.iter().map(|(_, a)| a.raw_name.clone()));
                if let Some(said_term) = term_re.match_at(&norm, wording.end()) {
                    if wording_of(wording.g(1), &named) {
                        let mut after_term = said_term.end();
                        while after_term < n && " ،,-".contains(norm.at(after_term).unwrap()) {
                            after_term += 1;
                        }
                        if pre!(r"و?قال(?!\w)").is_match(norm.sl(after_term, after_term + 5)) {
                            pos = after_term;
                        } else {
                            pos = wording.end();
                            skip_same = true;
                        }
                        continue;
                    }
                }
            }
            if let Some(aside) = p("_ASIDE_RE").match_at(&norm, pos) {
                pos = aside.end();
                skipped_to = aside.end();
                continue;
            }
            if let Some(about) = p("_ABOUT_RE").match_at(&norm, pos) {
                if !p("_PROPHET_RE").is_search(about.group0())
                    && !p("_PROPHET_NAME_RE").is_search(about.group0())
                {
                    pos = about.end();
                    skipped_to = about.end();
                    continue;
                }
            }
        }
        let filler = p("_SUNNI_FILLER_RE").match_at(&norm, pos);
        let filler_text = filler.as_ref().map(|f| f.group0().to_string());
        let said_before = filler_text
            .as_ref()
            .is_some_and(|f| pre!(r"(?<!\w)قال(?!\w)").is_search(f));
        if let Some(f) = &filler {
            pos = f.end();
        }
        let term = term_re.match_at(&norm, pos);
        if term.is_none() && !links.is_empty() {
            let mut asked = p("_ASKED_RE").match_at(&norm, pos);
            if asked.as_ref().is_some_and(|a| a.g(1) == "سال")
                && !filler_text
                    .as_ref()
                    .is_some_and(|f| pre!(r"(?<!\w)(?:انه|انها)(?!\w)").is_search(f))
            {
                asked = None;
            }
            if let Some(a) = &asked {
                if p("_PROPHET_RE").match_at(&norm, a.end()).is_none()
                    && p("_PROPHET_NAME_RE").match_at(&norm, a.end()).is_none()
                {
                    if let Some((ga, gb)) = asked_name(&norm, a.end()) {
                        if !strip(norm.sl(ga, gb), " ،,").is_empty() {
                            let g1 = a.g(1);
                            let term = if g1.starts_with("سي") {
                                "su'ila"
                            } else if g1 == "سال" {
                                "sa'ala"
                            } else {
                                "sa'altu"
                            };
                            links.push(Link::new(
                                links.len() + 1,
                                term,
                                &clean_name(&orig(ga, gb)),
                                false,
                            ));
                            ends.push((end_of_chain, gb));
                            end_of_chain = gb;
                            if g1.starts_with("قلت")
                                && pre!(r"[\s،,:]*ا?(?:حدثكم|اخبركم|حدثك|اخبرك)(?!\w)")
                                    .is_match(norm.sl(gb, gb + 12))
                            {
                                pos = gb;
                                continue;
                            }
                        }
                    }
                } else {
                    result.reaches_prophet = true;
                }
                break;
            }
        }
        let told = filler_text
            .as_ref()
            .and_then(|f| pre!(r"(?<!\w)(اخبره|حدثه|اخبرهم|حدثهم)[\s،,]*$").search(f));
        let colon = filler_text.as_ref().is_some_and(|f| f.contains(':'))
            && links
                .last()
                .is_some_and(|l| l.term == "qara'tu 'ala" || l.term == "quri'a 'ala");
        if term.is_none() && !links.is_empty() && (told.is_some() || colon) {
            if let Some((name_text, next_term_at, start)) = said_name(&norm, pos, true, 0) {
                let kind = match &told {
                    Some(t) if t.g(1).starts_with("حدث") => "haddathana",
                    Some(_) => "akhbarana",
                    None => "'an",
                };
                let l = pyre::len(&name_text);
                links.push(Link::new(
                    links.len() + 1,
                    kind,
                    &clean_name(&orig(start, start + l)),
                    false,
                ));
                ends.push((end_of_chain, start + l));
                end_of_chain = start + l;
                pos = next_term_at;
                continue;
            }
        }
        if term.is_none() && !links.is_empty() && said_before {
            let last_term_qala = links.last().unwrap().term == "qala";
            if let Some((name_text, next_term_at, at)) = said_name(&norm, pos, last_term_qala, 0) {
                pos = at;
                let li = links.len() - 1;
                let mut here = vec![links[li].raw_name.clone()];
                here.extend(
                    result
                        .alternates
                        .iter()
                        .filter(|(i, _)| *i == li)
                        .map(|(_, a)| a.raw_name.clone()),
                );
                if here
                    .iter()
                    .any(|h| same_narrator(&strip_said(&name_text), &strip_said(&norm_name(h))))
                {
                    pos = next_term_at;
                    skip_same = true;
                    continue;
                }
                let start = pos;
                let l = pyre::len(&name_text);
                links.push(Link::new(
                    links.len() + 1,
                    "qala",
                    &clean_name(&orig(start, start + l)),
                    false,
                ));
                ends.push((end_of_chain, start + l));
                end_of_chain = start + l;
                pos = next_term_at;
                continue;
            }
        }
        let Some(term) = term else {
            break;
        };
        let tid = term_id(term.g(1));
        if !links.is_empty() && term.group0().starts_with('و') && TERMS_HEARD.contains(&tid) {
            result.parallel_at = pos as i64;
            result.parallel_kind = if tid == "'an" {
                "an".into()
            } else if said_before {
                "said".into()
            } else {
                "first".into()
            };
            break;
        }
        let mut name_start = term.end();
        while name_start < n && " \t،,-".contains(norm.at(name_start).unwrap()) {
            name_start += 1;
        }
        if term.g(1) == "عمن" {
            name_start = term.end() - 2;
        }
        if tid == "sami'tu" || tid == "sami'a" {
            if let Some(h) = p("_HEARD_FROM_RE").match_at(&norm, name_start) {
                name_start = h.end();
            }
        }
        if let Some(nt) = term_re.match_at(&norm, name_start) {
            if links.is_empty() || !["anna", "za'ama"].contains(&term_id(nt.g(1))) {
                pos = name_start;
                continue;
            }
        }
        if p("_PROPHET_RE").match_at(&norm, name_start).is_some()
            || p("_PROPHET_NAME_RE").match_at(&norm, name_start).is_some()
        {
            result.reaches_prophet = true;
            break;
        }
        let next_comma = min_comma(&norm, name_start);
        let next_term = term_re.search_at(&norm, name_start);
        let nt_start = next_term.as_ref().map(|m| m.start()).unwrap_or(n);
        let mut name_end = next_comma.min(nt_start);
        let said = {
            let seg = Ps::new(norm.sl(name_start, name_end));
            p("_NAME_STOP_RE")
                .finditer_in(&seg, 0, seg.len())
                .into_iter()
                .find(|m| m.start() > 0)
        };
        if let Some(unnamed) = p("_UNNAMED_COMPANION_RE").match_at(&norm, name_start) {
            name_end = unnamed.end();
        } else if let Some(s) = &said {
            name_end = name_start + s.start();
        }
        let honor = p("_HONORIFIC_RE").search_in(&norm, name_start, name_end);
        if let Some(h) = &honor {
            name_end = h.start();
        } else if let Some(nt) = &next_term {
            if name_end == next_comma && next_comma < nt.start() && term_id(nt.g(1)) != "anna" {
                let mut seg_at = next_comma + 1;
                while seg_at < nt.start() && " ،,".contains(norm.at(seg_at).unwrap()) {
                    seg_at += 1;
                }
                let seg = strip(norm.sl(seg_at, nt.start()), " ،,").to_string();
                let seg = strip_ws(&pre!(r"(?:[\s،,]+(?:قال|قالت|يقول|حدثه|حدثهم|حدثها|حدثني|اخبره|اخبرهم|اخبرها|انه|انها|يحدث))+$")
                    .sub(&format!(" {seg}"), ""))
                .to_string();
                let first_words = split(norm.sl(name_start, name_end));
                let kunya_first = !first_words.is_empty()
                    && (["ابو", "ابا", "ابي", "ام", "ابن"].contains(&first_words[0])
                        || first_words.len() == 1);
                if !seg.is_empty()
                    && !seg.starts_with('و')
                    && split(&seg).len() <= 8
                    && !pre!(r"[،,.]").is_search(&seg)
                    && !pre!(r"(?<!\w)(?:قال|قالت|قالا|قالوا|يقول|يعني|هو|ح|حدث\w*|اخبر\w*|سمع\w*|يسال|يسل|سال\w*|يقرا|يذكر|يخطب|يفتي|ام المومنين|زوج|يحدث\w*|يخبر\w*|انه|انها|انهم|انهما)(?!\w)")
                        .is_search(&seg)
                    && ((kunya_first && pre!(r" (?:بن|بنت) ").is_search(&format!(" {seg} ")))
                        || pre!(r"(?:ابو|ابا|ابي|ام) \S").is_match(&seg)
                        || pre!(r"ال\S{2,}").fullmatch(&seg))
                {
                    name_end = seg_at + pyre::len(&seg);
                }
            }
        }
        let after = name_end;
        if let Some(desc) = p("_DESCRIPTOR_RE").match_at(&norm, name_end) {
            let nxt = term_re.search_at(&norm, desc.end());
            let nxt_comma = min_comma(&norm, desc.end());
            name_end = nxt.as_ref().map(|m| m.start()).unwrap_or(n).min(nxt_comma);
            if let Some(s) =
                pre!(r"(?<!\w)(?:قال|قالت|يقول|تقول|حدث\w*|اخبر\w*|انه|انها|انهما)(?!\w)|\s-")
                    .search(norm.sl(desc.end(), name_end))
            {
                name_end = desc.end() + s.start();
            }
        }
        if p("_LONE_HA_RE").fullmatch(norm.sl(name_start, name_end)) {
            break;
        }
        if let Some(lone) = p("_HA_END_RE").search(norm.sl(name_start, name_end)) {
            if lone.start() > 0 {
                name_end = name_start + lone.start();
            }
        }
        let mut core_norm = sub(&norm, name_start, name_end);
        if let Some(clar) = pre!(r"[،,]?\s*-\s*(?:و?يعني|و?هو)\s[^-]{1,150}-|[،,]?\s*(?:و?يعني|وهو)\s+(?:ابن|ابا|ابو|ال)[^،,.:\-]{0,60}?(?=[\s،,]*(?:حدثنا|حدثني|ثنا|اخبرنا|اخبرني|انبانا|سمعت|عن|قال)(?!\w))|[،,]\s*هو\s+(?:ابن|ال)[^،,.:\-]{0,60}?(?=[\s،,]*(?:حدثنا|حدثني|ثنا|اخبرنا|اخبرني|انبانا|سمعت|عن|قال|ان)(?!\w))")
            .match_(norm.sl(after, after + 170))
        {
            name_end = after + clar.end();
        }
        for waw in pre!(r"\sو(\S+)").finditer(&core_norm) {
            let word = waw.g(1).to_string();
            let before = pyre::upto(&core_norm, waw.start());
            if pre!(r"(?<!\w)(?:ابن|بن|ابي|ابو|ابا|ام|بنت|عبد)$").is_search(before)
                && strip_ws(before) != "ابي"
            {
                continue;
            }
            if tables().waw_names.contains(&format!("و{word}"))
                || ["هو", "هي", "كان", "كانت", "سلم", "غيره", "غيرهما", "ذكر"]
                    .contains(&word.as_str())
                || pre!(r"(?:اله|سلم)$").is_match(&word)
            {
                continue;
            }
            let cut = name_start + waw.start();
            let co = if cut > name_start {
                p("_CO_NARRATOR_RE").match_at(&norm, cut)
            } else {
                None
            };
            if co.is_some_and(|c| co_ok(strip_ws(c.g(1)))) {
                name_end = cut;
                core_norm = sub(&norm, name_start, name_end);
            }
            break;
        }
        let mut ws: Vec<String> = split(strip(&core_norm, " ،,:-"))
            .iter()
            .map(|s| s.to_string())
            .collect();
        if !links.is_empty()
            && ["wajadtu", "kataba", "balagha", "sa'ala"].contains(&tid)
            && p("_JUNK_NAME_RE").is_match(strip(&core_norm, " ،,:-"))
        {
            break;
        }
        let prophet_word = split(&p("_OF_PROPHET_RE").sub(&core_norm, " "))
            .iter()
            .position(|w| ["رسول", "النبي", "نبي"].contains(w));
        if prophet_word.is_some_and(|k| k >= 2) {
            if let Some(cut) = companion_prefix(&norm, name_start, name_end) {
                name_end = cut;
                core_norm = sub(&norm, name_start, name_end);
                ws = split(strip(&core_norm, " ،,:-"))
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }
        }
        let core_stripped = strip(&core_norm, " ،,:-").to_string();
        if !links.is_empty()
            && (pre!(r"(?:ما|بما|بحديث|بشيء|بكذا|في|فيما|لم|لا|هذا|ذلك|كذا|انا|نحن)(?!\w)")
                .is_match(&core_stripped)
                || (prophet_word.is_some_and(|k| k <= 1)
                    && !pre!(r"(?:رجل|رجلا|بعض|امراه|شيخ|ناس|اناس|نفر|رهط|اصحاب|اهل)(?!\w)")
                        .is_match(&core_stripped)))
        {
            break;
        }
        if pre!(r"و?(?:قال|قالت|قالا|يقول)").fullmatch(&core_stripped)
            && term_re.match_at(&norm, name_end).is_some()
        {
            pos = name_end;
            continue;
        }
        if !links.is_empty()
            && p("_NOT_A_NAME_RE").fullmatch(&core_stripped)
            && pre!(&format!(r"[\s،,]*{}", term_re.pattern)).is_match(norm.from(name_end))
        {
            pos = name_end;
            while pos < n && " ،,".contains(norm.at(pos).unwrap()) {
                pos += 1;
            }
            continue;
        }
        let mut bare_core = p("_OF_PROPHET_RE").sub(&core_norm, " ");
        if !links.is_empty()
            && (p("_NOT_A_NAME_RE").is_match(&core_stripped)
                || p("_SENTENCE_START_RE").is_match(&core_stripped)
                || (tid == "anna"
                    && (p("_PROPHET_RE").is_search(&bare_core)
                        || p("_PROPHET_NAME_RE").is_search(&bare_core))))
        {
            break;
        }
        let core_trim = strip_ws(&core_norm).to_string();
        if let Some(dot) = pre!(r"\.\s*\S").search(&core_trim) {
            let lead = pyre::len(&core_norm)
                - pyre::len(core_norm.trim_start_matches(|c: char| {
                    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
                }));
            let mut cut = name_start + lead + dot.start();
            while cut > name_start && " ،,\u{200f}\u{200e}".contains(norm.at(cut - 1).unwrap()) {
                cut -= 1;
            }
            if cut <= name_start || words(norm.sl(name_start, cut)) > 8 {
                break;
            }
            let fresh = !links.is_empty()
                && pre!(r"\.|(?<!\w)ح(?!\w)").is_search(norm.sl(end_of_chain, term.start()));
            links.push(Link::new(
                links.len() + 1,
                tid,
                &clean_name(&orig(name_start, cut)),
                fresh,
            ));
            ends.push((end_of_chain, cut));
            end_of_chain = cut;
            let after = name_start + lead + dot.end() - 1;
            let nxt = term_re
                .match_at(&norm, after)
                .or_else(|| term_re.match_at(&norm, after + 1));
            if let Some(nx) = nxt {
                if nx.group0().starts_with('و') {
                    result.parallel_at = nx.start() as i64;
                    result.parallel_kind = if term_id(nx.g(1)) == "'an" {
                        "an".into()
                    } else {
                        "first".into()
                    };
                }
            }
            break;
        }
        let sons = ws
            .iter()
            .filter(|w| ["بن", "بنت", "ابن"].contains(&w.as_str()))
            .count();
        let max_words = tables().max_name_words;
        if ws.is_empty()
            || (ws.len() > max_words && !(ws.len() <= 22 && sons * 3 + 2 >= ws.len()))
            || (ws.len() > 10 && sons < 3)
        {
            let mut cut = if !links.is_empty() {
                companion_prefix(&norm, name_start, name_end)
            } else {
                None
            };
            if cut.is_none() {
                cut = name_prefix(&norm, name_start, name_end);
                if let Some(c) = cut {
                    let upto_end = pyre::len(
                        norm.sl(0, name_end)
                            .trim_end_matches([' ', '،', ',', ':', '-']),
                    );
                    let most = if c >= upto_end { 12 } else { 9 };
                    if words(norm.sl(name_start, c)) > most
                        || !looks_named(norm.sl(name_start, c), 12)
                    {
                        cut = None;
                    }
                }
            }
            let Some(c) = cut else {
                break;
            };
            name_end = c;
            core_norm = sub(&norm, name_start, name_end);
            bare_core = p("_OF_PROPHET_RE").sub(&core_norm, " ");
            ws = split(&core_norm).iter().map(|s| s.to_string()).collect();
        }
        let _ = &ws;
        if tid == "anna" && !links.is_empty() && words(strip(&bare_core, " ،,:-")) > 6 {
            break;
        }
        let fresh = !links.is_empty()
            && pre!(r"\.|(?<!\w)ح(?!\w)").is_search(norm.sl(end_of_chain, term.start()));
        if skip_same
            && !links.is_empty()
            && same_narrator(
                &strip_said(&core_norm),
                &strip_said(&norm_name(&links.last().unwrap().raw_name)),
            )
        {
            skip_same = false;
            end_of_chain = name_end;
            pos = name_end;
            continue;
        }
        skip_same = false;
        while name_end > name_start + 1 && norm.at(name_end - 1) == Some(' ') {
            name_end -= 1;
        }
        if tid == "kataba"
            && !pre!(r"[\s،,:]*(?:قال[\s،,:]*)?(?:حدثنا|حدثني|اخبرنا|اخبرني|انبانا|عن|سمعت)(?!\w)")
                .is_match(norm.sl(name_end, name_end + 30))
        {
            break;
        }
        links.push(Link::new(
            links.len() + 1,
            tid,
            &clean_name(&orig(name_start, name_end)),
            fresh,
        ));
        ends.push((end_of_chain, name_end));
        end_of_chain = name_end;
        pos = match &honor {
            Some(h) if h.end() > name_end => h.end(),
            _ => name_end,
        };
        loop {
            let either = p("_OR_RE").match_at(&norm, pos);
            if let Some(e) = &either {
                let g = strip_ws(e.g(1));
                if pre!(r"ال\S+").fullmatch(g) || pre!(r"(?:من|غير|غيره|غيرهما)(?!\w)").is_match(g)
                {
                    pos = e.end();
                    end_of_chain = e.end();
                    continue;
                }
                if co_ok(g) {
                    let start = e.start_g(1);
                    let l = pyre::len(e.g(1).trim_end_matches(|c: char| c.is_whitespace()));
                    result.alternates.push((
                        links.len() - 1,
                        Link::new(
                            links.len(),
                            tid,
                            &clean_name(&orig(start, start + l)),
                            false,
                        ),
                    ));
                    pos = e.end();
                    end_of_chain = e.end();
                    continue;
                }
            }
            let co = p("_CO_NARRATOR_RE")
                .match_at(&norm, pos)
                .or_else(|| p("_CO_LAST_RE").match_at(&norm, pos));
            let Some(co) = co else {
                break;
            };
            let co_name = strip_ws(co.g(1)).to_string();
            if pre!(r"(?:غيره|غيرهما|غير واحد|غيرهم)")
                .fullmatch(strip(&p("_JOINED_RE").sub(&co_name, ""), " ،,"))
            {
                pos = co.end();
                end_of_chain = co.end();
                continue;
            }
            if !co_ok(&co_name) {
                break;
            }
            if is_companion(links.last().unwrap())
                && !is_companion_name(&p("_JOINED_RE").sub(&co_name, ""))
            {
                result.parallel_at = co.start_g(1) as i64 - 1;
                result.parallel_kind = "co".into();
                break;
            }
            let start = co.start_g(1);
            let mut co_len = pyre::len(co.g(1));
            let ha = p("_HA_END_RE").search(co.g(1));
            let ha_cut = ha.as_ref().is_some_and(|h| h.start() > 0);
            if ha_cut {
                co_len = ha.as_ref().unwrap().start();
            }
            let raw = strip_ws(&p("_JOINED_RE").sub(&orig(start, start + co_len), "")).to_string();
            result.alternates.push((
                links.len() - 1,
                Link::new(links.len(), tid, &clean_name(&raw), false),
            ));
            if ha_cut {
                pos = start + co_len;
                end_of_chain = pos;
                break;
            }
            pos = co.end();
            end_of_chain = co.end();
        }
        if result.parallel_at >= 0 {
            break;
        }
        if result.alternates.iter().any(|(i, _)| *i == links.len() - 1) {
            skip_same = true;
        }
        if tahwil_at(&norm, pos) {
            break;
        }
    }

    // the subject of a story after "أنّ" is no narrator
    let mut story_end = 0usize;
    while links.last().is_some_and(|l| l.term == "anna") {
        let mut after_name = sub(&norm, end_of_chain, end_of_chain + 120);
        for _ in 0..3 {
            after_name =
                p("_OF_PROPHET_START_RE").sub(&p("_HONORIFIC_TAIL_RE").sub(&after_name, ""), "");
            after_name = pre!(r"^[\s،,\-]*(?:وكان|وكانت|وهو|وهي)\s[^.]{0,70}?(?=(?<!\w)(?:ان|عن|قال|قالت|يقول|حدث\w*|اخبر\w*)\s)")
                .sub(&after_name, "");
        }
        let last = norm_name(&links.last().unwrap().raw_name);
        let last_s = strip(&last, " ،,");
        let generic = p("_GENERIC_SUBJECT_RE").is_match(&last)
            && !(p("_ANNA_NAME_VERB_RE").is_search(last_s)
                || p("_GENERIC_TRANSMITS_RE").is_match(&after_name));
        let narrates = p("_ANNA_NARRATES_RE").is_match(&after_name)
            || p("_ANNA_NAME_VERB_RE").is_search(last_s)
            || p("_FROM_PROPHET_RE").is_match(&after_name)
            || pre!(
                r"(?<!\w)(?:اخبراه|حدثاه|اخبروه|حدثوه|قالا|قالوا|حدثه|اخبره|حدثهم|اخبرهم)(?!\w)"
            )
            .is_search(norm.sl(ends.last().unwrap().1, end_of_chain));
        let before = if links.len() > 1 {
            links[links.len() - 2].raw_name.clone()
        } else {
            String::new()
        };
        let after_companion = !before.is_empty()
            && (is_companion_name(&before)
                || p("_COMPANION_HONOR_RE").is_match(&if ends.len() > 1 {
                    let e = ends[ends.len() - 2].1;
                    sub(&norm, e, e + 30)
                } else {
                    String::new()
                }));
        let story = after_companion && (!narrates || p("_STORY_SPEECH_RE").is_match(&after_name));
        if !(generic
            || !(narrates || p("_NARRATES_RE").is_search(pyre::upto(&after_name, 60)))
            || story)
        {
            break;
        }
        story_end = story_end.max(ends.last().unwrap().1);
        links.pop();
        end_of_chain = ends.pop().unwrap().0;
        if generic && !links.is_empty() && !result.reaches_prophet {
            let whose = pre!(r"[\s،,]*سال\s+").match_(norm.sl(story_end, story_end + 12));
            let got = whose.and_then(|w| asked_name(&norm, story_end + w.end()));
            if let Some((g0, g1)) = got {
                if !strip(norm.sl(g0, g1), " ،,").is_empty()
                    && p("_PROPHET_RE").match_at(&norm, g0).is_none()
                    && p("_PROPHET_NAME_RE").match_at(&norm, g0).is_none()
                    && pre!(r"[\s،,]*عن(?!\w)").is_match(norm.sl(g1, g1 + 6))
                {
                    links.push(Link::new(
                        links.len() + 1,
                        "su'ila",
                        &clean_name(&orig(g0, g1)),
                        false,
                    ));
                    ends.push((end_of_chain, g1));
                    end_of_chain = g1;
                    story_end = g1;
                    break;
                }
            }
        }
    }

    if !links.is_empty()
        && !result.reaches_prophet
        && p("_LONE_GENERIC_RE").is_match(strip(&norm_name(&links.last().unwrap().raw_name), " ،,"))
        && !p("_PROPHET_RE").is_search(norm.sl(end_of_chain, end_of_chain + 50))
        && !p("_PROPHET_NAME_RE").is_search(norm.sl(end_of_chain, end_of_chain + 50))
        && !pre!(r"(?<!\w)(?:سمعت|سمع|حدثنا|حدثني|اخبرنا|اخبرني|انبانا|يحدث|حدث|يخبر)(?!\w)")
            .is_search(norm.sl(end_of_chain, end_of_chain + 60))
    {
        story_end = story_end.max(ends.last().unwrap().1);
        links.pop();
        end_of_chain = ends.pop().unwrap().0;
    }

    if links.len() > 1
        && ["sami'tu", "sami'a"].contains(&links.last().unwrap().term.as_str())
        && is_companion(&links[links.len() - 2])
        && p("_HEARD_DOING_RE").is_match(norm.sl(end_of_chain, end_of_chain + 30))
    {
        story_end = story_end.max(ends.last().unwrap().1);
        links.pop();
        end_of_chain = ends.pop().unwrap().0;
    }

    if links.len() >= 3
        && !result.reaches_prophet
        && p("_BARE_KUNYA_RE").is_match(strip(&norm_name(&links.last().unwrap().raw_name), " ،,"))
        && [
            "sami'tu",
            "haddathani",
            "haddathana",
            "'an",
            "akhbarani",
            "akhbarana",
        ]
        .contains(&links.last().unwrap().term.as_str())
        && pre!(r"^[\s،,:]*(?:يقول|قال)(?!\w)").is_match(norm.sl(end_of_chain, end_of_chain + 20))
    {
        links.pop();
        end_of_chain = ends.pop().unwrap().0;
        result.reaches_prophet = true;
    }

    if result.parallel_at >= 0 {
        if links.is_empty() {
            result.parallel_at = -1;
            result.parallel_kind = String::new();
        } else {
            if !result.reaches_prophet {
                let to = (end_of_chain + 90).min(result.parallel_at as usize);
                result.reaches_prophet = p("_PROPHET_RE").is_search(norm.sl(end_of_chain, to));
            }
            result.raw_isnad = clean_name(&orig(0, end_of_chain));
            result.links = links;
            result.ends_with = if result.reaches_prophet {
                "prophet".into()
            } else {
                String::new()
            };
            let at = result.parallel_at as usize;
            result.parallel_at = if at < n {
                index_map[at] as i64
            } else {
                tx.len() as i64
            };
            return result;
        }
    }

    let tail = p("_HONORIFIC_TAIL_RE")
        .sub(norm.sl(end_of_chain, end_of_chain + 160), "")
        .trim_start_matches([' ', '،', ','])
        .to_string();
    if !links.is_empty() && !result.reaches_prophet {
        result.reaches_prophet =
            p("_PROPHET_RE").is_search(norm.sl(end_of_chain, end_of_chain + 90));
    }

    if !links.is_empty()
        && !result.reaches_prophet
        && p("_JOINT_START_RE").is_match(&tail)
        && p("_JOINT_WORD_RE").is_search(&tail)
    {
        let joint = p("_JOINT_WORD_RE").search_in(&norm, end_of_chain, n.min(end_of_chain + 200));
        let got = joint
            .and_then(|j| resume(&tx, &norm, &index_map, j.end(), n.min(j.end() + 160), depth));
        if let Some((again_at, rest)) = got {
            if !rest.links.is_empty() {
                let k = links.len();
                result.links = links.clone();
                result.links.extend(
                    rest.links
                        .iter()
                        .enumerate()
                        .map(|(i, l)| Link::new(k + i + 1, &l.term, &l.raw_name, l.fresh)),
                );
                result.raw_isnad = clean_name(&format!(
                    "{}{}",
                    tx.sl(0, index_map[again_at]),
                    rest.raw_isnad
                ));
                result.reaches_prophet = rest.reaches_prophet;
                result.ends_with = rest.ends_with;
                result.problem = rest.problem;
                return result;
            }
        }
        result.raw_isnad = clean_name(&orig(0, end_of_chain));
        result.links = links;
        return result;
    }

    if tahwil_before(&norm, end_of_chain + 60) {
        result.raw_isnad = clean_name(&orig(0, n.min(end_of_chain + 60)));
        result.problem = problem("tahwil").into();
        result.links = links;
        return result;
    }
    let cont = p("_CONTINUES_RE");
    let tail_at = |at: usize| -> String {
        p("_HONORIFIC_TAIL_RE")
            .sub(norm.sl(at, at + 160), "")
            .trim_start_matches([' ', '،', ','])
            .to_string()
    };
    let mut goes_on = cont.match_(&tail);
    if goes_on.is_some() && skipped_to > end_of_chain {
        goes_on = cont.match_(&tail_at(skipped_to));
    }
    if goes_on.is_some() && story_end > end_of_chain {
        goes_on = cont.match_(&tail_at(story_end));
    }
    let prophet_at = [
        p("_PROPHET_RE").search(&tail),
        p("_PROPHET_NAME_RE").search(&tail),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|m| m.start());
    if goes_on.is_some()
        && links
            .last()
            .is_some_and(|l| ["sa'altu", "su'ila", "sa'ala"].contains(&l.term.as_str()))
        && pre!(r"(?:[\s\-]*رضي الله \S+)?[\s\-،,]*(?:عن|حدثني|اخبرني|حدثيني|اخبريني)(?!\w)")
            .is_match(&tail)
    {
        goes_on = None;
    }
    if let Some(g) = &goes_on {
        let find = Ps::new(&tail).find("عن", 0).map(|x| x as i64).unwrap_or(-1);
        if pre!(r"(?:\S+\s+){0,3}?(?:سيل|سيلت|سال|سالته|سالها|يسال|سالت)\s+(?:\S+\s+){0,3}?عن\s")
            .is_match(&tail)
            && (g.end() as i64) <= 40 + find
        {
            goes_on = None;
        }
    }
    if !links.is_empty() {
        if let Some(g) = &goes_on {
            if prophet_at.as_ref().map_or(true, |pa| g.end() <= pa.start()) {
                let base = end_of_chain.max(skipped_to).max(story_end);
                if let Some((again_at, mut rest)) =
                    resume(&tx, &norm, &index_map, base, n.min(base + 320), depth)
                {
                    if !rest.links.is_empty() {
                        if is_companion(links.last().unwrap()) && !is_companion(&rest.links[0]) {
                            rest = Parsed {
                                reaches_prophet: result.reaches_prophet,
                                ..Default::default()
                            };
                        }
                        let k = links.len();
                        links.extend(
                            rest.links
                                .iter()
                                .enumerate()
                                .map(|(i, l)| Link::new(k + i + 1, &l.term, &l.raw_name, l.fresh)),
                        );
                        result.raw_isnad = clean_name(&format!(
                            "{}{}",
                            tx.sl(0, index_map[again_at]),
                            rest.raw_isnad
                        ));
                        result.reaches_prophet = rest.reaches_prophet
                            || (result.reaches_prophet && rest.links.is_empty());
                        result.ends_with = if result.reaches_prophet {
                            "prophet".into()
                        } else {
                            String::new()
                        };
                        result.problem = rest.problem.clone();
                        let shift = links.len() - rest.links.len();
                        result.alternates = rest
                            .alternates
                            .iter()
                            .map(|(i, a)| (i + shift, a.clone()))
                            .collect();
                        result.links = links;
                        return result;
                    }
                }
            }
        }
    }

    if tahwil_before(&norm, end_of_chain.max(pos) + 40) {
        result.raw_isnad = clean_name(&orig(0, end_of_chain.max(1)));
        result.problem = problem("tahwil").into();
        result.links = links;
        return result;
    }
    if links.is_empty() {
        result.problem = problem("no_narrator").into();
        return result;
    }
    result.raw_isnad = clean_name(&orig(0, end_of_chain));
    result.links = links;
    if result.reaches_prophet {
        result.ends_with = "prophet".into();
    }
    result
}

/// `_rescue_sunni`.
pub fn rescue_sunni(text: &str) -> Option<Vec<Parsed>> {
    let tx = Ps::new(text);
    let (norm, index_map) = norm_map(text);
    let lead = pre!(r"[\s\d.\-:]*(?:باب [^.]{0,120}?(?=و?(?:حدثني|حدثنا|اخبرنا|اخبرني)(?!\w))|قال (?:ابو عيسي|ابو داود|ابو عبد الرحمن|ابو عبد الله)\s*:?\s*(?:و\s*)?(?=قال\s+\S+(?:\s+بن\s+\S+)*\s+(?:سالت|سمعت|حدثنا|اخبرنا)|حدثنا|اخبرنا))")
        .match_at(&norm, 0)?;
    if lead.end() >= norm.len() {
        return None;
    }
    let mut rest = tx.from(index_map[lead.end()]).to_string();
    if let Some(said) = pre!(r"قال\s+").match_(norm.from(lead.end())) {
        let from = norm.from(lead.end() + said.end());
        rest = format!(
            "حدثنا {}",
            pre!(r"^([^،,]{3,40}?)\s+سالت\s").sub1_with(from, |m| format!("{}، قال سالت ", m.g(1)))
        );
    }
    let roads = parse_all_sunni(&rest, 0);
    if roads.is_empty() || roads[0].links.is_empty() || !roads[0].problem.is_empty() {
        return None;
    }
    if !roads[0].links.iter().all(|l| looks_named(&l.raw_name, 8)) {
        return None;
    }
    if roads[0].links.len() < 2
        && !pre!(r"(?<!\w)بلغه(?!\w)").is_search(pyre::upto(&norm_name(&rest), 120))
    {
        return None;
    }
    Some(roads)
}

/// `_complete_from_before`.
pub fn complete_from_before(chains: &mut Vec<Parsed>, text: &str, before: Option<&Parsed>) {
    let Some(before) = before else {
        return;
    };
    let norm = Ps::new(&norm_name(text));
    let mut added = vec![];
    for road in chains.iter_mut() {
        if road.links.is_empty()
            || road.reaches_prophet
            || !road.ends_with.is_empty()
            || is_companion(road.links.last().unwrap())
        {
            continue;
        }
        let last = road.links.last().unwrap().raw_name.clone();
        let tail_at = if last.is_empty() {
            None
        } else {
            norm.find(pyre::last_chars(&norm_name(&last), 12), 0)
        };
        let tail = match tail_at {
            Some(t) => norm.sl(t, t + 160).to_string(),
            None => norm.sl(0, 400).to_string(),
        };
        let same = p("_SAME_ISNAD_RE");
        let quiet =
            [problem("tahwil"), problem("second_chain"), ""].contains(&road.problem.as_str());
        if !same.is_search(&tail) && !(quiet && same.is_search(norm.sl(0, 600))) {
            continue;
        }
        let Some(k) = before.links.iter().position(|l| shared(&l.raw_name, &last)) else {
            continue;
        };
        if k + 1 >= before.links.len() {
            continue;
        }
        if road.problem == problem("tahwil") || road.problem == problem("second_chain") {
            road.problem = String::new();
        }
        let links: Vec<Link> = road
            .links
            .iter()
            .chain(before.links[k + 1..].iter())
            .enumerate()
            .map(|(i, l)| Link::new(i + 1, &l.term, &l.raw_name, l.fresh))
            .collect();
        added.push(Parsed {
            raw_isnad: road.raw_isnad.clone(),
            links,
            reaches_prophet: before.reaches_prophet,
            ends_with: before.ends_with.clone(),
            problem: problem("borrowed").into(),
            ..Default::default()
        });
    }
    chains.extend(added);
}

/// `_cited_companion`.
pub fn cited_companion(text: &str) -> Option<Parsed> {
    let norm = pyre::upto(&norm_name(text), 160).to_string();
    let m = pre!(r"(?<!\w)(?:من حديث|عن)\s+").search(&norm)?;
    let read = parse_sunni(
        &format!("عن {}", pyre::from(text, orig_len(text, m.end()))),
        0,
    );
    if read.links.is_empty() || words(&norm_name(&read.links[0].raw_name)) > 6 {
        return None;
    }
    let link = &read.links[0];
    if !is_companion_name(&strip_said(&link.raw_name)) {
        return None;
    }
    let prophet = p("_PROPHET_RE").is_search(&norm)
        || p("_PROPHET_NAME_RE").is_search(&norm)
        || read.reaches_prophet;
    Some(Parsed {
        raw_isnad: link.raw_name.clone(),
        links: vec![Link::new(1, "'an", &link.raw_name, false)],
        reaches_prophet: prophet,
        ends_with: if prophet {
            "prophet".into()
        } else {
            String::new()
        },
        ..Default::default()
    })
}
