# Native Markdown PDF

A native Markdown-to-PDF renderer in Rust. It parses Markdown into a small document IR, lays the document out directly, and paints PDF primitives without rendering HTML or using browser print.

## Supported Markdown

- headings
- paragraphs
- links
- inline code
- fenced code blocks with syntax highlighting
- inline math and display math
- horizontal rules from Markdown dividers
- tables
- local images with optional captions from alt text

## Requirements

- Rust stable
- For real math rendering: `latex` and `dvisvgm` on `PATH`

Math defaults to the LaTeX backend. That means common TeX packages and constructs are rendered by a real TeX toolchain and embedded as vector SVG-derived PDF XObjects. Use `--math-mode fallback` only when you explicitly want the simple emergency text fallback.

## Usage

```sh
cargo run -- examples/sample.md -o examples/sample.pdf
```

Useful options:

```sh
cargo run -- input.md -o output.pdf \
  --page-size a4 \
  --margin-x 64 \
  --margin-top 58 \
  --margin-bottom 64 \
  --body-size 11.5 \
  --body-line-height 17 \
  --table-size 9.5 \
  --table-line-height 13.5 \
  --code-size 9.3 \
  --code-line-height 13.4 \
  --code-theme InspiredGitHub \
  --math-mode latex \
  --max-image-height 300 \
  --image-caption-gap 4 \
  --title "Native PDF"
```

Disable specific defaults:

```sh
cargo run -- input.md --no-code-highlighting --no-page-numbers --no-image-captions
```

## Design Notes

The default typography uses a serif body face, sans-serif headings, generous paragraph leading, compact code blocks, table grid lines only for tables, and horizontal rules only when the Markdown contains a divider. Code highlighting is enabled by default through `syntect`.
