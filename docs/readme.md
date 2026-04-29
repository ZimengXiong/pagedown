# Pagedown

A CLI-only Markdown-to-PDF renderer in Rust. Pagedown parses Markdown into a small document model, lays it out directly, and paints PDF primitives. It does not render HTML and print a browser page.

## Quickstart

```sh
pagedown input.md
```

Writes `input.pdf` next to `input.md`. Relative images are resolved from the input file's directory.

## Install

```sh
cargo install --path .
```

Run without installing: `cargo run -- input.md`

## Common Commands

```sh
pagedown input.md --output report.pdf
pagedown input.md --config examples/render-config.toml
pagedown input.md --page-size a4 --no-page-numbers
```

## Math

```sh
pagedown input.md --math-mode lualatex
pagedown input.md --math-mode fallback
```

Main uses `lualatex` by default. Install a TeX distribution with `lualatex`, `latex`, and `dvisvgm` for full math support. Use `fallback` when TeX is unavailable.

## Status

`main` is intentionally CLI-only. The browser/WASM app is not supported here. This README page is rendered by Pagedown.
