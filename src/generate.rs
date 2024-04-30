use serde_json::json;

use crate::module::Documentation;

#[derive(Default)]
pub struct DocusaurusOptions {
    slug: Option<String>,
}

impl DocusaurusOptions {
    /// Format the slug in the metadata section of the generated MDX document by concatenating the `slug` parameter with the module name.
    ///
    /// For example, if the documentation for a module called `my_module` is generated with
    /// the slug `/docs/api/`, the slug set in the document will be `/docs/api/my_module`.
    ///
    /// By default the root `/` path is used.
    #[must_use]
    pub fn with_slug(mut self, slug: &str) -> Self {
        self.slug = Some(slug.to_string());

        self
    }

    /// Build MDX documentation for docusaurus from the given module documentation struct.
    ///
    /// Returns a hashmap with the name of the module as the key and its raw documentation as the value.
    ///
    /// # Errors
    ///
    /// Handlebar failed to render the variables in the module documentation.
    #[allow(clippy::missing_panics_doc)]
    pub fn generate(
        self,
        module: &Documentation,
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

/// Create a new builder to generate documentation for docusaurus from a [`super::module::Documentation`] object.
#[must_use]
pub fn docusaurus() -> DocusaurusOptions {
    DocusaurusOptions::default()
}

#[derive(Default)]
pub struct MDBookOptions;

impl MDBookOptions {
    /// Build html documentation for mdbook from the given module documentation struct.
    ///
    /// Returns a hashmap with the name of the module as the key and its raw documentation as the value.
    ///
    /// # Errors
    ///
    /// Handlebar failed to render the variables in the module documentation.
    #[allow(clippy::missing_panics_doc)]
    pub fn generate(
        self,
        module: &Documentation,
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

/// Create a new builder to generate documentation for mdbook from a [`super::module::Documentation`] object.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
pub fn mdbook() -> MDBookOptions {
    MDBookOptions
}

fn generate(
    module: &Documentation,
    template: &str,
    slug: Option<&str>,
    hbs_registry: &handlebars::Handlebars<'_>,
) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
    let mut documentation = std::collections::HashMap::default();
    let data = json!({
        "title": module.name,
        "slug": slug.map_or(format!("/{}", module.name), |slug| format!("{}/{}", slug, module.name)),
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
