//! The statistics in plain words (`stats_words.py`): every measure with what it measures, how to read it, what it
//! means for the science of hadith and an example; and the sentences that put one value into words.

/// key, name, what it measures, how to read it, what it means for hadith science, an example
pub const TERMS: [(&str, &str, &str, &str, &str, &str); 31] = [
    ("share", "Share (percentage)", "How many out of every hundred.", "50% is half, 10% one in ten.", "Every share on this page is counted from the imported books, never guessed.", "“78% of the chains reach the Prophet ﷺ”: of every 100 chains read, 78 end with him."),
    ("median", "Median", "The middle value: half of the cases are below it, half above.", "Unlike an average it is not pulled by a few very large or very small cases.", "The usual length of a chain: how many narrators usually stand between the compiler and the Prophet ﷺ.", "A median length of 5: half of the chains have 5 narrators or fewer, half 5 or more."),
    ("gini", "Gini coefficient", "How unevenly something is shared out: here, how unevenly the transmission is spread over the narrators.", "0: every narrator carries the same share. 1: one man carries everything. Above 0.6 the load rests on a few.", "A high value is the classical madar: a few great narrators (al-Zuhri, Qatada, al-A'mash …) through whom most hadith pass. It is a fact of transmission, not a defect.", "Gini 0.75: a small group of narrators carries most of the hadith."),
    ("lorenz", "Lorenz curve", "A picture of the same unevenness: the narrators sorted from the least to the most used, and how much of the transmission they carry together.", "The straight diagonal would be everyone carrying the same; the further the curve sags below it, the more the load rests on a few.", "Where the curve rises steeply at the end, those last few narrators are the pivots (madar) of the corpus.", "If the curve is at 20% when 80% of the narrators are counted, the busiest 20% carry 80% of the transmission."),
    ("pagerank", "PageRank (network weight)", "How central a narrator is in the web of who narrates from whom — not only how often he appears, but how many chains lead through him.", "Shown in thousandths (‰) of the whole flow: the larger, the more central.", "It finds the narrators the transmission flows through, like the critics' talk of the 'pivot' (madar) of a hadith.", "A narrator with 12‰ carries about one hundredth of the whole flow of transmission."),
    ("weakest", "The weakest link", "For each chain, the lowest-ranked of its identified narrators.", "A chain is graded by its weakest narrator: one weak man weakens the whole chain.", "This is the rule of the critics: “the chain follows the weakest of its men” (al-isnad yatba' akhass rijalih).", "A chain of four trustworthy narrators and one weak one has a weak weakest link."),
    ("time_gap", "Years between teacher and student", "For two neighbours in a chain, the year the student died minus the year the teacher died.", "Students usually die 20–60 years after their teachers. Over 110 years, or a student dying long before his teacher, is hard to believe.", "This is the critics' check of meeting (liqa') and contemporaneity (mu'asara): a pair that cannot have met points to a wrongly identified name, or a break in the chain (inqita').", "A teacher who died in 110 AH and a student in 250 AH are 140 years apart: one of the two names is probably someone else."),
    ("tabaqa_order", "Generations in order", "Whether the student belongs to the same or a later generation (tabaqa) than his teacher.", "Close to 100% is expected; the pairs out of order are listed.", "A student of an earlier generation than his teacher cannot be: a sign of a wrong identification.", "A Follower (tabaqa 3) narrating from a man of tabaqa 6 is out of order."),
    ("kendall", "Kendall's τb (rank correlation)", "How far two orderings agree: here, whether the weaker a chain's narrator, the weaker the grade.", "0: no relation. Up to 0.1 hardly any, 0.1–0.3 weak, 0.3–0.5 moderate, above 0.5 strong. A minus sign: they go in opposite directions.", "A clear positive value shows the scholars' grades follow the narrators; that it is not near 1 shows what else they weighed: other routes (mutaba'at, shawahid), hidden defects ('ilal), the text.", "τb 0.35: a moderate relation — weaker narrators usually go with weaker grades, but not always."),
    ("gamma", "Goodman–Kruskal γ", "The same question as τb, counting only the cases that are not tied.", "From −1 to +1; it is always larger than τb, because ties are left out.", "A second look at whether grades follow narrators.", "γ 0.55 beside τb 0.35 is the same relation, measured on the untied pairs."),
    ("interval", "95% interval", "The range the true value very probably lies in, given how many cases there are.", "A narrow interval is a sure figure; a wide one, an uncertain one. If the whole interval is above zero (or below it), the relation is not chance.", "It tells how much weight a finding can bear — like the critics' care to say how sure a judgement is.", "τb 0.30 [0.25–0.35]: the true value is very probably between 0.25 and 0.35 — surely above zero."),
    ("bootstrap", "Bootstrap", "A way to find the 95% interval: the hadith are drawn again at random many times and the measure recomputed each time.", "How much the result moves from draw to draw is how uncertain it is.", "It needs no assumption about how grades are spread.", "2,000 bootstrap samples: the measure computed 2,000 times on reshuffled hadith."),
    ("spearman", "Spearman's ρ (rank correlation)", "Whether two measures rise and fall together, by their order only.", "+1: they rise together perfectly. −1: one falls as the other rises. 0: no relation. Up to 0.1 hardly any, 0.1–0.3 weak, 0.3–0.5 moderate, above 0.5 strong.", "Ranks and grades are ordered, never averaged; ρ respects that.", "ρ −0.40 between rank and how often a narrator appears: the most used narrators are the stronger ones."),
    ("partial", "Partial correlation (“the others held equal”)", "The relation between two measures that is left when every other measure is held the same.", "Read like ρ; it removes what the two share only through a third.", "A long chain is not blamed for what its weak narrator does: each factor is credited only with its own part.", "Length and grade: ρ 0.20 alone, 0.05 with the weakest narrator held equal — length mattered mostly because long chains have more chances of a weak man."),
    ("pvalue", "p-value and q-value", "How likely a result this large would be if there were no relation at all. q is the same, corrected for having looked at many results at once.", "Below 0.05: a relation beyond chance. Above it: it could be chance. It says nothing of how large the relation is.", "With hundreds of pairs some look related by chance; q keeps those out.", "q < 0.001: practically impossible by chance."),
    ("odds", "Odds ratio", "How many times a factor multiplies the chance of a weaker grade.", "1: no effect. 2: twice the odds. 0.5: half. The interval beside it shows how sure it is.", "It measures how heavily a scholar's grade leans on a narrator of a given rank.", "Odds ratio 4.2 for a weak weakest narrator: such a chain has 4.2 times the odds of a weaker grade than one whose weakest narrator is trustworthy."),
    ("ordinal", "Ordinal regression (the hadith model)", "A model that foretells a scholar's grade from the chain: its weakest narrator, its length, its other routes — all at once.", "Each factor gets an odds ratio; together they show what the grade follows and how much is left unexplained.", "What it cannot foretell is where the scholar weighed more than one chain: other routes, hidden defects, the text.", "The model foretells “sahih” for a chain of trustworthy men; a scholar who grades it da'if saw something the chain does not show."),
    ("lr", "Likelihood-ratio χ²", "How much worse the model foretells the grades when one factor is taken out of it.", "The larger, the more that factor matters. p (Holm) below 0.05: it matters beyond chance.", "It ranks the parts of a chain by how much the grades depend on them.", "χ² 300 for the weakest narrator, 4 for length: the narrator decides far more than the length."),
    ("r2", "McFadden's R²", "How much of the grades the model explains, beside a model that knows only how often each grade is given.", "For human judgement 0.1–0.2 is usual, 0.2–0.4 very good. It is not a percentage.", "A modest value is the finding itself: a scholar grades on all routes and the text, not on one chain.", "R² 0.18: the chain explains a real but partial share of the grade."),
    ("somers", "Somers' D", "How well the model puts the hadith in the right order, from strong to weak.", "0: no better than chance. 1: perfect order. Above 0.4 is good.", "It asks whether the stronger chains get the stronger grades, even when the exact grade is missed.", "D 0.55: the model's order of the hadith mostly matches the scholar's."),
    ("calibration", "Calibration", "Whether the model's probabilities can be taken at their word.", "Points on the diagonal: when the model says 70%, about 70 of 100 such hadith got that grade.", "It keeps the model honest: a model that is sure must be right as often as it says.", "A point at 0.8 across and 0.78 up is well calibrated."),
    ("surprise", "Surprises (graded otherwise than the chain foretells)", "The hadith whose grade is far from what their chain alone would lead one to expect.", "The lower the model's probability for the grade given, the larger the surprise.", "Graded higher: usually strengthened by other routes (hasan or sahih li-ghayrihi). Graded lower: a hidden defect ('illa), an irregular text (shudhudh), a break the names do not show.", "A chain of trustworthy men graded da'if — worth reading: what did the scholar see?"),
    ("stuart", "Stuart–Maxwell test", "Whether one scholar gives stronger or weaker grades overall than another, on the same hadith.", "p below 0.05 (“no”): one is milder or stricter beyond chance. The counts show who and how often.", "It measures the classical labels: the lenient (mutasahil) and the strict (mutashaddid).", "On the same 3,000 hadith, scholar A grades higher than B 400 times and lower 90 times: A is milder."),
    ("brant", "Brant's test", "Whether a factor weighs the same at every step of the grades (sahih→hasan as hasan→da'if).", "“Yes, within chance”: the model's single weight is fair. “No”: the factor decides one step more than another.", "A weak narrator may decide between hasan and da'if more than between sahih and hasan.", "Odds ratio 1.5 at the first step, 4.0 at the second: the factor matters mostly for the lower grades."),
    ("kruskal", "Kruskal–Wallis test and ε²", "Whether the chains of several books differ in a measure (length, weakest narrator …). ε² is how large the difference is.", "p below 0.05: the books differ beyond chance. ε²: 0.01 small, 0.06 medium, 0.14 large.", "It compares the books on the same footing, without averaging ranks.", "ε² 0.08: a medium difference between the books."),
    ("cliff", "Mann–Whitney test and Cliff's δ", "Two books compared: how often a chain of the first is above one of the second, minus how often it is below.", "From −1 to +1. 0: no difference. Beyond ±0.15 small, ±0.33 medium, ±0.47 large.", "Which book's chains are longer, or rest on weaker narrators, and by how much.", "δ +0.30: a chain of the first book is longer than one of the second in 30% more of the pairs."),
    ("jaccard", "Shared narrators (Jaccard)", "Of the narrators found in either of two books, the share found in both.", "0%: no narrator in common. 100%: the very same narrators.", "Books of one school and one period share most; it shows the common routes of transmission.", "40%: four in ten of the narrators of the two books appear in both."),
    ("alpha", "Krippendorff's α", "How far all the graders of a book agree, beyond what chance would give, counting a near miss (hasan against sahih) as less than a far one (sahih against fabricated).", "1: complete agreement. 0: no better than chance. Above 0.8 strong, 0.67–0.8 allows tentative conclusions, below 0.67 weak.", "It measures how much the scholars' judgement is shared — and where ijtihad differs.", "α 0.72: the scholars mostly agree, with real differences of judgement."),
    ("kappa", "Weighted kappa (κw)", "The agreement of two graders beyond chance, a near miss counting less than a far one.", "1: perfect. 0: chance. Above 0.8 almost perfect, 0.6–0.8 substantial, 0.4–0.6 moderate, below 0.4 weak.", "Two scholars may agree 70% of the time only because most hadith are sahih; kappa removes that.", "κw 0.65: substantial agreement between the two."),
    ("dawid", "Dawid–Skene model (the “true grade”)", "A model that learns at once how each grader grades and what grade the graders' grades together most probably mean.", "For each hadith a grade and a certainty; for each grader a table of what he says for each true grade.", "It does not replace the scholars: the “true grade” is only what their grades imply together.", "Certainty 95%: the graders' grades point clearly to one grade."),
    ("strictness", "Strictness", "The chance a grader gives a hadith a lower grade than the others' grades imply, minus the chance he gives a higher one.", "Below zero: stricter (mutashaddid). Above zero: more lenient (mutasahil). Measured, not assumed.", "It puts a number on the classical labels of the critics.", "−0.08: the grader grades about 8 in 100 hadith lower than the others would."),
];

