use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use printpdf::{
    BuiltinFont, Color, Line, LineCapStyle, LinePoint, Mm, Op, PaintMode, PdfDocument,
    PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, RawImage, Rect, Rgb, Svg, TextItem,
    XObjectId, XObjectTransform,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SynStyle, ThemeSet},
    parsing::SyntaxSet,
};
use unicode_width::UnicodeWidthStr;

use crate::ir::{Alignment, Block, Document, Inline, ListItem, QuoteKind, inlines_to_plain_text};

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;
const MARGIN_X: f32 = 72.0;
const MARGIN_TOP: f32 = 64.0;
const MARGIN_BOTTOM: f32 = 68.0;
const BODY_SIZE: f32 = 11.35;
const BODY_LINE: f32 = 16.65;
const TABLE_SIZE: f32 = 9.45;
const TABLE_LINE: f32 = 13.3;
const CODE_SIZE: f32 = 9.35;
const CODE_LINE: f32 = 13.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathMode {
    Latex,
    Fallback,
}

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub page_width_pt: f32,
    pub page_height_pt: f32,
    pub margin_x_pt: f32,
    pub margin_top_pt: f32,
    pub margin_bottom_pt: f32,
    pub body_size_pt: f32,
    pub body_line_height_pt: f32,
    pub table_size_pt: f32,
    pub table_line_height_pt: f32,
    pub code_size_pt: f32,
    pub code_line_height_pt: f32,
    pub code_highlighting: bool,
    pub code_theme: String,
    pub math_mode: MathMode,
    pub page_numbers: bool,
    pub max_image_height_pt: f32,
    pub image_captions: bool,
    pub image_caption_size_pt: f32,
    pub image_caption_gap_pt: f32,
    pub title: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            page_width_pt: PAGE_WIDTH,
            page_height_pt: PAGE_HEIGHT,
            margin_x_pt: MARGIN_X,
            margin_top_pt: MARGIN_TOP,
            margin_bottom_pt: MARGIN_BOTTOM,
            body_size_pt: BODY_SIZE,
            body_line_height_pt: BODY_LINE,
            table_size_pt: TABLE_SIZE,
            table_line_height_pt: TABLE_LINE,
            code_size_pt: CODE_SIZE,
            code_line_height_pt: CODE_LINE,
            code_highlighting: true,
            code_theme: "InspiredGitHub".to_string(),
            math_mode: MathMode::Latex,
            page_numbers: true,
            max_image_height_pt: 286.0,
            image_captions: true,
            image_caption_size_pt: 9.2,
            image_caption_gap_pt: 5.0,
            title: "Native Markdown PDF".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FontFace {
    Serif,
    SerifBold,
    SerifItalic,
    Sans,
    SansBold,
    SansItalic,
    Mono,
    MonoBold,
}

#[derive(Clone, Copy, Debug)]
struct Style {
    font: FontFace,
    size: f32,
    color: (f32, f32, f32),
    bg: Option<(f32, f32, f32)>,
    atomic: bool,
    underline: bool,
    strike: bool,
    shift_y: f32,
    pad_x: f32,
}

#[derive(Clone, Debug)]
struct Fragment {
    kind: FragmentKind,
    style: Style,
    width: f32,
}

#[derive(Clone, Debug)]
enum FragmentKind {
    Text(String),
    Math(RenderedMath),
}

#[derive(Clone, Debug)]
struct RenderedMath {
    id: XObjectId,
    width: f32,
    height: f32,
    y_offset: f32,
}

#[derive(Clone, Debug)]
struct LayoutLine {
    fragments: Vec<Fragment>,
    width: f32,
}

pub fn render_pdf_with_options(
    doc: &Document,
    output: &Path,
    base_dir: &Path,
    options: RenderOptions,
) -> Result<()> {
    let mut renderer = Renderer::new(base_dir, options);
    renderer.render_document(doc)?;
    let bytes = renderer.finish();
    fs::write(output, bytes).with_context(|| format!("failed to write {}", output.display()))?;
    Ok(())
}

