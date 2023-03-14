use std::str::FromStr;

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
    /// # rhai-autodocs:index:2
    #[rhai_fn(global)]
    pub fn add(a: rhai::INT, b: rhai::INT) -> rhai::INT {
        a + b
    }
}

fn main() {
    let mut engine = rhai::Engine::new();

    engine.register_static_module("my_module", exported_module!(my_module).into());

    let path = "./my-module.md";

    // register custom functions and types ...
    let docs = rhai_autodocs::options()
        .include_standard_packages(false)
        .order_with(rhai_autodocs::options::FunctionOrder::ByIndex)
        .generate(&engine)
        .expect("failed to generate documentation");

    // Write the documentation in a file.
    write_docs(path, &docs);

    println!("documentation generated to {path:?}");
}

fn write_docs(path: &str, docs: &rhai_autodocs::ModuleDocumentation) {
    std::fs::write(
        std::path::PathBuf::from_str(path).unwrap(),
        &docs.documentation,
    )
    .expect("failed to write documentation");

    for doc in &docs.sub_modules {
        write_docs(path, doc);
    }
}
