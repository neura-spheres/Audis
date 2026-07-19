use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

pub struct ReportMeta {
    pub title: String,
    pub session_type: String,
    pub started_at: String,
    pub language: String,
    pub generated_at: String,
}

const PAGE_W: f32 = 595.28;
const PAGE_H: f32 = 841.89;
const MARGIN_L: f32 = 56.0;
const MARGIN_R: f32 = 56.0;
const MARGIN_TOP: f32 = 58.0;
const MARGIN_BOTTOM: f32 = 66.0;
const CONTENT_TOP: f32 = PAGE_H - MARGIN_TOP;
const USABLE_W: f32 = PAGE_W - MARGIN_L - MARGIN_R;

const INK: (f32, f32, f32) = (0.13, 0.14, 0.16);
const ACCENT: (f32, f32, f32) = (0.10, 0.16, 0.26);
const MUTED: (f32, f32, f32) = (0.42, 0.45, 0.5);
const HAIRLINE: (f32, f32, f32) = (0.80, 0.82, 0.85);

const BULLET_INDENT: f32 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Regular,
    Bold,
    Italic,
}

#[derive(Clone)]
struct Word {
    text: String,
    style: Style,
    glue: bool,
}

enum Block {
    Heading { level: u8, words: Vec<Word> },
    Paragraph { words: Vec<Word> },
    ListItem { marker: String, words: Vec<Word> },
    Rule,
}

pub fn markdown_to_pdf(markdown: &str, meta: &ReportMeta) -> Vec<u8> {
    let blocks = parse_blocks(markdown);
    let mut doc = Doc::new();
    doc.header(meta);
    for block in &blocks {
        doc.block(block);
    }
    doc.build()
}

fn parse_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    let mut bold = 0u32;
    let mut italic = 0u32;
    let mut heading: Option<u8> = None;
    let mut list_counters: Vec<Option<u64>> = Vec::new();
    let mut pending_marker: Option<String> = None;
    let mut in_item = false;
    let mut space_before = false;

    let style = |bold: u32, italic: u32| {
        if bold > 0 {
            Style::Bold
        } else if italic > 0 {
            Style::Italic
        } else {
            Style::Regular
        }
    };

    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(heading_level(level));
                words.clear();
                space_before = false;
            }
            Event::End(TagEnd::Heading(_)) => {
                if !words.is_empty()
                    && let Some(level) = heading.take()
                {
                    blocks.push(Block::Heading {
                        level,
                        words: std::mem::take(&mut words),
                    });
                }
                heading = None;
                words.clear();
            }
            Event::Start(Tag::Paragraph) => {
                if !in_item {
                    words.clear();
                }
                space_before = false;
            }
            Event::End(TagEnd::Paragraph) => {
                if !in_item && !words.is_empty() {
                    blocks.push(Block::Paragraph {
                        words: std::mem::take(&mut words),
                    });
                }
            }
            Event::Start(Tag::List(start)) => {
                list_counters.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                list_counters.pop();
            }
            Event::Start(Tag::Item) => {
                in_item = true;
                words.clear();
                space_before = false;
                let marker = match list_counters.last_mut() {
                    Some(Some(n)) => {
                        let m = format!("{n}.");
                        *n += 1;
                        m
                    }
                    _ => "\u{2022}".to_owned(),
                };
                pending_marker = Some(marker);
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                let marker = pending_marker
                    .take()
                    .unwrap_or_else(|| "\u{2022}".to_owned());
                if !words.is_empty() {
                    blocks.push(Block::ListItem {
                        marker,
                        words: std::mem::take(&mut words),
                    });
                }
            }
            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),
            Event::Text(text) => {
                push_tokens(&mut words, &text, style(bold, italic), &mut space_before)
            }
            Event::Code(text) => push_tokens(&mut words, &text, Style::Regular, &mut space_before),
            Event::SoftBreak | Event::HardBreak => space_before = true,
            Event::Rule => blocks.push(Block::Rule),
            _ => {}
        }
    }

    if !words.is_empty() {
        blocks.push(Block::Paragraph { words });
    }

    blocks
}

