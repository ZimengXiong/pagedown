use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Command,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use lopdf::{
    Dictionary as LoDictionary, Document as LoDocument, Object as LoObject, ObjectId as LoObjectId,
    Stream as LoStream,
};
use printpdf::{
    Actions, BorderArray, BuiltinFont, Color, ColorArray, DictItem, HighlightingMode, Line,
    LineCapStyle, LinePoint, LinkAnnotation, Mm, Op, PaintMode, PdfDocument, PdfFontHandle,
    PdfPage, PdfSaveOptions, Point, Pt, RawImage, Rect, Rgb, Svg, TextItem, XObjectId,
    XObjectTransform,
};
use serde::Deserialize;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SynStyle, ThemeSet},
    parsing::SyntaxSet,
};

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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MathMode {
    Katex,
    Lualatex,
    #[serde(alias = "latex")]
    Latex,
    Fallback,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
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
    pub paragraph: ParagraphOptions,
    pub headings: HeadingOptions,
    pub code_block: CodeBlockOptions,
    pub math_block: MathBlockOptions,
    pub divider: DividerOptions,
    pub quote: QuoteOptions,
    pub list: ListOptions,
    pub footnote: FootnoteOptions,
    pub table: TableOptions,
    pub image: ImageOptions,
    pub inline: InlineOptions,
    pub footer: FooterOptions,
    pub pagination: PaginationOptions,
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
            math_mode: MathMode::Katex,
            page_numbers: true,
            max_image_height_pt: 286.0,
            image_captions: true,
            image_caption_size_pt: 9.2,
            image_caption_gap_pt: 5.0,
            title: "Native Markdown PDF".to_string(),
            paragraph: ParagraphOptions::default(),
            headings: HeadingOptions::default(),
            code_block: CodeBlockOptions::default(),
            math_block: MathBlockOptions::default(),
            divider: DividerOptions::default(),
            quote: QuoteOptions::default(),
            list: ListOptions::default(),
            footnote: FootnoteOptions::default(),
            table: TableOptions::default(),
            image: ImageOptions::default(),
            inline: InlineOptions::default(),
            footer: FooterOptions::default(),
            pagination: PaginationOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ParagraphOptions {
    pub after_pt: f32,
}

impl Default for ParagraphOptions {
    fn default() -> Self {
        Self { after_pt: 11.5 }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HeadingOptions {
    pub level1: HeadingLevelOptions,
    pub level2: HeadingLevelOptions,
    pub level3: HeadingLevelOptions,
    pub other: HeadingLevelOptions,
    pub keep_with_next_level1_to_3_pt: f32,
    pub keep_with_next_other_pt: f32,
}

impl Default for HeadingOptions {
    fn default() -> Self {
        Self {
            level1: HeadingLevelOptions {
                size_pt: 29.4,
                line_height_pt: 36.0,
                space_before_pt: 26.0,
                first_space_before_pt: 0.0,
                space_after_pt: 14.0,
            },
            level2: HeadingLevelOptions {
                size_pt: 18.35,
                line_height_pt: 24.0,
                space_before_pt: 24.0,
                first_space_before_pt: 0.0,
                space_after_pt: 9.5,
            },
            level3: HeadingLevelOptions {
                size_pt: 14.2,
                line_height_pt: 20.0,
                space_before_pt: 18.0,
                first_space_before_pt: 18.0,
                space_after_pt: 7.5,
            },
            other: HeadingLevelOptions {
                size_pt: 12.4,
                line_height_pt: 18.0,
                space_before_pt: 15.0,
                first_space_before_pt: 15.0,
                space_after_pt: 6.5,
            },
            keep_with_next_level1_to_3_pt: 126.0,
            keep_with_next_other_pt: 70.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct HeadingLevelOptions {
    pub size_pt: f32,
    pub line_height_pt: f32,
    pub space_before_pt: f32,
    pub first_space_before_pt: f32,
    pub space_after_pt: f32,
}

impl Default for HeadingLevelOptions {
    fn default() -> Self {
        Self {
            size_pt: 12.4,
            line_height_pt: 18.0,
            space_before_pt: 15.0,
            first_space_before_pt: 15.0,
            space_after_pt: 6.5,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CodeBlockOptions {
    pub pad_x_pt: f32,
    pub pad_y_pt: f32,
    pub title_height_pt: f32,
    pub title_y_offset_pt: f32,
    pub title_size_pt: f32,
    pub after_pt: f32,
    pub rule_x_offset_pt: f32,
    pub rule_y_inset_pt: f32,
    pub rule_total_y_inset_pt: f32,
    pub rule_thickness_pt: f32,
}

impl Default for CodeBlockOptions {
    fn default() -> Self {
        Self {
            pad_x_pt: 14.0,
            pad_y_pt: 13.0,
            title_height_pt: 14.0,
            title_y_offset_pt: -1.5,
            title_size_pt: 7.7,
            after_pt: 16.0,
            rule_x_offset_pt: 4.0,
            rule_y_inset_pt: 10.0,
            rule_total_y_inset_pt: 20.0,
            rule_thickness_pt: 2.2,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MathBlockOptions {
    pub size_multiplier: f32,
    pub x_inset_pt: f32,
    pub content_width_inset_pt: f32,
    pub vertical_padding_pt: f32,
    pub draw_y_inset_pt: f32,
    pub min_math_height_pt: f32,
    pub after_pt: f32,
}

impl Default for MathBlockOptions {
    fn default() -> Self {
        Self {
            size_multiplier: 1.18,
            x_inset_pt: 12.0,
            content_width_inset_pt: 24.0,
            vertical_padding_pt: 24.0,
            draw_y_inset_pt: 12.0,
            min_math_height_pt: 28.0,
            after_pt: 15.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DividerOptions {
    pub keep_height_pt: f32,
    pub space_before_pt: f32,
    pub space_after_pt: f32,
    pub x_inset_pt: f32,
    pub thickness_pt: f32,
}

impl Default for DividerOptions {
    fn default() -> Self {
        Self {
            keep_height_pt: 32.0,
            space_before_pt: 13.0,
            space_after_pt: 18.0,
            x_inset_pt: 0.0,
            thickness_pt: 0.9,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct QuoteOptions {
    pub pad_x_pt: f32,
    pub pad_y_pt: f32,
    pub title_height_pt: f32,
    pub title_y_offset_pt: f32,
    pub title_size_pt: f32,
    pub after_pt: f32,
    pub rule_x_offset_pt: f32,
    pub rule_y_inset_pt: f32,
    pub rule_total_y_inset_pt: f32,
    pub rule_thickness_pt: f32,
}

impl Default for QuoteOptions {
    fn default() -> Self {
        Self {
            pad_x_pt: 16.0,
            pad_y_pt: 12.0,
            title_height_pt: 13.0,
            title_y_offset_pt: -1.0,
            title_size_pt: 8.4,
            after_pt: 14.0,
            rule_x_offset_pt: 4.0,
            rule_y_inset_pt: 9.0,
            rule_total_y_inset_pt: 18.0,
            rule_thickness_pt: 2.2,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ListOptions {
    pub item_gap_pt: f32,
    pub after_pt: f32,
    pub ensure_extra_pt: f32,
    pub marker_text_gap_pt: f32,
    pub checkbox_x_pt: f32,
    pub checkbox_y_pt: f32,
    pub checkbox_size_pt: f32,
    pub checkbox_thickness_pt: f32,
    pub check_thickness_pt: f32,
    pub check_start_x_pt: f32,
    pub check_start_y_pt: f32,
    pub check_mid_x_pt: f32,
    pub check_mid_y_pt: f32,
    pub check_end_x_pt: f32,
    pub check_end_y_pt: f32,
    pub bullet_diameter_pt: f32,
    pub ordered_size_multiplier: f32,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            item_gap_pt: 4.5,
            after_pt: 6.0,
            ensure_extra_pt: 8.0,
            marker_text_gap_pt: 8.0,
            checkbox_x_pt: 1.5,
            checkbox_y_pt: 3.7,
            checkbox_size_pt: 7.6,
            checkbox_thickness_pt: 0.8,
            check_thickness_pt: 1.0,
            check_start_x_pt: 1.6,
            check_start_y_pt: 4.0,
            check_mid_x_pt: 3.3,
            check_mid_y_pt: 5.9,
            check_end_x_pt: 6.3,
            check_end_y_pt: 1.8,
            bullet_diameter_pt: 3.6,
            ordered_size_multiplier: 0.86,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FootnoteOptions {
    pub line_height_pt: f32,
    pub label_size_pt: f32,
    pub label_gap_pt: f32,
    pub body_size_pt: f32,
    pub block_extra_height_pt: f32,
    pub ensure_extra_pt: f32,
}

impl Default for FootnoteOptions {
    fn default() -> Self {
        Self {
            line_height_pt: 12.2,
            label_size_pt: 7.8,
            label_gap_pt: 14.0,
            body_size_pt: 8.9,
            block_extra_height_pt: 4.0,
            ensure_extra_pt: 4.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TableOptions {
    pub cell_pad_x_pt: f32,
    pub cell_pad_y_pt: f32,
    pub border_thickness_pt: f32,
    pub after_pt: f32,
    pub initial_keep_height_pt: f32,
    pub row_ensure_extra_pt: f32,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            cell_pad_x_pt: 7.0,
            cell_pad_y_pt: 7.0,
            border_thickness_pt: 0.45,
            after_pt: 16.0,
            initial_keep_height_pt: 20.0,
            row_ensure_extra_pt: 2.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ImageOptions {
    pub px_to_pt: f32,
    pub border_outset_pt: f32,
    pub caption_line_height_multiplier: f32,
    pub block_after_pt: f32,
    pub placeholder_height_pt: f32,
    pub placeholder_after_pt: f32,
    pub placeholder_label_x_pt: f32,
    pub placeholder_label_y_pt: f32,
    pub placeholder_label_size_pt: f32,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            px_to_pt: 0.75,
            border_outset_pt: 1.0,
            caption_line_height_multiplier: 1.52,
            block_after_pt: 12.0,
            placeholder_height_pt: 96.0,
            placeholder_after_pt: 14.0,
            placeholder_label_x_pt: 18.0,
            placeholder_label_y_pt: 40.0,
            placeholder_label_size_pt: 10.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct InlineOptions {
    pub code_size_multiplier: f32,
    pub code_pad_x_pt: f32,
    pub footnote_ref_size_multiplier: f32,
    pub footnote_ref_shift_multiplier: f32,
    pub citation_size_multiplier: f32,
    pub link_hit_padding_pt: f32,
    pub link_hit_extra_height_pt: f32,
    pub underline_offset_multiplier: f32,
    pub strike_offset_multiplier: f32,
    pub decoration_thickness_pt: f32,
    pub inline_bg_height_extra_pt: f32,
    pub inline_bg_line_height_inset_pt: f32,
    pub inline_bg_y_offset_pt: f32,
    pub serif_decoration_width_multiplier: f32,
    pub sans_decoration_width_multiplier: f32,
    pub mono_decoration_width_multiplier: f32,
}

impl Default for InlineOptions {
    fn default() -> Self {
        Self {
            code_size_multiplier: 0.94,
            code_pad_x_pt: 0.0,
            footnote_ref_size_multiplier: 0.58,
            footnote_ref_shift_multiplier: -0.28,
            citation_size_multiplier: 0.96,
            link_hit_padding_pt: 1.5,
            link_hit_extra_height_pt: 3.0,
            underline_offset_multiplier: 0.12,
            strike_offset_multiplier: -0.30,
            decoration_thickness_pt: 0.45,
            inline_bg_height_extra_pt: 3.4,
            inline_bg_line_height_inset_pt: 2.0,
            inline_bg_y_offset_pt: -1.2,
            serif_decoration_width_multiplier: 0.92,
            sans_decoration_width_multiplier: 0.96,
            mono_decoration_width_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FooterOptions {
    pub bottom_offset_pt: f32,
    pub size_pt: f32,
}

impl Default for FooterOptions {
    fn default() -> Self {
        Self {
            bottom_offset_pt: 34.0,
            size_pt: 8.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PaginationOptions {
    pub new_page_top_tolerance_pt: f32,
    pub image_keep_height_pt: f32,
    pub table_keep_base_pt: f32,
    pub table_keep_row_pt: f32,
    pub table_keep_max_rows: usize,
    pub code_keep_base_pt: f32,
    pub code_keep_min_lines: usize,
    pub code_keep_max_lines: usize,
    pub math_keep_height_pt: f32,
    pub quote_keep_height_pt: f32,
    pub list_keep_base_pt: f32,
    pub list_keep_item_pt: f32,
    pub list_keep_max_items: usize,
    pub paragraph_keep_height_pt: f32,
    pub divider_keep_height_pt: f32,
    pub heading_keep_height_pt: f32,
    pub footnote_keep_height_pt: f32,
}

impl Default for PaginationOptions {
    fn default() -> Self {
        Self {
            new_page_top_tolerance_pt: 1.0,
            image_keep_height_pt: 285.0,
            table_keep_base_pt: 70.0,
            table_keep_row_pt: 34.0,
            table_keep_max_rows: 2,
            code_keep_base_pt: 34.0,
            code_keep_min_lines: 4,
            code_keep_max_lines: 12,
            math_keep_height_pt: 72.0,
            quote_keep_height_pt: 76.0,
            list_keep_base_pt: 30.0,
            list_keep_item_pt: 24.0,
            list_keep_max_items: 3,
            paragraph_keep_height_pt: 62.0,
            divider_keep_height_pt: 32.0,
            heading_keep_height_pt: 82.0,
            footnote_keep_height_pt: 24.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    link: Option<String>,
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
    baseline: f32,
}

struct MathFormPdf {
    bytes: Vec<u8>,
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
    let bytes = renderer.finish()?;
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
    math_forms: HashMap<String, MathFormPdf>,
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
            math_forms: HashMap::new(),
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
                .map(|block| self.estimated_keep_height(block))
                .unwrap_or(0.0);
            self.render_block(block, idx == 0, next_keep)?;
        }
        Ok(())
    }

    fn estimated_keep_height(&self, block: &Block) -> f32 {
        let cfg = &self.options.pagination;
        match block {
            Block::Image { .. } => cfg.image_keep_height_pt,
            Block::Table(table) => {
                cfg.table_keep_base_pt
                    + table.rows.len().min(cfg.table_keep_max_rows) as f32 * cfg.table_keep_row_pt
            }
            Block::CodeBlock { text, .. } => {
                let lines = text
                    .lines()
                    .count()
                    .clamp(cfg.code_keep_min_lines, cfg.code_keep_max_lines)
                    as f32;
                cfg.code_keep_base_pt + lines * self.options.code_line_height_pt
            }
            Block::MathBlock(_) => cfg.math_keep_height_pt,
            Block::Quote { .. } => cfg.quote_keep_height_pt,
            Block::List { items, .. } => {
                cfg.list_keep_base_pt
                    + items.len().min(cfg.list_keep_max_items) as f32 * cfg.list_keep_item_pt
            }
            Block::Paragraph(_) => cfg.paragraph_keep_height_pt,
            Block::Divider => cfg.divider_keep_height_pt,
            Block::Heading { .. } => cfg.heading_keep_height_pt,
            Block::Footnote { .. } => cfg.footnote_keep_height_pt,
        }
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

    fn finish(mut self) -> Result<Vec<u8>> {
        self.push_page();
        let bytes = self
            .doc
            .with_pages(self.pages)
            .save(&PdfSaveOptions::default(), &mut Vec::new());
        if self.math_forms.is_empty() {
            Ok(bytes)
        } else {
            replace_math_xobjects(bytes, &self.math_forms)
        }
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
            1 => heading_metrics(self.options.headings.level1, first),
            2 => heading_metrics(self.options.headings.level2, first),
            3 => heading_metrics(self.options.headings.level3, first),
            _ => heading_metrics(self.options.headings.other, first),
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
            next_keep.max(self.options.headings.keep_with_next_level1_to_3_pt)
        } else {
            next_keep.max(self.options.headings.keep_with_next_other_pt)
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
        self.ensure_height(height + self.options.paragraph.after_pt);
        self.draw_lines(
            &lines,
            self.options.margin_x_pt,
            self.cursor_y,
            self.options.body_line_height_pt,
        );
        self.cursor_y += height + self.options.paragraph.after_pt;
        Ok(())
    }

    fn code_block(&mut self, lang: Option<&str>, text: &str) -> Result<()> {
        let cfg = self.options.code_block.clone();
        let pad_x = cfg.pad_x_pt;
        let pad_y = cfg.pad_y_pt;
        let title_h = if lang.is_some() {
            cfg.title_height_pt
        } else {
            0.0
        };
        let lines = self.highlighted_code_lines(lang, text);
        let block_h =
            pad_y * 2.0 + title_h + lines.len().max(1) as f32 * self.options.code_line_height_pt;
        self.ensure_height(block_h + cfg.after_pt);

        let x = self.options.margin_x_pt;
        let y = self.cursor_y;
        let accent = code_accent(lang);
        self.rect(x, y, self.content_width(), block_h, (0.965, 0.972, 0.982));
        self.draw_vertical_rule(
            x + cfg.rule_x_offset_pt,
            y + cfg.rule_y_inset_pt,
            block_h - cfg.rule_total_y_inset_pt,
            cfg.rule_thickness_pt,
            accent,
        );

        if let Some(lang) = lang {
            self.text(
                x + pad_x,
                y + pad_y + cfg.title_y_offset_pt,
                FontFace::SansBold,
                cfg.title_size_pt,
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
        self.cursor_y += block_h + cfg.after_pt;
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
        let cfg = self.options.math_block.clone();
        let math = self.math(
            tex.trim(),
            true,
            self.options.body_size_pt * cfg.size_multiplier,
        )?;
        let block_h = math.height.max(cfg.min_math_height_pt) + cfg.vertical_padding_pt;
        self.ensure_height(block_h + cfg.after_pt);

        let x = self.options.margin_x_pt + cfg.x_inset_pt;
        let y = self.cursor_y;
        let w = self.content_width() - cfg.content_width_inset_pt;
        self.rect(x, y, w, block_h, (0.955, 0.971, 0.988));
        let start = x + (w - math.width) / 2.0;
        self.draw_math(&math, start, y + cfg.draw_y_inset_pt);
        self.cursor_y += block_h + cfg.after_pt;
        Ok(())
    }

    fn divider(&mut self) -> Result<()> {
        let cfg = self.options.divider.clone();
        self.ensure_height(cfg.keep_height_pt);
        self.cursor_y += cfg.space_before_pt;
        self.draw_rule_at(
            self.options.margin_x_pt + cfg.x_inset_pt,
            self.cursor_y,
            self.content_width() - cfg.x_inset_pt * 2.0,
            cfg.thickness_pt,
            (0.78, 0.81, 0.86),
        );
        self.cursor_y += cfg.space_after_pt;
        Ok(())
    }

    fn quote(&mut self, kind: QuoteKind, content: &[Inline]) -> Result<()> {
        let (title, accent, fill, text_color) = quote_palette(kind);
        let x = self.options.margin_x_pt;
        let cfg = self.options.quote.clone();
        let pad_x = cfg.pad_x_pt;
        let pad_y = cfg.pad_y_pt;
        let title_h = if title.is_some() {
            cfg.title_height_pt
        } else {
            0.0
        };
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
        self.ensure_height(block_h + cfg.after_pt);

        let y = self.cursor_y;
        self.rect(x, y, self.content_width(), block_h, fill);
        self.draw_vertical_rule(
            x + cfg.rule_x_offset_pt,
            y + cfg.rule_y_inset_pt,
            block_h - cfg.rule_total_y_inset_pt,
            cfg.rule_thickness_pt,
            accent,
        );
        if let Some(title) = title {
            self.text(
                x + pad_x,
                y + pad_y + cfg.title_y_offset_pt,
                FontFace::SansBold,
                cfg.title_size_pt,
                accent,
                title,
            );
        }
        self.draw_lines(&lines, x + pad_x, y + pad_y + title_h, line_h);
        self.cursor_y += block_h + cfg.after_pt;
        Ok(())
    }

    fn list(&mut self, ordered: bool, start: u64, items: &[ListItem]) -> Result<()> {
        let marker_w = list_marker_column_width(
            ordered,
            start,
            items,
            self.options.body_size_pt,
            &self.options.list,
        );
        let item_gap = self.options.list.item_gap_pt;
        let line_h = self.options.body_line_height_pt;
        let style = self.body_style();
        self.ensure_height(line_h + self.options.list.ensure_extra_pt + item_gap);

        for (idx, item) in items.iter().enumerate() {
            if item.gap_before {
                self.cursor_y += self.options.list.after_pt;
            }
            let lines = self.wrap_inlines(&item.content, style, self.content_width() - marker_w)?;
            let item_h = lines.len().max(1) as f32 * line_h;
            self.ensure_height(item_h + item_gap + self.options.list.ensure_extra_pt);
            let y = self.cursor_y;
            self.draw_list_marker(
                ordered,
                start + idx as u64,
                item.checked,
                marker_w,
                self.options.margin_x_pt,
                y,
            );
            self.draw_lines(&lines, self.options.margin_x_pt + marker_w, y, line_h);
            self.cursor_y += item_h + item_gap;
        }
        self.cursor_y += self.options.list.after_pt;
        Ok(())
    }

    fn footnote(&mut self, label: &str, content: &[Inline]) -> Result<()> {
        let cfg = self.options.footnote.clone();
        let line_h = cfg.line_height_pt;
        let number = self.footnote_number(label);
        let label = format!("{number}.");
        let label_w = measure(&label, FontFace::SansBold, cfg.label_size_pt) + cfg.label_gap_pt;
        let style = Style {
            font: FontFace::Serif,
            size: cfg.body_size_pt,
            color: (0.28, 0.31, 0.36),
            bg: None,
            atomic: false,
            underline: false,
            strike: false,
            shift_y: 0.0,
            pad_x: 0.0,
        };
        let lines = self.wrap_inlines(content, style, self.content_width() - label_w)?;
        let block_h = lines.len().max(1) as f32 * line_h + cfg.block_extra_height_pt;
        self.ensure_height(block_h + cfg.ensure_extra_pt);

        let y = self.cursor_y;
        let baseline_y = text_baseline_y(y, line_h, style.font, style.size);
        self.text_at_baseline(
            self.options.margin_x_pt,
            baseline_y,
            FontFace::SansBold,
            cfg.label_size_pt,
            (0.42, 0.47, 0.54),
            &label,
        );
        self.draw_lines(&lines, self.options.margin_x_pt + label_w, y, line_h);
        self.cursor_y += block_h;
        Ok(())
    }

    fn table(&mut self, table: &crate::ir::Table) -> Result<()> {
        let cfg = self.options.table.clone();
        let cols = table
            .header
            .len()
            .max(table.rows.iter().map(Vec::len).max().unwrap_or(0))
            .max(1);
        let col_w = self.content_width() / cols as f32;
        let cell_pad_x = cfg.cell_pad_x_pt;
        let cell_pad_y = cfg.cell_pad_y_pt;
        let mut rows = Vec::new();

        if !table.header.is_empty() {
            rows.push((true, table.header.clone()));
        }
        rows.extend(table.rows.iter().cloned().map(|row| (false, row)));

        self.ensure_height(cfg.initial_keep_height_pt);
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

            self.ensure_height(row_h + cfg.row_ensure_extra_pt);
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
                self.draw_vertical_rule(
                    x,
                    row_y,
                    row_h,
                    cfg.border_thickness_pt,
                    (0.78, 0.81, 0.86),
                );
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
                cfg.border_thickness_pt,
                (0.78, 0.81, 0.86),
            );
            self.draw_rule_at(
                self.options.margin_x_pt,
                row_y,
                self.content_width(),
                cfg.border_thickness_pt,
                (0.78, 0.81, 0.86),
            );
            self.draw_rule_at(
                self.options.margin_x_pt,
                row_y + row_h,
                self.content_width(),
                cfg.border_thickness_pt,
                (0.78, 0.81, 0.86),
            );
            self.cursor_y += row_h;
        }
        self.cursor_y += cfg.after_pt;
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
        let cfg = self.options.image.clone();
        let natural_w = raw.width as f32 * cfg.px_to_pt;
        let natural_h = raw.height as f32 * cfg.px_to_pt;
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
            self.options.image_caption_size_pt * cfg.caption_line_height_multiplier
        } else {
            0.0
        };
        let block_h = draw_h + caption_gap + caption_h + cfg.block_after_pt;
        self.ensure_height(block_h);

        let x = self.options.margin_x_pt + (self.content_width() - draw_w) / 2.0;
        let y = self.cursor_y;
        self.rect(
            x - cfg.border_outset_pt,
            y - cfg.border_outset_pt,
            draw_w + cfg.border_outset_pt * 2.0,
            draw_h + cfg.border_outset_pt * 2.0,
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
        let cfg = self.options.image.clone();
        let h = cfg.placeholder_height_pt;
        self.ensure_height(h + cfg.placeholder_after_pt);
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
            self.options.margin_x_pt + cfg.placeholder_label_x_pt,
            y + cfg.placeholder_label_y_pt,
            FontFace::SansBold,
            cfg.placeholder_label_size_pt,
            (0.42, 0.47, 0.54),
            label,
        );
        self.cursor_y += h + cfg.placeholder_after_pt;
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
            && self.cursor_y
                > self.options.margin_top_pt + self.options.pagination.new_page_top_tolerance_pt
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
        let text = format!("Page {}", self.page_number);
        let size = self.options.footer.size_pt;
        let x = self.options.page_width_pt
            - self.options.margin_x_pt
            - measure(&text, FontFace::Sans, size);
        self.text(
            x,
            self.options.page_height_pt - self.options.footer.bottom_offset_pt,
            FontFace::Sans,
            size,
            (0.54, 0.58, 0.64),
            &text,
        );
    }

    fn draw_lines(&mut self, lines: &[LayoutLine], x: f32, y: f32, line_height: f32) {
        let mut line_y = y;
        for line in lines {
            let mut frag_x = x;
            let baseline_y = line_baseline_y(line, line_y, line_height);
            for frag in &line.fragments {
                if let Some(bg) = frag.style.bg {
                    let bg_h = (frag.style.size + self.options.inline.inline_bg_height_extra_pt)
                        .min(line_height - self.options.inline.inline_bg_line_height_inset_pt);
                    let bg_y = baseline_y - text_ascent(frag.style.font, frag.style.size)
                        + self.options.inline.inline_bg_y_offset_pt;
                    self.rect(frag_x, bg_y, frag.width + frag.style.pad_x * 2.0, bg_h, bg);
                }
                let text_x = frag_x + frag.style.pad_x;
                if let Some(href) = &frag.link {
                    let hit_h = link_hit_height(frag, &self.options.inline);
                    self.link_annotation(
                        text_x,
                        baseline_y - link_hit_ascent(frag, &self.options.inline),
                        decoration_width(frag, &self.options.inline),
                        hit_h,
                        href,
                    );
                }
                match &frag.kind {
                    FragmentKind::Text(text) => {
                        let text_baseline_y = baseline_y + frag.style.shift_y;
                        self.text_at_baseline(
                            text_x,
                            text_baseline_y,
                            frag.style.font,
                            frag.style.size,
                            frag.style.color,
                            text,
                        );
                        if frag.style.underline {
                            self.draw_rule_at(
                                text_x,
                                text_baseline_y
                                    + frag.style.size
                                        * self.options.inline.underline_offset_multiplier,
                                decoration_width(frag, &self.options.inline),
                                self.options.inline.decoration_thickness_pt,
                                frag.style.color,
                            );
                        }
                        if frag.style.strike {
                            self.draw_rule_at(
                                text_x,
                                text_baseline_y
                                    + frag.style.size
                                        * self.options.inline.strike_offset_multiplier,
                                decoration_width(frag, &self.options.inline),
                                self.options.inline.decoration_thickness_pt,
                                frag.style.color,
                            );
                        }
                    }
                    FragmentKind::Math(math) => {
                        self.draw_math(math, text_x, baseline_y - math.baseline)
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

    fn text_at_baseline(
        &mut self,
        x: f32,
        baseline_y: f32,
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
                    y: Pt(self.options.page_height_pt - baseline_y),
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

    fn link_annotation(&mut self, x: f32, y_top: f32, w: f32, h: f32, href: &str) {
        if href.trim().is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }
        self.ops.push(Op::LinkAnnotation {
            link: LinkAnnotation::new(
                Rect::from_xywh(
                    Pt(x),
                    Pt(self.options.page_height_pt - y_top - h),
                    Pt(w),
                    Pt(h),
                ),
                Actions::uri(href.to_string()),
                Some(BorderArray::Solid([0.0, 0.0, 0.0])),
                Some(ColorArray::Transparent),
                Some(HighlightingMode::None),
            ),
        });
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
        self.draw_line_segment_with_cap(
            x1,
            y1_top,
            x2,
            y2_top,
            thickness,
            color,
            LineCapStyle::Butt,
        );
    }

    fn draw_line_segment_with_cap(
        &mut self,
        x1: f32,
        y1_top: f32,
        x2: f32,
        y2_top: f32,
        thickness: f32,
        color: (f32, f32, f32),
        cap: LineCapStyle,
    ) {
        let y1 = self.options.page_height_pt - y1_top;
        let y2 = self.options.page_height_pt - y2_top;
        self.ops.extend_from_slice(&[
            Op::SetOutlineColor { col: rgb(color) },
            Op::SetOutlineThickness { pt: Pt(thickness) },
            Op::SetLineCapStyle { cap },
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
        marker_w: f32,
        x: f32,
        y: f32,
    ) {
        let color = (0.32, 0.38, 0.46);
        let cfg = self.options.list.clone();
        if let Some(checked) = checked {
            let box_x = x + cfg.checkbox_x_pt;
            let box_y = y + cfg.checkbox_y_pt;
            self.rect_outline(
                box_x,
                box_y,
                cfg.checkbox_size_pt,
                cfg.checkbox_size_pt,
                cfg.checkbox_thickness_pt,
                color,
            );
            if checked {
                self.draw_line_segment(
                    box_x + cfg.check_start_x_pt,
                    box_y + cfg.check_start_y_pt,
                    box_x + cfg.check_mid_x_pt,
                    box_y + cfg.check_mid_y_pt,
                    cfg.check_thickness_pt,
                    color,
                );
                self.draw_line_segment(
                    box_x + cfg.check_mid_x_pt,
                    box_y + cfg.check_mid_y_pt,
                    box_x + cfg.check_end_x_pt,
                    box_y + cfg.check_end_y_pt,
                    cfg.check_thickness_pt,
                    color,
                );
            }
        } else if ordered {
            let marker = format!("{number}.");
            let size = self.options.body_size_pt * cfg.ordered_size_multiplier;
            let width = measure(&marker, FontFace::SansBold, size);
            let baseline_y = text_baseline_y(
                y,
                self.options.body_line_height_pt,
                FontFace::Serif,
                self.options.body_size_pt,
            );
            self.text_at_baseline(
                x + marker_w - cfg.marker_text_gap_pt - width,
                baseline_y,
                FontFace::SansBold,
                size,
                color,
                &marker,
            );
        } else {
            self.draw_round_dot(
                x + cfg.checkbox_x_pt + cfg.checkbox_size_pt / 2.0,
                y + cfg.checkbox_y_pt + cfg.checkbox_size_pt / 2.0,
                cfg.bullet_diameter_pt,
                color,
            );
        }
    }

    fn draw_round_dot(&mut self, x: f32, y_top: f32, diameter: f32, color: (f32, f32, f32)) {
        self.draw_line_segment_with_cap(
            x,
            y_top,
            x + 0.01,
            y_top,
            diameter,
            color,
            LineCapStyle::Round,
        );
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
                        size: base.size * self.options.inline.code_size_multiplier,
                        color: (0.05, 0.11, 0.18),
                        bg: Some((0.935, 0.945, 0.958)),
                        atomic: true,
                        underline: false,
                        strike: false,
                        shift_y: 0.0,
                        pad_x: self.options.inline.code_pad_x_pt,
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
                            pad_x: 0.0,
                        },
                        link: None,
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
                    let mut link_style = base;
                    link_style.color = (0.02, 0.28, 0.62);
                    link_style.underline = true;
                    let start = spans.len();
                    self.push_inline_spans(content, link_style, spans)?;
                    for fragment in &mut spans[start..] {
                        fragment.link = Some(href.clone());
                    }
                }
                Inline::FootnoteRef(label) => spans.push(text_fragment(
                    self.footnote_number(label).to_string(),
                    Style {
                        font: FontFace::SansBold,
                        size: base.size * self.options.inline.footnote_ref_size_multiplier,
                        color: (0.02, 0.28, 0.62),
                        bg: None,
                        atomic: true,
                        underline: false,
                        strike: false,
                        shift_y: base.size * self.options.inline.footnote_ref_shift_multiplier,
                        pad_x: 0.0,
                    },
                )),
                Inline::Citation(key) => {
                    let text = format!("({})", citation_label(key));
                    let style = Style {
                        font: FontFace::SerifItalic,
                        size: base.size * self.options.inline.citation_size_multiplier,
                        color: (0.30, 0.28, 0.40),
                        bg: None,
                        atomic: true,
                        underline: false,
                        strike: false,
                        shift_y: 0.0,
                        pad_x: 0.0,
                    };
                    spans.push(Fragment {
                        width: measure(&text, style.font, style.size),
                        kind: FragmentKind::Text(text),
                        style,
                        link: None,
                    });
                }
            }
        }
        Ok(())
    }

    fn math(&mut self, tex: &str, display: bool, size: f32) -> Result<RenderedMath> {
        match self.options.math_mode {
            MathMode::Katex => self.render_math_katex(tex, display, size),
            MathMode::Lualatex | MathMode::Latex => self.render_math_lualatex(tex, display),
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
                baseline: size,
            };
        }
        RenderedMath {
            id: XObjectId::new(),
            width: 0.0,
            height: 0.0,
            baseline: 0.0,
        }
    }

    fn render_math_katex(&mut self, tex: &str, display: bool, size: f32) -> Result<RenderedMath> {
        // Native builds keep the high-fidelity path until the shared KaTeX WASM
        // layout engine is wired in. Browser builds will replace this backend.
        self.render_math_lualatex(tex, display)
            .or_else(|_| Ok(self.math_fallback(tex, size)))
    }

    fn render_math_lualatex(&mut self, tex: &str, display: bool) -> Result<RenderedMath> {
        let key = format!("{}:{}", if display { "display" } else { "inline" }, tex);
        if let Some(rendered) = self.math_cache.get(&key) {
            return Ok(rendered.clone());
        }

        let rendered = self
            .render_math_pdf_form(tex, display)
            .or_else(|_| self.render_math_svg(tex, display))?;
        self.math_cache.insert(key, rendered.clone());
        Ok(rendered)
    }

    fn render_math_pdf_form(&mut self, tex: &str, display: bool) -> Result<RenderedMath> {
        let pdf = tex_to_pdf(tex, display)?;
        let width = pdf.width.unwrap_or(80.0).max(1.0);
        let height = pdf.height.unwrap_or(18.0).max(1.0);
        let baseline = pdf.baseline.unwrap_or(height * 0.78);
        let svg = pdf
            .fallback_svg
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("TeX SVG fallback was not generated"))?;
        let mut warnings = Vec::new();
        let mut external = Svg::parse(svg, &mut warnings)
            .map_err(|err| anyhow::anyhow!("failed to parse TeX SVG placeholder: {err}"))?;
        let marker = XObjectId::new().0;
        external.stream.dict.insert(
            "NativeMdPdfMath".to_string(),
            DictItem::String {
                data: marker.as_bytes().to_vec(),
                literal: true,
            },
        );
        let id = self.doc.add_xobject(&external);
        self.math_forms
            .insert(marker, MathFormPdf { bytes: pdf.bytes });
        Ok(RenderedMath {
            id,
            width,
            height,
            baseline: if display { height } else { baseline },
        })
    }

    fn render_math_svg(&mut self, tex: &str, display: bool) -> Result<RenderedMath> {
        let svg = tex_to_svg(tex, display)?;
        let (width, height) = svg_size_pt(&svg).unwrap_or((80.0, 18.0));
        let baseline = svg_baseline_pt(&svg).unwrap_or(height * 0.78);
        let mut warnings = Vec::new();
        let external = Svg::parse(&svg, &mut warnings)
            .map_err(|err| anyhow::anyhow!("failed to parse TeX SVG: {err}"))?;
        let id = self.doc.add_xobject(&external);
        Ok(RenderedMath {
            id,
            width,
            height,
            baseline: if display { height } else { baseline },
        })
    }

    fn draw_math(&mut self, math: &RenderedMath, x: f32, y_top: f32) {
        let y = self.options.page_height_pt - y_top - math.height;
        self.ops.push(Op::SetFillColor {
            col: rgb((0.10, 0.12, 0.16)),
        });
        self.ops.push(Op::SetOutlineColor {
            col: rgb((0.10, 0.12, 0.16)),
        });
        self.ops.push(Op::UseXobject {
            id: math.id.clone(),
            transform: XObjectTransform {
                translate_x: Some(Pt(x)),
                translate_y: Some(Pt(y)),
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
        link: None,
    }
}

fn heading_metrics(level: HeadingLevelOptions, first: bool) -> (f32, f32, f32, f32) {
    (
        level.size_pt,
        level.line_height_pt,
        if first {
            level.first_space_before_pt
        } else {
            level.space_before_pt
        },
        level.space_after_pt,
    )
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
                    let mut frag = text_fragment(token, span.style);
                    frag.link.clone_from(&span.link);
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
                let mut piece = text_fragment(ch.to_string(), fragment.style);
                piece.link.clone_from(&fragment.link);
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
            if same_style(last.style, frag.style) && last.link == frag.link {
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
    let metrics = metric_font(font);
    text.chars()
        .map(|ch| {
            if ch == '\t' {
                metrics.advance(' ') * 4.0
            } else {
                metrics.advance(ch)
            }
        })
        .sum::<f32>()
        * size
        / metrics.units_per_em
}

fn text_ascent(font: FontFace, size: f32) -> f32 {
    let metrics = metric_font(font);
    metrics.ascent * size / metrics.units_per_em
}

fn text_descent(font: FontFace, size: f32) -> f32 {
    let metrics = metric_font(font);
    metrics.descent * size / metrics.units_per_em
}

fn fragment_ascent(fragment: &Fragment) -> f32 {
    match &fragment.kind {
        FragmentKind::Text(_) => text_ascent(fragment.style.font, fragment.style.size),
        FragmentKind::Math(math) => math.baseline,
    }
}

fn fragment_descent(fragment: &Fragment) -> f32 {
    match &fragment.kind {
        FragmentKind::Text(_) => text_descent(fragment.style.font, fragment.style.size),
        FragmentKind::Math(math) => (math.height - math.baseline).max(0.0),
    }
}

fn link_hit_ascent(fragment: &Fragment, inline: &InlineOptions) -> f32 {
    match &fragment.kind {
        FragmentKind::Text(_) => {
            text_ascent(fragment.style.font, fragment.style.size) + inline.link_hit_padding_pt
        }
        FragmentKind::Math(math) => math.baseline + inline.link_hit_padding_pt,
    }
}

fn link_hit_height(fragment: &Fragment, inline: &InlineOptions) -> f32 {
    match &fragment.kind {
        FragmentKind::Text(_) => {
            text_ascent(fragment.style.font, fragment.style.size)
                + text_descent(fragment.style.font, fragment.style.size)
                + inline.link_hit_extra_height_pt
        }
        FragmentKind::Math(math) => math.height + inline.link_hit_extra_height_pt,
    }
}

fn line_baseline_y(line: &LayoutLine, line_y: f32, line_height: f32) -> f32 {
    let ascent = line
        .fragments
        .iter()
        .map(fragment_ascent)
        .fold(0.0, f32::max);
    let descent = line
        .fragments
        .iter()
        .map(fragment_descent)
        .fold(0.0, f32::max);
    line_y + (line_height - ascent - descent).max(0.0) / 2.0 + ascent
}

fn text_baseline_y(line_y: f32, line_height: f32, font: FontFace, size: f32) -> f32 {
    let ascent = text_ascent(font, size);
    let descent = text_descent(font, size);
    line_y + (line_height - ascent - descent).max(0.0) / 2.0 + ascent
}

#[derive(Debug)]
struct MetricFont {
    units_per_em: f32,
    ascent: f32,
    descent: f32,
    fallback_advance: f32,
    advances: HashMap<char, f32>,
}

impl MetricFont {
    fn advance(&self, ch: char) -> f32 {
        self.advances
            .get(&ch)
            .copied()
            .unwrap_or(self.fallback_advance)
    }
}

fn metric_font(font: FontFace) -> &'static MetricFont {
    metric_fonts()
        .get(&font)
        .expect("all renderer fonts have metrics")
}

fn metric_fonts() -> &'static HashMap<FontFace, MetricFont> {
    static METRICS: OnceLock<HashMap<FontFace, MetricFont>> = OnceLock::new();
    METRICS.get_or_init(|| {
        let mut fonts = HashMap::new();
        for font in [
            FontFace::Serif,
            FontFace::SerifBold,
            FontFace::SerifItalic,
            FontFace::Sans,
            FontFace::SansBold,
            FontFace::SansItalic,
            FontFace::Mono,
            FontFace::MonoBold,
        ] {
            fonts.insert(font, build_metric_font(font));
        }
        fonts
    })
}

fn build_metric_font(font: FontFace) -> MetricFont {
    let afm = afm_data(font);
    let mut advances = HashMap::new();
    let mut ascent = 800.0;
    let mut descent = 200.0;

    for line in afm.lines() {
        if let Some(value) = line.strip_prefix("Ascender ") {
            ascent = value.trim().parse().unwrap_or(ascent);
        } else if let Some(value) = line.strip_prefix("Descender ") {
            let parsed: f32 = value.trim().parse().unwrap_or(-descent);
            descent = -parsed;
        } else if line.starts_with("C ") {
            let mut code = None;
            let mut width = None;
            for part in line.split(';') {
                let part = part.trim();
                if let Some(value) = part.strip_prefix("C ") {
                    code = value.trim().parse::<i32>().ok();
                } else if let Some(value) = part.strip_prefix("WX ") {
                    width = value.trim().parse::<f32>().ok();
                }
            }
            if let (Some(code), Some(width)) = (code, width) {
                if (0..=255).contains(&code) {
                    if let Some(ch) = char::from_u32(code as u32) {
                        advances.insert(ch, width);
                    }
                }
            }
        }
    }

    let fallback_advance = advances.get(&'?').copied().unwrap_or(500.0);
    MetricFont {
        units_per_em: 1000.0,
        ascent,
        descent,
        fallback_advance,
        advances,
    }
}

fn afm_data(font: FontFace) -> &'static str {
    match font {
        FontFace::Serif => include_str!("../assets/afm/Times-Roman.afm"),
        FontFace::SerifBold => include_str!("../assets/afm/Times-Bold.afm"),
        FontFace::SerifItalic => include_str!("../assets/afm/Times-Italic.afm"),
        FontFace::Sans => include_str!("../assets/afm/Helvetica.afm"),
        FontFace::SansBold => include_str!("../assets/afm/Helvetica-Bold.afm"),
        FontFace::SansItalic => include_str!("../assets/afm/Helvetica-Oblique.afm"),
        FontFace::Mono => include_str!("../assets/afm/Courier.afm"),
        FontFace::MonoBold => include_str!("../assets/afm/Courier-Bold.afm"),
    }
}

fn fragment_advance(fragment: &Fragment) -> f32 {
    (fragment.width + fragment.style.pad_x * 2.0).max(0.0)
}

fn list_marker_column_width(
    ordered: bool,
    start: u64,
    items: &[ListItem],
    body_size: f32,
    list: &ListOptions,
) -> f32 {
    if ordered {
        let last = start + items.len().saturating_sub(1) as u64;
        let marker = format!("{last}.");
        measure(
            &marker,
            FontFace::SansBold,
            body_size * list.ordered_size_multiplier,
        ) + list.marker_text_gap_pt
    } else {
        list.checkbox_x_pt + list.checkbox_size_pt + list.marker_text_gap_pt
    }
}

fn decoration_width(fragment: &Fragment, inline: &InlineOptions) -> f32 {
    let FragmentKind::Text(text) = &fragment.kind else {
        return fragment.width;
    };
    let trimmed = text.trim_end();
    let width = measure(trimmed, fragment.style.font, fragment.style.size);

    match fragment.style.font {
        FontFace::Serif | FontFace::SerifBold | FontFace::SerifItalic => {
            width * inline.serif_decoration_width_multiplier
        }
        FontFace::Sans | FontFace::SansBold | FontFace::SansItalic => {
            width * inline.sans_decoration_width_multiplier
        }
        FontFace::Mono | FontFace::MonoBold => width * inline.mono_decoration_width_multiplier,
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

struct TeXPdf {
    bytes: Vec<u8>,
    width: Option<f32>,
    height: Option<f32>,
    baseline: Option<f32>,
    fallback_svg: Option<String>,
}

fn replace_math_xobjects(bytes: Vec<u8>, forms: &HashMap<String, MathFormPdf>) -> Result<Vec<u8>> {
    let mut doc = LoDocument::load_mem(&bytes).context("failed to reopen rendered PDF")?;
    let targets: Vec<_> = doc
        .objects
        .iter()
        .filter_map(|(id, object)| math_xobject_marker(object).map(|marker| (*id, marker)))
        .collect();

    for (target_id, marker) in targets {
        let Some(form) = forms.get(&marker) else {
            continue;
        };
        let stream = imported_math_form_stream(&mut doc, &form.bytes)?;
        doc.set_object(target_id, LoObject::Stream(stream));
    }

    let mut out = Vec::new();
    doc.save_to(&mut out)
        .context("failed to save PDF with selectable TeX forms")?;
    Ok(out)
}

fn math_xobject_marker(object: &LoObject) -> Option<String> {
    let stream = object.as_stream().ok()?;
    let marker = stream.dict.get(b"NativeMdPdfMath").ok()?;
    match marker {
        LoObject::String(bytes, _) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

fn imported_math_form_stream(dst: &mut LoDocument, bytes: &[u8]) -> Result<LoStream> {
    let src = LoDocument::load_mem(bytes).context("failed to load generated TeX PDF")?;
    let pages = src.get_pages();
    let page_id = *pages
        .values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("generated TeX PDF did not contain a page"))?;
    let page = src
        .get_dictionary(page_id)
        .context("failed to read generated TeX PDF page dictionary")?;
    let content = src
        .get_page_content(page_id)
        .context("failed to read generated TeX PDF page content")?;
    let (resource_dict, resource_ids) = src
        .get_page_resources(page_id)
        .context("failed to read generated TeX PDF page resources")?;
    let resources = if let Ok(resources) = page.get(b"Resources") {
        import_lopdf_object(
            &src,
            dst,
            resources,
            &mut HashMap::new(),
            &mut HashSet::new(),
        )?
    } else if let Some(resource_id) = resource_ids.first() {
        import_lopdf_object(
            &src,
            dst,
            &LoObject::Reference(*resource_id),
            &mut HashMap::new(),
            &mut HashSet::new(),
        )?
    } else if let Some(resources) = resource_dict {
        import_lopdf_object(
            &src,
            dst,
            &LoObject::Dictionary(resources.clone()),
            &mut HashMap::new(),
            &mut HashSet::new(),
        )?
    } else {
        LoObject::Dictionary(LoDictionary::new())
    };
    let bbox = page_box(page).unwrap_or([0.0, 0.0, 80.0, 18.0]);
    let width = (bbox[2] - bbox[0]).abs().max(1.0);
    let height = (bbox[3] - bbox[1]).abs().max(1.0);

    let mut dict = LoDictionary::new();
    dict.set("Type", LoObject::Name(b"XObject".to_vec()));
    dict.set("Subtype", LoObject::Name(b"Form".to_vec()));
    dict.set("FormType", LoObject::Integer(1));
    dict.set(
        "BBox",
        LoObject::Array(bbox.iter().copied().map(LoObject::Real).collect()),
    );
    dict.set(
        "Matrix",
        LoObject::Array(vec![
            LoObject::Real(1.0 / width),
            LoObject::Real(0.0),
            LoObject::Real(0.0),
            LoObject::Real(1.0 / height),
            LoObject::Real(0.0),
            LoObject::Real(0.0),
        ]),
    );
    dict.set("Resources", resources);

    Ok(LoStream::new(dict, content).with_compression(false))
}

fn import_lopdf_object(
    src: &LoDocument,
    dst: &mut LoDocument,
    object: &LoObject,
    imported: &mut HashMap<LoObjectId, LoObjectId>,
    seen: &mut HashSet<LoObjectId>,
) -> Result<LoObject> {
    Ok(match object {
        LoObject::Array(values) => LoObject::Array(
            values
                .iter()
                .map(|value| import_lopdf_object(src, dst, value, imported, seen))
                .collect::<Result<_>>()?,
        ),
        LoObject::Dictionary(dict) => {
            let mut out = LoDictionary::new();
            for (key, value) in dict.iter() {
                out.set(
                    key.clone(),
                    import_lopdf_object(src, dst, value, imported, seen)?,
                );
            }
            LoObject::Dictionary(out)
        }
        LoObject::Stream(stream) => {
            let mut dict = LoDictionary::new();
            for (key, value) in stream.dict.iter() {
                dict.set(
                    key.clone(),
                    import_lopdf_object(src, dst, value, imported, seen)?,
                );
            }
            LoObject::Stream(LoStream::new(dict, stream.content.clone()).with_compression(false))
        }
        LoObject::Reference(id) => {
            if let Some(new_id) = imported.get(id) {
                return Ok(LoObject::Reference(*new_id));
            }
            if !seen.insert(*id) {
                return Ok(LoObject::Null);
            }
            let new_id = dst.new_object_id();
            imported.insert(*id, new_id);
            let resolved = src
                .objects
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("missing referenced TeX PDF object"))?;
            let copied = import_lopdf_object(src, dst, resolved, imported, seen)?;
            dst.set_object(new_id, copied);
            seen.remove(id);
            LoObject::Reference(new_id)
        }
        other => other.clone(),
    })
}

fn page_box(page: &lopdf::Dictionary) -> Option<[f32; 4]> {
    let values = page.get(b"MediaBox").ok()?.as_array().ok()?;
    if values.len() != 4 {
        return None;
    }
    Some([
        lopdf_number(&values[0])?,
        lopdf_number(&values[1])?,
        lopdf_number(&values[2])?,
        lopdf_number(&values[3])?,
    ])
}

fn lopdf_number(object: &LoObject) -> Option<f32> {
    match object {
        LoObject::Integer(value) => Some(*value as f32),
        LoObject::Real(value) => Some(*value),
        _ => None,
    }
}

fn tex_to_pdf(tex: &str, display: bool) -> Result<TeXPdf> {
    let dir = math_temp_dir()?;
    let tex_path = dir.join("math.tex");
    fs::write(&tex_path, math_pdf_tex_document(tex, display))?;

    let lualatex = Command::new("lualatex")
        .current_dir(&dir)
        .args(["-interaction=nonstopmode", "-halt-on-error", "math.tex"])
        .output()
        .context("failed to run lualatex for selectable math rendering")?;
    if !lualatex.status.success() {
        return Err(anyhow::anyhow!(
            "lualatex failed: {}{}",
            String::from_utf8_lossy(&lualatex.stdout),
            String::from_utf8_lossy(&lualatex.stderr)
        ));
    }

    let pdf_path = dir.join("math.pdf");
    let bytes = fs::read(&pdf_path).context("failed to read generated math PDF")?;
    fs::write(&tex_path, math_dvi_tex_document(tex, display))?;
    let metrics = tex_svg_metrics(&dir).ok();
    let fallback_svg = fs::read_to_string(dir.join("math.svg")).ok();

    Ok(TeXPdf {
        bytes,
        width: metrics.as_ref().map(|m| m.0),
        height: metrics.as_ref().map(|m| m.1),
        baseline: metrics.as_ref().map(|m| m.2),
        fallback_svg,
    })
}

fn tex_to_svg(tex: &str, display: bool) -> Result<String> {
    let dir = math_temp_dir()?;
    let tex_path = dir.join("math.tex");
    fs::write(&tex_path, math_dvi_tex_document(tex, display))?;
    tex_svg_metrics(&dir)?;
    fs::read_to_string(dir.join("math.svg")).context("failed to read generated math SVG")
}

fn math_temp_dir() -> Result<std::path::PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("native-mdpdf-math-{nonce}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn math_pdf_tex_document(tex: &str, display: bool) -> String {
    let body = if display {
        format!("$\\displaystyle\n{tex}\n$")
    } else {
        format!("${tex}$")
    };
    format!(
        "\\documentclass[preview,border=0pt]{{standalone}}\n\\usepackage{{amsmath,amssymb,mathtools,bm}}\n\\usepackage{{unicode-math}}\n\\setmathfont{{Latin Modern Math}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn math_dvi_tex_document(tex: &str, display: bool) -> String {
    let body = math_tex_body(tex, display);
    format!(
        "\\documentclass[preview,border=0pt]{{standalone}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{amsmath,amssymb,mathtools,bm}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn math_tex_body(tex: &str, display: bool) -> String {
    let body = if display {
        format!("\\[\\displaystyle\n{tex}\n\\]")
    } else {
        format!("${tex}$")
    };
    body
}

fn tex_svg_metrics(dir: &Path) -> Result<(f32, f32, f32)> {
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

    let svg =
        fs::read_to_string(dir.join("math.svg")).context("failed to read generated math SVG")?;
    let (width, height) = svg_size_pt(&svg).unwrap_or((80.0, 18.0));
    let baseline = svg_baseline_pt(&svg).unwrap_or(height * 0.78);
    Ok((width, height, baseline))
}

fn svg_size_pt(svg: &str) -> Option<(f32, f32)> {
    Some((svg_attr_pt(svg, "width")?, svg_attr_pt(svg, "height")?))
}

fn svg_baseline_pt(svg: &str) -> Option<f32> {
    let values = svg_attr(svg, "viewBox")?;
    let mut parts = values.split_whitespace();
    let _x_min: f32 = parts.next()?.parse().ok()?;
    let y_min: f32 = parts.next()?.parse().ok()?;
    Some((-y_min).max(0.0))
}

fn svg_attr_pt(svg: &str, attr: &str) -> Option<f32> {
    svg_attr(svg, attr)?.trim_end_matches("pt").parse().ok()
}

fn svg_attr<'a>(svg: &'a str, attr: &str) -> Option<&'a str> {
    let needle = format!("{attr}='");
    let start = svg.find(&needle)? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[allow(dead_code)]
fn plain(inlines: &[Inline]) -> String {
    inlines_to_plain_text(inlines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_links_as_pdf_uri_annotations() {
        let doc = crate::parser::parse_markdown(
            "A [wrapped native PDF link](https://example.com/native-pdf) should be clickable.",
        );
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let output = std::env::temp_dir().join(format!("native-mdpdf-link-test-{nonce}.pdf"));
        let mut options = RenderOptions::default();
        options.math_mode = MathMode::Fallback;
        options.page_numbers = false;

        render_pdf_with_options(&doc, &output, Path::new("."), options).unwrap();
        let pdf = fs::read(&output).unwrap();
        let _ = fs::remove_file(&output);
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert!(pdf_text.contains("/Subtype/Link"));
        assert!(pdf_text.contains("/S/URI"));
        assert!(pdf_text.contains("https://example.com/native-pdf"));
    }

    #[test]
    fn blank_line_inside_list_matches_gap_between_list_blocks() {
        let doc = Document::new(vec![Block::List {
            ordered: false,
            start: 1,
            items: vec![
                ListItem {
                    checked: None,
                    gap_before: false,
                    content: vec![Inline::Text("Bullet item".to_string())],
                },
                ListItem {
                    checked: Some(true),
                    gap_before: true,
                    content: vec![Inline::Text("Task item".to_string())],
                },
            ],
        }]);
        let mut options = RenderOptions::default();
        options.page_numbers = false;
        options.margin_top_pt = 0.0;
        options.body_line_height_pt = 10.0;
        options.list.item_gap_pt = 2.0;
        options.list.after_pt = 6.0;
        options.list.ensure_extra_pt = 0.0;

        let mut renderer = Renderer::new(Path::new("."), options);
        renderer.render_document(&doc).unwrap();

        let expected = 10.0 + 2.0 + 6.0 + 10.0 + 2.0 + 6.0;
        assert!((renderer.cursor_y - expected).abs() < f32::EPSILON);
    }
}
