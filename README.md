# rhai-autodocs

Generate Markdown/MDX documentation from a `rhai::Engine` instance.

Published with [Mdbook](https://rust-lang.github.io/mdBook/index.html).
![generated documentation for mdbook](assets/mdbook.png)
Published with [Docusaurus](https://docusaurus.io/).
![generated documentation for docusaurus](assets/docusaurus.jpg)

## Features

- Output native Rust Rhai function and custom types documentation as Markdown with HTML and Docusaurus with MDX.
- Function ordering using the `# rhai-autodocs:index:x` directive in your docs.
- Rust docs 'sections' (`# Section` in markdown) displayed with tabs.

## How to use

```rust
use rhai::exported_module;
use rhai::plugin::*;

// 1. Create a plugin module or any kind of Rhai API that supports documentation on functions and types.

/// My own module.
#[export_module]
mod my_module {
    /// A function that prints to stdout.
    ///
    /// # Args
    ///
    /// * message - append a message to the greeting. (optional)
    ///
    /// # rhai-autodocs:index:1
    #[rhai_fn(global, name = "hello_world")]
    pub fn hello_world_message(message: &str) {
        println!("Hello, World! {message}");
    }

    /// A function that prints to stdout.
    ///
    /// # Args
    ///
    /// * message - append a message to the greeting. (optional)
    ///
    /// # rhai-autodocs:index:1
    #[rhai_fn(global, name = "hello_world")]
    pub fn hello_world() {
        println!("Hello, World!");
    }

    /// A function that adds two integers together.
    ///
    /// # rhai-autodocs:index:2
    #[rhai_fn(global)]
    pub fn add(a: rhai::INT, b: rhai::INT) -> rhai::INT {
        a + b
    }
}

// 2. Generate the docs with autodocs. This library can be imported as a build dependency into your build script.
//    A typical documentation generation workflow would look like this:
fn main() {
    // Specify an environment variable that points to the directory
    // where the documentation will be generated.
    let docs_path = std::env::var("DOCS_DIR").unwrap_or("target/docs".to_string());

    let mut engine = rhai::Engine::new();

    // We register the module defined in the previous code block for this example,
    // but you could register other functions and types ...
    engine.register_static_module("my_module", exported_module!(my_module).into());

    let docs = rhai_autodocs::options()
        .include_standard_packages(false)
        .generate(&engine)
        .expect("failed to generate documentation");

    // Write the documentation in a file, or output to stdout, etc.
    for (name, docs) in rhai_autodocs::generate_for_docusaurus(&docs).unwrap() {
        println!("docs for module {name}");
        println!("{docs}");
    }
}

```

You need to import the `styles/default.css` file and `src/tabs.js` script for everything to work correctly using the [mdbook](https://rust-lang.github.io/mdBook/index.html) generation. (You can of course override the styles and javascript code if you wish)

For more details, see the examples.
