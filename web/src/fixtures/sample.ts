export const sampleMarkdown = `# Native Markdown Rendering

This document is a compact fixture for the renderer. It checks the typographic rhythm between headings, paragraph text, inline code like \`LayoutLine\`, inline math like $E = mc^2$, links such as [the Rust project](https://www.rust-lang.org), tables, images, code blocks, and display math.

## Spacing System

The page uses a quiet serif body face, a stronger sans heading face, and a modular scale. Paragraph leading is intentionally generous without drifting into double-spaced territory. The result should feel like a designed technical note rather than a browser printout.

Inline code should sit inside the line without throwing off the baseline. Links are colored and underlined, while math remains visually distinct: $a^2 + b^2 = c^2$.

## Notes, Lists, and Citations

> [!NOTE]
> Callouts use GitHub-style note syntax and render as native PDF blocks, not HTML boxes. They can include **strong text**, *emphasis*, inline code like \`RenderOptions\`, and citations like [@knuth1984].

- Plain bullet markers use native shapes and align with the list text.
- Citation markers such as [@lamport1994] are styled inline until bibliography resolution exists.

- [x] Task list markers keep their checked state.
- [ ] Plain list layout shares the same body rhythm as paragraphs.

1. Ordered list markers use native numbering instead of browser text layout.
2. Continuation lines stay aligned with the ordered list body text.

Footnote references render inline[^note].

[^note]: Footnotes are parsed from standard GitHub-style Markdown definitions and rendered in compact note text.

---

## Code Blocks

\`\`\`rust
fn wrap_line(words: &[&str], max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in words {
        if current.len() + word.len() + 1 > max_width as usize {
            lines.push(current);
            current = String::new();
        }
        current.push_str(word);
        current.push(' ');
    }

    lines.push(current.trim_end().to_owned());
    lines
}
\`\`\`

\`\`\`python
def accent_for(language: str) -> str:
    return {"python": "blue", "rust": "red"}.get(language, "slate")
\`\`\`

## Display Math

$$
\\int_0^1 x^2 dx = \\frac{1}{3}
$$

Math is rendered through a real TeX toolchain and imported back into the PDF as native form content, so it remains selectable instead of being flattened into browser output. The parser keeps inline and display math as distinct nodes, so layout, pagination, tables, and image handling stay independent from the math backend.

## Tables

| Feature | Rendering choice | Status |
| :-- | :-- | --: |
| Headings | Golden-ratio-ish modular scale with measured spacing | Done |
| Inline code | Monospace with subtle background and padding | Done |
| Tables | Native PDF lines, fills, and wrapped cell text | Done |
| Images | Embedded through PDF image XObjects | Done |

## Images

![A structured document pipeline diagram rendered as a bitmap fixture.](sample-image.png)

### Smaller Heading

The last paragraph checks subheading spacing after media. It should land cleanly, with enough top space to signal a new section and enough bottom space to return to normal reading rhythm.
`;

