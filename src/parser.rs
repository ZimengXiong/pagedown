use pulldown_cmark::{
    Alignment as MdAlignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};

use crate::ir::{Alignment, Block, Document, Inline, Table};

#[derive(Debug)]
enum ActiveBlock {
    Paragraph(Vec<Inline>),
    Heading { level: u8, content: Vec<Inline> },
    CodeBlock { lang: Option<String>, text: String },
}

#[derive(Debug)]
struct ActiveLink {
    href: String,
    content: Vec<Inline>,
}

#[derive(Debug)]
struct ActiveImage {
    src: String,
    alt: String,
}

#[derive(Debug)]
struct ActiveTable {
    alignments: Vec<Alignment>,
    header: Vec<Vec<Inline>>,
    rows: Vec<Vec<Vec<Inline>>>,
    current_row: Vec<Vec<Inline>>,
    current_cell: Vec<Inline>,
    in_header: bool,
    in_cell: bool,
}

pub fn parse_markdown(input: &str) -> Document {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_MATH);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(input, options);
    let mut blocks = Vec::new();
    let mut active_block: Option<ActiveBlock> = None;
    let mut active_link: Option<ActiveLink> = None;
    let mut active_image: Option<ActiveImage> = None;
    let mut active_table: Option<ActiveTable> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if active_table.as_ref().is_none_or(|table| !table.in_cell) {
                        active_block = Some(ActiveBlock::Paragraph(Vec::new()));
                    }
                }
                Tag::Heading { level, .. } => {
                    active_block = Some(ActiveBlock::Heading {
                        level: heading_level(level),
                        content: Vec::new(),
                    });
                }
                Tag::CodeBlock(kind) => {
                    active_block = Some(ActiveBlock::CodeBlock {
                        lang: code_lang(kind),
                        text: String::new(),
                    });
                }
                Tag::Link { dest_url, .. } => {
                    active_link = Some(ActiveLink {
                        href: dest_url.to_string(),
                        content: Vec::new(),
                    });
                }
                Tag::Image { dest_url, .. } => {
                    active_image = Some(ActiveImage {
                        src: dest_url.to_string(),
                        alt: String::new(),
                    });
                }
                Tag::Table(alignments) => {
                    active_table = Some(ActiveTable {
                        alignments: alignments.into_iter().map(map_alignment).collect(),
                        header: Vec::new(),
                        rows: Vec::new(),
                        current_row: Vec::new(),
                        current_cell: Vec::new(),
                        in_header: false,
                        in_cell: false,
                    });
                }
                Tag::TableHead => {
                    if let Some(table) = &mut active_table {
                        table.in_header = true;
                        table.current_row = Vec::new();
                    }
                }
                Tag::TableRow => {
                    if let Some(table) = &mut active_table {
                        table.current_row = Vec::new();
                    }
                }
                Tag::TableCell => {
                    if let Some(table) = &mut active_table {
                        table.current_cell = Vec::new();
                        table.in_cell = true;
                    }
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    if active_table.as_ref().is_some_and(|table| table.in_cell) {
                        continue;
                    }
                    if let Some(ActiveBlock::Paragraph(content)) = active_block.take() {
                        push_paragraph_or_math(&mut blocks, content);
                    }
                }
                TagEnd::Heading(_) => {
                    if let Some(ActiveBlock::Heading { level, content }) = active_block.take() {
                        blocks.push(Block::Heading { level, content });
                    }
                }
                TagEnd::CodeBlock => {
                    if let Some(ActiveBlock::CodeBlock { lang, text }) = active_block.take() {
                        blocks.push(Block::CodeBlock { lang, text });
                    }
                }
                TagEnd::Link => {
                    if let Some(link) = active_link.take() {
                        push_inline(
                            &mut active_block,
                            &mut active_table,
                            Inline::Link {
                                href: link.href,
                                content: link.content,
                            },
                        );
                    }
                }
                TagEnd::Image => {
                    if let Some(image) = active_image.take() {
                        blocks.push(Block::Image {
                            src: image.src,
                            alt: image.alt,
                        });
                    }
                }
                TagEnd::TableCell => {
                    if let Some(table) = &mut active_table {
                        table
                            .current_row
                            .push(std::mem::take(&mut table.current_cell));
                        table.in_cell = false;
                    }
                }
                TagEnd::TableRow => {
                    if let Some(table) = &mut active_table {
                        if table.in_header {
                            table.header = std::mem::take(&mut table.current_row);
                        } else {
                            table.rows.push(std::mem::take(&mut table.current_row));
                        }
                    }
                }
                TagEnd::TableHead => {
                    if let Some(table) = &mut active_table {
                        if !table.current_row.is_empty() {
                            table.header = std::mem::take(&mut table.current_row);
                        }
                        table.in_header = false;
                    }
                }
                TagEnd::Table => {
                    if let Some(table) = active_table.take() {
                        blocks.push(Block::Table(Table {
                            alignments: table.alignments,
                            header: table.header,
                            rows: table.rows,
                        }));
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if let Some(image) = &mut active_image {
                    image.alt.push_str(&text);
                } else if let Some(link) = &mut active_link {
                    link.content.push(Inline::Text(text.to_string()));
                } else if let Some(ActiveBlock::CodeBlock { text: code, .. }) = &mut active_block {
                    code.push_str(&text);
                } else {
                    push_inline(
                        &mut active_block,
                        &mut active_table,
                        Inline::Text(text.to_string()),
                    );
                }
            }
            Event::Code(text) => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    Inline::Code(text.to_string()),
                );
            }
            Event::InlineMath(text) => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    Inline::Math(text.to_string()),
                );
            }
            Event::DisplayMath(text) => {
                if active_block.is_none() && active_table.is_none() {
                    blocks.push(Block::MathBlock(text.to_string()));
                } else {
                    push_inline(
                        &mut active_block,
                        &mut active_table,
                        Inline::Math(text.to_string()),
                    );
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    Inline::Text(" ".to_string()),
                );
            }
            Event::Rule => blocks.push(Block::Divider),
            _ => {}
        }
    }

    Document::new(blocks)
}

