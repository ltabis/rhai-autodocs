# rhai-autodocs

Generate Markdown/MDX documentation from a `rhai::Engine` instance.

Published with [Mdbook](https://rust-lang.github.io/mdBook/index.html).
![generated documentation for mdbook](assets/mdbook.png)
Published with [Docusaurus](https://docusaurus.io/).
![generated documentation for docusaurus](assets/docusaurus.png)

## Features

- Output Rhai documentation as Markdown w/ HTML and Docusaurus MDX.
- Function ordering.
- Rust docs 'sections' format with default Markdown format or displayed using tabs.

## How to use

This library can be imported as a build dependency into your build script. A typical
documentation generation would look like this:

```rust
// -- build.rs
fn main() {
    // Specify an environment variable that points to the directory
    // where the documentation will be generated.
    if let Ok(docs_path) = std::env::var("DOCS_DIR") {
        let mut engine = rhai::Engine::new();

        // register custom functions and types ...

        let docs = rhai_autodocs::options()
            .include_standard_packages(false)
            .generate(&engine)
            .expect("failed to generate documentation");

        // Write the documentation in a file, or output to stdout, etc.
    }
}
```

You need to import the `styles/default.css` file and `src/tabs.js` script for everything to work correctly using the [mdbook](https://rust-lang.github.io/mdBook/index.html) generation. (You can of course override the styles and javascript code if you wish)

For more details, see the examples.
