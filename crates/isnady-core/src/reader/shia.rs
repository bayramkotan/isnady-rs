//! Road B: the Shia chains (`_parse_shia` and its helpers in core/isnad.py), line for line.

use super::pyre::{self, split, strip, Ps};
use super::*;

fn masum_after(norm: &Ps, at: usize, window: usize) -> String {
    let tail = norm.sl(at, at + window).to_string();
    if let Some(honor) = p("_MASUM_HONOR_RE").search(&tail) {
        if !pyre::strip_ws(honor.group0()).is_empty() {
            let before = pyre::upto(&tail, honor.start());
            let ws: Vec<String> = split(&pre!(r"[()\[\]،,:]").sub(before, " "))
                .iter()
                .map(|s| s.to_string())
                .collect();
            let from = ws.len().saturating_sub(6);
            let key = masum_in(&ws[from..].join(" "));
            return if key.is_empty() { "imam".into() } else { key };
        }
    }
    let head = pyre::upto(&tail, 70);
    for (form, rx) in [
        ("رسول الله", pre!(r"(?<!\w)(?:ل|ب)?رسول الله(?!\w)")),
        ("النبي", pre!(r"(?<!\w)(?:ل|ب)?النبي(?!\w)")),
        ("امير المومنين", pre!(r"(?<!\w)(?:ل|ب)?امير المومنين(?!\w)")),
    ] {
        if rx.is_search(head) {
            return which_masum(form);
        }
    }
    String::new()
}

fn second_road(norm: &Ps, at: usize, links_so_far: usize) -> &'static str {
    let head = norm.sl(at, at + 120);
    let word = split(head)
        .first()
        .map(|s| s.to_string())
        .unwrap_or_default();
    if word.is_empty() || tables().description_words.contains(&word) {
        return "end";
    }
    let Some(nxt) = p("_SHIA_TERM_RE").search_at(norm, at) else {
        return "end";
    };
    let name = p("_PRAISE_RE").sub(norm.sl(at, nxt.start()), " ");
    let name = pre!(r"(?<!\w)(?:قالا|قالوا|قال)(?!\w)|[-:؛]").sub(&name, " ");
    let name = strip(&name, " ،,").to_string();
    if pre!(r"(?<!\w)جميعا$").is_search(&name) && !p("_MASUM_HONOR_RE").is_search(&name) {
        return "joined";
    }
    let parts: Vec<String> = pre!(r"\sو(?=\S)")
        .split(&name)
        .into_iter()
        .filter(|x| !pyre::strip_ws(x).is_empty())
        .collect();
    if parts.is_empty()
        || parts.iter().any(|x| words(x) > 8)
        || p("_SHIA_STOP_RE").is_search(&name)
        || p("_MASUM_HONOR_RE").is_search(&name)
    {
        return "end";
    }
    let name = pyre::strip_ws(&parts[0]).to_string();
    if let Some(meet) = p("_ALL_RE").search_in(norm, nxt.start(), nxt.start() + 400) {
        if p("_MASUM_HONOR_RE")
            .search_in(norm, nxt.start(), meet.start())
            .is_none()
        {
            return "joined";
        }
    }
    if links_so_far >= 2
        && tables()
            .compiler_teachers
            .iter()
            .any(|t| name == *t || name.starts_with(&format!("{t} ")))
    {
        return "new road";
    }
    "same link"
}

fn min_comma(norm: &Ps, from: usize) -> usize {
    [norm.find("،", from), norm.find(",", from)]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(norm.len())
}

const BI_ISNADIHI: &str = "refers to the chain of an earlier hadith (bi-isnadihi); not split";