fn push_inline(
    active_block: &mut Option<ActiveBlock>,
    active_table: &mut Option<ActiveTable>,
    inline: Inline,
) {
    if let Some(table) = active_table {
        if table.in_cell {
            table.current_cell.push(inline);
            return;
        }
    }

    match active_block {
        Some(ActiveBlock::Paragraph(content)) | Some(ActiveBlock::Heading { content, .. }) => {
            content.push(inline);
        }
        Some(ActiveBlock::CodeBlock { text, .. }) => {
            text.push_str(&inline.plain_text());
        }
        None => {
            *active_block = Some(ActiveBlock::Paragraph(vec![inline]));
        }
    }
}

fn push_paragraph_or_math(blocks: &mut Vec<Block>, content: Vec<Inline>) {
    let meaningful = content
        .iter()
        .filter(|inline| !matches!(inline, Inline::Text(text) if text.trim().is_empty()))
        .collect::<Vec<_>>();

    if meaningful.len() == 1 {
        if let Inline::Math(tex) = meaningful[0] {
            blocks.push(Block::MathBlock(tex.clone()));
            return;
        }
    }

    if !content.is_empty() {
        blocks.push(Block::Paragraph(content));
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

fn code_lang(kind: CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Indented => None,
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(ToOwned::to_owned),
    }
}

fn map_alignment(alignment: MdAlignment) -> Alignment {
    match alignment {
        MdAlignment::Left => Alignment::Left,
        MdAlignment::Center => Alignment::Center,
        MdAlignment::Right => Alignment::Right,
        MdAlignment::None => Alignment::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Inline;

    #[test]
    fn parses_supported_block_types() {
        let doc = parse_markdown(
            r#"# Title

Paragraph with `code`, $x^2$, and [link](https://example.com).

---

```rust
fn main() {}
```

$$
\frac{1}{3}
$$

![Alt text](image.png)
"#,
        );

        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph(_)));
        assert!(matches!(doc.blocks[2], Block::Divider));
        assert!(matches!(doc.blocks[3], Block::CodeBlock { .. }));
        assert!(matches!(doc.blocks[4], Block::MathBlock(_)));
        assert!(matches!(doc.blocks[5], Block::Image { .. }));
    }

    #[test]
    fn keeps_inline_code_math_and_links_distinct() {
        let doc = parse_markdown("Text `LayoutLine`, $a^2$, [Rust](https://rust-lang.org).");
        let Block::Paragraph(inlines) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };

        assert!(
            inlines
                .iter()
                .any(|inline| matches!(inline, Inline::Code(code) if code == "LayoutLine"))
        );
        assert!(
            inlines
                .iter()
                .any(|inline| matches!(inline, Inline::Math(tex) if tex == "a^2"))
        );
        assert!(inlines.iter().any(|inline| matches!(inline, Inline::Link { href, content } if href == "https://rust-lang.org" && content.len() == 1)));
    }

    #[test]
    fn parses_table_header_rows_and_alignment() {
        let doc = parse_markdown(
            r#"| Feature | Status |
| :-- | --: |
| Math | Done |
"#,
        );
        let Block::Table(table) = &doc.blocks[0] else {
            panic!("expected table");
        };

        assert_eq!(table.header.len(), 2);
        assert_eq!(table.rows.len(), 1);
        assert!(matches!(table.alignments[0], Alignment::Left));
        assert!(matches!(table.alignments[1], Alignment::Right));
    }
}
