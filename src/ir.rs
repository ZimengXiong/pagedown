#[derive(Debug, Clone)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, content: Vec<Inline> },
    Paragraph(Vec<Inline>),
    CodeBlock { lang: Option<String>, text: String },
    MathBlock(String),
    Divider,
    Table(Table),
    Image { src: String, alt: String },
}

#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Code(String),
    Math(String),
    Link { href: String, content: Vec<Inline> },
}

#[derive(Debug, Clone)]
pub struct Table {
    pub alignments: Vec<Alignment>,
    pub header: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
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
            Inline::Link { content, .. } => content.iter().map(Inline::plain_text).collect(),
        }
    }
}

pub fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    inlines.iter().map(Inline::plain_text).collect()
}
