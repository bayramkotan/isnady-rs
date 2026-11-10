//! Rules of the rijal works, taken from the works themselves (`core/rijal.py`), and the Shia rijal read on their own
//! terms (`core/shia_rijal.py`). Ibn Hajar's twelve ranks and twelve tabaqat, his death years written without the
//! hundreds, the book marks. Nothing here decides who a narrator is; that is the matcher.

use rusqlite::{params, Connection, OptionalExtension};

use crate::pre;
use crate::reader::pyre;

// ---------------------------------------------------------------- ranks
const LAPSE: &str = "يهم|أوهام|اوهام|يخطئ|يخطىء|يغرب|سيء الحفظ|سيىء الحفظ|تغير|اختلط|رمي|بدعة|يتشيع|تشيع|القدر|نصب|\
                     إرجاء|ارجاء|يدلس|مدلس|مناكير|غرائب|أفراد";

/// The part of a Taqrib verdict that is Ibn Hajar's own judgement (`own_words`).
pub fn own_words(phrase: &str) -> String {
    let others = pre!(
        r"\s(?:و|ف)?(?:قال|قالوا|قيل|أفرط|ضعفه|ضعفوه|وثقه|وثقوه|لينه|كذبه|كذبوه|تكلم|تحامل|غمزه|وهاه|اتهمه|جرحه|عابه|أنكر|وثق|ضعف)(?!\w)"
    );
    let spaced = format!(" {phrase}");
    match others.search(&spaced) {
        Some(m) if m.start() > 1 => pyre::strip_ws(pyre::upto(phrase, m.start() - 1)).to_string(),
        _ => phrase.to_string(),
    }
}

fn has(p: &str, re: &pyre::Re) -> bool {
    re.search(p).is_some()
}

/// Ibn Hajar's rank for a verdict phrase of the Taqrib, or None if it cannot be read with confidence (`rank_of`).
pub fn rank_of(phrase: &str) -> Option<i64> {
    let p = format!(" {} ", pre!(r"\s+").sub(&own_words(phrase), " "));
    let p = p.as_str();
    if has(
        p,
        pre!(
            r"(?<![\w])(?:(?:ال)?صحابي|(?:ال)?صحابية|له صحبة|لها صحبة|له رؤية|لها رؤية|صحب النبي|أم المؤمنين|زوج النبي|من الصحابة|المكثرين من الصحابة|من السابقين الأولين|شهد بدرا|أحد العشرة)(?![\w])"
        ),
    ) {
        return Some(1);
    }
    if has(p, pre!(r"(?<![\w])(?:كذاب|وضاع|يضع الحديث|كذبوه)(?![\w])")) {
        return Some(12);
    }
    if has(p, pre!(r"(?<![\w])(?:متهم|اتهم|اتهموه)(?![\w])")) {
        return Some(11);
    }
    if has(
        p,
        pre!(r"(?<![\w])(?:متروك|متروكة|متروكا|واه|ساقط|تركوه)(?![\w])"),
    ) {
        return Some(10);
    }
    if has(
        p,
        pre!(r"(?<![\w])(?:مجهول|مجهولة|لا يعرف|لا تعرف|لا يعرفان)(?![\w])"),
    ) && !has(p, pre!(r"(?<![\w])(?:مجهول الحال|مجهولة الحال)(?![\w])"))
    {
        return Some(9);
    }
    if has(p, pre!(r"(?<![\w])(?:ضعيف|ضعيفة|ضعيفا)(?![\w])")) {
        return Some(8);
    }
    if has(
        p,
        pre!(r"(?<![\w])(?:مستور|مستورة|مجهول الحال|مجهولة الحال)(?![\w])"),
    ) {
        return Some(7);
    }
    if has(p, pre!(r"(?<![\w])(?:مقبول|مقبولة|لين الحديث|لين)(?![\w])")) {
        return Some(6);
    }
    if has(
        p,
        pre!(r"(?<![\w])(?:صدوق|صدوقة|صدوقا|لا بأس به|ليس به بأس)(?![\w])"),
    ) {
        static LAPSE_RE: std::sync::OnceLock<pyre::Re> = std::sync::OnceLock::new();
        let lapse =
            LAPSE_RE.get_or_init(|| pyre::Re::new(&format!(r"(?<![\w])(?:{LAPSE})(?![\w])")));
        return Some(if has(p, lapse) { 5 } else { 4 });
    }
    if has(
        p,
        pre!(r"(?<![\w])(?:ثقة|ثبت|متقن|عدل|حجة|حافظ|إمام|امام|أوثق|اوثق)(?![\w])"),
    ) {
        if has(p, pre!(r"(?<![\w])(?:أوثق|اوثق)(?![\w])")) {
            return Some(2);
        }
        let n = pre!(r"(?<![\w])(ثقة|ثبت|متقن|عدل|حجة|حافظ|إمام|امام)(?![\w])")
            .finditer(p)
            .len();
        return Some(if n >= 2 { 2 } else { 3 });
    }
    if has(p, pre!(r"(?<![\w])(?:أمير المؤمنين)(?![\w])")) {
        return Some(1);
    }
    None
}

