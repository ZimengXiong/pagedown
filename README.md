# Native Markdown PDF

A native Markdown-to-PDF renderer in Rust. It parses Markdown into a small document IR, lays the document out directly, and paints PDF primitives without rendering HTML or using browser print.

## Supported Markdown

- headings
- paragraphs
- links
- inline code
- fenced code blocks with syntax highlighting
- language-aware code block accent bars
- inline math and display math
- horizontal rules from Markdown dividers
- blockquotes and GitHub-style callouts (`> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`, `> [!CAUTION]`)
- ordered, unordered, and task lists
- tables
- local images with optional captions from alt text
- footnote references and definitions
- lightweight citation markers such as `[@doe2024]`

## Requirements

- Rust stable
- For high-fidelity CLI math rendering: `lualatex`, `latex`, and `dvisvgm` on `PATH`

Math defaults to the shared `katex` mode, which is the browser/WASM-compatible path. The CLI also keeps `lualatex` for high-fidelity exports: TeX is rendered with LuaLaTeX, imported back into the final file as native PDF form XObjects, and kept visually backed by the existing SVG path during assembly. Use `--math-mode lualatex` for that path, or `--math-mode fallback` only when you explicitly want the simple emergency text fallback. The old `latex` value remains accepted as an alias for `lualatex`.

## Usage

```sh
cargo run -- examples/sample.md -o examples/sample.pdf
```

Use a TOML config file for full layout control:

```sh
cargo run -- examples/sample.md -o examples/sample.pdf --config examples/render-config.toml
```

The config file can be partial. Missing values fall back to the built-in defaults, and CLI flags override config values. `examples/render-config.toml` contains the full default geometry surface: page size, margins, heading scale and spacing, paragraph spacing, code block padding/rules, math block padding, divider width/insets, callout padding/rules, list marker geometry and gaps, table padding/grid widths, image/caption spacing, inline code/link/footnote/citation multipliers, footer placement, and pagination keep heights.

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
  --math-mode katex \
  --max-image-height 300 \
  --image-caption-gap 4 \
  --title "Native PDF"
```

Disable specific defaults:

```sh
cargo run -- input.md --no-code-highlighting --no-page-numbers --no-image-captions
```

## Design Notes

The default typography uses a serif body face, sans-serif headings, generous paragraph leading, compact code blocks, table grid lines only for tables, and horizontal rules only when the Markdown contains a divider. Code highlighting is enabled by default through `syntect`, and code block accent bars are colored from the fence language when known.

Citations are currently inline citation tokens, not a full bibliography engine. Resolving `.bib` or CSL JSON into a references section is a separate backend layer.
