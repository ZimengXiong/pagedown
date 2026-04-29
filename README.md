# Native Markdown PDF

A CLI-first Markdown-to-PDF renderer in Rust that does not render HTML and then print it. It parses Markdown into a small document IR, lays the document out directly, and paints PDF primitives.

The project currently supports the CLI only. The old browser/WASM app has been removed.

Math is rendered either through the PageTeX submodule for native selectable PDF primitives, or through a local TeX toolchain with `lualatex`/`latex` when those modes are selected.

[example](examples/sample.pdf)

![](examples/sample-page-00.png)
![](examples/sample-page-01.png)
![](examples/sample-page-02.png)
![](examples/sample-page-03.png)