pub fn parse_shia(text: &str) -> Parsed {
    let text =
        pre!(r"\[([^\]\d\[\s]+\s[^\]\d\[]{1,38})\]").sub_with(text, |m| format!(" {} ", m.g(1)));
    let text = pre!(r"(?<=[،,]\s)غن(?=\s)").sub(&text, "عن");
    let tx = Ps::new(&text);
    let (norm_s, index_map) = normalize_with_map(&text);
    let norm = Ps::new(&norm_s);
    let n = norm.len();
    let orig = |a: usize, b: usize| -> String {
        let s = if a < n { index_map[a] } else { tx.len() };
        let e = if b < n { index_map[b] } else { tx.len() };
        tx.sl(s, e).to_string()
    };
    let term_re = p("_SHIA_TERM_RE");
    let honor_re = p("_MASUM_HONOR_RE");
    let mut result = Parsed::default();
    let mut pos = p("_PREAMBLE_RE")
        .match_at(&norm, 0)
        .map(|m| m.end())
        .unwrap_or(0);
    if p("_ALL_START_RE").match_at(&norm, pos).is_some() {
        result.problem = BI_ISNADIHI.into();
        return result;
    }
    let from_pos = norm.from(pos).to_string();
    let mut lead = pre!(r"(?:(?:(?:رجال|كتاب|تفسير|امالي)\s+\S+|الكافي|التهذيب|الاستبصار|الفقيه|الخصال|العيون|الامالي|المحاسن|البصاير|العلل|التوحيد|المعاني|الغيبه|الارشاد|الاحتجاج|الاختصاص|الكشي)\s*:\s*|(?:و)?قال\s+(?:الشيخ\s+)?[^:]{0,90}?مصنف هذا الكتاب[^:]{0,40}:\s*|(?:و)?قال\s*:?\s+(?=ابي\s*\(?\s*(?:رحمه|ره|رضي))|(?:و)?قال\s+(?!و\s|و?(?:حدثني|حدثنا|اخبرني|اخبرنا|عن|سمعت|لي|لنا)(?!\w))(?=[^:،,]{2,40}?\s+عن\s)|(?:[^\s:،,]+\s+){0,2}[^\s:،,]+\s*:\s*(?=\S))")
        .match_(&from_pos);
    if let Some(l) = &lead {
        let g = l.group0();
        let first_word = split(norm.sl(pos, pos + l.end()))
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default();
        if g.trim_end().ends_with(':')
            && !g.contains("مصنف هذا الكتاب")
            && (name_words().contains(&first_word)
                || term_re.is_search(g)
                || pre!(r"(?<!\w)(?:قال|قالت|قالا|يقول|سالت|قلت)(?!\w)").is_search(g)
                || !pre!(r"(?<!\w)(?:الكافي|التهذيب|الاستبصار|الفقيه|الصدوق|الطوسي|الكشي|المفيد|البرقي|الصفار|العياشي|القمي|النعماني|رجال|كتاب|تفسير|امالي|اصول|فروع|توحيد|غيبه|علل|عيون|معاني|خصال|محاسن|بصاير|ثواب|عقاب|كامل|اختيار|الامالي|التوحيد|الخصال|العيون|المعاني|العلل|المحاسن|البصاير|الغيبه|الارشاد|الاحتجاج|الاختصاص|الوسايل|البحار)(?!\w)").is_search(g))
        {
            lead = None;
        }
    }
    if let Some(l) = &lead {
        if pre!(r"باسناده\s+عن\s").is_match(norm.from(pos + l.end())) {
            let rx = Re::new(&format!(r"{}باسناده\s+(?=عن\s)", regex::escape(l.group0())));
            lead = rx.match_(&from_pos);
        }
    }
    if let Some(l) = &lead {
        pos += l.end();
    }
    let said_first = lead
        .as_ref()
        .is_some_and(|l| l.group0().trim_start_matches('و').starts_with("قال"));
    let mut links: Vec<Link> = vec![];
    let mut end_of_chain = pos;
    let mut first = true;
    let mut met = false;
    while pos < n {
        if let Some(filler) = p("_FILLER_RE").match_at(&norm, pos) {
            if !(first
                && norm.sl(pos, pos + 1) == "و"
                && p("_JOIN_RE").match_at(&norm, pos).is_none())
            {
                pos = filler.end();
            }
        }
        let term = term_re.match_at(&norm, pos);
        let (tid, mut name_start) = match &term {
            Some(t) => (term_id(t.g(1)).to_string(), t.end()),
            None if first => ((if said_first { "qala" } else { "bare" }).to_string(), pos),
            None => break,
        };
        first = false;
        while name_start < n && " \t".contains(norm.at(name_start).unwrap()) {
            name_start += 1;
        }
        let next_comma = min_comma(&norm, name_start);
        let next_term = term_re.search_at(&norm, name_start);
        let mut name_end = next_comma
            .min(next_term.as_ref().map(|m| m.start()).unwrap_or(n))
            .min(name_start + 160);
        if let Some(stop) = p("_SHIA_STOP_RE").search_in(&norm, name_start, name_end) {
            name_end = stop.start();
        }
        let honor = honor_re.search_in(&norm, name_start, name_end);
        if let Some(h) = &honor {
            name_end = h.start();
        }
        if let Some(join) = pre!(r"\sو(?=[^\s])|؛|\[").search(norm.sl(name_start, name_end)) {
            if join.start() > 0 {
                name_end = name_start + join.start();
            }
        }
        if let Some(place) = p("_PLACE_RE").search_in(&norm, name_start, name_end) {
            if place.start() > name_start {
                name_end = place.start();
            }
        }
        let mut core = strip(norm.sl(name_start, name_end), " ،,:-()").to_string();
        let mut ws = split(&core).len();
        let sons = pre!(r"(?<!\w)(?:بن|ابن)(?!\w)").finditer(&core).len();
        let ancestor = honor.as_ref().is_some_and(|h| {
            sons >= 3
                && pre!(r"[\s،,:\-)]*(?:(?:قال\s*:?\s*)?و?(?:حدثنا|حدثني|اخبرنا|اخبرني|عن)(?!\w)|و(?!\s))").is_match(norm.from(h.end()))
        });
        let masum = if !core.is_empty() && !ancestor {
            which_masum(&core)
        } else {
            String::new()
        };
        let honoured = honor.as_ref().is_some_and(|h| h.start() <= name_end + 1) && !ancestor;
        if !masum.is_empty()
            && (honor.is_some()
                || tables()
                    .always_masum
                    .iter()
                    .any(|f| core == *f || core.starts_with(&format!("{f} "))))
        {
            result.reaches_prophet = true;
            result.ends_with = masum;
            break;
        }
        if honoured && ws > 0 && ws <= 6 {
            result.reaches_prophet = true;
            result.ends_with = "imam".into();
            break;
        }
        let max_words = tables().max_name_words;
        let long_ok = |core: &str| -> bool {
            let w = words(core);
            ancestor || (w <= 26 && sons * 3 + 2 >= w)
        };
        if ws > 0 && words(&core) > max_words && !long_ok(&core) {
            let cut = name_prefix(&norm, name_start, name_end);
            let said = cut.and_then(|c| p("_SAID_TERM_RE").search_in(&norm, c, n.min(c + 200)));
            if let (Some(c), Some(s)) = (cut, &said) {
                if c > 0
                    && looks_named(norm.sl(name_start, c), 8)
                    && pre!(r"[\s:،,]*و?(?:حدثنا|حدثني|اخبرنا|اخبرني)(?!\w)")
                        .is_match(norm.from(s.end()))
                {
                    name_end = c;
                    core = strip(norm.sl(name_start, name_end), " ،,:-()").to_string();
                    ws = split(&core).len();
                }
            }
        }
        if ws == 0 || (words(&core) > max_words && !long_ok(&core)) {
            break;
        }
        if met
            && !links.is_empty()
            && name_key(&core) == name_key(&norm_name(&links.last().unwrap().raw_name))
        {
            met = false;
            end_of_chain = name_end;
            pos = name_end;
            continue;
        }
        met = false;
        links.push(Link::new(
            links.len() + 1,
            &tid,
            &clean_name(&orig(name_start, name_end)),
            false,
        ));
        end_of_chain = name_end;
        pos = name_end;
        if let Some(h) = &honor {
            if h.start() == name_end {
                pos = h.end();
            }
        }
        let mut same = if links.len() == 1 {
            p("_SAME_CHAIN_RE").match_at(&norm, pos)
        } else {
            None
        };
        if let Some(s) = &same {
            let past =
                pre!(r"(?:\s+(?:المعتبر|الثلاثه|الصحيح|المتقدم|المذكور))?(?:\s*\([^)]{0,60}\))?")
                    .match_(norm.from(s.end()))
                    .map(|m| m.end())
                    .unwrap_or(0);
            let after = s.end() + past;
            if pre!(r"[\s،,]*(?:عن\s|ي?رفع(?:ه|وه)(?!\w))").is_match(norm.from(after)) {
                pos = after;
                same = None;
            } else if pre!(r"[\s،,:]*قال(?!\w)").is_match(norm.from(after)) {
                result.raised = true;
                result.ends_with = masum_after(&norm, after, 110);
                result.reaches_prophet = !result.ends_with.is_empty();
                result.links = links;
                result.raw_isnad = clean_name(&orig(0, after));
                return result;
            }
        }
        if same.is_some() {
            result.problem = BI_ISNADIHI.into();
            result.raw_isnad = clean_name(&orig(0, n.min(pos + 30)));
            return result;
        }
        let praise = p("_PRAISE_RE").match_at(&norm, pos);
        let at = praise.as_ref().map(|m| m.end()).unwrap_or(pos);
        let skip_any = pre!(r"[\s،,]*(?:(?:وغيره|وغيرهما)[\s،,]*)?(?:(?:باسانيد مختلفه|باسناده|بسنده)[\s،,]*)?(?=ي?رفع(?:ه|وه)(?!\w))")
            .match_(norm.from(at));
        if let Some(s) = &skip_any {
            if let Some(pr) = &praise {
                pos = pr.end();
            }
            pos += s.end();
        }
        if pre!(r"[\s،,]*ي?رفع(?:ه|وه)(?!\w)").is_match(norm.from(pos)) {
            result.raised = true;
            result.ends_with = masum_after(&norm, pos, 110);
            result.reaches_prophet = !result.ends_with.is_empty();
            break;
        }
        if let Some(all) = p("_ALL_RE").match_at(&norm, pos) {
            pos = all.end();
            met = true;
            continue;
        }
        if let Some(joint) = p("_JOIN_RE").match_at(&norm, pos) {
            let road = second_road(&norm, joint.end(), links.len());
            if road == "same link" {
                result.joined += 1;
                pos = term_re.search_at(&norm, joint.end()).unwrap().start();
                continue;
            }
            if road == "new road" {
                if let Some(meet) = p("_ALL_RE").search_in(&norm, pos, n.min(pos + 400)) {
                    if honor_re.search_in(&norm, pos, meet.start()).is_none() {
                        result.joined += 1;
                        pos = meet.end();
                        met = true;
                        continue;
                    }
                }
                let from = if joint.end() < n {
                    index_map[joint.end()]
                } else {
                    tx.len()
                };
                let mut second = parse_shia(tx.from(from));
                if !second.links.is_empty() && second.problem.is_empty() {
                    let last = links.last().unwrap().raw_name.clone();
                    let k = second.links.iter().position(|l| shared(&l.raw_name, &last));
                    let tail: Vec<Link> = match k {
                        Some(k) => second.links[k + 1..].to_vec(),
                        None => vec![],
                    };
                    let has_tail = !tail.is_empty();
                    let mut first_road = Parsed {
                        raw_isnad: clean_name(&text),
                        links: links
                            .iter()
                            .chain(tail.iter())
                            .enumerate()
                            .map(|(i, l)| Link::new(i + 1, &l.term, &l.raw_name, false))
                            .collect(),
                        reaches_prophet: if has_tail {
                            second.reaches_prophet
                        } else {
                            false
                        },
                        ends_with: if has_tail {
                            second.ends_with.clone()
                        } else {
                            String::new()
                        },
                        raised: if has_tail { second.raised } else { false },
                        ..Default::default()
                    };
                    let more = std::mem::take(&mut second.others);
                    let mut others = vec![second];
                    others.extend(more);
                    first_road.others = others;
                    return first_road;
                }
                result.raw_isnad = clean_name(&orig(0, n.min(pos + 160)));
                result.problem =
                    "two roads from the compiler join without \"jami'an\"; not split".into();
                return result;
            }
            if road == "joined" {
                let Some(meet) = p("_ALL_RE").search_in(&norm, pos, n.min(pos + 400)) else {
                    break;
                };
                result.joined += 1;
                pos = meet.end();
                met = true;
                continue;
            }
            break;
        }
        if let Some(pr) = p("_PRAISE_RE").match_at(&norm, pos) {
            pos = pr.end();
        }
        if let Some(cited) = p("_SHIA_SAID_RE").match_at(&norm, pos) {
            let g = cited.g(1);
            if !honor_re.is_search(g)
                && words(g) <= 7
                && !term_re.is_search(g)
                && !p("_SHIA_STOP_RE").is_search(g)
                && !pre!(r"(?:لي|لنا|له|لها|قال)(?!\w)").is_match(g)
            {
                links.push(Link::new(
                    links.len() + 1,
                    "qala",
                    &clean_name(&orig(cited.start_g(1), cited.end_g(1))),
                    false,
                ));
                end_of_chain = cited.end_g(1);
                pos = end_of_chain;
                continue;
            }
        }
        if !links.is_empty() && pre!(r"[\s،,]*باسناده[\s،,]*(?=عن\s)").is_match(norm.from(pos))
        {
            pos += pre!(r"[\s،,]*باسناده[\s،,]*")
                .match_(norm.from(pos))
                .unwrap()
                .end();
            continue;
        }
        let filler = p("_FILLER_RE").match_at(&norm, pos);
        let after_filler = filler.as_ref().map(|f| f.end()).unwrap_or(pos);
        if tid != "bare" && term_re.match_at(&norm, after_filler).is_none() {
            if let Some(said) = p("_SAID_TERM_RE").search_in(&norm, pos, n.min(pos + 140)) {
                if honor_re.search_in(&norm, pos, said.start()).is_none()
                    && !pre!(r"(?<!\w)(?:وجدت|كتاب|كتب)(?!\w)")
                        .is_search(norm.sl(pos, said.start()))
                {
                    pos = said.end();
                }
            }
        }
    }

    if !links.is_empty() && !result.reaches_prophet && !result.raised {
        let key = masum_after(&norm, end_of_chain, 110);
        if !key.is_empty() {
            result.reaches_prophet = true;
            result.ends_with = key;
        } else if p("_MUDMAR_RE").is_match(norm.sl(end_of_chain, end_of_chain + 40)) {
            result.ends_with = "mudmar".into();
        }
    }
    if links.is_empty() {
        result.problem = "no narrator found before the text".into();
        return result;
    }
    if links.len() == 1
        && links[0].term == "bare"
        && !result.reaches_prophet
        && !result.raised
        && !pre!(r"[\s،,:]*(?:قال|قالا|قالوا|يقول)(?!\w)")
            .is_match(norm.sl(end_of_chain, end_of_chain + 12))
        && !pre!(r"[\s،,]*و[^:،,.]{2,50}?(?:وغيرهما|وغيره|وغيرهم)?\s*(?:قالوا|قالا)(?!\w)")
            .is_match(norm.sl(end_of_chain, end_of_chain + 90))
    {
        result.problem = "no chain: a name without a transmission after it".into();
        return result;
    }
    result.raw_isnad = clean_name(&orig(0, end_of_chain));
    result.links = links;
    result
}

