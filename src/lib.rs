#![doc = include_str!("../README.md")]

pub mod custom_types;
pub mod doc_item;
pub mod function;
pub mod glossary;
pub mod module;

pub use glossary::ModuleGlossary;
pub use module::{
    options::{export, ItemsOrder, MarkdownProcessor, SectionFormat},
    ModuleDocumentation,
};
use serde_json::json;

#[derive(Default)]
pub struct DocusaurusOptions {
    pub(crate) slug: Option<String>,
}

impl DocusaurusOptions {
    /// Format the slug in the metadata section of the generated MDX document by concatenating the `slug` parameter with the module name.
    ///
    /// For example, if the documentation for a module called `my_module` is generated with
    /// the slug `/docs/api/`, the slug set in the document will be `/docs/api/my_module`.
    ///
    /// By default the root `/` path is used.
    pub fn with_slug(mut self, slug: &str) -> Self {
        self.slug = Some(slug.to_string());

        self
    }

    /// Build MDX documentation for docusaurus from the given module documentation struct.
    ///
    /// Returns a hashmap with the name of the module as the key and its raw documentation as the value.
    pub fn build(
        self,
        module: &ModuleDocumentation,
    ) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
        let mut hbs_registry = handlebars::Handlebars::new();

        hbs_registry
            .register_template_string(
                "docusaurus-module",
                include_str!("handlebars/docusaurus/header.hbs"),
            )
            .expect("template is valid");

        // A partial used to keep indentation for mdx to render correctly.
        hbs_registry
            .register_partial("ContentPartial", "{{{content}}}")
            .expect("partial is valid");

        generate(
            module,
            "docusaurus-module",
            self.slug.as_deref(),
            &hbs_registry,
        )
    }
}

#[derive(Default)]
pub struct MDBookOptions;

impl MDBookOptions {
    /// Build html documentation for mdbook from the given module documentation struct.
    ///
    /// Returns a hashmap with the name of the module as the key and its raw documentation as the value.
    pub fn build(
        self,
        module: &ModuleDocumentation,
    ) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
        let mut hbs_registry = handlebars::Handlebars::new();

        hbs_registry
            .register_template_string(
                "mdbook-module",
                include_str!("handlebars/mdbook/header.hbs"),
            )
            .expect("template is valid");

        // A partial used to keep indentation for md to render correctly.
        hbs_registry
            .register_partial("ContentPartial", "{{{content}}}")
            .expect("partial is valid");

        generate(module, "mdbook-module", None, &hbs_registry)
    }
}

pub mod generate {
    /// Create a new builder to generate documentation for docusaurus from a [`ModuleDocumentation`] object.
    pub fn docusaurus() -> super::DocusaurusOptions {
        super::DocusaurusOptions::default()
    }

    /// Create a new builder to generate documentation for mdbook from a [`ModuleDocumentation`] object.
    pub fn mdbook() -> super::MDBookOptions {
        super::MDBookOptions::default()
    }
}

fn generate(
    module: &ModuleDocumentation,
    template: &str,
    slug: Option<&str>,
    hbs_registry: &handlebars::Handlebars,
) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
    let mut documentation = std::collections::HashMap::default();
    let data = json!({
        "title": module.name,
        "slug": slug.unwrap_or(&module.name),
        "description": module.documentation,
        "namespace": module.namespace,
        "items": module.items,
    });

    documentation.insert(
        module.name.to_string(),
        hbs_registry.render(template, &data)?,
    );

    for sub in &module.sub_modules {
        documentation.extend(generate(sub, template, slug, hbs_registry)?);
    }

    Ok(documentation)
}