fn push_tokens(words: &mut Vec<Word>, text: &str, style: Style, space_before: &mut bool) {
    let is_ws = |c: char| c.is_whitespace();
    let leading = text.starts_with(is_ws);
    let mut first = true;
    for token in text.split_whitespace() {
        let glue = first && !(*space_before || leading) && !words.is_empty();
        words.push(Word {
            text: token.to_owned(),
            style,
            glue,
        });
        first = false;
    }
    if text.ends_with(is_ws) {
        *space_before = true;
    } else if !text.is_empty() {
        *space_before = false;
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

struct Doc {
    pages: Vec<String>,
    cur: String,
    y: f32,
}

impl Doc {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            cur: String::new(),
            y: CONTENT_TOP,
        }
    }

    fn new_page(&mut self) {
        self.pages.push(std::mem::take(&mut self.cur));
        self.y = CONTENT_TOP;
    }

    fn need(&mut self, height: f32) {
        if self.y - height < MARGIN_BOTTOM {
            self.new_page();
        }
    }

    fn header(&mut self, meta: &ReportMeta) {
        let title = if meta.title.trim().is_empty() {
            format!("{} report", meta.session_type)
        } else {
            meta.title.clone()
        };

        for line in wrap(&bold_words(&title), USABLE_W, 22.0) {
            draw_line(&mut self.cur, MARGIN_L, self.y, &line, 22.0, ACCENT);
            self.y -= 27.0;
        }
        self.y -= 4.0;

        let subtitle = [
            meta.session_type.trim(),
            meta.language.trim(),
            meta.started_at.trim(),
        ]
        .iter()
        .filter(|part| !part.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("   \u{2022}   ");
        draw_text(
            &mut self.cur,
            MARGIN_L,
            self.y,
            &subtitle,
            "F1",
            10.0,
            MUTED,
        );
        self.y -= 14.0;

        draw_text(
            &mut self.cur,
            MARGIN_L,
            self.y,
            &format!("Generated {} by Audis", meta.generated_at.trim()),
            "F3",
            9.0,
            MUTED,
        );
        self.y -= 16.0;

        rule(&mut self.cur, MARGIN_L, self.y, PAGE_W - MARGIN_R);
        self.y -= 22.0;
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, words } => self.heading(*level, words),
            Block::Paragraph { words } => self.paragraph(words),
            Block::ListItem { marker, words } => self.list_item(marker, words),
            Block::Rule => {
                self.need(16.0);
                self.y -= 6.0;
                rule(&mut self.cur, MARGIN_L, self.y, PAGE_W - MARGIN_R);
                self.y -= 10.0;
            }
        }
    }

    fn heading(&mut self, level: u8, words: &[Word]) {
        let (size, gap_before, line_h, gap_after) = match level {
            1 => (17.0, 16.0, 21.0, 6.0),
            2 => (14.0, 15.0, 18.0, 5.0),
            _ => (12.0, 11.0, 15.0, 3.0),
        };
        self.y -= gap_before;
        let bold: Vec<Word> = words
            .iter()
            .map(|w| Word {
                text: w.text.clone(),
                style: Style::Bold,
                glue: w.glue,
            })
            .collect();
        for line in wrap(&bold, USABLE_W, size) {
            self.need(line_h);
            draw_line(&mut self.cur, MARGIN_L, self.y, &line, size, ACCENT);
            self.y -= line_h;
        }
        self.y -= gap_after;
    }

    fn paragraph(&mut self, words: &[Word]) {
        let size = 10.5;
        let line_h = 15.0;
        for line in wrap(words, USABLE_W, size) {
            self.need(line_h);
            draw_line(&mut self.cur, MARGIN_L, self.y, &line, size, INK);
            self.y -= line_h;
        }
        self.y -= 6.0;
    }

    fn list_item(&mut self, marker: &str, words: &[Word]) {
        let size = 10.5;
        let line_h = 15.0;
        let text_x = MARGIN_L + BULLET_INDENT;
        let lines = wrap(words, USABLE_W - BULLET_INDENT, size);
        for (i, line) in lines.iter().enumerate() {
            self.need(line_h);
            if i == 0 {
                draw_text(&mut self.cur, MARGIN_L, self.y, marker, "F2", size, ACCENT);
            }
            draw_line(&mut self.cur, text_x, self.y, line, size, INK);
            self.y -= line_h;
        }
        self.y -= 3.0;
    }

    fn build(mut self) -> Vec<u8> {
        self.pages.push(std::mem::take(&mut self.cur));
        let total = self.pages.len().max(1);

        for (index, page) in self.pages.iter_mut().enumerate() {
            let footer = footer_ops(index + 1, total);
            page.push_str(&footer);
        }

        assemble(&self.pages)
    }
}

