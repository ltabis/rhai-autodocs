# rhai-autodocs

Generate Markdown documentation from a `rhai::Engine` instance.

This library can be imported as a build dependency into your build script. A typical
documentation generation workflow would look like this:

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
