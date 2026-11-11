//! PDF (A5): the exports and the Statistics as PDF, written here (the Python isnady has Qt draw them).
//!
//! Text is set in Amiri (SIL OFL, `data/fonts`), which has both the Arabic and the Latin letters isnady needs:
//! shaped by rustybuzz, put in visual order by the Unicode bidirectional algorithm, broken into lines by width. The
//! font is embedded whole (Type0 / CIDFontType2, Identity-H) with a ToUnicode map, so the text can be searched and
//! copied. Pictures (the Statistics' findings) are JPEG, placed as they are.

use flate2::{write::ZlibEncoder, Compression};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

const REGULAR: &[u8] = include_bytes!("../data/fonts/Amiri-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../data/fonts/Amiri-Bold.ttf");
pub const PAGE_W: f32 = 595.28;
pub const PAGE_H: f32 = 841.89;
/// 15 mm, as the Python isnady prints
pub const MARGIN: f32 = 42.52;

struct Font {
    data: &'static [u8],
    name: &'static str,
    face: rustybuzz::Face<'static>,
    /// glyph → (advance in 1/1000 em, the text it stands for)
    used: BTreeMap<u16, (i32, String)>,
}

impl Font {
    fn new(data: &'static [u8], name: &'static str) -> Font {
        Font {
            data,
            name,
            face: rustybuzz::Face::from_slice(data, 0).expect("the bundled font"),
            used: BTreeMap::new(),
        }
    }

    fn scale(&self, size: f32) -> f32 {
        size / self.face.units_per_em() as f32
    }
}

/// One shaped glyph, placed: (glyph, x, y offset, advance), in points.
struct Placed {
    gid: u16,
    x: f32,
    dy: f32,
}

/// A run shaped: its glyphs in visual order (left to right) and its width.
struct Shaped {
    glyphs: Vec<(u16, f32, f32, f32, String)>,
    width: f32,
}

