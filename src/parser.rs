use pulldown_cmark::{
    Alignment as MdAlignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser,
    Tag, TagEnd,
};

use crate::ir::{Alignment, Block, Document, Inline, ListItem, QuoteKind, Table};

#[derive(Debug)]
enum ActiveBlock {
    Paragraph(Vec<Inline>),
    Heading { level: u8, content: Vec<Inline> },
    CodeBlock { lang: Option<String>, text: String },
}

#[derive(Debug)]
enum InlineContainerKind {
    Link { href: String },
    Emphasis,
    Strong,
    Strikethrough,
}

#[derive(Debug)]
struct InlineContainer {
    kind: InlineContainerKind,
    content: Vec<Inline>,
}

#[derive(Debug)]
struct ActiveImage {
    src: String,
    alt: String,
}

#[derive(Debug)]
struct ActiveQuote {
    kind: QuoteKind,
    content: Vec<Inline>,
}

#[derive(Debug)]
struct ActiveList {
    ordered: bool,
    start: u64,
    items: Vec<ListItem>,
    current_item: Vec<Inline>,
    current_checked: Option<bool>,
    in_item: bool,
}

#[derive(Debug)]
struct ActiveFootnote {
    label: String,
    content: Vec<Inline>,
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
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_MATH);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_GFM);

    let parser = Parser::new_ext(input, options);
    let mut blocks = Vec::new();
    let mut active_block: Option<ActiveBlock> = None;
    let mut inline_stack: Vec<InlineContainer> = Vec::new();
    let mut active_image: Option<ActiveImage> = None;
    let mut active_quote: Option<ActiveQuote> = None;
    let mut active_list: Option<ActiveList> = None;
    let mut active_footnote: Option<ActiveFootnote> = None;
    let mut active_table: Option<ActiveTable> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if active_table.as_ref().is_none_or(|table| !table.in_cell)
                        && active_quote.is_none()
                        && active_list.as_ref().is_none_or(|list| !list.in_item)
                        && active_footnote.is_none()
                    {
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
                Tag::BlockQuote(kind) => {
                    active_quote = Some(ActiveQuote {
                        kind: quote_kind(kind),
                        content: Vec::new(),
                    });
                }
                Tag::List(start) => {
                    active_list = Some(ActiveList {
                        ordered: start.is_some(),
                        start: start.unwrap_or(1),
                        items: Vec::new(),
                        current_item: Vec::new(),
                        current_checked: None,
                        in_item: false,
                    });
                }
                Tag::Item => {
                    if let Some(list) = &mut active_list {
                        list.current_item = Vec::new();
                        list.current_checked = None;
                        list.in_item = true;
                    }
                }
                Tag::FootnoteDefinition(label) => {
                    active_footnote = Some(ActiveFootnote {
                        label: label.to_string(),
                        content: Vec::new(),
                    });
                }
                Tag::Emphasis => inline_stack.push(InlineContainer {
                    kind: InlineContainerKind::Emphasis,
                    content: Vec::new(),
                }),
                Tag::Strong => inline_stack.push(InlineContainer {
                    kind: InlineContainerKind::Strong,
                    content: Vec::new(),
                }),
                Tag::Strikethrough => inline_stack.push(InlineContainer {
                    kind: InlineContainerKind::Strikethrough,
                    content: Vec::new(),
                }),
                Tag::Link { dest_url, .. } => {
                    inline_stack.push(InlineContainer {
                        kind: InlineContainerKind::Link {
                            href: dest_url.to_string(),
                        },
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
                    if active_quote.is_some()
                        || active_list.as_ref().is_some_and(|list| list.in_item)
                        || active_footnote.is_some()
                    {
                        push_context_separator(
                            &mut active_quote,
                            &mut active_list,
                            &mut active_footnote,
                        );
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
                TagEnd::BlockQuote(_) => {
                    if let Some(quote) = active_quote.take() {
                        blocks.push(Block::Quote {
                            kind: quote.kind,
                            content: trim_inline_edges(quote.content),
                        });
                    }
                }
                TagEnd::List(_) => {
                    if let Some(list) = active_list.take() {
                        blocks.push(Block::List {
                            ordered: list.ordered,
                            start: list.start,
                            items: list.items,
                        });
                    }
                }
                TagEnd::Item => {
                    if let Some(list) = &mut active_list {
                        list.items.push(ListItem {
                            checked: list.current_checked,
                            content: trim_inline_edges(std::mem::take(&mut list.current_item)),
                        });
                        list.current_checked = None;
                        list.in_item = false;
                    }
                }
                TagEnd::FootnoteDefinition => {
                    if let Some(footnote) = active_footnote.take() {
                        blocks.push(Block::Footnote {
                            label: footnote.label,
                            content: trim_inline_edges(footnote.content),
                        });
                    }
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                    if let Some(container) = inline_stack.pop() {
                        let inline = match container.kind {
                            InlineContainerKind::Link { href } => Inline::Link {
                                href,
                                content: container.content,
                            },
                            InlineContainerKind::Emphasis => Inline::Emphasis(container.content),
                            InlineContainerKind::Strong => Inline::Strong(container.content),
                            InlineContainerKind::Strikethrough => {
                                Inline::Strikethrough(container.content)
                            }
                        };
                        push_inline(
                            &mut active_block,
                            &mut active_table,
                            &mut active_quote,
                            &mut active_list,
                            &mut active_footnote,
                            &mut inline_stack,
                            inline,
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
                } else if let Some(ActiveBlock::CodeBlock { text: code, .. }) = &mut active_block {
                    code.push_str(&text);
                } else {
                    for inline in text_to_inlines(&text) {
                        push_inline(
                            &mut active_block,
                            &mut active_table,
                            &mut active_quote,
                            &mut active_list,
                            &mut active_footnote,
                            &mut inline_stack,
                            inline,
                        );
                    }
                }
            }
            Event::Code(text) => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    &mut active_quote,
                    &mut active_list,
                    &mut active_footnote,
                    &mut inline_stack,
                    Inline::Code(text.to_string()),
                );
            }
            Event::InlineMath(text) => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    &mut active_quote,
                    &mut active_list,
                    &mut active_footnote,
                    &mut inline_stack,
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
                        &mut active_quote,
                        &mut active_list,
                        &mut active_footnote,
                        &mut inline_stack,
                        Inline::Math(text.to_string()),
                    );
                }
            }
            Event::FootnoteReference(label) => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    &mut active_quote,
                    &mut active_list,
                    &mut active_footnote,
                    &mut inline_stack,
                    Inline::FootnoteRef(label.to_string()),
                );
            }
            Event::SoftBreak | Event::HardBreak => {
                push_inline(
                    &mut active_block,
                    &mut active_table,
                    &mut active_quote,
                    &mut active_list,
                    &mut active_footnote,
                    &mut inline_stack,
                    Inline::Text(" ".to_string()),
                );
            }
            Event::TaskListMarker(checked) => {
                if let Some(list) = &mut active_list {
                    list.current_checked = Some(checked);
                }
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
    active_quote: &mut Option<ActiveQuote>,
    active_list: &mut Option<ActiveList>,
    active_footnote: &mut Option<ActiveFootnote>,
    inline_stack: &mut Vec<InlineContainer>,
    inline: Inline,
) {
    if let Some(container) = inline_stack.last_mut() {
        push_inline_vec(&mut container.content, inline);
        return;
    }

    if let Some(table) = active_table {
        if table.in_cell {
            push_inline_vec(&mut table.current_cell, inline);
            return;
        }
    }

    if let Some(list) = active_list {
        if list.in_item {
            push_inline_vec(&mut list.current_item, inline);
            return;
        }
    }

    if let Some(quote) = active_quote {
        push_inline_vec(&mut quote.content, inline);
        return;
    }

    if let Some(footnote) = active_footnote {
        push_inline_vec(&mut footnote.content, inline);
        return;
    }

    match active_block {
        Some(ActiveBlock::Paragraph(content)) | Some(ActiveBlock::Heading { content, .. }) => {
            push_inline_vec(content, inline);
        }
        Some(ActiveBlock::CodeBlock { text, .. }) => {
            text.push_str(&inline.plain_text());
        }
        None => {
            *active_block = Some(ActiveBlock::Paragraph(vec![inline]));
        }
    }
}

