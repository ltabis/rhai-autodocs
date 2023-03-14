use crate::{
    error::AutodocsError, function::FunctionMetadata, generate_documentation,
    module::ModuleDocumentation,
};

pub const RHAI_FUNCTION_INDEX_PATTERN: &str = "# rhai-autodocs:index:";

#[derive(Default)]
/// Options to configure documentation generation.
pub struct Options {
    pub(crate) order: FunctionOrder,
    pub(crate) include_standard_packages: bool,
}

/// Create new options used to configure docs generation.
pub fn options() -> Options {
    Options::default()
}

impl Options {
    /// Include the standard package functions and modules documentation
    /// in the generated documentation markdown.
    pub fn include_standard_packages(mut self, include_standard_packages: bool) -> Self {
        self.include_standard_packages = include_standard_packages;

        self
    }

    /// Order functions in a specific way.
    /// See [`FunctionOrder`] for more details.
    pub fn order_functions_with(mut self, order: FunctionOrder) -> Self {
        self.order = order;

        self
    }

    /// Order functions in a specific way.
    /// See [`FunctionOrder`] for more details.
    pub fn format_sections_with(mut self, order: FunctionOrder) -> Self {
        self.order = order;

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
    pub fn generate(self, engine: &rhai::Engine) -> Result<ModuleDocumentation, AutodocsError> {
        generate_documentation(engine, self)
    }
}

#[derive(Default)]
/// Select in which order each functions will be displayed.
pub enum FunctionOrder {
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
    ByIndex,
}

impl FunctionOrder {
    pub(crate) fn order_function_groups<'a>(
        &'_ self,
        mut function_groups: Vec<(String, Vec<&'a FunctionMetadata>)>,
    ) -> Result<Vec<(String, Vec<&'a FunctionMetadata>)>, AutodocsError> {
        match self {
            FunctionOrder::Alphabetical => {
                function_groups.sort_by(|(a, _), (b, _)| a.cmp(b));

                Ok(function_groups)
            }
            FunctionOrder::ByIndex => {
                let mut ordered = function_groups.clone();

                'groups: for (function, polymorphisms) in function_groups {
                    for metadata in polymorphisms
                        .iter()
                        .filter_map(|item| item.doc_comments.as_ref())
                    {
                        if let Some((_, index)) = metadata
                            .iter()
                            .find_map(|line| line.rsplit_once(RHAI_FUNCTION_INDEX_PATTERN))
                        {
                            let index = index
                                .parse::<usize>()
                                .map_err(|err| AutodocsError::PreProcessing(err.to_string()))?;

                            ordered[index - 1] = (function, polymorphisms);
                            continue 'groups;
                        }
                    }

                    return Err(AutodocsError::PreProcessing(format!(
                        "missing ord metadata in function {function}"
                    )));
                }

                Ok(ordered)
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