struct Renderer<'a> {
    doc: PdfDocument,
    pages: Vec<PdfPage>,
    ops: Vec<Op>,
    cursor_y: f32,
    page_number: usize,
    base_dir: &'a Path,
    options: RenderOptions,
    math_cache: HashMap<String, RenderedMath>,
    footnote_numbers: HashMap<String, usize>,
    next_footnote_number: usize,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl<'a> Renderer<'a> {
    fn new(base_dir: &'a Path, options: RenderOptions) -> Self {
        Self {
            doc: PdfDocument::new(&options.title),
            pages: Vec::new(),
            ops: Vec::new(),
            cursor_y: options.margin_top_pt,
            page_number: 1,
            base_dir,
            options,
            math_cache: HashMap::new(),
            footnote_numbers: HashMap::new(),
            next_footnote_number: 1,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn render_document(&mut self, doc: &Document) -> Result<()> {
        self.index_footnotes(doc);
        for (idx, block) in doc.blocks.iter().enumerate() {
            let next_keep = doc
                .blocks
                .get(idx + 1)
                .map(estimated_keep_height)
                .unwrap_or(0.0);
            self.render_block(block, idx == 0, next_keep)?;
        }
        Ok(())
    }

    fn index_footnotes(&mut self, doc: &Document) {
        for block in &doc.blocks {
            if let Block::Footnote { label, .. } = block {
                self.footnote_number(label);
            }
        }
    }

    fn footnote_number(&mut self, label: &str) -> usize {
        if let Some(number) = self.footnote_numbers.get(label) {
            return *number;
        }
        let number = self.next_footnote_number;
        self.next_footnote_number += 1;
        self.footnote_numbers.insert(label.to_string(), number);
        number
    }

    fn finish(mut self) -> Vec<u8> {
        self.push_page();
        self.doc
            .with_pages(self.pages)
            .save(&PdfSaveOptions::default(), &mut Vec::new())
    }

    fn render_block(&mut self, block: &Block, first: bool, next_keep: f32) -> Result<()> {
        match block {
            Block::Heading { level, content } => self.heading(*level, content, first, next_keep),
            Block::Paragraph(content) => self.paragraph(content),
            Block::CodeBlock { lang, text } => self.code_block(lang.as_deref(), text),
            Block::MathBlock(tex) => self.math_block(tex),
            Block::Divider => self.divider(),
            Block::Quote { kind, content } => self.quote(*kind, content),
            Block::List {
                ordered,
                start,
                items,
            } => self.list(*ordered, *start, items),
            Block::Table(table) => self.table(table),
            Block::Image { src, alt } => self.image(src, alt),
            Block::Footnote { label, content } => self.footnote(label, content),
        }
    }

    fn heading(
        &mut self,
        level: u8,
        content: &[Inline],
        first: bool,
        next_keep: f32,
    ) -> Result<()> {
        let (size, line, before, after) = match level {
            1 => (29.4, 36.0, if first { 0.0 } else { 26.0 }, 14.0),
            2 => (18.35, 24.0, if first { 0.0 } else { 24.0 }, 9.5),
            3 => (14.2, 20.0, 18.0, 7.5),
            _ => (12.4, 18.0, 15.0, 6.5),
        };
        self.add_space(before);

        let style = Style {
            font: FontFace::SansBold,
            size,
            color: if level == 1 {
                (0.07, 0.11, 0.16)
            } else {
                (0.10, 0.14, 0.19)
            },
            bg: None,
            atomic: false,
            underline: false,
            strike: false,
            shift_y: 0.0,
            pad_x: 0.0,
        };
        let lines = self.wrap_inlines(content, style, self.content_width())?;
        let height = lines.len() as f32 * line;
        let keep_with_next = if level <= 3 {
            next_keep.max(126.0)
        } else {
            next_keep.max(70.0)
        };
        self.ensure_height(height + after + keep_with_next);

        self.draw_lines(&lines, self.options.margin_x_pt, self.cursor_y, line);
        self.cursor_y += height;
        self.add_space(after);
        Ok(())
    }

    fn paragraph(&mut self, content: &[Inline]) -> Result<()> {
        let lines = self.wrap_inlines(content, self.body_style(), self.content_width())?;
        let height = lines.len() as f32 * self.options.body_line_height_pt;
        self.ensure_height(height + 11.5);
        self.draw_lines(
            &lines,
            self.options.margin_x_pt,
            self.cursor_y,
            self.options.body_line_height_pt,
        );
        self.cursor_y += height + 11.5;
        Ok(())
    }

    fn code_block(&mut self, lang: Option<&str>, text: &str) -> Result<()> {
        let pad_x = 14.0;
        let pad_y = 13.0;
        let title_h = if lang.is_some() { 14.0 } else { 0.0 };
        let lines = self.highlighted_code_lines(lang, text);
        let block_h =
            pad_y * 2.0 + title_h + lines.len().max(1) as f32 * self.options.code_line_height_pt;
        self.ensure_height(block_h + 16.0);

        let x = self.options.margin_x_pt;
        let y = self.cursor_y;
        let accent = code_accent(lang);
        self.rect(x, y, self.content_width(), block_h, (0.965, 0.972, 0.982));
        self.draw_vertical_rule(x + 4.0, y + 10.0, block_h - 20.0, 2.2, accent);

        if let Some(lang) = lang {
            self.text(
                x + pad_x,
                y + pad_y - 1.5,
                FontFace::SansBold,
                7.7,
                (0.43, 0.49, 0.58),
                &lang.to_ascii_uppercase(),
            );
        }

        let mut line_y = y + pad_y + title_h;
        for line in lines {
            let mut frag_x = x + pad_x;
            for (segment, color) in line {
                self.text(
                    frag_x,
                    line_y,
                    FontFace::Mono,
                    self.options.code_size_pt,
                    color,
                    &segment,
                );
                frag_x += measure(&segment, FontFace::Mono, self.options.code_size_pt);
            }
            line_y += self.options.code_line_height_pt;
        }
        self.cursor_y += block_h + 16.0;
        Ok(())
    }

    fn highlighted_code_lines(
        &self,
        lang: Option<&str>,
        text: &str,
    ) -> Vec<Vec<(String, (f32, f32, f32))>> {
        let syntax = lang
            .and_then(|lang| self.syntax_set.find_syntax_by_token(lang))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = self
            .theme_set
            .themes
            .get(&self.options.code_theme)
            .or_else(|| self.theme_set.themes.get("InspiredGitHub"))
            .or_else(|| self.theme_set.themes.values().next());

        let mut out = Vec::new();
        if self.options.code_highlighting {
            if let Some(theme) = theme {
                let mut highlighter = HighlightLines::new(syntax, theme);
                for raw in text.trim_end().lines() {
                    let ranges = highlighter
                        .highlight_line(raw, &self.syntax_set)
                        .unwrap_or_default();
                    out.push(
                        ranges
                            .into_iter()
                            .map(|(style, segment)| (segment.to_string(), syn_color(style)))
                            .collect(),
                    );
                }
            } else {
                out.extend(
                    text.trim_end()
                        .lines()
                        .map(|line| vec![(line.to_string(), (0.10, 0.13, 0.18))]),
                );
            }
        } else {
            out.extend(
                text.trim_end()
                    .lines()
                    .map(|line| vec![(line.to_string(), (0.10, 0.13, 0.18))]),
            );
        }

        if out.is_empty() {
            out.push(vec![(String::new(), (0.10, 0.13, 0.18))]);
        }
        out
    }

    fn body_style(&self) -> Style {
        Style {
            font: FontFace::Serif,
            size: self.options.body_size_pt,
            color: (0.13, 0.14, 0.17),
            bg: None,
            atomic: false,
            underline: false,
            strike: false,
            shift_y: 0.0,
            pad_x: 0.0,
        }
    }

    fn math_block(&mut self, tex: &str) -> Result<()> {
        let math = self.math(tex.trim(), true, self.options.body_size_pt * 1.18)?;
        let block_h = math.height.max(28.0) + 24.0;
        self.ensure_height(block_h + 15.0);

        let x = self.options.margin_x_pt + 12.0;
        let y = self.cursor_y;
        let w = self.content_width() - 24.0;
        self.rect(x, y, w, block_h, (0.955, 0.971, 0.988));
        let start = x + (w - math.width) / 2.0;
        self.draw_math(&math, start, y + 12.0);
        self.cursor_y += block_h + 15.0;
        Ok(())
    }

    fn divider(&mut self) -> Result<()> {
        self.ensure_height(32.0);
        self.cursor_y += 13.0;
        self.draw_rule_at(
            self.options.margin_x_pt + self.content_width() * 0.16,
            self.cursor_y,
            self.content_width() * 0.68,
            0.9,
            (0.78, 0.81, 0.86),
        );
        self.cursor_y += 18.0;
        Ok(())
    }

    fn quote(&mut self, kind: QuoteKind, content: &[Inline]) -> Result<()> {
        let (title, accent, fill, text_color) = quote_palette(kind);
        let x = self.options.margin_x_pt;
        let pad_x = 16.0;
        let pad_y = 12.0;
        let title_h = if title.is_some() { 13.0 } else { 0.0 };
        let style = Style {
            font: if kind == QuoteKind::Regular {
                FontFace::SerifItalic
            } else {
                FontFace::Serif
            },
            size: self.options.body_size_pt,
            color: text_color,
            bg: None,
            atomic: false,
            underline: false,
            strike: false,
            shift_y: 0.0,
            pad_x: 0.0,
        };
        let lines = self.wrap_inlines(content, style, self.content_width() - pad_x * 2.0)?;
        let line_h = self.options.body_line_height_pt;
        let block_h = pad_y * 2.0 + title_h + lines.len().max(1) as f32 * line_h;
        self.ensure_height(block_h + 14.0);

        let y = self.cursor_y;
        self.rect(x, y, self.content_width(), block_h, fill);
        self.draw_vertical_rule(x + 4.0, y + 9.0, block_h - 18.0, 2.2, accent);
        if let Some(title) = title {
            self.text(
                x + pad_x,
                y + pad_y - 1.0,
                FontFace::SansBold,
                8.4,
                accent,
                title,
            );
        }
        self.draw_lines(&lines, x + pad_x, y + pad_y + title_h, line_h);
        self.cursor_y += block_h + 14.0;
        Ok(())
    }

    fn list(&mut self, ordered: bool, start: u64, items: &[ListItem]) -> Result<()> {
        let marker_w = 28.0;
        let item_gap = 4.5;
        let line_h = self.options.body_line_height_pt;
        let style = self.body_style();
        self.ensure_height(line_h + 12.0);

        for (idx, item) in items.iter().enumerate() {
            let lines = self.wrap_inlines(&item.content, style, self.content_width() - marker_w)?;
            let item_h = lines.len().max(1) as f32 * line_h;
            self.ensure_height(item_h + item_gap + 8.0);
            let y = self.cursor_y;
            self.draw_list_marker(
                ordered,
                start + idx as u64,
                item.checked,
                self.options.margin_x_pt,
                y,
            );
            self.draw_lines(&lines, self.options.margin_x_pt + marker_w, y, line_h);
            self.cursor_y += item_h + item_gap;
        }
        self.cursor_y += 6.0;
        Ok(())
    }

    fn footnote(&mut self, label: &str, content: &[Inline]) -> Result<()> {
        let label_w = 24.0;
        let line_h = 12.2;
        let number = self.footnote_number(label);
        let style = Style {
            font: FontFace::Serif,
            size: 8.9,
            color: (0.28, 0.31, 0.36),
            bg: None,
            atomic: false,
            underline: false,
            strike: false,
            shift_y: 0.0,
            pad_x: 0.0,
        };
        let lines = self.wrap_inlines(content, style, self.content_width() - label_w)?;
        let block_h = lines.len().max(1) as f32 * line_h + 4.0;
        self.ensure_height(block_h + 4.0);

        let y = self.cursor_y;
        self.text(
            self.options.margin_x_pt,
            y,
            FontFace::SansBold,
            7.8,
            (0.42, 0.47, 0.54),
            &format!("{number}."),
        );
        self.draw_lines(&lines, self.options.margin_x_pt + label_w, y, line_h);
        self.cursor_y += block_h;
        Ok(())
    }

    fn table(&mut self, table: &crate::ir::Table) -> Result<()> {
        let cols = table
            .header
            .len()
            .max(table.rows.iter().map(Vec::len).max().unwrap_or(0))
            .max(1);
        let col_w = self.content_width() / cols as f32;
        let cell_pad_x = 7.0;
        let cell_pad_y = 7.0;
        let mut rows = Vec::new();

        if !table.header.is_empty() {
            rows.push((true, table.header.clone()));
        }
        rows.extend(table.rows.iter().cloned().map(|row| (false, row)));

        self.ensure_height(20.0);
        for (is_header, row) in rows {
            let mut laid_out_cells = Vec::new();
            let mut row_h: f32 = 0.0;
            for col in 0..cols {
                let content = row.get(col).cloned().unwrap_or_default();
                let style = if is_header {
                    Style {
                        font: FontFace::SansBold,
                        size: self.options.table_size_pt,
                        color: (0.10, 0.13, 0.18),
                        bg: None,
                        atomic: false,
                        underline: false,
                        strike: false,
                        shift_y: 0.0,
                        pad_x: 0.0,
                    }
                } else {
                    Style {
                        font: FontFace::Serif,
                        size: self.options.table_size_pt,
                        color: (0.16, 0.18, 0.22),
                        bg: None,
                        atomic: false,
                        underline: false,
                        strike: false,
                        shift_y: 0.0,
                        pad_x: 0.0,
                    }
                };
                let lines = self.wrap_inlines(&content, style, col_w - cell_pad_x * 2.0)?;
                row_h = row_h.max(
                    lines.len().max(1) as f32 * self.options.table_line_height_pt
                        + cell_pad_y * 2.0,
                );
                laid_out_cells.push(lines);
            }

            self.ensure_height(row_h + 2.0);
            let row_y = self.cursor_y;
            if is_header {
                self.rect(
                    self.options.margin_x_pt,
                    row_y,
                    self.content_width(),
                    row_h,
                    (0.935, 0.945, 0.960),
                );
            }

            for col in 0..cols {
                let x = self.options.margin_x_pt + col as f32 * col_w;
                self.draw_vertical_rule(x, row_y, row_h, 0.45, (0.78, 0.81, 0.86));
                let align = table
                    .alignments
                    .get(col)
                    .copied()
                    .unwrap_or(Alignment::None);
                let mut line_y = row_y + cell_pad_y;
                for line in &laid_out_cells[col] {
                    let tx = match align {
                        Alignment::Right => x + col_w - cell_pad_x - line.width,
                        Alignment::Center => x + (col_w - line.width) / 2.0,
                        Alignment::Left | Alignment::None => x + cell_pad_x,
                    };
                    self.draw_lines(
                        std::slice::from_ref(line),
                        tx,
                        line_y,
                        self.options.table_line_height_pt,
                    );
                    line_y += self.options.table_line_height_pt;
                }
            }
            self.draw_vertical_rule(
                self.options.margin_x_pt + self.content_width(),
                row_y,
                row_h,
                0.45,
                (0.78, 0.81, 0.86),
            );
            self.draw_rule_at(
                self.options.margin_x_pt,
                row_y,
                self.content_width(),
                0.45,
                (0.78, 0.81, 0.86),
            );
            self.draw_rule_at(
                self.options.margin_x_pt,
                row_y + row_h,
                self.content_width(),
                0.45,
                (0.78, 0.81, 0.86),
            );
            self.cursor_y += row_h;
        }
        self.cursor_y += 16.0;
        Ok(())
    }

    fn image(&mut self, src: &str, alt: &str) -> Result<()> {
        let path = self.base_dir.join(src);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.placeholder_image(src, alt);
                return Ok(());
            }
        };

        let mut warnings = Vec::new();
        let raw = RawImage::decode_from_bytes(&bytes, &mut warnings)
            .map_err(|err| anyhow::anyhow!("failed to decode image {}: {err}", path.display()))?;
        let natural_w = raw.width as f32 * 0.75;
        let natural_h = raw.height as f32 * 0.75;
        let max_w = self.content_width();
        let max_h = self.options.max_image_height_pt;
        let scale = (max_w / natural_w).min(max_h / natural_h).min(1.0);
        let draw_w = natural_w * scale;
        let draw_h = natural_h * scale;
        let show_caption = self.options.image_captions && !alt.trim().is_empty();
        let caption_gap = if show_caption {
            self.options.image_caption_gap_pt
        } else {
            0.0
        };
        let caption_h = if show_caption {
            self.options.image_caption_size_pt * 1.52
        } else {
            0.0
        };
        let block_h = draw_h + caption_gap + caption_h + 12.0;
        self.ensure_height(block_h);

        let x = self.options.margin_x_pt + (self.content_width() - draw_w) / 2.0;
        let y = self.cursor_y;
        self.rect(
            x - 1.0,
            y - 1.0,
            draw_w + 2.0,
            draw_h + 2.0,
            (0.90, 0.91, 0.93),
        );

        let id = self.doc.add_image(&raw);
        self.ops.push(Op::UseXobject {
            id,
            transform: XObjectTransform {
                translate_x: Some(Pt(x)),
                translate_y: Some(Pt(self.options.page_height_pt - y - draw_h)),
                scale_x: Some(draw_w / raw.width as f32),
                scale_y: Some(draw_h / raw.height as f32),
                dpi: Some(72.0),
                ..Default::default()
            },
        });

        if show_caption {
            self.text(
                self.options.margin_x_pt,
                y + draw_h + caption_gap,
                FontFace::SerifItalic,
                self.options.image_caption_size_pt,
                (0.39, 0.43, 0.49),
                alt,
            );
        }
        self.cursor_y += block_h;
        Ok(())
    }

    fn placeholder_image(&mut self, src: &str, alt: &str) {
        let h = 96.0;
        self.ensure_height(h + 14.0);
        let y = self.cursor_y;
        self.rect(
            self.options.margin_x_pt,
            y,
            self.content_width(),
            h,
            (0.96, 0.965, 0.972),
        );
        let label = if alt.trim().is_empty() { src } else { alt };
        self.text(
            self.options.margin_x_pt + 18.0,
            y + 40.0,
            FontFace::SansBold,
            10.0,
            (0.42, 0.47, 0.54),
            label,
        );
        self.cursor_y += h + 14.0;
    }

    fn content_width(&self) -> f32 {
        self.options.page_width_pt - self.options.margin_x_pt * 2.0
    }

    fn usable_bottom(&self) -> f32 {
        self.options.page_height_pt - self.options.margin_bottom_pt
    }

    fn add_space(&mut self, amount: f32) {
        if self.cursor_y > self.options.margin_top_pt + 1.0 {
            self.cursor_y += amount;
        }
    }

    fn ensure_height(&mut self, height: f32) {
        if self.cursor_y + height > self.usable_bottom()
            && self.cursor_y > self.options.margin_top_pt + 1.0
        {
            self.push_page();
        }
    }

    fn push_page(&mut self) {
        self.footer();
        let ops = std::mem::take(&mut self.ops);
        self.pages.push(PdfPage::new(
            pt_to_mm(self.options.page_width_pt),
            pt_to_mm(self.options.page_height_pt),
            ops,
        ));
        self.cursor_y = self.options.margin_top_pt;
        self.page_number += 1;
    }

    fn footer(&mut self) {
        if !self.options.page_numbers {
            return;
        }
        self.text(
            self.options.page_width_pt - self.options.margin_x_pt - 42.0,
            self.options.page_height_pt - 34.0,
            FontFace::Sans,
            8.0,
            (0.54, 0.58, 0.64),
            &format!("Page {}", self.page_number),
        );
    }

    fn draw_lines(&mut self, lines: &[LayoutLine], x: f32, y: f32, line_height: f32) {
        let mut line_y = y;
        for line in lines {
            let mut frag_x = x;
            let max_text_size = line
                .fragments
                .iter()
                .filter_map(|frag| match frag.kind {
                    FragmentKind::Text(_) => Some(frag.style.size),
                    FragmentKind::Math(_) => None,
                })
                .fold(0.0, f32::max);
            for frag in &line.fragments {
                if let Some(bg) = frag.style.bg {
                    let bg_h = (frag.style.size + 3.4).min(line_height - 2.0);
                    let bg_y = line_y + (line_height - bg_h) / 2.0;
                    self.rect(frag_x, bg_y, frag.width + frag.style.pad_x * 2.0, bg_h, bg);
                }
                let text_x = frag_x + frag.style.pad_x;
                match &frag.kind {
                    FragmentKind::Text(text) => {
                        let text_y = line_y
                            + (max_text_size - frag.style.size).max(0.0)
                            + frag.style.shift_y;
                        self.text(
                            text_x,
                            text_y,
                            frag.style.font,
                            frag.style.size,
                            frag.style.color,
                            text,
                        );
                        if frag.style.underline {
                            self.draw_rule_at(
                                text_x,
                                text_y + frag.style.size + 1.7,
                                decoration_width(frag),
                                0.45,
                                frag.style.color,
                            );
                        }
                        if frag.style.strike {
                            self.draw_rule_at(
                                text_x,
                                text_y + frag.style.size * 0.62,
                                decoration_width(frag),
                                0.45,
                                frag.style.color,
                            );
                        }
                    }
                    FragmentKind::Math(math) => {
                        self.draw_math(math, text_x, line_y + math.y_offset)
                    }
                }
                frag_x += fragment_advance(frag);
            }
            line_y += line_height;
        }
    }

    fn text(
        &mut self,
        x: f32,
        y_top: f32,
        font: FontFace,
        size: f32,
        color: (f32, f32, f32),
        text: &str,
    ) {
        if text.is_empty() {
            return;
        }
        self.ops.extend_from_slice(&[
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: Pt(x),
                    y: Pt(self.options.page_height_pt - y_top - size),
                },
            },
            Op::SetFont {
                font: PdfFontHandle::Builtin(font.to_builtin()),
                size: Pt(size),
            },
            Op::SetLineHeight {
                lh: Pt(size * 1.25),
            },
            Op::SetFillColor { col: rgb(color) },
            Op::ShowText {
                items: vec![TextItem::Text(pdf_safe_text(text))],
            },
            Op::EndTextSection,
        ]);
    }

    fn rect(&mut self, x: f32, y_top: f32, w: f32, h: f32, color: (f32, f32, f32)) {
        let mut rect = Rect::from_xywh(
            Pt(x),
            Pt(self.options.page_height_pt - y_top - h),
            Pt(w),
            Pt(h),
        );
        rect.mode = Some(PaintMode::Fill);
        self.ops.push(Op::SetFillColor { col: rgb(color) });
        self.ops.push(Op::DrawRectangle { rectangle: rect });
    }

    fn draw_rule_at(&mut self, x: f32, y_top: f32, w: f32, thickness: f32, color: (f32, f32, f32)) {
        self.draw_line_segment(x, y_top, x + w, y_top, thickness, color);
    }

    fn draw_line_segment(
        &mut self,
        x1: f32,
        y1_top: f32,
        x2: f32,
        y2_top: f32,
        thickness: f32,
        color: (f32, f32, f32),
    ) {
        let y1 = self.options.page_height_pt - y1_top;
        let y2 = self.options.page_height_pt - y2_top;
        self.ops.extend_from_slice(&[
            Op::SetOutlineColor { col: rgb(color) },
            Op::SetOutlineThickness { pt: Pt(thickness) },
            Op::SetLineCapStyle {
                cap: LineCapStyle::Butt,
            },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point {
                                x: Pt(x1),
                                y: Pt(y1),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x2),
                                y: Pt(y2),
                            },
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
        ]);
    }

    fn rect_outline(
        &mut self,
        x: f32,
        y_top: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: (f32, f32, f32),
    ) {
        self.draw_line_segment(x, y_top, x + w, y_top, thickness, color);
        self.draw_line_segment(x + w, y_top, x + w, y_top + h, thickness, color);
        self.draw_line_segment(x + w, y_top + h, x, y_top + h, thickness, color);
        self.draw_line_segment(x, y_top + h, x, y_top, thickness, color);
    }

    fn draw_list_marker(
        &mut self,
        ordered: bool,
        number: u64,
        checked: Option<bool>,
        x: f32,
        y: f32,
    ) {
        let color = (0.32, 0.38, 0.46);
        if let Some(checked) = checked {
            let box_x = x + 1.5;
            let box_y = y + 3.7;
            self.rect_outline(box_x, box_y, 7.6, 7.6, 0.8, color);
            if checked {
                self.draw_line_segment(
                    box_x + 1.6,
                    box_y + 4.0,
                    box_x + 3.3,
                    box_y + 5.9,
                    1.0,
                    color,
                );
                self.draw_line_segment(
                    box_x + 3.3,
                    box_y + 5.9,
                    box_x + 6.3,
                    box_y + 1.8,
                    1.0,
                    color,
                );
            }
        } else if ordered {
            self.text(
                x,
                y + 0.1,
                FontFace::SansBold,
                self.options.body_size_pt * 0.86,
                color,
                &format!("{number}."),
            );
        } else {
            self.rect(x + 5.0, y + 6.2, 3.6, 3.6, color);
        }
    }

    fn draw_vertical_rule(
        &mut self,
        x: f32,
        y_top: f32,
        h: f32,
        thickness: f32,
        color: (f32, f32, f32),
    ) {
        let y1 = self.options.page_height_pt - y_top;
        let y2 = self.options.page_height_pt - y_top - h;
        self.ops.extend_from_slice(&[
            Op::SetOutlineColor { col: rgb(color) },
            Op::SetOutlineThickness { pt: Pt(thickness) },
            Op::SetLineCapStyle {
                cap: LineCapStyle::Butt,
            },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point {
                                x: Pt(x),
                                y: Pt(y1),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x),
                                y: Pt(y2),
                            },
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
        ]);
    }

    fn wrap_inlines(
        &mut self,
        inlines: &[Inline],
        base: Style,
        max_width: f32,
    ) -> Result<Vec<LayoutLine>> {
        let mut spans = Vec::new();
        self.push_inline_spans(inlines, base, &mut spans)?;
        Ok(wrap_spans(&spans, max_width))
    }

    fn push_inline_spans(
        &mut self,
        inlines: &[Inline],
        base: Style,
        spans: &mut Vec<Fragment>,
    ) -> Result<()> {
        for inline in inlines {
            match inline {
                Inline::Text(text) => spans.push(text_fragment(text.clone(), base)),
                Inline::Code(code) => spans.push(text_fragment(
                    code.clone(),
                    Style {
                        font: FontFace::Mono,
                        size: base.size * 0.94,
                        color: (0.05, 0.11, 0.18),
                        bg: Some((0.935, 0.945, 0.958)),
                        atomic: true,
                        underline: false,
                        strike: false,
                        shift_y: -0.9,
                        pad_x: 0.95,
                    },
                )),
                Inline::Math(tex) => {
                    let math = self.math(tex, false, base.size)?;
                    spans.push(Fragment {
                        width: math.width,
                        kind: FragmentKind::Math(math),
                        style: Style {
                            font: FontFace::Serif,
                            size: base.size,
                            color: (0.08, 0.18, 0.32),
                            bg: None,
                            atomic: true,
                            underline: false,
                            strike: false,
                            shift_y: 0.0,
                            pad_x: 1.0,
                        },
                    });
                }
                Inline::Emphasis(content) => {
                    let mut style = base;
                    style.font = match base.font {
                        FontFace::Serif | FontFace::SerifBold => FontFace::SerifItalic,
                        FontFace::Sans | FontFace::SansBold => FontFace::SansItalic,
                        other => other,
                    };
                    self.push_inline_spans(content, style, spans)?;
                }
                Inline::Strong(content) => {
                    let mut style = base;
                    style.font = match base.font {
                        FontFace::Serif | FontFace::SerifItalic => FontFace::SerifBold,
                        FontFace::Sans | FontFace::SansItalic => FontFace::SansBold,
                        other => other,
                    };
                    self.push_inline_spans(content, style, spans)?;
                }
                Inline::Strikethrough(content) => {
                    let mut style = base;
                    style.strike = true;
                    self.push_inline_spans(content, style, spans)?;
                }
                Inline::Link { href, content } => {
                    let _target = href;
                    let mut link_style = base;
                    link_style.color = (0.02, 0.28, 0.62);
                    link_style.underline = true;
                    self.push_inline_spans(content, link_style, spans)?;
                }
                Inline::FootnoteRef(label) => spans.push(text_fragment(
                    self.footnote_number(label).to_string(),
                    Style {
                        font: FontFace::SansBold,
                        size: base.size * 0.58,
                        color: (0.02, 0.28, 0.62),
                        bg: None,
                        atomic: true,
                        underline: false,
                        strike: false,
                        shift_y: -3.2,
                        pad_x: 0.0,
                    },
                )),
                Inline::Citation(key) => {
                    let text = format!("({})", citation_label(key));
                    let style = Style {
                        font: FontFace::SerifItalic,
                        size: base.size * 0.96,
                        color: (0.30, 0.28, 0.40),
                        bg: None,
                        atomic: true,
                        underline: false,
                        strike: false,
                        shift_y: 0.0,
                        pad_x: 0.0,
                    };
                    spans.push(Fragment {
                        width: measure(&text, style.font, style.size) + 3.0,
                        kind: FragmentKind::Text(text),
                        style,
                    });
                }
            }
        }
        Ok(())
    }

    fn math(&mut self, tex: &str, display: bool, size: f32) -> Result<RenderedMath> {
        match self.options.math_mode {
            MathMode::Latex => self.render_math(tex, display),
            MathMode::Fallback => Ok(self.math_fallback(tex, size)),
        }
    }

    fn math_fallback(&mut self, tex: &str, size: f32) -> RenderedMath {
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{w}pt' height='{h}pt'><text x='0' y='{baseline}' font-family='Times' font-size='{size}' font-style='italic'>{}</text></svg>",
            escape_xml(tex),
            w = measure(tex, FontFace::SerifItalic, size).max(8.0),
            h = size * 1.3,
            baseline = size,
            size = size,
        );
        let mut warnings = Vec::new();
        if let Ok(external) = Svg::parse(&svg, &mut warnings) {
            let id = self.doc.add_xobject(&external);
            return RenderedMath {
                id,
                width: measure(tex, FontFace::SerifItalic, size).max(8.0),
                height: size * 1.3,
                y_offset: 0.0,
            };
        }
        RenderedMath {
            id: XObjectId::new(),
            width: 0.0,
            height: 0.0,
            y_offset: 0.0,
        }
    }

    fn render_math(&mut self, tex: &str, display: bool) -> Result<RenderedMath> {
        let key = format!("{}:{}", if display { "display" } else { "inline" }, tex);
        if let Some(rendered) = self.math_cache.get(&key) {
            return Ok(rendered.clone());
        }

        let svg = tex_to_svg(tex, display)?;
        let (width, height) = svg_size_pt(&svg).unwrap_or((80.0, 18.0));
        let mut warnings = Vec::new();
        let external = Svg::parse(&svg, &mut warnings)
            .map_err(|err| anyhow::anyhow!("failed to parse TeX SVG: {err}"))?;
        let id = self.doc.add_xobject(&external);
        let rendered = RenderedMath {
            id,
            width,
            height,
            y_offset: if display {
                0.0
            } else {
                (self.options.body_line_height_pt - height) * 0.58
            },
        };
        self.math_cache.insert(key, rendered.clone());
        Ok(rendered)
    }

    fn draw_math(&mut self, math: &RenderedMath, x: f32, y_top: f32) {
        self.ops.push(Op::UseXobject {
            id: math.id.clone(),
            transform: XObjectTransform {
                translate_x: Some(Pt(x)),
                translate_y: Some(Pt(self.options.page_height_pt - y_top - math.height)),
                scale_x: Some(1.0),
                scale_y: Some(1.0),
                dpi: Some(300.0),
                ..Default::default()
            },
        });
    }
}

