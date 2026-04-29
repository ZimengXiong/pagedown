# Pagedown

A CLI-only Markdown-to-PDF renderer in Rust that does not render HTML and then print it. It parses Markdown into a small document IR, lays the document out directly, and paints PDF primitives.

Main uses a local TeX toolchain for math rendering, with `lualatex` as the default backend. The browser/WASM app is not supported on `main`.

## Quickstart

```sh
pagedown input.md
```

By default, Pagedown writes `input.pdf` next to `input.md`. Relative images are resolved from the input file's directory.

Common options:

```sh
pagedown input.md --output report.pdf
pagedown input.md --config examples/render-config.toml
pagedown input.md --page-size a4 --no-page-numbers
```

## Install

Build and install the Rust binary from this repository:

```sh
cargo install --path .
```

Or run it without installing:

```sh
cargo run -- input.md
```

Math rendering defaults to `lualatex`, so install a TeX distribution that provides `lualatex`, `latex`, and `dvisvgm` for full math support. On macOS, MacTeX provides these tools. For environments without TeX, use `--math-mode fallback`.

## Usage

```sh
pagedown [OPTIONS] <INPUT>
```

Important flags:

- `-o, --output <PDF>`: choose the output PDF path.
- `-c, --config <TOML>`: load render settings from a TOML config file.
- `--math-mode <lualatex|latex|fallback>`: select the math backend.
- `--page-size <letter|a4>`: choose the page size.
- `--title <TITLE>`: set the PDF document title.

[example](examples/sample.pdf)

![](examples/sample-page-00.png)
![](examples/sample-page-01.png)
![](examples/sample-page-02.png)
![](examples/sample-page-03.png)