pub struct Term {
    pub key: &'static str,
    pub name: String,
    pub what: String,
    pub read: String,
    pub hadith: String,
    pub example: String,
}

fn tr(text: &str) -> String {
    crate::i18n::t(&crate::i18n::ui_language(), text, &[])
}

fn tr_with(text: &str, values: &[(&str, String)]) -> String {
    crate::i18n::t(&crate::i18n::ui_language(), text, values)
}

/// One measure, in the interface language.
pub fn term(key: &str) -> Option<Term> {
    let t = TERMS.iter().find(|t| t.0 == key)?;
    Some(Term {
        key: t.0,
        name: tr(t.1),
        what: tr(t.2),
        read: tr(t.3),
        hadith: tr(t.4),
        example: tr(t.5),
    })
}

pub fn all_terms() -> Vec<Term> {
    TERMS.iter().filter_map(|t| term(t.0)).collect()
}

fn casefold(s: &str) -> String {
    caseless::default_case_fold_str(s)
}

/// A measure by its key or (a part of) its name (`find`).
pub fn find(text: &str) -> Option<Term> {
    let text = casefold(text.trim());
    for t in TERMS.iter() {
        let one = term(t.0)?;
        let name = casefold(&one.name);
        if text == t.0 || text == name || name.contains(&text) || casefold(t.1).contains(&text) {
            return Some(one);
        }
    }
    None
}

