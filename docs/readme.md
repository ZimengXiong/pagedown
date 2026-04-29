@title{pagedown}
@sub{A md2PDF renderer that paints primitives and does NOT print HTML}

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

> [!NOTE]
> Docs are an WIP, see [examples/](examples/) for configuration details