fn footer_ops(page: usize, total: usize) -> String {
    let mut out = String::new();
    rule(&mut out, MARGIN_L, 52.0, PAGE_W - MARGIN_R);
    draw_text(
        &mut out,
        MARGIN_L,
        40.0,
        "Audis \u{2014} Session report",
        "F1",
        8.0,
        MUTED,
    );
    let label = format!("Page {page} of {total}");
    let width = text_width(&label, Style::Regular) / 1000.0 * 8.0;
    draw_text(
        &mut out,
        PAGE_W - MARGIN_R - width,
        40.0,
        &label,
        "F1",
        8.0,
        MUTED,
    );
    out
}

fn bold_words(text: &str) -> Vec<Word> {
    text.split_whitespace()
        .map(|token| Word {
            text: token.to_owned(),
            style: Style::Bold,
            glue: false,
        })
        .collect()
}

fn wrap(words: &[Word], max_width: f32, size: f32) -> Vec<Vec<Word>> {
    let space = glyph_width(b' ', Style::Regular) / 1000.0 * size;
    let mut lines: Vec<Vec<Word>> = Vec::new();
    let mut line: Vec<Word> = Vec::new();
    let mut width = 0.0f32;

    for word in words {
        let ww = text_width(&word.text, word.style) / 1000.0 * size;
        let leads = line.is_empty() || word.glue;
        let advance = if leads { ww } else { space + ww };
        if !line.is_empty() && !word.glue && width + advance > max_width {
            lines.push(std::mem::take(&mut line));
            line.push(word.clone());
            width = ww;
        } else {
            line.push(word.clone());
            width += advance;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

fn draw_line(
    content: &mut String,
    x0: f32,
    y: f32,
    words: &[Word],
    size: f32,
    color: (f32, f32, f32),
) {
    let space = glyph_width(b' ', Style::Regular) / 1000.0 * size;
    let mut x = x0;
    for (i, word) in words.iter().enumerate() {
        if i > 0 && !word.glue {
            x += space;
        }
        draw_text(
            content,
            x,
            y,
            &word.text,
            font_name(word.style),
            size,
            color,
        );
        x += text_width(&word.text, word.style) / 1000.0 * size;
    }
}

fn draw_text(
    content: &mut String,
    x: f32,
    y: f32,
    text: &str,
    font: &str,
    size: f32,
    color: (f32, f32, f32),
) {
    if text.is_empty() {
        return;
    }
    content.push_str(&format!(
        "BT\n/{} {:.2} Tf\n{:.3} {:.3} {:.3} rg\n1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\nET\n",
        font,
        size,
        color.0,
        color.1,
        color.2,
        x,
        y,
        escape(text)
    ));
}

fn rule(content: &mut String, x0: f32, y: f32, x1: f32) {
    content.push_str(&format!(
        "{:.3} {:.3} {:.3} RG\n0.6 w\n{:.2} {:.2} m\n{:.2} {:.2} l\nS\n",
        HAIRLINE.0, HAIRLINE.1, HAIRLINE.2, x0, y, x1, y
    ));
}

fn font_name(style: Style) -> &'static str {
    match style {
        Style::Regular => "F1",
        Style::Bold => "F2",
        Style::Italic => "F3",
    }
}

fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let b = win_ansi(c).unwrap_or(b'?');
        match b {
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\{b:03o}")),
        }
    }
    out
}

fn win_ansi(c: char) -> Option<u8> {
    let b = match c {
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201C}' => 0x93,
        '\u{201D}' => 0x94,
        '\u{2022}' => 0x95,
        '\u{2013}' => 0x96,
        '\u{2014}' => 0x97,
        '\u{2026}' => 0x85,
        '\u{2039}' => 0x8B,
        '\u{203A}' => 0x9B,
        c if (0x20..=0x7E).contains(&(c as u32)) => c as u8,
        c if (0xA0..=0xFF).contains(&(c as u32)) => c as u8,
        _ => return None,
    };
    Some(b)
}

fn text_width(text: &str, style: Style) -> f32 {
    text.chars()
        .map(|c| {
            let b = win_ansi(c).unwrap_or(b'?');
            glyph_width(b, style)
        })
        .sum()
}

