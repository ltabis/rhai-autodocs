use serde_json::json;

use crate::{
    doc_item::DocItem,
    glossary::{generate_module_glossary, ModuleGlossary},
    module::ModuleDocumentation,
    templates::build_templates_registry,
};

use super::{error::AutodocsError, generate_module_documentation};

pub const RHAI_ITEM_INDEX_PATTERN: &str = "# rhai-autodocs:index:";

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
    pub(crate) markdown_processor: MarkdownProcessor,
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

    /// Generate markdown code compatible for a specific markdown processor.
    /// See [`MarkdownProcessor`] for more details.
    pub fn for_markdown_processor(mut self, markdown_processor: MarkdownProcessor) -> Self {
        self.markdown_processor = markdown_processor;

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
        let registry = build_templates_registry();
        generate_module_documentation(engine, &self, &registry)
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
    pub fn generate_with_glossary(
        &self,
        engine: &rhai::Engine,
    ) -> Result<(ModuleDocumentation, ModuleGlossary), AutodocsError> {
        let registry = build_templates_registry();

        Ok((
            generate_module_documentation(engine, self, &registry)?,
            generate_module_glossary(engine, self, &registry)?,
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
    ///
    /// NOTE: [`SectionFormat::fmt_sections`] is called after [`remove_test_code`],
    /// so checking for code blocks and `#` line start is not required because it
    /// was supposed to be removed.
    Tabs,
}

impl SectionFormat {
    pub(crate) fn fmt_sections(
        &self,
        function_name: &str,
        markdown_processor: &MarkdownProcessor,
        docs: String,
        hbs_registry: &handlebars::Handlebars,
    ) -> String {
        let format = match self {
            // TODO: replace by handlebars.
            SectionFormat::Rust => hbs_registry
                .render("sections-rust", &json!({ "body": docs }))
                .unwrap(),
            SectionFormat::Tabs => {
                let mut sections = vec![];
                let mut current = Section::default();

                // Start by extracting all sections from markdown comments.
                docs.lines().fold(true, |first, line| {
                    if let Some((_prefix, name)) = line.split_once("# ") {
                        if !first {
                            sections.push(current.clone());
                        }

                        current = Section {
                            name: name.to_string(),
                            body: String::default(),
                        };
                    } else {
                        current.body.push_str(line);
                        current.body.push('\n');
                    }

                    false
                });

                let data = json!({
                    "function_name": function_name,
                    "sections": sections
                });

                match markdown_processor {
                    MarkdownProcessor::MdBook => {
                        hbs_registry.render("mdbook-sections-tabs", &data).unwrap()
                        //                         let mut sections = vec![];
                        //                         let mut tab_content = docs.lines().fold(
                        //                             format!(
                        //                                 r#"
                        // <div group="{function_name}" id="{function_name}-description" style="display: block;" markdown="span" class="tabcontent">
                        // "#
                        //                             ),
                        //                             |mut state, line| {
                        //                                 if let Some((_, section)) = line.split_once("# ") {
                        //                                     sections.push(section);
                        //                                     state.push_str("\n</div>\n");
                        //                                     state.push_str(&format!(
                        //                                         r#"
                        // <div group="{function_name}" id="{function_name}-{section}" class="tabcontent">
                        // "#
                        //                                     ));
                        //                                 } else {
                        //                                     state.push_str(line);
                        //                                     state.push('\n');
                        //                                 }

                        //                                 state
                        //                             },
                        //                         );

                        //                         tab_content += "</div>\n";

                        //                         sections.into_iter().fold(
                        //                             format!(
                        //                                 r#"
                        // <div class="tab">
                        //     <button
                        //         group="{function_name}"
                        //         id="link-{function_name}-description"
                        //         class="tablinks active"
                        //         onclick="openTab(event, '{function_name}', 'description')">
                        //         Description
                        //     </button>"#
                        //                             ),
                        //                             |state, section| {
                        //                                 state
                        //                                     + format!(
                        //                                         r#"
                        // <button
                        //     group="{function_name}"
                        //     id="link-{function_name}-{section}"
                        //     class="tablinks"
                        //     onclick="openTab(event, '{function_name}', '{section}')">
                        //     {section}
                        // </button>"#
                        //                                     )
                        //                                     .as_str()
                        //                             },
                        //                         ) + "</div>\n"
                        //                             + tab_content.as_str()
                    }

                    MarkdownProcessor::Docusaurus => {
                        // let mut content =
                        //     "<Tabs>\n<TabItem value=\"Description\" default>\n".to_string();

                        // for line in docs.lines() {
                        //     if let Some((_, section)) = line.split_once("# ") {
                        //         content.push_str("</TabItem>\n\n");
                        //         content
                        //             .push_str(&format!("<TabItem value=\"{section}\" default>\n"));
                        //     } else {
                        //         content.push_str(
                        //             // Removing rust links wrapped in the '<>' characters because they
                        //             // are treated as components.
                        //             &line.replace(['<', '>'], ""),
                        //         );
                        //         content.push('\n');
                        //     }
                        // }

                        // content += "\n</TabItem>\n</Tabs>\n";
                        // content
                        hbs_registry
                            .render("docusaurus-sections-tabs", &data)
                            .unwrap()
                    }
                }
            }
        };

        dbg!(format)
    }
}

#[derive(Default, Clone, serde::Serialize)]
struct Section {
    pub name: String,
    pub body: String,
}