fn rtl_char(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

pub struct Pdf {
    fonts: [Font; 2],
    pages: Vec<(Vec<u8>, Vec<usize>)>,
    content: Vec<u8>,
    page_images: Vec<usize>,
    images: Vec<(Vec<u8>, u32, u32)>,
    y: f32,
    title: String,
}

impl Default for Pdf {
    fn default() -> Self {
        Self::new("")
    }
}

impl Pdf {
    pub fn new(title: &str) -> Pdf {
        Pdf {
            fonts: [
                Font::new(REGULAR, "Amiri-Regular"),
                Font::new(BOLD, "Amiri-Bold"),
            ],
            pages: vec![],
            content: vec![],
            page_images: vec![],
            images: vec![],
            y: PAGE_H - MARGIN,
            title: title.to_string(),
        }
    }

    fn new_page(&mut self) {
        let content = std::mem::take(&mut self.content);
        let images = std::mem::take(&mut self.page_images);
        self.pages.push((content, images));
        self.y = PAGE_H - MARGIN;
    }

    fn room(&mut self, h: f32) {
        if self.y - h < MARGIN && self.y < PAGE_H - MARGIN - 1.0 {
            self.new_page();
        }
    }

    pub fn space(&mut self, h: f32) {
        self.y -= h;
    }

    fn shape(&mut self, bold: bool, text: &str, rtl: bool, size: f32) -> Shaped {
        let font = &mut self.fonts[bold as usize];
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(if rtl {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
        buffer.guess_segment_properties();
        let out = rustybuzz::shape(&font.face, &[], buffer);
        let k = font.scale(size);
        // each cluster's text: from its start to the next cluster's start, in the text's own order
        let mut starts: Vec<usize> = out
            .glyph_infos()
            .iter()
            .map(|g| g.cluster as usize)
            .collect();
        starts.sort_unstable();
        starts.dedup();
        let text_of = |c: usize| -> String {
            let end = starts
                .iter()
                .find(|s| **s > c)
                .copied()
                .unwrap_or(text.len());
            text.get(c..end).unwrap_or("").to_string()
        };
        let mut glyphs = vec![];
        let mut width = 0.0;
        let mut seen_cluster: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for (info, pos) in out.glyph_infos().iter().zip(out.glyph_positions()) {
            let gid = info.glyph_id as u16;
            let cluster = info.cluster as usize;
            let unicode = if seen_cluster.insert(cluster) {
                text_of(cluster)
            } else {
                String::new()
            };
            glyphs.push((
                gid,
                pos.x_offset as f32 * k,
                pos.y_offset as f32 * k,
                pos.x_advance as f32 * k,
                unicode,
            ));
            width += pos.x_advance as f32 * k;
        }
        Shaped { glyphs, width }
    }

    fn width(&mut self, bold: bool, text: &str, size: f32) -> f32 {
        let rtl = text.chars().any(rtl_char);
        self.shape(bold, text, rtl, size).width
    }

    /// The lines of a paragraph that fit in `width`: byte ranges of the text, in its own order.
    fn lines(&mut self, bold: bool, text: &str, size: f32, width: f32) -> Vec<(usize, usize)> {
        let space = self.width(bold, " ", size);
        let mut out = vec![];
        let (mut start, mut used) = (None::<usize>, 0.0f32);
        let mut last_end = 0;
        let mut at = 0;
        for word in text.split(' ') {
            let w_start = at;
            let w_end = at + word.len();
            at = w_end + 1;
            if word.is_empty() {
                continue;
            }
            let w = self.width(bold, word, size);
            if w > width && word.chars().count() > 1 {
                // a word wider than the column: broken between letters, as many on a line as fit
                if let Some(s) = start.take() {
                    out.push((s, last_end));
                }
                let mut piece = w_start;
                let mut acc = String::new();
                for (off, c) in word.char_indices() {
                    acc.push(c);
                    if self.width(bold, &acc, size) > width && acc.chars().count() > 1 {
                        out.push((piece, w_start + off));
                        piece = w_start + off;
                        acc = c.to_string();
                    }
                }
                start = Some(piece);
                used = self.width(bold, &acc, size);
                last_end = w_end;
                continue;
            }
            match start {
                None => {
                    start = Some(w_start);
                    used = w;
                }
                Some(s) => {
                    if used + space + w > width {
                        out.push((s, last_end));
                        start = Some(w_start);
                        used = w;
                    } else {
                        used += space + w;
                    }
                }
            }
            last_end = w_end;
        }
        if let Some(s) = start {
            out.push((s, last_end));
        }
        out
    }

    /// One line set at (x, baseline y): the runs of the line in visual order.
    fn set_line(
        &mut self,
        bold: bool,
        text: &str,
        size: f32,
        left: f32,
        right: f32,
        rtl_base: bool,
    ) -> f32 {
        let bidi = unicode_bidi::BidiInfo::new(
            text,
            Some(if rtl_base {
                unicode_bidi::Level::rtl()
            } else {
                unicode_bidi::Level::ltr()
            }),
        );
        let mut placed: Vec<Placed> = vec![];
        let mut x = 0.0;
        if let Some(para) = bidi.paragraphs.first() {
            let (levels, runs) = bidi.visual_runs(para, para.range.clone());
            for run in runs {
                let rtl = levels[run.start].is_rtl();
                let shaped = self.shape(bold, &text[run.clone()], rtl, size);
                for (gid, dx, dy, adv, unicode) in shaped.glyphs {
                    placed.push(Placed { gid, x: x + dx, dy });
                    let font = &mut self.fonts[bold as usize];
                    let advance = (font
                        .face
                        .glyph_hor_advance(rustybuzz::ttf_parser::GlyphId(gid))
                        .unwrap_or(0) as f32
                        * 1000.0
                        / font.face.units_per_em() as f32) as i32;
                    let entry = font.used.entry(gid).or_insert((advance, String::new()));
                    if entry.1.is_empty() && !unicode.is_empty() {
                        entry.1 = unicode;
                    }
                    x += adv;
                }
            }
        }
        let start = if rtl_base { right - x } else { left };
        let y = self.y;
        let f = if bold { "F2" } else { "F1" };
        let mut s = format!("BT /{f} {size:.2} Tf\n");
        for p in placed {
            s.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm <{:04X}> Tj\n",
                start + p.x,
                y + p.dy,
                p.gid
            ));
        }
        s.push_str("ET\n");
        self.content.extend(s.as_bytes());
        x
    }

    /// A paragraph (lines broken by width), its first line's baseline lowered by the leading; RTL paragraphs set from
    /// the right. `gray` writes it in the muted colour.
    pub fn paragraph(&mut self, text: &str, size: f32, bold: bool, gray: bool, indent: f32) {
        let text = text.replace('‖', "|").replace('\t', " ");
        let (left, right) = (MARGIN + indent, PAGE_W - MARGIN);
        if gray {
            self.content.extend(b"0.42 0.45 0.5 rg\n");
        }
        for para in text.split('\n') {
            let rtl_base = para
                .chars()
                .find(|c| c.is_alphabetic())
                .is_some_and(rtl_char);
            // Arabic with its vowel marks needs more room between the lines
            let leading = size
                * if para.chars().any(rtl_char) {
                    1.9
                } else {
                    1.45
                };
            for (a, b) in self.lines(bold, para, size, right - left) {
                self.room(leading);
                self.y -= leading;
                self.set_line(bold, &para[a..b], size, left, right, rtl_base);
            }
        }
        if gray {
            self.content.extend(b"0 0 0 rg\n");
        }
    }

    pub fn rule(&mut self) {
        self.room(8.0);
        self.y -= 6.0;
        self.content.extend(
            format!(
                "0.8 0.82 0.86 RG 0.5 w {MARGIN:.2} {:.2} m {:.2} {:.2} l S\n",
                self.y,
                PAGE_W - MARGIN,
                self.y
            )
            .as_bytes(),
        );
    }

    /// A table: the header in bold, every cell broken into lines in its column; a row never split between pages.
    pub fn table(&mut self, header: &[String], rows: &[Vec<String>], size: f32) {
        let avail = PAGE_W - 2.0 * MARGIN;
        let pad = 3.0;
        let n = header.len().max(1);
        let mut natural = vec![0f32; n];
        for (i, h) in header.iter().enumerate() {
            natural[i] = self.width(true, h, size) + 2.0 * pad;
        }
        for r in rows.iter().take(400) {
            for (i, c) in r.iter().enumerate().take(n) {
                let w = self.width(false, c, size).min(avail) + 2.0 * pad;
                natural[i] = natural[i].max(w);
            }
        }
        let total: f32 = natural.iter().sum();
        let widths: Vec<f32> = if total <= avail {
            natural.clone()
        } else {
            // the narrow columns keep their width; the wide ones share what is left
            let fair = avail / n as f32;
            let narrow: f32 = natural.iter().filter(|w| **w <= fair).sum();
            let wide: f32 = natural.iter().filter(|w| **w > fair).sum();
            natural
                .iter()
                .map(|w| {
                    if *w <= fair {
                        *w
                    } else {
                        (avail - narrow) * w / wide
                    }
                })
                .collect()
        };
        let leading = size * 1.35;
        let draw = |pdf: &mut Pdf, cells: &[String], bold: bool| {
            let wrapped: Vec<Vec<String>> = cells
                .iter()
                .enumerate()
                .take(n)
                .map(|(i, c)| {
                    let c = c.replace('\n', " ");
                    pdf.lines(bold, &c, size, widths[i] - 2.0 * pad)
                        .into_iter()
                        .map(|(a, b)| c[a..b].to_string())
                        .collect()
                })
                .collect();
            let h = wrapped.iter().map(|l| l.len().max(1)).max().unwrap_or(1) as f32 * leading
                + 2.0 * pad;
            pdf.room(h);
            let top = pdf.y;
            let mut x = MARGIN;
            for (i, lines) in wrapped.iter().enumerate() {
                pdf.y = top - pad;
                for l in lines {
                    pdf.y -= leading;
                    let rtl = l.chars().find(|c| c.is_alphabetic()).is_some_and(rtl_char);
                    pdf.set_line(bold, l, size, x + pad, x + widths[i] - pad, rtl);
                }
                x += widths[i];
            }
            pdf.y = top - h;
            let mut lines = format!(
                "0.8 0.82 0.86 RG 0.4 w {MARGIN:.2} {:.2} {:.2} {:.2} re S\n",
                pdf.y,
                x - MARGIN,
                h
            );
            let mut cx = MARGIN;
            for w in &widths[..widths.len() - 1] {
                cx += w;
                lines.push_str(&format!("{cx:.2} {:.2} m {cx:.2} {top:.2} l S\n", pdf.y));
            }
            pdf.content.extend(lines.as_bytes());
        };
        draw(self, header, true);
        for r in rows {
            draw(self, r, false);
        }
    }

    /// A JPEG picture, as wide as the page (smaller pictures are not enlarged), on a new page when it does not fit.
    pub fn picture_jpeg(&mut self, jpeg: Vec<u8>, shown_width: f32) -> Result<(), String> {
        let (w, h) = jpeg_size(&jpeg).ok_or("not a JPEG picture")?;
        let avail_w = PAGE_W - 2.0 * MARGIN;
        let mut dw = shown_width.min(avail_w);
        let mut dh = dw * h as f32 / w as f32;
        let avail_h = PAGE_H - 2.0 * MARGIN;
        if dh > avail_h {
            dh = avail_h;
            dw = dh * w as f32 / h as f32;
        }
        self.room(dh);
        self.y -= dh;
        self.images.push((jpeg, w, h));
        let n = self.images.len() - 1;
        self.page_images.push(n);
        self.content.extend(
            format!(
                "q {dw:.2} 0 0 {dh:.2} {MARGIN:.2} {:.2} cm /Im{n} Do Q\n",
                self.y
            )
            .as_bytes(),
        );
        self.y -= 8.0;
        Ok(())
    }

    pub fn page_break(&mut self) {
        if self.y < PAGE_H - MARGIN - 1.0 {
            self.new_page();
        }
    }

    /// The file: every page, the fonts with the glyphs used, the pictures.
    pub fn save(mut self, path: &Path) -> Result<(), String> {
        // the document's title is the file's name, as Qt's writer is given it
        self.title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if !self.content.is_empty() || self.pages.is_empty() {
            self.new_page();
        }
        let objects: std::cell::RefCell<Vec<Vec<u8>>> = std::cell::RefCell::new(vec![]);
        let add = |o: Vec<u8>| -> usize {
            let mut all = objects.borrow_mut();
            all.push(o);
            all.len()
        };
        let deflate = |data: &[u8]| -> Vec<u8> {
            let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
            let _ = z.write_all(data);
            z.finish().unwrap_or_default()
        };
        let stream = |dict: String, data: Vec<u8>| -> Vec<u8> {
            let mut o = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
            o.extend(data);
            o.extend(b"\nendstream");
            o
        };
        // the fonts
        let mut font_refs = vec![];
        for font in &self.fonts {
            let file = add(stream(
                format!("/Filter /FlateDecode /Length1 {}", font.data.len()),
                deflate(font.data),
            ));
            let face = &font.face;
            let k = 1000.0 / face.units_per_em() as f32;
            let bb = face.global_bounding_box();
            let descriptor = add(format!(
                "<< /Type /FontDescriptor /FontName /{} /Flags 4 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} \
                 /CapHeight {} /StemV 80 /FontFile2 {file} 0 R >>",
                font.name,
                (bb.x_min as f32 * k) as i32,
                (bb.y_min as f32 * k) as i32,
                (bb.x_max as f32 * k) as i32,
                (bb.y_max as f32 * k) as i32,
                (face.ascender() as f32 * k) as i32,
                (face.descender() as f32 * k) as i32,
                (face.capital_height().unwrap_or(face.ascender()) as f32 * k) as i32,
            )
            .into_bytes());
            let widths: String = font
                .used
                .iter()
                .map(|(g, (w, _))| format!("{g} [{w}]"))
                .collect::<Vec<_>>()
                .join(" ");
            let cid = add(format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) \
                 /Supplement 0 >> /FontDescriptor {descriptor} 0 R /DW 500 /W [{widths}] /CIDToGIDMap /Identity >>",
                font.name
            )
            .into_bytes());
            let mut cmap = String::from(
                "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) \
                 /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\n\
                 endcodespacerange\n",
            );
            let mapped: Vec<(&u16, &String)> = font
                .used
                .iter()
                .filter(|(_, (_, u))| !u.is_empty())
                .map(|(g, (_, u))| (g, u))
                .collect();
            for chunk in mapped.chunks(100) {
                cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
                for (g, u) in chunk {
                    let hex: String = u.encode_utf16().map(|c| format!("{c:04X}")).collect();
                    cmap.push_str(&format!("<{g:04X}> <{hex}>\n"));
                }
                cmap.push_str("endbfchar\n");
            }
            cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
            let to_unicode = add(stream(
                "/Filter /FlateDecode".into(),
                deflate(cmap.as_bytes()),
            ));
            font_refs.push(add(format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {to_unicode} 0 R >>",
                font.name
            )
            .into_bytes()));
        }
        let image_refs: Vec<usize> = std::mem::take(&mut self.images)
            .into_iter()
            .map(|(data, w, h)| {
                add(stream(
                    format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"),
                    data,
                ))
            })
            .collect();
        let pages_id = objects.borrow().len() + 1 + self.pages.len() * 2;
        let mut kids = vec![];
        for (content, images) in std::mem::take(&mut self.pages) {
            let c = add(stream("/Filter /FlateDecode".into(), deflate(&content)));
            let xobjects: String = images
                .iter()
                .map(|n| format!("/Im{n} {} 0 R", image_refs[*n]))
                .collect::<Vec<_>>()
                .join(" ");
            kids.push(add(format!(
                "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] /Contents {c} 0 R \
                 /Resources << /Font << /F1 {} 0 R /F2 {} 0 R >> /XObject << {xobjects} >> >> >>",
                font_refs[0], font_refs[1]
            )
            .into_bytes()));
        }
        let pages = add(format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.iter()
                .map(|k| format!("{k} 0 R"))
                .collect::<Vec<_>>()
                .join(" "),
            kids.len()
        )
        .into_bytes());
        debug_assert_eq!(pages, pages_id);
        let title: String = self
            .title
            .encode_utf16()
            .map(|c| format!("{c:04X}"))
            .collect();
        let info = add(format!(
            "<< /Title <FEFF{title}> /Producer (isnady-rs {}) >>",
            crate::VERSION
        )
        .into_bytes());
        let catalog = add(format!("<< /Type /Catalog /Pages {pages} 0 R >>").into_bytes());
        let objects = objects.into_inner();
        let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = vec![];
        for (i, o) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend(o);
            out.extend(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for o in offsets {
            out.extend(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend(
            format!("trailer\n<< /Size {} /Root {catalog} 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1)
                .as_bytes(),
        );
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::write(path, out).map_err(|e| e.to_string())
    }
}

/// (width, height) of a JPEG, from its frame header.
fn jpeg_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
            let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
            return Some((w, h));
        }
        i += 2 + len;
    }
    None
}