/// The size of a correlation (τ, ρ, κ …) in words.
pub fn strength(v: Option<f64>) -> String {
    let Some(v) = v else {
        return tr("not measurable");
    };
    let a = v.abs();
    tr(if a < 0.1 {
        "hardly any relation"
    } else if a < 0.3 {
        "a weak relation"
    } else if a < 0.5 {
        "a moderate relation"
    } else {
        "a strong relation"
    })
}

/// Whether a 95% interval rules out "no relation".
pub fn sure(lo: Option<f64>, hi: Option<f64>) -> String {
    let (Some(lo), Some(hi)) = (lo, hi) else {
        return String::new();
    };
    if lo > 0.0 || hi < 0.0 {
        tr("too clear to be chance")
    } else {
        tr("could still be chance")
    }
}

pub fn by_q(q: Option<f64>) -> String {
    match q {
        None => String::new(),
        Some(q) if q < 0.05 => tr("too clear to be chance"),
        Some(_) => tr("could still be chance"),
    }
}

pub fn odds(x: f64) -> String {
    if x >= 1.05 {
        return tr_with("{x:.1f} times the odds", &[("x:.1f", format!("{x:.1}"))]);
    }
    if x <= 0.95 {
        return tr_with(
            "about {share} of the odds",
            &[("share", format!("{:.0}%", 100.0 * x))],
        );
    }
    tr("no change in the odds")
}

