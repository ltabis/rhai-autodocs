use crate::{
    doc_item::DocItem,
    glossary::{generate_module_glossary, ModuleGlossary},
    module::{generate_module_documentation, Error, ModuleDocumentation},
};

pub(crate) const RHAI_ITEM_INDEX_PATTERN: &str = "# rhai-autodocs:index:";

/// Types of markdown processor where the documentation generated will be hosted.
#[derive(Default)]
pub enum MarkdownProcessor {
    /// Generate documentation for mdbook: <https://rust-lang.github.io/mdBook/>
    MdBook,
    /// Generate documentation for docusaurus. <https://docusaurus.io/>
    #[default]
    Docusaurus,
}

#[derive(Default)]
/// Options to configure documentation generation.
pub struct Options {
    pub(crate) items_order: ItemsOrder,
    pub(crate) sections_format: SectionFormat,
    pub(crate) include_standard_packages: bool,
}

impl Options {
    /// Include the standard package functions and modules documentation
    /// in the generated documentation markdown.
    pub fn include_standard_packages(mut self, include_standard_packages: bool) -> Self {
        self.include_standard_packages = include_standard_packages;

        self
    }

    /// Order documentation items in a specific way.
    /// See [`ItemsOrder`] for more details.
    pub fn order_items_with(mut self, items_order: ItemsOrder) -> Self {
        self.items_order = items_order;

        self
    }

    /// Format doc comments 'sections', markdown that starts with the `#` character,
    /// with special formats.
    /// See [`SectionFormat`] for more details.
    pub fn format_sections_with(mut self, sections_format: SectionFormat) -> Self {
        self.sections_format = sections_format;

        self
    }

    /// Generate documentation based on an engine instance.
    /// Make sure all the functions, operators, plugins, etc. are registered inside this instance.
    ///
    /// # Result
    /// * A vector of documented modules.
    ///
    /// # Errors
    /// * Failed to generate function metadata as json.
    /// * Failed to parse module metadata.
    pub fn export(self, engine: &rhai::Engine) -> Result<ModuleDocumentation, Error> {
        generate_module_documentation(engine, &self)
    }

    /// Generate documentation based on an engine instance and a list of all functions signature.
    /// Make sure all the functions, operators, plugins, etc. are registered inside this instance.
    ///
    /// # Result
    /// * A vector of documented modules and the glossary.
    ///
    /// # Errors
    /// * Failed to generate function metadata as json.
    /// * Failed to parse module metadata.
    pub fn export_with_glossary(
        &self,
        engine: &rhai::Engine,
    ) -> Result<(ModuleDocumentation, ModuleGlossary), Error> {
        Ok((
            generate_module_documentation(engine, self)?,
            generate_module_glossary(engine, self)?,
        ))
    }
}

/// Select in which order each doc item will be displayed.
#[derive(Default)]
pub enum ItemsOrder {
    /// Display functions by alphabetical order.
    #[default]
    Alphabetical,
    /// Display functions by index using a pre-processing comment with the `# rhai-autodocs:index:<number>` syntax.
    /// The `# rhai-autodocs:index:<number>` line will be removed in the final generated markdown.
    ///
    /// # Example
    ///
    /// ```ignore
    /// /// Function that will appear first in docs.
    /// ///
    /// /// # rhai-autodocs:index:1
    /// #[rhai_fn(global)]
    /// pub fn my_function1() {}
    ///
    /// /// Function that will appear second in docs.
    /// ///
    /// /// # rhai-autodocs:index:2
    /// #[rhai_fn(global)]
    /// pub fn my_function2() {}
    /// ```
    ///
    /// Adding, removing or re-ordering your functions from your api can be a chore
    /// because you have to update all indexes by hand. Thankfully, you will found
    /// a python script in the `scripts` folder of the `rhai-autodocs` repository
    /// that will update the indexes by hand just for you.
    ///
    /// The script generates a .autodocs file from your original source file,
    /// make sure to check that it did not mess with your source code using
    /// a diff tool.
    ByIndex,
}

impl ItemsOrder {
    /// Order [`DocItem`]s following the given option.
    pub(crate) fn order_items(&'_ self, mut items: Vec<DocItem>) -> Vec<DocItem> {
        match self {
            Self::Alphabetical => {
                items.sort_by(|i1, i2| i1.name().cmp(i2.name()));
                items
            }
            Self::ByIndex => {
                items.sort_by_key(DocItem::index);
                items
            }
        }
    }
}

/// Options to format the display of sections marked with the `#`
/// tag in markdown.
#[derive(Default)]
pub enum SectionFormat {
    /// Display sections the same as Rust doc comments, using the
    /// default markdown titles.
    #[default]
    Rust,
    /// Display sections using tabs that wraps all underlying
    /// documentation in them.
    Tabs,
}

#[derive(Default, Clone, serde::Serialize)]
struct Section {
    pub name: String,
    pub body: String,
}

/// Create new options used to configure docs generation.
pub fn options() -> Options {
    Options::default()
}
