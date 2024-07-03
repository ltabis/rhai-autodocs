use crate::{
    item::Item,
    module::{generate_module_documentation, Documentation, Error},
};

pub(crate) const RHAI_ITEM_INDEX_PATTERN: &str = "# rhai-autodocs:index:";

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
    #[must_use]
    pub const fn include_standard_packages(mut self, include_standard_packages: bool) -> Self {
        self.include_standard_packages = include_standard_packages;

        self
    }

    /// Order documentation items in a specific way.
    /// See [`ItemsOrder`] for more details.
    #[must_use]
    pub const fn order_items_with(mut self, items_order: ItemsOrder) -> Self {
        self.items_order = items_order;

        self
    }

    /// Format doc comments 'sections', markdown that starts with the `#` character,
    /// with special formats.
    /// See [`SectionFormat`] for more details.
    #[must_use]
    pub const fn format_sections_with(mut self, sections_format: SectionFormat) -> Self {
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
    pub fn export(self, engine: &rhai::Engine) -> Result<Documentation, Error> {
        generate_module_documentation(engine, &self)
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
    pub(crate) fn order_items(&'_ self, mut items: Vec<Item>) -> Vec<Item> {
        match self {
            Self::Alphabetical => {
                items.sort_by(|i1, i2| i1.name().cmp(i2.name()));
                items
            }
            Self::ByIndex => {
                items.sort_by_key(Item::index);
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

/// Create new options used to configure docs generation.
#[must_use]
pub fn options() -> Options {
    Options::default()
}
