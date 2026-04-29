#[derive(Debug, Clone)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Title(Vec<Inline>),
    Subtitle(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    CodeBlock {
        lang: Option<String>,
        text: String,
    },
    MathBlock(String),
    Divider,
    Quote {
        kind: QuoteKind,
        content: Vec<Inline>,
    },
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Table(Table),
    Image {
        src: String,
        alt: String,
    },
    Footnote {
        label: String,
        content: Vec<Inline>,
    },
}

#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Code(String),
    Math(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link { href: String, content: Vec<Inline> },
    FootnoteRef(String),
    Citation(String),
}

#[derive(Debug, Clone)]
pub struct Table {
    pub alignments: Vec<Alignment>,
    pub header: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub gap_before: bool,
    pub content: Vec<Inline>,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteKind {
    Regular,
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
    None,
}

impl Document {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}

impl Inline {
    pub fn plain_text(&self) -> String {
        match self {
            Inline::Text(text) | Inline::Code(text) | Inline::Math(text) => text.clone(),
            Inline::Emphasis(content)
            | Inline::Strong(content)
            | Inline::Strikethrough(content) => content.iter().map(Inline::plain_text).collect(),
            Inline::Link { content, .. } => content.iter().map(Inline::plain_text).collect(),
            Inline::FootnoteRef(label) => format!("[^{label}]"),
            Inline::Citation(key) => format!("[@{key}]"),
        }
    }
}

pub fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    inlines.iter().map(Inline::plain_text).collect()
}