// -------------------------------------------------------------- tabaqat
/// `TABAQA_WORDS`.
pub fn tabaqa_word(word: &str) -> Option<i64> {
    Some(match word {
        "الأولى" => 1,
        "الثانية" => 2,
        "الثالثة" => 3,
        "الرابعة" => 4,
        "الخامسة" => 5,
        "السادسة" => 6,
        "السابعة" => 7,
        "الثامنة" => 8,
        "التاسعة" => 9,
        "العاشرة" => 10,
        "الحادية عشرة" => 11,
        "الثانية عشرة" => 12,
        _ => return None,
    })
}

/// `generation`.
pub fn generation(tabaqa: Option<i64>) -> Option<&'static str> {
    let t = tabaqa?;
    Some(match t {
        1 => "sahabi",
        2..=5 => "tabi'i",
        6 => "contemporary of junior tabi'in",
        7..=9 => "tabi' al-tabi'in",
        _ => "later",
    })
}

// ---------------------------------------------------------- death years
fn unit(w: &str) -> Option<i64> {
    Some(match w {
        "واحد" | "واحدة" | "إحدى" | "احدى" => 1,
        "اثنتين" | "اثنين" | "اثنتي" | "اثني" | "ثنتين" | "ثنتي" => 2,
        "ثلاث" | "ثلاثة" => 3,
        "أربع" | "اربع" | "أربعة" => 4,
        "خمس" | "خمسة" => 5,
        "ست" | "ستة" => 6,
        "سبع" | "سبعة" => 7,
        "ثمان" | "ثماني" | "ثمانية" => 8,
        "تسع" | "تسعة" => 9,
        "عشر" | "عشرة" => 10,
        _ => return None,
    })
}

fn tens(w: &str) -> Option<i64> {
    Some(match w {
        "عشرين" => 20,
        "ثلاثين" => 30,
        "أربعين" | "اربعين" => 40,
        "خمسين" => 50,
        "ستين" => 60,
        "سبعين" => 70,
        "ثمانين" => 80,
        "تسعين" => 90,
        _ => return None,
    })
}

fn hundreds(w: &str) -> Option<i64> {
    Some(match w {
        "مائة" | "مئة" | "المائة" => 100,
        "مائتين" | "المائتين" => 200,
        "ثلاثمائة" => 300,
        "أربعمائة" | "اربعمائة" => 400,
        _ => return None,
    })
}

fn digits_value(s: &str) -> i64 {
    s.chars()
        .filter_map(|c| {
            c.to_digit(10).map(|d| d as i64).or_else(|| {
                let u = c as u32;
                [0x660u32, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xFF10]
                    .iter()
                    .find(|b| (**b..**b + 10).contains(&u))
                    .map(|b| (u - b) as i64)
            })
        })
        .fold(0, |a, d| a * 10 + d)
}

