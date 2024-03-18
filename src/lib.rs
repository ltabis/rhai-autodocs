#![doc = include_str!("../README.md")]

pub mod custom_types;
pub mod doc_item;
pub mod function;
pub mod glossary;
pub mod module;
// TODO: provide custom templates.
mod templates;

pub use glossary::ModuleGlossary;
pub use module::{
    options::{options, ItemsOrder, MarkdownProcessor, SectionFormat},
    ModuleDocumentation,
};
use serde_json::json;

/// Generate documentation for the docusaurus markdown processor.
pub fn generate_for_docusaurus(
    module: &ModuleDocumentation,
) -> Result<Vec<String>, handlebars::RenderError> {
    let mut handlebars = handlebars::Handlebars::new();

    handlebars
        .register_template_string(
            "docusaurus-module",
            include_str!("handlebars/docusaurus/header.hbs"),
        )
        .expect("template is valid");

    let mut documentation = vec![];
    let data = json!({
        "title": module.name,
        "slug": module.name,
        "description": module.documentation,
        "namespace": module.namespace,
        "items": module.items,
    });

    documentation.push(handlebars.render("docusaurus-module", &data)?);

    for sub in &module.sub_modules {
        documentation.extend(generate_for_docusaurus(sub)?);
    }

    Ok(documentation)
}
