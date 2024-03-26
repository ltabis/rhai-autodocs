use rhai::plugin::*;
use rhai_autodocs::generate_for_docusaurus;

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

    /// A new type that does stuff.
    /// # rhai-autodocs:index:3
    #[allow(dead_code)]
    pub type NewType = ();
}

use rhai::{CustomType, TypeBuilder};

/// This is another type implemented with the `CustomType` trait.
/// # rhai-autodocs:index:4
#[allow(dead_code)]
#[derive(Default, Clone, CustomType)]
#[rhai_type(name = "Tragedy", extra = Self::build_extra)]
pub struct DocumentedType {
    /// Age of the character.
    ///
    /// ```js
    /// let character = new_romeo();
    /// print(character.age); // getter.
    /// character.age = 20;   // setter.
    /// ```
    /// # rhai-autodocs:index:5
    pub age: i64,
    /// Name of the character.
    ///
    /// ```js
    /// let character = new_romeo();
    /// print(character.name);
    /// ```
    /// # rhai-autodocs:index:6
    #[rhai_type(readonly)]
    pub name: String,
}

impl DocumentedType {
    fn build_extra(builder: &mut TypeBuilder<'_, Self>) {
        builder
            .with_fn("new_romeo", || Self {
                age: 16,
                name: "Romeo".to_string(),
            })
            .and_comments(&[
                "/// build a new Romeo character",
                "/// # rhai-autodocs:index:7",
            ])
            .with_fn("new_juliet", || Self {
                age: 13,
                name: "Juliet".to_string(),
            })
            .and_comments(&[
                "/// build a new Juliet character",
                "/// # rhai-autodocs:index:8",
            ]);
    }
}

fn main() {
    let mut engine = rhai::Engine::new();

    // Register custom functions and types ...
    engine.register_static_module("my_module", exported_module!(my_module).into());
    engine.build_type::<DocumentedType>();

    // Generate documentation structure.
    let docs = rhai_autodocs::module::options()
        .include_standard_packages(false)
        .order_items_with(rhai_autodocs::module::options::ItemsOrder::ByIndex)
        .format_sections_with(rhai_autodocs::module::options::SectionFormat::Tabs)
        .generate(&engine)
        .expect("failed to generate documentation");

    let path = "./examples/docusaurus/docusaurus-example/docs/rhai-autodocs";

    // Write the documentation in files for docusaurus.
    for (name, doc) in generate_for_docusaurus(&docs).unwrap() {
        std::fs::write(
            std::path::PathBuf::from_iter([path, &format!("{}.mdx", &name)]),
            doc,
        )
        .expect("failed to write documentation");
    }

    println!("documentation generated to {path:?}");
}