impl FontFace {
    fn to_builtin(self) -> BuiltinFont {
        match self {
            FontFace::Serif => BuiltinFont::TimesRoman,
            FontFace::SerifBold => BuiltinFont::TimesBold,
            FontFace::SerifItalic => BuiltinFont::TimesItalic,
            FontFace::Sans => BuiltinFont::Helvetica,
            FontFace::SansBold => BuiltinFont::HelveticaBold,
            FontFace::SansItalic => BuiltinFont::HelveticaOblique,
            FontFace::Mono => BuiltinFont::Courier,
            FontFace::MonoBold => BuiltinFont::CourierBold,
        }
    }
}

fn text_fragment(text: String, style: Style) -> Fragment {
    Fragment {
        width: measure(&text, style.font, style.size),
        kind: FragmentKind::Text(text),
        style,
    }
}

fn wrap_spans(spans: &[Fragment], max_width: f32) -> Vec<LayoutLine> {
    let mut lines = Vec::new();
    let mut current = LayoutLine {
        fragments: Vec::new(),
        width: 0.0,
    };

    for span in spans {
        match &span.kind {
            FragmentKind::Math(_) => {
                push_fragment(&mut lines, &mut current, span.clone(), max_width)
            }
            FragmentKind::Text(text) => {
                if span.style.atomic || span.style.bg.is_some() {
                    push_fragment(&mut lines, &mut current, span.clone(), max_width);
                    continue;
                }
                for token in tokenize(text) {
                    let is_space = token.trim().is_empty();
                    if is_space && current.fragments.is_empty() {
                        continue;
                    }
                    let frag = text_fragment(token, span.style);
                    push_fragment(&mut lines, &mut current, frag, max_width);
                }
            }
        }
    }

    trim_trailing_space(&mut current);
    merge_adjacent_fragments(&mut current);
    if !current.fragments.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(LayoutLine {
            fragments: Vec::new(),
            width: 0.0,
        });
    }
    lines
}