fn push_inline_vec(content: &mut Vec<Inline>, inline: Inline) {
    if let Inline::Text(text) = &inline {
        if text == "]" && content.len() >= 2 {
            let last = content.pop();
            let prev = content.pop();
            match (prev, last) {
                (Some(Inline::Text(open)), Some(Inline::Text(key)))
                    if open == "[" && key.starts_with('@') && is_citation_key(&key[1..]) =>
                {
                    content.push(Inline::Citation(key[1..].to_string()));
                    return;
                }
                (Some(Inline::Text(open)), Some(Inline::Text(key)))
                    if open == "[" && key.starts_with('^') && is_citation_key(&key[1..]) =>
                {
                    content.push(Inline::FootnoteRef(key[1..].to_string()));
                    return;
                }
                (prev, last) => {
                    if let Some(prev) = prev {
                        content.push(prev);
                    }
                    if let Some(last) = last {
                        content.push(last);
                    }
                }
            }
        }
        if content
            .last()
            .is_some_and(|inline| matches!(inline, Inline::Citation(_) | Inline::FootnoteRef(_)))
            && text
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphanumeric())
        {
            content.push(Inline::Text(format!(" {text}")));
            return;
        }
    }
    content.push(inline);
}

fn push_context_separator(
    active_quote: &mut Option<ActiveQuote>,
    active_list: &mut Option<ActiveList>,
    active_footnote: &mut Option<ActiveFootnote>,
) {
    let separator = Inline::Text(" ".to_string());
    if let Some(list) = active_list {
        if list.in_item && !list.current_item.last().is_some_and(is_space_inline) {
            list.current_item.push(separator);
        }
        return;
    }
    if let Some(quote) = active_quote {
        if !quote.content.last().is_some_and(is_space_inline) {
            quote.content.push(separator);
        }
        return;
    }
    if let Some(footnote) = active_footnote {
        if !footnote.content.last().is_some_and(is_space_inline) {
            footnote.content.push(separator);
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

fn quote_kind(kind: Option<BlockQuoteKind>) -> QuoteKind {
    match kind {
        Some(BlockQuoteKind::Note) => QuoteKind::Note,
        Some(BlockQuoteKind::Tip) => QuoteKind::Tip,
        Some(BlockQuoteKind::Important) => QuoteKind::Important,
        Some(BlockQuoteKind::Warning) => QuoteKind::Warning,
        Some(BlockQuoteKind::Caution) => QuoteKind::Caution,
        None => QuoteKind::Regular,
    }
}

fn trim_inline_edges(mut content: Vec<Inline>) -> Vec<Inline> {
    while content.first().is_some_and(is_space_inline) {
        content.remove(0);
    }
    while content.last().is_some_and(is_space_inline) {
        content.pop();
    }
    content
}

fn is_space_inline(inline: &Inline) -> bool {
    matches!(inline, Inline::Text(text) if text.trim().is_empty())
}

fn text_to_inlines(text: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("[@") {
        if start > 0 {
            out.push(Inline::Text(rest[..start].to_string()));
        }
        let citation_start = start + 2;
        if let Some(end) = rest[citation_start..].find(']') {
            let key = &rest[citation_start..citation_start + end];
            if is_citation_key(key) {
                out.push(Inline::Citation(key.to_string()));
                rest = &rest[citation_start + end + 1..];
                continue;
            }
        }
        out.push(Inline::Text(rest[start..start + 2].to_string()));
        rest = &rest[start + 2..];
    }

    if !rest.is_empty() {
        out.push(Inline::Text(rest.to_string()));
    }
    out
}

fn is_citation_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch, '_' | '-' | ':' | '.' | ';' | ' ' | '@' | ',')
        })
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

    #[test]
    fn parses_notes_lists_footnotes_and_citations() {
        let doc = parse_markdown(
            r#"> [!NOTE]
> Callouts render as native note blocks with **strong** text.

- [x] Done item with [@doe2024]
- [ ] Pending item

Footnote ref[^why].

[^why]: Footnote body with *emphasis*.
"#,
        );

        assert!(matches!(
            doc.blocks[0],
            Block::Quote {
                kind: QuoteKind::Note,
                ..
            }
        ));
        let Block::List { items, .. } = &doc.blocks[1] else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].checked, Some(true));
        assert!(
            items[0]
                .content
                .iter()
                .any(|inline| matches!(inline, Inline::Citation(key) if key == "doe2024"))
        );
        let Block::Paragraph(inlines) = &doc.blocks[2] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|inline| matches!(inline, Inline::FootnoteRef(label) if label == "why"))
        );
        assert!(matches!(doc.blocks[3], Block::Footnote { .. }));
    }
}
