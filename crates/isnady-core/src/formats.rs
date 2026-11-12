//! File formats for export (`core/formats.py`, EX1): every writer isnady has.
//!
//! A TABLE (rows of plain values under named columns): json, jsonl, csv, tsv, xlsx, md, html, txt, sqlite.
//! A DOCUMENT (a title and sections of entries with headings and paragraphs): html, md, txt, epub, docx.
//! A NETWORK: graphml, gexf, json, csv. XLSX, DOCX and EPUB are the zip of XML their standards define.

use serde_json::{json, Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::pre;
use crate::stats::pyjson;

/// `EXTENSIONS`.
pub fn extension(fmt: &str) -> String {
    match fmt {
        "isnady-book" => ".isnady-book.zip".into(),
        "isnady-profile" => ".isnady-profile.zip".into(),
        "backup" => ".isnady-backup.zip".into(),
        f => format!(".{f}"),
    }
}

/// `LABELS`.
pub fn label(fmt: &str) -> &str {
    match fmt {
        "json" => "JSON",
        "jsonl" => "JSON Lines",
        "csv" => "CSV",
        "tsv" => "TSV",
        "xlsx" => "Excel (XLSX)",
        "md" => "Markdown",
        "html" => "HTML",
        "txt" => "Text",
        "sqlite" => "SQLite",
        "pdf" => "PDF",
        "epub" => "EPUB",
        "docx" => "Word (DOCX)",
        "graphml" => "GraphML (Gephi, yEd)",
        "gexf" => "GEXF (Gephi)",
        "isnady-book" => "isnady book (importable)",
        "isnady-profile" => "isnady profile",
        "backup" => "isnady backup",
        f => f,
    }
}

/// Mostly Arabic script: written right to left (`rtl`), counted over the first 400 characters.
pub fn rtl(text: &str) -> bool {
    let (mut arabic, mut other) = (0usize, 0usize);
    for c in text.chars().take(400) {
        if ('\u{600}'..='\u{6ff}').contains(&c) {
            arabic += 1;
        } else if c.is_alphabetic() {
            other += 1;
        }
    }
    arabic > 0 && arabic > other
}

pub type Row = Map<String, Value>;

#[derive(Debug, Clone, Default)]
pub struct Table {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
    /// written into JSON as it is (what, when, by which isnady)
    pub meta: Row,
}

#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub heading: String,
    /// (label, text): a label may be ""
    pub paragraphs: Vec<(String, String)>,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct Section {
    pub heading: String,
    pub entries: Vec<Entry>,
}

/// `_cell`.
pub fn cell(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| cell(Some(x)))
            .collect::<Vec<_>>()
            .join(" | "),
        Some(o @ Value::Object(_)) => pyjson::dumps(o),
        Some(x) => crate::config::py_str(x),
    }
}

fn io<T>(r: std::io::Result<T>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

fn zerr<T>(r: zip::result::ZipResult<T>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

fn parent(path: &Path) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
}

/// `html.escape(text)` (quotes too) or without them.
pub fn escape(text: &str, quote: bool) -> String {
    let mut s = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    if quote {
        s = s.replace('"', "&quot;").replace('\'', "&#x27;");
    }
    s
}

fn xml(text: &str) -> String {
    let clean: String = text
        .chars()
        .filter(|c| !matches!(*c as u32, 0..=8 | 0xb | 0xc | 0xe..=0x1f))
        .collect();
    escape(&clean, false)
}

/// One row as Python's csv.writer writes it (excel dialect, QUOTE_MINIMAL, "\r\n").
pub fn csv_line(fields: &[String], delimiter: char) -> String {
    let mut out = String::new();
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            out.push(delimiter);
        }
        if f.contains(delimiter) || f.contains('"') || f.contains('\n') || f.contains('\r') {
            out.push('"');
            out.push_str(&f.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(f);
        }
    }
    if fields.len() == 1 && fields[0].is_empty() {
        out.push_str("\"\"");
    }
    out.push_str("\r\n");
    out
}