/// Arabic number words (or digits) → (number, hundreds written) (`parse_year_words`).
pub fn parse_year_words(words: &str) -> (Option<i64>, bool) {
    if let Some(m) = pre!(r"\d{1,3}").search(words) {
        let n = digits_value(m.group0());
        return (Some(n), n >= 100);
    }
    let (mut total, mut seen, mut hundred) = (0i64, false, false);
    let spaced = words.replace('و', " و ");
    for token in pre!(r"\s+").split(pyre::strip_ws(&spaced)) {
        let token = pyre::strip(&token, "،,.");
        if token == "و" || token.is_empty() {
            continue;
        }
        if let Some(v) = unit(token) {
            total += v;
            seen = true;
        } else if let Some(v) = tens(token) {
            total += v;
            seen = true;
        } else if let Some(v) = hundreds(token) {
            total += v;
            seen = true;
            hundred = true;
        } else {
            break;
        }
    }
    (seen.then_some(total), hundred)
}

/// The full hijri year from Ibn Hajar's wording, adding the hundreds his rule leaves out (`death_year`).
pub fn death_year(phrase: &str, tabaqa: Option<i64>) -> Option<i64> {
    let (number, mut hundreds_written) = parse_year_words(phrase);
    let mut number = number?;
    if number < 10 && !hundreds_written {
        let mut rest = phrase.to_string();
        loop {
            let other = pre!(r"(?<!\w)(?:وقيل|ويقال|أو|او|سنة)\s+(?:سنة\s+|في\s+سنة\s+)?(.+)$")
                .search(&rest);
            let Some(other) = other else { break };
            if pre!(r"(?:مولده|ولد|وله|وهو بن|عن)\s")
                .search(pyre::upto(&rest, other.start()))
                .is_some()
            {
                break;
            }
            let (value, other_hundreds) = parse_year_words(other.g(1));
            if let Some(value) = value {
                if value % 100 >= 20 || value >= 100 {
                    number += (value - value % 100)
                        + if value % 100 >= 20 {
                            (value % 100) - (value % 10)
                        } else {
                            0
                        };
                    hundreds_written = hundreds_written || other_hundreds;
                    break;
                }
            }
            rest = other.g(1).to_string();
        }
    }
    if hundreds_written || tabaqa.is_none() {
        return hundreds_written.then_some(number);
    }
    let t = tabaqa.unwrap_or(0);
    Some(if t <= 2 {
        number
    } else if t <= 8 {
        if t <= 6 && number >= 80 {
            number
        } else {
            number + 100
        }
    } else if t == 9 && number >= 80 {
        number + 100
    } else {
        number + 200
    })
}

/// The year of Ibn Hajar's death phrase as the Taqrib gives it (`year_of_death`).
pub fn year_of_death(note: &str, tabaqa: Option<i64>) -> Option<i64> {
    let words = pre!(r"^(?:مات|قتل|استشهد|توفي)(?: سنة| في سنة)? ").sub(note, "");
    let mut words = pre!(r"(?<!\S)(و?)عسر(ين|ة)?(?!\S)")
        .sub_with(&words, |m| format!("{}عشر{}", m.g(1), m.g(2)));
    let within = pre!(r"^((?:\S+ ){1,5}?)سنة (.+)$").search(&words);
    if let Some(w) = within {
        if pre!(r"(?:قبل|بعد|في حدود)").match_(&words).is_none()
            && parse_year_words(w.g(1)).0.is_none()
            && pre!(r"(?<!\S)(?:وقيل|ويقال|أو|او)(?!\S)")
                .search(w.g(1))
                .is_none()
        {
            words = w.g(2).to_string();
        }
    }
    if pre!(r"(?:قبل|بعد|في حدود|بضع|سنة بضع)")
        .match_(&words)
        .is_some()
    {
        return None;
    }
    death_year(&words, tabaqa)
}

/// The years read again once when the rule changes (`DEATH_RULE`).
pub const DEATH_RULE: &str = "5";

