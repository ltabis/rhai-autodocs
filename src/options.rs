use crate::{
    error::AutodocsError, function::FunctionMetadata, generate_documentation,
    module::ModuleDocumentation,
};

pub const RHAI_FUNCTION_INDEX_PATTERN: &str = "# rhai-autodocs:index:";

/// Options to configure documentation generation.
pub struct Options {
    pub(crate) functions_order: FunctionOrder,
    pub(crate) sections_format: SectionFormat,
    pub(crate) include_standard_packages: bool,
    pub(crate) include_default_css: bool,
    pub(crate) custom_css_files: Vec<std::path::PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            functions_order: Default::default(),
            sections_format: Default::default(),
            include_standard_packages: false,
            include_default_css: true,
            custom_css_files: Default::default(),
        }
    }
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

    /// Include CSS used to render custom html elements generated
    /// by the crate.
    pub fn include_default_css(mut self, include_default_css: bool) -> Self {
        self.include_default_css = include_default_css;

        self
    }

    /// Order functions in a specific way.
    /// See [`FunctionOrder`] for more details.
    pub fn order_functions_with(mut self, functions_order: FunctionOrder) -> Self {
        self.functions_order = functions_order;

        self
    }

    /// Format doc comments 'sections', markdown that starts with the `#` character,
    /// with special formats.
    /// See [`SectionFormat`] for more details.
    pub fn format_sections_with(mut self, sections_format: SectionFormat) -> Self {
        self.sections_format = sections_format;

        self
    }

    /// Add custom css to render pages.
    pub fn with_css_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.custom_css_files.push(path.into());

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
