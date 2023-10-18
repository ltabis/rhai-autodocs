use rhai::plugin::*;

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
    /// # Args
    ///
    /// * a - the first integer.
    /// * b - the second integer.
    ///
    /// # Return
    ///
    /// * An integer, the result of the addition of `a` and `b`.
    ///
    /// # rhai-autodocs:index:2
    #[rhai_fn(global)]
    pub fn add(a: rhai::INT, b: rhai::INT) -> rhai::INT {
        a + b
    }
}

fn main() {
    let mut engine = rhai::Engine::new();

    engine.register_static_module("my_module", exported_module!(my_module).into());

    // register custom functions and types ...
    let docs = rhai_autodocs::module::options()
        .include_standard_packages(false)
        .order_items_with(rhai_autodocs::module::options::ItemsOrder::ByIndex)
        .format_sections_with(rhai_autodocs::module::options::SectionFormat::Tabs)
        .for_markdown_processor(rhai_autodocs::module::options::MarkdownProcessor::Docusaurus)
        .generate(&engine)
        .expect("failed to generate documentation");

    let path = "./examples/docusaurus/docusaurus-example/docs/rhai-autodocs";

    // Write the documentation in a file.
    write_docs(path, &docs);

    println!("documentation generated to {path:?}");
}

fn write_docs(path: &str, docs: &rhai_autodocs::ModuleDocumentation) {
    std::fs::write(
        std::path::PathBuf::from_iter([path, &format!("{}.mdx", &docs.name)]),
        &docs.documentation,
    )
    .expect("failed to write documentation");

    for doc in &docs.sub_modules {
        write_docs(path, doc);
    }
}