fn push_fragment(
    lines: &mut Vec<LayoutLine>,
    current: &mut LayoutLine,
    fragment: Fragment,
    max_width: f32,
) {
    let is_space = matches!(&fragment.kind, FragmentKind::Text(text) if text.trim().is_empty());
    let advance = fragment_advance(&fragment);

    if !current.fragments.is_empty() && current.width + advance > max_width {
        trim_trailing_space(current);
        merge_adjacent_fragments(current);
        lines.push(std::mem::replace(
            current,
            LayoutLine {
                fragments: Vec::new(),
                width: 0.0,
            },
        ));
        if is_space {
            return;
        }
    }

    if advance > max_width {
        if let FragmentKind::Text(text) = &fragment.kind {
            for ch in text.chars() {
                let piece = text_fragment(ch.to_string(), fragment.style);
                push_fragment(lines, current, piece, max_width);
            }
            return;
        }
    }

    current.width += advance;
    current.fragments.push(fragment);
}

fn merge_adjacent_fragments(line: &mut LayoutLine) {
    let mut merged: Vec<Fragment> = Vec::new();
    for frag in line.fragments.drain(..) {
        if let Some(last) = merged.last_mut() {
            if same_style(last.style, frag.style) {
                if let (FragmentKind::Text(last_text), FragmentKind::Text(text)) =
                    (&mut last.kind, &frag.kind)
                {
                    last_text.push_str(text);
                    last.width = measure(last_text, last.style.font, last.style.size);
                    continue;
                }
            }
        }
        merged.push(frag);
    }
    line.width = merged.iter().map(fragment_advance).sum();
    line.fragments = merged;
}