pub fn gini(g: f64) -> String {
    tr(if g < 0.3 {
        "evenly shared"
    } else if g < 0.5 {
        "somewhat uneven"
    } else if g < 0.7 {
        "uneven: a minority carries most"
    } else {
        "very uneven: a few carry most of it"
    })
}

pub fn alpha(a: f64) -> String {
    tr(if a >= 0.8 {
        "strong agreement"
    } else if a >= 0.667 {
        "enough agreement for tentative conclusions"
    } else {
        "weak agreement"
    })
}

pub fn kappa(k: f64) -> String {
    tr(if k >= 0.8 {
        "almost perfect agreement"
    } else if k >= 0.6 {
        "substantial agreement"
    } else if k >= 0.4 {
        "moderate agreement"
    } else {
        "weak agreement"
    })
}

pub fn r2(v: f64) -> String {
    tr(if v < 0.1 {
        "a small part"
    } else if v < 0.2 {
        "a usual share for human judgement"
    } else {
        "a very good share for human judgement"
    })
}

pub fn somers(v: f64) -> String {
    tr(if v < 0.2 {
        "an order hardly better than chance"
    } else if v < 0.4 {
        "a fair order"
    } else if v < 0.6 {
        "a good order"
    } else {
        "a very good order"
    })
}

pub fn eps2(v: f64) -> String {
    tr(if v < 0.01 {
        "a negligible difference"
    } else if v < 0.06 {
        "a small difference"
    } else if v < 0.14 {
        "a medium difference"
    } else {
        "a large difference"
    })
}

pub fn cliff(d: f64) -> String {
    let a = d.abs();
    tr(if a < 0.15 {
        "a negligible difference"
    } else if a < 0.33 {
        "a small difference"
    } else if a < 0.47 {
        "a medium difference"
    } else {
        "a large difference"
    })
}