fn glyph_width(b: u8, style: Style) -> f32 {
    let table = match style {
        Style::Bold => &HELV_BOLD,
        _ => &HELV,
    };
    if (0x20..=0x7E).contains(&b) {
        return f32::from(table[(b - 0x20) as usize]);
    }
    match b {
        0x91 | 0x92 => 222.0,
        0x93 | 0x94 => 333.0,
        0x95 => 350.0,
        0x96 => 556.0,
        0x97 => 1000.0,
        0x85 => 1000.0,
        _ => 556.0,
    }
}

#[rustfmt::skip]
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

#[rustfmt::skip]
const HELV_BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

fn assemble(pages: &[String]) -> Vec<u8> {
    let font_objs = 3usize;
    let fixed = 2 + font_objs;
    let total_objs = fixed + pages.len() * 2;

    let mut kids = String::new();
    for i in 0..pages.len() {
        let page_obj = fixed + 1 + i * 2;
        kids.push_str(&format!("{page_obj} 0 R "));
    }

    let mut bodies: Vec<String> = Vec::with_capacity(total_objs);
    bodies.push("<< /Type /Catalog /Pages 2 0 R >>".to_owned());
    bodies.push(format!(
        "<< /Type /Pages /Kids [ {}] /Count {} >>",
        kids.trim_end(),
        pages.len()
    ));
    bodies.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );
    bodies.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );
    bodies.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Oblique /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );

    for (i, page) in pages.iter().enumerate() {
        let content_obj = fixed + 2 + i * 2;
        bodies.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W:.2} {PAGE_H:.2}] \
             /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {content_obj} 0 R >>"
        ));
        bodies.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            page.len(),
            page
        ));
    }

    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize; total_objs + 1];
    for (i, body) in bodies.iter().enumerate() {
        let n = i + 1;
        offsets[n] = pdf.len();
        pdf.push_str(&format!("{n} 0 obj\n{body}\nendobj\n"));
    }

    let xref_off = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n", total_objs + 1));
    pdf.push_str("0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        total_objs + 1,
        xref_off
    ));

    pdf.into_bytes()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn meta() -> ReportMeta {
        ReportMeta {
            title: "Weekly Sync".to_owned(),
            session_type: "Meeting".to_owned(),
            started_at: "18 July 2026".to_owned(),
            language: "English".to_owned(),
            generated_at: "18 July 2026, 16:30".to_owned(),
        }
    }

    #[test]
    fn produces_a_valid_pdf() {
        let md = "# Weekly Sync\n\n## Overview\n\nThe team **shipped** 0.1.1 and agreed next steps.\n\n## Action Items\n\n- Alice: publish the release\n- Bob: write the changelog\n\n## Open Questions\n\n1. Do we bump the minor version?\n2. Who owns QA?\n";
        let pdf = markdown_to_pdf(md, &meta());
        assert!(pdf.starts_with(b"%PDF-1.7"), "must have a PDF header");
        assert!(pdf.ends_with(b"%%EOF\n"), "must end with EOF");
        assert!(pdf.windows(9).any(|w| w == b"/Catalog "), "needs a catalog");
        assert!(pdf.len() > 1200, "a real report is not tiny: {}", pdf.len());
    }

    #[test]
    fn wraps_long_paragraphs_across_lines() {
        let long = "word ".repeat(1500);
        let md = format!("# T\n\n{long}");
        let pdf = markdown_to_pdf(&md, &meta());
        let text = String::from_utf8_lossy(&pdf);
        let pages = text.matches("/Type /Page ").count();
        assert!(
            pages >= 2,
            "a very long report must paginate, got {pages} pages"
        );
    }

    #[test]
    fn handles_empty_input() {
        let pdf = markdown_to_pdf("", &meta());
        assert!(pdf.starts_with(b"%PDF-1.7"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn inline_punctuation_glues_to_the_previous_word() {
        let words = super::parse_blocks("**Darrien**: add the secret");
        if let Some(Block::Paragraph { words }) = words.first() {
            let colon = words.iter().find(|w| w.text == ":").expect("a colon word");
            assert!(
                colon.glue,
                "punctuation after a bold span must not gain a space"
            );
        } else {
            panic!("expected a paragraph");
        }
    }
}
