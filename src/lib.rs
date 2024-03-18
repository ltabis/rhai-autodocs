#![doc = include_str!("../README.md")]

pub mod custom_types;
pub mod doc_item;
pub mod function;
pub mod glossary;
pub mod module;

pub use glossary::ModuleGlossary;
pub use module::{
    options::{options, ItemsOrder, MarkdownProcessor, SectionFormat},
    ModuleDocumentation,
};
use serde_json::json;

/// Generate documentation for the docusaurus markdown processor.
///
/// Returns a hashmap with the name of the module as the key and its raw documentation as the value.
pub fn generate_for_docusaurus(
    module: &ModuleDocumentation,
) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
    let mut hbs_registry = handlebars::Handlebars::new();

    hbs_registry
        .register_template_string(
            "docusaurus-module",
            include_str!("handlebars/docusaurus/header.hbs"),
        )
        .expect("template is valid");
    hbs_registry
        .register_partial("ContentPartial", "{{{content}}}")
        .expect("partial is valid");

    generate(module, "docusaurus-module", &hbs_registry)
}

/// Generate documentation for the mdbook markdown processor.
///
/// Returns a hashmap with the name of the module as the key and its raw documentation as the value.
pub fn generate_for_mdbook(
    module: &ModuleDocumentation,
) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
    let mut hbs_registry = handlebars::Handlebars::new();

    hbs_registry
        .register_template_string(
            "mdbook-module",
            include_str!("handlebars/mdbook/header.hbs"),
        )
        .expect("template is valid");
    hbs_registry
        .register_partial("ContentPartial", "{{{content}}}")
        .expect("partial is valid");

    generate(module, "mdbook-module", &hbs_registry)
}

fn generate(
    module: &ModuleDocumentation,
    template: &str,
    hbs_registry: &handlebars::Handlebars,
) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
    let mut documentation = std::collections::HashMap::default();
    let data = json!({
        "title": module.name,
        "slug": module.name,
        "description": module.documentation,
        "namespace": module.namespace,
        "items": module.items,
    });

    documentation.insert(
        module.name.to_string(),
        hbs_registry.render(template, &data)?,
    );

    for sub in &module.sub_modules {
        documentation.extend(generate(sub, template, hbs_registry)?);
    }

    Ok(documentation)
}
