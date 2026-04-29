# Pagedown

A CLI-only Markdown-to-PDF renderer in Rust that does not render HTML and then print it. It parses Markdown into a small document IR, lays the document out directly, and paints PDF primitives.

Main uses a local TeX toolchain for math rendering, with `lualatex` as the default backend. The browser/WASM app is not supported on `main`.

[example](examples/sample.pdf)

![](examples/sample-page-00.png)
![](examples/sample-page-01.png)
![](examples/sample-page-02.png)
![](examples/sample-page-03.png)