fn same_style(a: Style, b: Style) -> bool {
    a.font == b.font
        && (a.size - b.size).abs() < 0.01
        && a.color == b.color
        && a.bg == b.bg
        && a.atomic == b.atomic
        && a.underline == b.underline
        && a.strike == b.strike
        && (a.shift_y - b.shift_y).abs() < 0.01
        && (a.pad_x - b.pad_x).abs() < 0.01
}

fn trim_trailing_space(line: &mut LayoutLine) {
    while line.fragments.last().is_some_and(is_space_fragment) {
        if let Some(frag) = line.fragments.pop() {
            line.width -= fragment_advance(&frag);
        }
    }
}

fn is_space_fragment(fragment: &Fragment) -> bool {
    matches!(&fragment.kind, FragmentKind::Text(text) if text.trim().is_empty())
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_space = None;

    for ch in text.chars() {
        let is_space = ch.is_whitespace();
        if current_is_space == Some(is_space) {
            current.push(if is_space { ' ' } else { ch });
        } else {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            current.push(if is_space { ' ' } else { ch });
            current_is_space = Some(is_space);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn measure(text: &str, font: FontFace, size: f32) -> f32 {
    if font == FontFace::Mono || font == FontFace::MonoBold {
        return text.width() as f32 * size * 0.60;
    }

    let raw = text.chars().fold(0.0, |acc, ch| {
        let factor = glyph_factor(ch);
        acc + size * factor
    });
    raw * font_measure_adjust(font)
}

fn glyph_factor(ch: char) -> f32 {
    if ch == ' ' {
        0.34
    } else if ".,:;!|`'".contains(ch) {
        0.255
    } else if "ilI[](){}".contains(ch) {
        0.31
    } else if "mwMW@#%&".contains(ch) {
        0.72
    } else if ch.is_ascii_uppercase() {
        0.57
    } else if ch.is_ascii_digit() {
        0.47
    } else {
        0.44
    }
}

fn font_measure_adjust(font: FontFace) -> f32 {
    match font {
        FontFace::SerifBold => 0.95,
        FontFace::SerifItalic => 1.0,
        FontFace::Sans | FontFace::SansBold | FontFace::SansItalic => 0.98,
        FontFace::Serif | FontFace::Mono | FontFace::MonoBold => 1.0,
    }
}

fn fragment_advance(fragment: &Fragment) -> f32 {
    let width = if fragment.style.underline {
        decoration_width(fragment)
    } else {
        fragment.width
    };
    (width + fragment.style.pad_x * 2.0).max(0.0)
}

fn decoration_width(fragment: &Fragment) -> f32 {
    let FragmentKind::Text(text) = &fragment.kind else {
        return fragment.width;
    };
    let trimmed = text.trim_end();
    let width = measure(trimmed, fragment.style.font, fragment.style.size);

    match fragment.style.font {
        FontFace::Serif | FontFace::SerifBold | FontFace::SerifItalic => width * 0.92,
        FontFace::Sans | FontFace::SansBold | FontFace::SansItalic => width * 0.96,
        FontFace::Mono | FontFace::MonoBold => width,
    }
}

fn code_accent(lang: Option<&str>) -> (f32, f32, f32) {
    match lang.unwrap_or_default().to_ascii_lowercase().as_str() {
        "rust" | "rs" => (0.82, 0.25, 0.18),
        "typescript" | "ts" | "tsx" => (0.18, 0.41, 0.72),
        "javascript" | "js" | "jsx" => (0.78, 0.56, 0.12),
        "python" | "py" => (0.21, 0.45, 0.68),
        "go" | "golang" => (0.00, 0.55, 0.70),
        "swift" => (0.90, 0.35, 0.14),
        "html" | "xml" => (0.86, 0.32, 0.18),
        "css" | "scss" => (0.43, 0.30, 0.70),
        "json" | "yaml" | "yml" | "toml" => (0.34, 0.48, 0.28),
        "bash" | "sh" | "zsh" => (0.24, 0.55, 0.34),
        "sql" => (0.56, 0.35, 0.70),
        "tex" | "latex" => (0.12, 0.48, 0.56),
        _ => (0.42, 0.47, 0.54),
    }
}

fn quote_palette(
    kind: QuoteKind,
) -> (
    Option<&'static str>,
    (f32, f32, f32),
    (f32, f32, f32),
    (f32, f32, f32),
) {
    match kind {
        QuoteKind::Regular => (
            None,
            (0.50, 0.55, 0.62),
            (0.965, 0.970, 0.976),
            (0.22, 0.25, 0.30),
        ),
        QuoteKind::Note => (
            Some("NOTE"),
            (0.08, 0.36, 0.72),
            (0.940, 0.966, 0.995),
            (0.12, 0.20, 0.32),
        ),
        QuoteKind::Tip => (
            Some("TIP"),
            (0.12, 0.55, 0.34),
            (0.940, 0.980, 0.952),
            (0.12, 0.25, 0.18),
        ),
        QuoteKind::Important => (
            Some("IMPORTANT"),
            (0.45, 0.26, 0.72),
            (0.962, 0.948, 0.992),
            (0.20, 0.16, 0.30),
        ),
        QuoteKind::Warning => (
            Some("WARNING"),
            (0.80, 0.48, 0.08),
            (0.996, 0.970, 0.910),
            (0.32, 0.22, 0.10),
        ),
        QuoteKind::Caution => (
            Some("CAUTION"),
            (0.78, 0.22, 0.20),
            (0.994, 0.942, 0.938),
            (0.32, 0.14, 0.14),
        ),
    }
}

fn citation_label(key: &str) -> String {
    let cleaned = key.replace(['_', '-', ':'], " ");
    if cleaned.len() > 4 {
        let split = cleaned.len() - 4;
        let (author, year) = cleaned.split_at(split);
        if year.chars().all(|ch| ch.is_ascii_digit()) {
            return format!("{} {}", title_word(author.trim()), year);
        }
    }
    title_word(cleaned.trim())
}

fn title_word(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_uppercase(), chars.as_str())
}

fn syn_color(style: SynStyle) -> (f32, f32, f32) {
    (
        style.foreground.r as f32 / 255.0,
        style.foreground.g as f32 / 255.0,
        style.foreground.b as f32 / 255.0,
    )
}

fn rgb((r, g, b): (f32, f32, f32)) -> Color {
    Color::Rgb(Rgb {
        r,
        g,
        b,
        icc_profile: None,
    })
}

fn pdf_safe_text(text: &str) -> String {
    text.replace('\t', "    ")
}

fn pt_to_mm(pt: f32) -> Mm {
    Mm(pt * 25.4 / 72.0)
}

fn tex_to_svg(tex: &str, display: bool) -> Result<String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("native-mdpdf-math-{nonce}"));
    fs::create_dir_all(&dir)?;
    let tex_path = dir.join("math.tex");
    let body = if display {
        format!("\\[\\displaystyle\n{tex}\n\\]")
    } else {
        format!("${tex}$")
    };
    fs::write(
        &tex_path,
        format!(
            "\\documentclass[preview,border=0pt]{{standalone}}\n\\usepackage{{amsmath,amssymb,mathtools,bm}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
        ),
    )?;

    let latex = Command::new("latex")
        .current_dir(&dir)
        .args(["-interaction=nonstopmode", "-halt-on-error", "math.tex"])
        .output()
        .context("failed to run latex for math rendering")?;
    if !latex.status.success() {
        return Err(anyhow::anyhow!(
            "latex failed: {}",
            String::from_utf8_lossy(&latex.stderr)
        ));
    }

    let dvisvgm = Command::new("dvisvgm")
        .current_dir(&dir)
        .args(["--no-fonts", "--exact-bbox", "math.dvi", "-o", "math.svg"])
        .output()
        .context("failed to run dvisvgm for math rendering")?;
    if !dvisvgm.status.success() {
        return Err(anyhow::anyhow!(
            "dvisvgm failed: {}",
            String::from_utf8_lossy(&dvisvgm.stderr)
        ));
    }

    fs::read_to_string(dir.join("math.svg")).context("failed to read generated math SVG")
}