pub const CSS: &str = "body{font-family:'Noto Sans','Segoe UI',sans-serif;margin:2em;color:#1d2433;line-height:1.5}
h1{font-size:1.6em;margin-bottom:.2em}h2{font-size:1.25em;margin-top:1.6em;border-bottom:1px solid #ccd}
h3{font-size:1.05em;margin:1.2em 0 .3em}.sub{color:#667}.label{color:#667;font-size:.85em}
.ar{direction:rtl;text-align:right;font-family:'Amiri','Scheherazade New','Noto Naskh Arabic',serif;font-size:1.3em}
.note{color:#667;font-size:.85em}table{border-collapse:collapse;font-size:.9em}
td,th{border:1px solid #ccd;padding:4px 7px;vertical-align:top}th{background:#eef}";

pub fn table_markdown(t: &Table) -> String {
    let esc = |v: Option<&Value>| cell(v).replace('|', "\\|").replace('\n', " ");
    let mut out = vec![
        format!("# {}", t.title),
        String::new(),
        format!("| {} |", t.columns.join(" | ")),
        format!(
            "|{}|",
            t.columns
                .iter()
                .map(|_| "---")
                .collect::<Vec<_>>()
                .join("|")
        ),
    ];
    for r in &t.rows {
        out.push(format!(
            "| {} |",
            t.columns
                .iter()
                .map(|c| esc(r.get(c)))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    out.join("\n") + "\n"
}

pub fn table_html(t: &Table) -> String {
    let head: String = t
        .columns
        .iter()
        .map(|c| format!("<th>{}</th>", escape(c, true)))
        .collect();
    let mut body = String::new();
    for r in &t.rows {
        body.push_str("<tr>");
        for c in &t.columns {
            let v = cell(r.get(c));
            let cls = if !v.is_empty() && rtl(&v) {
                " class='ar'"
            } else {
                ""
            };
            body.push_str(&format!("<td{cls}>{}</td>", escape(&v, true)));
        }
        body.push_str("</tr>");
    }
    format!(
        "<!doctype html><html><head><meta charset='utf-8'><title>{0}</title><style>{CSS}</style></head><body>\
         <h1>{0}</h1><table><tr>{head}</tr>{body}</table></body></html>",
        escape(&t.title, true)
    )
}

pub fn write_table(t: &Table, fmt: &str, path: &Path) -> Result<PathBuf, String> {
    parent(path);
    let cols = &t.columns;
    match fmt {
        "json" => {
            let mut body = Map::new();
            body.insert("schema".into(), json!("isnady-export/1"));
            body.insert("title".into(), json!(t.title));
            for (k, v) in &t.meta {
                body.insert(k.clone(), v.clone());
            }
            body.insert("columns".into(), json!(cols));
            body.insert(
                "rows".into(),
                Value::Array(t.rows.iter().cloned().map(Value::Object).collect()),
            );
            io(std::fs::write(
                path,
                pyjson::dumps_indent(&Value::Object(body), 1),
            ))?;
        }
        "jsonl" => {
            let mut out = String::new();
            for r in &t.rows {
                out.push_str(&pyjson::dumps(&Value::Object(r.clone())));
                out.push('\n');
            }
            io(std::fs::write(path, out))?;
        }
        "csv" | "tsv" => {
            let d = if fmt == "csv" { ',' } else { '\t' };
            let mut out = String::from("\u{feff}");
            out.push_str(&csv_line(cols, d));
            for r in &t.rows {
                out.push_str(&csv_line(
                    &cols.iter().map(|c| cell(r.get(c))).collect::<Vec<_>>(),
                    d,
                ));
            }
            io(std::fs::write(path, out))?;
        }
        "xlsx" => write_workbook(std::slice::from_ref(t), path)?,
        "md" => io(std::fs::write(path, table_markdown(t)))?,
        "html" => io(std::fs::write(path, table_html(t)))?,
        "pdf" => {
            let mut pdf = crate::pdf::Pdf::new(&t.title);
            pdf.paragraph(&t.title, 16.0, true, false, 0.0);
            pdf.space(6.0);
            let rows: Vec<Vec<String>> = t
                .rows
                .iter()
                .map(|r| cols.iter().map(|c| cell(r.get(c))).collect())
                .collect();
            pdf.table(cols, &rows, 8.0);
            pdf.save(path)?;
        }
        "txt" => {
            let mut lines = vec![t.title.clone(), String::new()];
            for r in &t.rows {
                for c in cols {
                    let v = cell(r.get(c));
                    if !v.is_empty() {
                        lines.push(format!("{c}: {v}"));
                    }
                }
                lines.push(String::new());
            }
            io(std::fs::write(path, lines.join("\n")))?;
        }
        "sqlite" => {
            let _ = std::fs::remove_file(path);
            let run = || -> rusqlite::Result<()> {
                let db = rusqlite::Connection::open(path)?;
                let safe: Vec<String> = cols
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let s = pre!(r"\W+").sub(c, "_");
                        if s.is_empty() {
                            format!("c{i}")
                        } else {
                            s
                        }
                    })
                    .collect();
                db.execute(
                    &format!(
                        "CREATE TABLE rows ({})",
                        safe.iter()
                            .map(|s| format!("{s} TEXT"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    [],
                )?;
                let tx = db.unchecked_transaction()?;
                {
                    let mut st = tx.prepare(&format!(
                        "INSERT INTO rows VALUES ({})",
                        vec!["?"; cols.len()].join(", ")
                    ))?;
                    for r in &t.rows {
                        st.execute(rusqlite::params_from_iter(
                            cols.iter().map(|c| cell(r.get(c))),
                        ))?;
                    }
                    tx.execute("CREATE TABLE meta (key TEXT, value TEXT)", [])?;
                    tx.execute("INSERT INTO meta VALUES ('title', ?)", [&t.title])?;
                    for (k, v) in &t.meta {
                        tx.execute(
                            "INSERT INTO meta VALUES (?, ?)",
                            rusqlite::params![k, cell(Some(v))],
                        )?;
                    }
                }
                tx.commit()
            };
            run().map_err(|e| e.to_string())?;
        }
        other => return Err(format!("A table is not written as {other}.")),
    }
    Ok(path.to_path_buf())
}

fn col(mut n: usize) -> String {
    let mut s = String::new();
    n += 1;
    while n > 0 {
        let r = (n - 1) % 26;
        n = (n - 1) / 26;
        s.insert(0, (b'A' + r as u8) as char);
    }
    s
}

fn sheet_xml(t: &Table) -> String {
    let mut sheet = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    );
    let mut rows: Vec<Vec<String>> = vec![t.columns.clone()];
    rows.extend(
        t.rows
            .iter()
            .map(|r| t.columns.iter().map(|c| cell(r.get(c))).collect()),
    );
    for (i, row) in rows.iter().enumerate() {
        let i = i + 1;
        let bold = if i == 1 { " s=\"1\"" } else { "" };
        sheet.push_str(&format!("<row r=\"{i}\">"));
        for (j, v) in row.iter().enumerate() {
            let v: String = v.chars().take(32000).collect();
            sheet.push_str(&format!(
                "<c r=\"{}{i}\" t=\"inlineStr\"{bold}><is><t xml:space=\"preserve\">{}</t></is></c>",
                col(j),
                xml(&v)
            ));
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    sheet
}

type Zip = zip::ZipWriter<std::fs::File>;

fn put(z: &mut Zip, name: &str, data: &[u8], deflate: bool) -> Result<(), String> {
    let method = if deflate {
        zip::CompressionMethod::Deflated
    } else {
        zip::CompressionMethod::Stored
    };
    zerr(z.start_file(
        name,
        zip::write::FileOptions::default().compression_method(method),
    ))?;
    io(z.write_all(data))
}

/// An Excel workbook with a sheet for each table (`write_workbook`).
pub fn write_workbook(tables: &[Table], path: &Path) -> Result<(), String> {
    parent(path);
    let empty = [Table {
        title: "isnady".into(),
        ..Default::default()
    }];
    let tables = if tables.is_empty() {
        &empty[..]
    } else {
        tables
    };
    let (mut names, mut used): (Vec<String>, Vec<String>) = (vec![], vec![]);
    for t in tables {
        let base = pre!(r"[\[\]:*?/\\]").sub(&t.title, " ");
        let base: String = crate::py::trim(&base).chars().take(28).collect();
        let base = if base.is_empty() {
            "isnady".to_string()
        } else {
            base
        };
        let (mut name, mut n) = (base.clone(), 1);
        while used.contains(&name.to_lowercase()) {
            n += 1;
            name = format!("{} {n}", base.chars().take(25).collect::<String>());
        }
        used.push(name.to_lowercase());
        names.push(name);
    }
    let n = tables.len();
    let overrides: String = (1..=n).map(|i| format!("<Override PartName=\"/xl/worksheets/sheet{i}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>")).collect();
    let sheets: String = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            format!(
                "<sheet name=\"{}\" sheetId=\"{1}\" r:id=\"rId{1}\"/>",
                xml(name),
                i + 1
            )
        })
        .collect();
    let rels: String = (1..=n).map(|i| format!("<Relationship Id=\"rId{i}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{i}.xml\"/>")).collect();
    let mut z = zip::ZipWriter::new(io(std::fs::File::create(path))?);
    put(&mut z, "[Content_Types].xml", format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>{overrides}<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>").as_bytes(), true)?;
    put(&mut z, "_rels/.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>", true)?;
    put(&mut z, "xl/workbook.xml", format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>{sheets}</sheets></workbook>").as_bytes(), true)?;
    put(&mut z, "xl/_rels/workbook.xml.rels", format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{rels}<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>", n + 1).as_bytes(), true)?;
    put(&mut z, "xl/styles.xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"2\"><font/><font><b/></font></fonts><fills count=\"1\"><fill/></fills><borders count=\"1\"><border/></borders><cellStyleXfs count=\"1\"><xf/></cellStyleXfs><cellXfs count=\"2\"><xf/><xf fontId=\"1\" applyFont=\"1\"/></cellXfs></styleSheet>", true)?;
    for (i, t) in tables.iter().enumerate() {
        put(
            &mut z,
            &format!("xl/worksheets/sheet{}.xml", i + 1),
            sheet_xml(t).as_bytes(),
            true,
        )?;
    }
    zerr(z.finish())?;
    Ok(())
}

// ------------------------------------------------------------------ documents, written as they come (EX3)
fn section_html(s: &Section) -> String {
    let mut out = vec![];
    if !s.heading.is_empty() {
        out.push(format!("<h2>{}</h2>", escape(&s.heading, true)));
    }
    for e in &s.entries {
        out.push(format!("<h3>{}</h3>", escape(&e.heading, true)));
        for (label, text) in &e.paragraphs {
            if text.is_empty() {
                continue;
            }
            let ar = rtl(text);
            let lab = if !label.is_empty() && !ar {
                format!("<span class='label'>{}:</span> ", escape(label, true))
            } else {
                String::new()
            };
            out.push(format!(
                "<p class='{}' dir='{}'>{lab}{}</p>",
                if ar { "ar" } else { "" },
                if ar { "rtl" } else { "ltr" },
                escape(text, true)
            ));
        }
        if !e.note.is_empty() {
            out.push(format!("<p class='note'>{}</p>", escape(&e.note, true)));
        }
    }
    out.join("\n")
}

fn section_markdown(s: &Section) -> String {
    let mut out: Vec<String> = if s.heading.is_empty() {
        vec![]
    } else {
        vec![format!("## {}", s.heading), String::new()]
    };
    for e in &s.entries {
        out.push(format!("### {}", e.heading));
        out.push(String::new());
        for (label, text) in &e.paragraphs {
            if !text.is_empty() {
                let lead = if !label.is_empty() && !rtl(text) {
                    format!("**{label}:** ")
                } else {
                    String::new()
                };
                out.push(format!("{lead}{text}"));
                out.push(String::new());
            }
        }
        if !e.note.is_empty() {
            out.push(format!("_{}_", e.note));
            out.push(String::new());
        }
    }
    out.join("\n")
}

fn section_text(s: &Section) -> String {
    let mut out: Vec<String> = if s.heading.is_empty() {
        vec![]
    } else {
        vec![
            s.heading.clone(),
            "-".repeat(s.heading.chars().count().min(70)),
            String::new(),
        ]
    };
    for e in &s.entries {
        out.push(e.heading.clone());
        for (label, text) in &e.paragraphs {
            if !text.is_empty() {
                let lead = if !label.is_empty() && !rtl(text) {
                    format!("{label}: ")
                } else {
                    String::new()
                };
                out.push(format!("{lead}{text}"));
            }
        }
        if !e.note.is_empty() {
            out.push(e.note.clone());
        }
        out.push(String::new());
    }
    out.join("\n")
}

fn section_xhtml(s: &Section, title: &str) -> String {
    let mut parts = format!(
        "<h2>{}</h2>",
        xml(if s.heading.is_empty() {
            title
        } else {
            &s.heading
        })
    );
    for e in &s.entries {
        parts.push_str(&format!("<h3>{}</h3>", xml(&e.heading)));
        for (label, text) in &e.paragraphs {
            if text.is_empty() {
                continue;
            }
            if rtl(text) {
                parts.push_str(&format!("<p class=\"ar\" dir=\"rtl\">{}</p>", xml(text)));
            } else {
                let lead = if label.is_empty() {
                    String::new()
                } else {
                    format!("<span class=\"label\">{}:</span> ", xml(label))
                };
                parts.push_str(&format!("<p>{lead}{}</p>", xml(text)));
            }
        }
        if !e.note.is_empty() {
            parts.push_str(&format!("<p class=\"note\">{}</p>", xml(&e.note)));
        }
    }
    parts
}

fn docx_para(text: &str, style: &str, bold: bool) -> String {
    let right = rtl(text);
    let ppr = format!(
        "{}{}",
        if style.is_empty() {
            String::new()
        } else {
            format!("<w:pStyle w:val=\"{style}\"/>")
        },
        if right {
            "<w:bidi/><w:jc w:val=\"right\"/>"
        } else {
            ""
        }
    );
    let rpr = format!(
        "{}{}",
        if bold { "<w:b/>" } else { "" },
        if right {
            "<w:rtl/><w:rFonts w:cs=\"Amiri\"/><w:szCs w:val=\"32\"/>"
        } else {
            ""
        }
    );
    format!("<w:p><w:pPr>{ppr}</w:pPr><w:r><w:rPr>{rpr}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", xml(text))
}

fn section_docx(s: &Section) -> String {
    let mut body = if s.heading.is_empty() {
        String::new()
    } else {
        docx_para(&s.heading, "Heading1", false)
    };
    for e in &s.entries {
        body.push_str(&docx_para(&e.heading, "Heading2", false));
        for (label, text) in &e.paragraphs {
            if !text.is_empty() {
                let t = if rtl(text) || label.is_empty() {
                    text.clone()
                } else {
                    format!("{label}: {text}")
                };
                body.push_str(&docx_para(&t, "", false));
            }
        }
        if !e.note.is_empty() {
            body.push_str(&docx_para(&e.note, "", false));
        }
    }
    body
}

fn xhtml(title: &str, body: &str) -> String {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><!DOCTYPE html><html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\"><head><meta charset=\"utf-8\"/><title>{}</title><style>{CSS}</style></head><body>{body}</body></html>", xml(title))
}

fn uuid4() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let part = || {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.finish()
    };
    let (a, b) = (part(), part());
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        a >> 32,
        (a >> 16) & 0xffff,
        a & 0xfff,
        ((b >> 48) & 0x3fff) | 0x8000,
        b & 0xffff_ffff_ffff
    )
}

/// html, md, txt, epub and docx, each section written as it comes (`DocumentWriter`). `abort` removes the
/// half-written file.
pub struct DocumentWriter {
    fmt: String,
    path: PathBuf,
    title: String,
    chapters: Vec<(String, String)>,
    zip: Option<Zip>,
    file: Option<std::io::BufWriter<std::fs::File>>,
    body_path: Option<PathBuf>,
    pdf: Option<crate::pdf::Pdf>,
}

/// A section of a document on the PDF's pages (as section_html shows it).
fn section_pdf(pdf: &mut crate::pdf::Pdf, s: &Section) {
    if !s.heading.is_empty() {
        pdf.space(10.0);
        pdf.paragraph(&s.heading, 14.0, true, false, 0.0);
        pdf.rule();
    }
    for e in &s.entries {
        pdf.space(6.0);
        if !e.heading.is_empty() {
            pdf.paragraph(&e.heading, 11.5, true, false, 0.0);
        }
        // as section_html lays them out: Arabic on its own, right to left; the others after their label
        for (label, text) in &e.paragraphs {
            if text.is_empty() {
                continue;
            }
            if rtl(text) {
                pdf.paragraph(text, 13.0, false, false, 0.0);
            } else if !label.is_empty() {
                pdf.paragraph(&format!("{label}: {text}"), 10.5, false, false, 0.0);
            } else {
                pdf.paragraph(text, 10.5, false, false, 0.0);
            }
            pdf.space(3.0);
        }
        if !e.note.is_empty() {
            pdf.paragraph(&e.note, 8.5, false, true, 0.0);
        }
    }
}

impl DocumentWriter {
    pub fn new(
        fmt: &str,
        path: &Path,
        title: &str,
        subtitle: &str,
    ) -> Result<DocumentWriter, String> {
        if !["html", "md", "txt", "epub", "docx", "pdf"].contains(&fmt) {
            return Err(format!("A document is not written as {fmt}."));
        }
        parent(path);
        let mut w = DocumentWriter {
            fmt: fmt.into(),
            path: path.to_path_buf(),
            title: title.into(),
            chapters: vec![],
            zip: None,
            file: None,
            body_path: None,
            pdf: None,
        };
        let open = |p: &Path| io(std::fs::File::create(p)).map(std::io::BufWriter::new);
        match fmt {
            "pdf" => {
                let mut pdf = crate::pdf::Pdf::new(title);
                pdf.paragraph(title, 18.0, true, false, 0.0);
                if !subtitle.is_empty() {
                    pdf.paragraph(subtitle, 9.0, false, true, 0.0);
                }
                pdf.space(6.0);
                w.pdf = Some(pdf);
            }
            "epub" => {
                let mut z = zip::ZipWriter::new(io(std::fs::File::create(path))?);
                put(&mut z, "mimetype", b"application/epub+zip", false)?;
                w.zip = Some(z);
            }
            "docx" => {
                let mut name = path.file_name().unwrap_or_default().to_os_string();
                name.push(".part");
                let body = path.with_file_name(name);
                let mut f = open(&body)?;
                let head = docx_para(title, "Title", false)
                    + &(if subtitle.is_empty() {
                        String::new()
                    } else {
                        docx_para(subtitle, "", false)
                    });
                io(f.write_all(head.as_bytes()))?;
                w.file = Some(f);
                w.body_path = Some(body);
            }
            _ => {
                let mut f = open(path)?;
                let head = match fmt {
                    "html" => format!(
                        "<!doctype html><html><head><meta charset='utf-8'><title>{0}</title><style>{CSS}</style></head><body><h1>{0}</h1>\n{1}",
                        escape(title, true),
                        if subtitle.is_empty() { String::new() } else { format!("<p class='sub'>{}</p>\n", escape(subtitle, true)) }
                    ),
                    "md" => {
                        let mut l = vec![format!("# {title}"), String::new()];
                        if !subtitle.is_empty() {
                            l.push(format!("*{subtitle}*"));
                            l.push(String::new());
                        }
                        l.join("\n") + "\n"
                    }
                    _ => {
                        let mut l = vec![title.to_string(), "=".repeat(title.chars().count().min(70)), String::new()];
                        if !subtitle.is_empty() {
                            l.push(subtitle.to_string());
                            l.push(String::new());
                        }
                        l.join("\n") + "\n"
                    }
                };
                io(f.write_all(head.as_bytes()))?;
                w.file = Some(f);
            }
        }
        Ok(w)
    }

    pub fn section(&mut self, s: &Section) -> Result<(), String> {
        let text = match self.fmt.as_str() {
            "html" => section_html(s) + "\n",
            "md" => section_markdown(s) + "\n",
            "txt" => section_text(s) + "\n",
            "docx" => section_docx(s),
            "pdf" => {
                if let Some(pdf) = self.pdf.as_mut() {
                    section_pdf(pdf, s);
                }
                return Ok(());
            }
            _ => {
                let name = format!("c{:05}.xhtml", self.chapters.len() + 1);
                let heading = if s.heading.is_empty() {
                    self.title.clone()
                } else {
                    s.heading.clone()
                };
                let body = xhtml(&heading, &section_xhtml(s, &self.title));
                if let Some(z) = self.zip.as_mut() {
                    put(z, &format!("OEBPS/{name}"), body.as_bytes(), true)?;
                }
                self.chapters.push((name, heading));
                return Ok(());
            }
        };
        match self.file.as_mut() {
            Some(f) => io(f.write_all(text.as_bytes())),
            None => Ok(()),
        }
    }

    pub fn close(mut self) -> Result<PathBuf, String> {
        match self.fmt.as_str() {
            "pdf" => {
                if let Some(pdf) = self.pdf.take() {
                    pdf.save(&self.path)?;
                }
            }
            "epub" => self.close_epub()?,
            "docx" => {
                if let Some(mut f) = self.file.take() {
                    io(f.flush())?;
                }
                let body = self.body_path.clone().unwrap_or_default();
                docx_zip(&self.path, &body)?;
                let _ = std::fs::remove_file(body);
            }
            _ => {
                if let Some(mut f) = self.file.take() {
                    if self.fmt == "html" {
                        io(f.write_all(b"</body></html>"))?;
                    }
                    io(f.flush())?;
                }
            }
        }
        Ok(self.path.clone())
    }

    pub fn abort(mut self) {
        self.zip.take();
        self.file.take();
        let _ = std::fs::remove_file(&self.path);
        if let Some(b) = &self.body_path {
            let _ = std::fs::remove_file(b);
        }
    }

    fn close_epub(&mut self) -> Result<(), String> {
        let Some(mut z) = self.zip.take() else {
            return Ok(());
        };
        if self.chapters.is_empty() {
            put(
                &mut z,
                "OEBPS/c00001.xhtml",
                xhtml(&self.title, &format!("<h2>{}</h2>", xml(&self.title))).as_bytes(),
                true,
            )?;
            self.chapters
                .push(("c00001.xhtml".into(), self.title.clone()));
        }
        let nav: String = self
            .chapters
            .iter()
            .map(|(f, t)| format!("<li><a href=\"{f}\">{}</a></li>", xml(t)))
            .collect();
        put(&mut z, "META-INF/container.xml", b"<?xml version=\"1.0\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>", true)?;
        let manifest: String = self
            .chapters
            .iter()
            .enumerate()
            .map(|(i, (f, _))| {
                format!(
                    "<item id=\"c{}\" href=\"{f}\" media-type=\"application/xhtml+xml\"/>",
                    i + 1
                )
            })
            .collect();
        let spine: String = (1..=self.chapters.len())
            .map(|i| format!("<itemref idref=\"c{i}\"/>"))
            .collect();
        put(&mut z, "OEBPS/content.opf", format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"id\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:identifier id=\"id\">urn:uuid:{}</dc:identifier><dc:title>{}</dc:title><dc:language>ar</dc:language><meta property=\"dcterms:modified\">{}</meta></metadata><manifest><item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>{manifest}</manifest><spine>{spine}</spine></package>", uuid4(), xml(&self.title), chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")).as_bytes(), true)?;
        put(
            &mut z,
            "OEBPS/nav.xhtml",
            xhtml(
                &self.title,
                &format!(
                    "<nav epub:type=\"toc\"><h1>{}</h1><ol>{nav}</ol></nav>",
                    xml(&self.title)
                ),
            )
            .as_bytes(),
            true,
        )?;
        zerr(z.finish())?;
        Ok(())
    }
}

fn docx_zip(path: &Path, body_path: &Path) -> Result<(), String> {
    let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:rPr><w:sz w:val=\"22\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:rPr><w:b/><w:sz w:val=\"40\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:pPr><w:outlineLvl w:val=\"0\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"30\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:pPr><w:outlineLvl w:val=\"1\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"24\"/></w:rPr></w:style></w:styles>";
    let mut z = zip::ZipWriter::new(io(std::fs::File::create(path))?);
    put(&mut z, "[Content_Types].xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>", true)?;
    put(&mut z, "_rels/.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>", true)?;
    put(&mut z, "word/_rels/document.xml.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>", true)?;
    zerr(
        z.start_file(
            "word/document.xml",
            zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .large_file(true),
        ),
    )?;
    io(z.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>"))?;
    let mut body = io(std::fs::File::open(body_path))?;
    io(std::io::copy(&mut body, &mut z))?;
    io(z.write_all(b"<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\"/></w:sectPr></w:body></w:document>"))?;
    put(&mut z, "word/styles.xml", styles.as_bytes(), true)?;
    zerr(z.finish())?;
    Ok(())
}

// ------------------------------------------------------------------ networks
/// An isnad network: nodes {id, label, arabic, tabaqa, tradition}, edges {source, target, weight} (`write_graph`).
pub fn write_graph(
    nodes: &[Value],
    edges: &[Value],
    fmt: &str,
    path: &Path,
    title: &str,
) -> Result<PathBuf, String> {
    parent(path);
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let n = |v: &Value, k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
    let text = match fmt {
        "graphml" => {
            let mut out = vec![
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">".to_string(),
                "<key id=\"label\" for=\"node\" attr.name=\"label\" attr.type=\"string\"/>".into(),
                "<key id=\"arabic\" for=\"node\" attr.name=\"arabic\" attr.type=\"string\"/>".into(),
                "<key id=\"tabaqa\" for=\"node\" attr.name=\"tabaqa\" attr.type=\"int\"/>".into(),
                "<key id=\"tradition\" for=\"node\" attr.name=\"tradition\" attr.type=\"string\"/>".into(),
                "<key id=\"weight\" for=\"edge\" attr.name=\"weight\" attr.type=\"int\"/>".into(),
                format!("<graph id=\"{}\" edgedefault=\"directed\">", xml(title)),
            ];
            for x in nodes {
                out.push(format!("<node id=\"n{}\"><data key=\"label\">{}</data><data key=\"arabic\">{}</data><data key=\"tabaqa\">{}</data><data key=\"tradition\">{}</data></node>", n(x, "id"), xml(&s(x, "label")), xml(&s(x, "arabic")), n(x, "tabaqa"), xml(&s(x, "tradition"))));
            }
            for (i, e) in edges.iter().enumerate() {
                out.push(format!("<edge id=\"e{i}\" source=\"n{}\" target=\"n{}\"><data key=\"weight\">{}</data></edge>", n(e, "source"), n(e, "target"), n(e, "weight")));
            }
            out.push("</graph></graphml>".into());
            out.join("\n")
        }
        "gexf" => {
            let mut out = vec![
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><gexf xmlns=\"http://gexf.net/1.3\" version=\"1.3\">".to_string(),
                format!("<meta><creator>isnady</creator><description>{}</description></meta>", xml(title)),
                "<graph defaultedgetype=\"directed\"><attributes class=\"node\"><attribute id=\"0\" title=\"arabic\" type=\"string\"/><attribute id=\"1\" title=\"tabaqa\" type=\"integer\"/><attribute id=\"2\" title=\"tradition\" type=\"string\"/></attributes><nodes>".into(),
            ];
            for x in nodes {
                out.push(format!("<node id=\"{}\" label=\"{}\"><attvalues><attvalue for=\"0\" value=\"{}\"/><attvalue for=\"1\" value=\"{}\"/><attvalue for=\"2\" value=\"{}\"/></attvalues></node>", n(x, "id"), xml(&s(x, "label")), xml(&s(x, "arabic")), n(x, "tabaqa"), xml(&s(x, "tradition"))));
            }
            out.push("</nodes><edges>".into());
            for (i, e) in edges.iter().enumerate() {
                out.push(format!(
                    "<edge id=\"{i}\" source=\"{}\" target=\"{}\" weight=\"{}\"/>",
                    n(e, "source"),
                    n(e, "target"),
                    n(e, "weight")
                ));
            }
            out.push("</edges></graph></gexf>".into());
            out.join("\n")
        }
        "json" => pyjson::dumps_indent(
            &json!({"schema": "isnady-network/1", "title": title, "nodes": nodes, "edges": edges}),
            1,
        ),
        "csv" => {
            let label: std::collections::HashMap<i64, String> =
                nodes.iter().map(|x| (n(x, "id"), s(x, "label"))).collect();
            let mut out = String::from("\u{feff}");
            out.push_str(&csv_line(
                &["teacher_id", "teacher", "student_id", "student", "chains"].map(String::from),
                ',',
            ));
            for e in edges {
                let (a, b) = (n(e, "source"), n(e, "target"));
                out.push_str(&csv_line(
                    &[
                        a.to_string(),
                        label.get(&a).cloned().unwrap_or_default(),
                        b.to_string(),
                        label.get(&b).cloned().unwrap_or_default(),
                        n(e, "weight").to_string(),
                    ],
                    ',',
                ));
            }
            out
        }
        other => return Err(format!("A network is not written as {other}.")),
    };
    io(std::fs::write(path, text))?;
    Ok(path.to_path_buf())
}