export const sampleConfig = `# All numeric geometry values are points unless noted.
# This file mirrors the built-in defaults; edit only the values you want.

page_width_pt = 612.0
page_height_pt = 792.0
margin_x_pt = 72.0
margin_top_pt = 64.0
margin_bottom_pt = 68.0
body_size_pt = 11.35
body_line_height_pt = 16.65
table_size_pt = 9.45
table_line_height_pt = 13.3
code_size_pt = 9.35
code_line_height_pt = 13.4
code_highlighting = true
code_theme = "InspiredGitHub"
math_mode = "katex"
page_numbers = true
max_image_height_pt = 286.0
image_captions = true
image_caption_size_pt = 9.2
image_caption_gap_pt = 5.0
title = "Native Markdown PDF"

[paragraph]
after_pt = 11.5

[headings]
keep_with_next_level1_to_3_pt = 126.0
keep_with_next_other_pt = 70.0

[headings.level1]
size_pt = 29.4
line_height_pt = 36.0
space_before_pt = 26.0
first_space_before_pt = 0.0
space_after_pt = 14.0

[headings.level2]
size_pt = 18.35
line_height_pt = 24.0
space_before_pt = 24.0
first_space_before_pt = 0.0
space_after_pt = 9.5

[headings.level3]
size_pt = 14.2
line_height_pt = 20.0
space_before_pt = 18.0
first_space_before_pt = 18.0
space_after_pt = 7.5

[headings.other]
size_pt = 12.4
line_height_pt = 18.0
space_before_pt = 15.0
first_space_before_pt = 15.0
space_after_pt = 6.5

[code_block]
pad_x_pt = 14.0
pad_y_pt = 13.0
title_height_pt = 14.0
title_y_offset_pt = -1.5
title_size_pt = 7.7
after_pt = 16.0
rule_x_offset_pt = 4.0
rule_y_inset_pt = 10.0
rule_total_y_inset_pt = 20.0
rule_thickness_pt = 2.2

[math_block]
size_multiplier = 1.18
x_inset_pt = 12.0
content_width_inset_pt = 24.0
vertical_padding_pt = 24.0
draw_y_inset_pt = 12.0
min_math_height_pt = 28.0
after_pt = 15.0

[divider]
keep_height_pt = 32.0
space_before_pt = 13.0
space_after_pt = 18.0
x_inset_pt = 0.0
thickness_pt = 0.9

[quote]
pad_x_pt = 16.0
pad_y_pt = 12.0
title_height_pt = 13.0
title_y_offset_pt = -1.0
title_size_pt = 8.4
after_pt = 14.0
rule_x_offset_pt = 4.0
rule_y_inset_pt = 9.0
rule_total_y_inset_pt = 18.0
rule_thickness_pt = 2.2

[list]
item_gap_pt = 4.5
after_pt = 6.0
ensure_extra_pt = 8.0
marker_text_gap_pt = 8.0
checkbox_x_pt = 1.5
checkbox_y_pt = 3.7
checkbox_size_pt = 7.6
checkbox_thickness_pt = 0.8
check_thickness_pt = 1.0
check_start_x_pt = 1.6
check_start_y_pt = 4.0
check_mid_x_pt = 3.3
check_mid_y_pt = 5.9
check_end_x_pt = 6.3
check_end_y_pt = 1.8
bullet_diameter_pt = 3.6
ordered_size_multiplier = 0.86

[footnote]
line_height_pt = 12.2
label_size_pt = 7.8
label_gap_pt = 14.0
body_size_pt = 8.9
block_extra_height_pt = 4.0
ensure_extra_pt = 4.0

[table]
cell_pad_x_pt = 7.0
cell_pad_y_pt = 7.0
border_thickness_pt = 0.45
after_pt = 16.0
initial_keep_height_pt = 20.0
row_ensure_extra_pt = 2.0

[image]
px_to_pt = 0.75
border_outset_pt = 1.0
caption_line_height_multiplier = 1.52
block_after_pt = 12.0
placeholder_height_pt = 96.0
placeholder_after_pt = 14.0
placeholder_label_x_pt = 18.0
placeholder_label_y_pt = 40.0
placeholder_label_size_pt = 10.0

[inline]
code_size_multiplier = 0.94
code_pad_x_pt = 0.0
footnote_ref_size_multiplier = 0.58
footnote_ref_shift_multiplier = -0.28
citation_size_multiplier = 0.96
link_hit_padding_pt = 1.5
link_hit_extra_height_pt = 3.0
underline_offset_multiplier = 0.12
strike_offset_multiplier = -0.30
decoration_thickness_pt = 0.45
inline_bg_height_extra_pt = 3.4
inline_bg_line_height_inset_pt = 2.0
inline_bg_y_offset_pt = -1.2
serif_decoration_width_multiplier = 0.92
sans_decoration_width_multiplier = 0.96
mono_decoration_width_multiplier = 1.0

[footer]
bottom_offset_pt = 34.0
size_pt = 8.0

[pagination]
new_page_top_tolerance_pt = 1.0
image_keep_height_pt = 285.0
table_keep_base_pt = 70.0
table_keep_row_pt = 34.0
table_keep_max_rows = 2
code_keep_base_pt = 34.0
code_keep_min_lines = 4
code_keep_max_lines = 12
math_keep_height_pt = 72.0
quote_keep_height_pt = 76.0
list_keep_base_pt = 30.0
list_keep_item_pt = 24.0
list_keep_max_items = 3
paragraph_keep_height_pt = 62.0
divider_keep_height_pt = 32.0
heading_keep_height_pt = 82.0
footnote_keep_height_pt = 24.0
`;