fn svg_size_pt(svg: &str) -> Option<(f32, f32)> {
    Some((svg_attr_pt(svg, "width")?, svg_attr_pt(svg, "height")?))
}

fn svg_attr_pt(svg: &str, attr: &str) -> Option<f32> {
    let needle = format!("{attr}='");
    let start = svg.find(&needle)? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('\'')?;
    rest[..end].trim_end_matches("pt").parse().ok()
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn estimated_keep_height(block: &Block) -> f32 {
    match block {
        Block::Image { .. } => 285.0,
        Block::Table(table) => 70.0 + table.rows.len().min(2) as f32 * 34.0,
        Block::CodeBlock { text, .. } => {
            let lines = text.lines().count().clamp(4, 12) as f32;
            34.0 + lines * CODE_LINE
        }
        Block::MathBlock(_) => 72.0,
        Block::Quote { .. } => 76.0,
        Block::List { items, .. } => 30.0 + items.len().min(3) as f32 * 24.0,
        Block::Paragraph(_) => 62.0,
        Block::Divider => 32.0,
        Block::Heading { .. } => 82.0,
        Block::Footnote { .. } => 24.0,
    }
}

#[allow(dead_code)]
fn plain(inlines: &[Inline]) -> String {
    inlines_to_plain_text(inlines)
}