/// Read the years of death of the Taqrib's narrators again when the rule changed; how many changed
/// (`fix_death_years`).
pub fn fix_death_years(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )?;
    let rule: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'death_rule'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if rule.as_deref() == Some(DEATH_RULE) {
        return Ok(0);
    }
    let mut changes = vec![];
    {
        let mut st = conn.prepare(
            "SELECT id, death_year_note, tabaqa, death_year_ah FROM persons \
             WHERE key LIKE 'taqrib:%' AND death_year_note IS NOT NULL",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        })?;
        for row in rows {
            let (pid, note, tabaqa, year) = row?;
            let new = year_of_death(&note, tabaqa);
            if new != year {
                changes.push((new, pid));
            }
        }
    }
    let tx = conn.unchecked_transaction()?;
    for (new, pid) in &changes {
        tx.execute(
            "UPDATE persons SET death_year_ah = ? WHERE id = ?",
            params![new, pid],
        )?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('death_rule', ?)",
        [DEATH_RULE],
    )?;
    tx.commit()?;
    if !changes.is_empty() {
        crate::cache::clear();
    }
    Ok(changes.len())
}

// ----------------------------------------------------------- book marks
/// The marks of `BOOK_MARKS`.
pub fn is_book_mark(w: &str) -> bool {
    matches!(
        w,
        "خ" | "خت"
            | "بخ"
            | "عخ"
            | "ر"
            | "ي"
            | "م"
            | "د"
            | "مد"
            | "صد"
            | "خد"
            | "قد"
            | "ف"
            | "ل"
            | "كد"
            | "ت"
            | "تم"
            | "س"
            | "عس"
            | "كن"
            | "ق"
            | "فق"
            | "ع"
            | "4"
            | "تمييز"
    )
}

// ------------------------------------------------------- Shia rijal (core/shia_rijal.py)
/// The part of an entry in which the critic speaks of the narrator himself (`shia_rijal.opening`).
pub fn opening(entry_text: &str) -> String {
    match pre!(r"(?<!\w)(أخبرنا|أخبرني|أخبرناه|حدثنا|حدثني|حدثناه|أنبأنا|عنه بكتاب|بكتابه)(?!\w)")
        .search(entry_text)
    {
        Some(m) => pyre::upto(entry_text, m.start()).to_string(),
        None => entry_text.to_string(),
    }
}

pub const NON_IMAMI: [&str; 6] = ["waqifi", "fathi", "zaydi", "ammi", "nawusi", "kaysani"];

/// The ranks of the Shia scale, for the import's report (`RANKS[k][1]`).
pub const SHIA_RANKS: [(i64, &str); 4] = [
    (1, "trustworthy (thiqa)"),
    (2, "praised (mamduh)"),
    (3, "trustworthy, not Imami (muwaththaq)"),
    (4, "weak (da'if)"),
];