/// `_shia_without_chain`.
pub fn shia_without_chain(text: &str, read: &Parsed) -> Parsed {
    let lead = &tables().shia_lead;
    let head = pyre::upto(&norm_name(text), 220).to_string();
    let ht = Ps::new(&head);
    let start = pre!(&format!(r"{lead}(?:و\s*)?(?:عن|قال|وقال)\s+")).match_(&head);
    if let Some(s) = &start {
        if !read.ends_with.is_empty() {
            let six: Vec<&str> = split(ht.from(s.end())).into_iter().take(6).collect();
            if !masum_in(&six.join(" ")).is_empty()
                || p("_MASUM_HONOR_RE")
                    .search_in(&ht, s.end(), s.end() + 50)
                    .is_some()
            {
                return Parsed {
                    problem: problem("direct").into(),
                    reaches_prophet: true,
                    ends_with: read.ends_with.clone(),
                    ..Default::default()
                };
            }
        }
    }
    if p("_SAME_CHAIN_OLD_RE").is_match(&head) || p("_FROM_HIM_RE").is_match(&head) {
        return read.clone();
    }
    if pre!(&format!(r"{lead}و(?!منهم)")).is_match(&head)
        && !pre!(&format!(r"{lead}و(?:قد|كان|اما|هذا|ذلك)(?!\w)")).is_match(&head)
    {
        return Parsed::with_problem(problem("continued"));
    }
    if let Some(cited) = pre!(&format!(r"{lead}(?:و)?قال\s+([^:،,.()]{{2,45}}?)\s*:")).match_(&head)
    {
        let g = cited.g(1);
        if words(g) <= 6
            && !p("_PROPHET_RE").is_search(g)
            && !p("_MASUM_HONOR_RE").is_search(g)
            && !pre!(r"(?:مصنف|المصنف|الشيخ|ابو جعفر مصنف)").is_match(g)
        {
            let tx = Ps::new(text);
            let name = tx
                .sl(
                    orig_len(text, cited.start_g(1)),
                    orig_len(text, cited.end_g(1)),
                )
                .to_string();
            return Parsed {
                raw_isnad: clean_name(&name),
                links: vec![Link::new(1, "qala", &clean_name(&name), false)],
                ..Default::default()
            };
        }
    }
    if pre!(&format!(r"{lead}(?:قال\s+(?:الشيخ\s+)?[^:]{{0,60}}?(?:مصنف|المصنف)|قلنا|فان قيل|فان قلتم|قيل|اقول|\(اقول\))")).is_match(&head) {
        return Parsed::with_problem(problem("note"));
    }
    Parsed::with_problem(problem("no_chain"))
}