/// (rank 1–4, madhhab) for an entry's opening words (`shia_rijal.assess`).
pub fn assess(text: &str) -> (Option<i64>, Option<&'static str>) {
    let t = pre!(r"[ً-ْٰ]").sub(text, "");
    let t = pre!(r"[أإآ]").sub(&t, "ا");
    let found = |re: &pyre::Re, t: &str| re.search(t).is_some();
    let negated = found(
        pre!(
            r"(?<![\w])(?:[وف])?(?:ليس بثقة|غير ثقة|ليس بذاك|ليس بالثقة|لم يكن بذاك|لم يكن بذلك|ليس بذلك|ليس بالقوي|لم يكن بالمرضي|ليس بالمرضي|غير مرضي)(?![\w])"
        ),
        &t,
    );
    let thiqa = !negated
        && found(
            pre!(
                r"(?<![\w])(?:[وف])?(?:ثقة ثقة|ثقة|ثقه|الثقة|ثقتهم|ثقتها|ثقتنا|ثقات|كلهم ثقات|اوثق|من اوثق الناس|اوثق الناس)(?![\w])"
            ),
            &t,
        );
    let weak = found(
        pre!(
            r"(?<![\w])(?:[وف])?(?:ضعيف|ضعيفا|ضعفه|ضعفوه|مضطرب الحديث|مضطرب|مخلط|يعرف وينكر|فاسد الحديث|غمز فيه|لين الحديث)(?![\w])"
        ),
        &t,
    ) || negated;
    let rejected = found(
        pre!(
            r"(?<![\w])(?:[وف])?(?:كذاب|كذابا|وضاع|يضع الحديث|متهم|ملعون|فاسد المذهب|مرتفع القول|غال|غاليا|من الغلاة|غلا)(?![\w])"
        ),
        &t,
    );
    let praised = found(
        pre!(
            r"(?<![\w])(?:[وف])?(?:جليل|جليل القدر|عين|وجه|وجها|وجههم|وجهها|وجيه|فقيههم|شيخ القميين|شيخ الطائفة|فاضل|خير|خيرا|صحيح الحديث|لا باس به|حسن الحديث|نقي الحديث|سليم الجنبة|قريب الامر|صدوق|شيخ اصحابنا|من وجوه اصحابنا|عظيم المنزلة|ممدوح|وجوه|جلة|من الجلة)(?![\w])"
        ),
        &t,
    );
    let t = pre!(
        r"(عدة|جماعة|بعض|جماعه) من اصحابنا|ذكره اصحابنا|ذكرها اصحابنا|قال اصحابنا|رواه اصحابنا"
    )
    .sub(&t, " ");
    let schools: [(&pyre::Re, &str); 7] = [
        (
            pre!(r"(?<![\w])(?:[وف])?(?:واقف|واقفي|واقفيا|واقفا|وقف|من الواقفة)(?![\w])"),
            "waqifi",
        ),
        (
            pre!(r"(?<![\w])(?:[وف])?(?:فطحي|فطحيا|من الفطحية)(?![\w])"),
            "fathi",
        ),
        (
            pre!(r"(?<![\w])(?:[وف])?(?:زيدي|زيديا|بتري|جارودي|من الزيدية)(?![\w])"),
            "zaydi",
        ),
        (
            pre!(r"(?<![\w])(?:[وف])?(?:عامي|عاميا|من العامة|من رجال العامة)(?![\w])"),
            "ammi",
        ),
        (
            pre!(r"(?<![\w])(?:[وف])?(?:ناووسي|من الناووسية)(?![\w])"),
            "nawusi",
        ),
        (
            pre!(r"(?<![\w])(?:[وف])?(?:كيساني|من الكيسانية)(?![\w])"),
            "kaysani",
        ),
        (
            pre!(r"(?<![\w])(?:[وف])?(?:غال|غاليا|من الغلاة|مرتفع القول)(?![\w])"),
            "ghali",
        ),
    ];
    let mut madhhab = schools
        .iter()
        .find(|(re, _)| found(re, &t))
        .map(|(_, k)| *k);
    if madhhab.is_none()
        && found(
            pre!(
                r"(?<![\w])(?:[وف])?(?:من اصحابنا|احد اصحابنا|من جلة اصحابنا|شيخ اصحابنا|وجه اصحابنا|من وجوه اصحابنا|من خيار الشيعة|امامي|من شيوخنا|شيخ الطائفة)(?![\w])"
            ),
            &t,
        )
    {
        madhhab = Some("imami");
    }
    let rank = if thiqa && madhhab.is_some_and(|m| NON_IMAMI.contains(&m)) {
        Some(3)
    } else if thiqa && madhhab != Some("ghali") {
        Some(1)
    } else if weak || rejected {
        Some(4)
    } else if praised {
        Some(2)
    } else {
        None
    };
    (rank, madhhab)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ibn_hajar() {
        assert_eq!(rank_of("ثقة ثبت"), Some(2));
        assert_eq!(rank_of("ثقة"), Some(3));
        assert_eq!(rank_of("صدوق يهم"), Some(5));
        assert_eq!(year_of_death("مات سنة سبع وخمسين", Some(1)), Some(57));
        assert_eq!(year_of_death("مات سنة ست وتسعين", Some(9)), Some(196));
        assert_eq!(
            year_of_death("مات سنة إحدى وعسرين ومائتين", Some(9)),
            Some(221)
        );
    }
}
