use serde_json::json;

use crate::{item::Item, module::Documentation};

/// Glossary of all function for a module and it's submodules.
#[derive(Debug)]
pub struct Glossary {
    /// Formatted function signatures by submodules.
    pub content: String,
}

pub const GLOSSARY_COLOR_FN: &str = "#C6cacb";
pub const GLOSSARY_COLOR_OP: &str = "#16c6f3";
pub const GLOSSARY_COLOR_GETSET: &str = "#25c2a0";
pub const GLOSSARY_COLOR_INDEX: &str = "#25c2a0";

#[derive(Default)]
pub struct DocusaurusOptions {
    slug: Option<String>,
    module_name: Option<String>,
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

    /// When registering stuff into your engine, some items will be exported in the "global" module, a module
    /// that is accessible without the need to specify it's name. For documentation sake, you can use this method
    /// to rename the global module so that you can split multiple items groups into multiple global modules without
    /// having the "global" slug everywhere.
    ///
    /// For example, if the documentation exports items under the global namespace with
    /// the slug `/docs/api/` and the module renamed as `my_module`, the slug set in the document will be
    /// `/docs/api/my_module` instead of `/docs/api/global`.
    ///
    /// By default the root `global` module name is used.
    #[must_use]
    pub fn rename_root_module(mut self, name: &str) -> Self {
        self.module_name = Some(name.to_string());

        self
    }

    /// Build MDX documentation for docusaurus from the given module documentation struct.
    ///
    /// # Return
    ///
    /// A hashmap with the name of the module as the key and its raw documentation as the value.
    //
    /// # Errors
    ///
    /// Handlebar failed to render the variables in the module documentation.
    #[allow(clippy::missing_panics_doc)]
    pub fn generate(
        self,
        module: &Documentation,
    ) -> Result<std::collections::HashMap<String, String>, handlebars::RenderError> {
        let mut hbs_registry = handlebars::Handlebars::new();
        let mut module = module.clone();

        if let Some(module_name) = self.module_name {
            module.name = module_name;
        }

        hbs_registry
            .register_template_string(
                "docusaurus-module",
                include_str!("handlebars/docusaurus/module.hbs"),
            )
            .expect("template is valid");

        // A partial used to keep indentation for mdx to render correctly.
        hbs_registry
            .register_partial("ContentPartial", "{{{content}}}")
            .expect("partial is valid");

        generate(
            &module,
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
pub struct DocusaurusGlossaryOptions {
    slug: Option<String>,
}

impl DocusaurusGlossaryOptions {
    /// Format the slug in the metadata section of the generated MDX document.
    ///
    /// By default the root `/glossary` path is used.
    #[must_use]
    pub fn with_slug(mut self, slug: &str) -> Self {
        self.slug = Some(slug.to_string());

        self
    }

    /// Build MDX documentation for docusaurus from the given module documentation struct, with
    /// a glossary that group all functions from all submodules.
    ///
    /// # Return
    ///
    /// A glossary and a hashmap with the name of the module as the key and its raw documentation as the value.
    ///
    /// # Errors
    ///
    /// Handlebar failed to render the variables in the module documentation.
    #[allow(clippy::missing_panics_doc)]
    pub fn generate(self, module: &Documentation) -> Result<String, handlebars::RenderError> {
        let mut hbs = handlebars::Handlebars::new();

        hbs.register_template_string(
            "docusaurus-glossary",
            include_str!("handlebars/docusaurus/glossary.hbs"),
        )
        .expect("template is valid");

        self.generate_inner(&hbs, true, module)
    }

    fn generate_inner(
        &self,
        hbs: &handlebars::Handlebars<'_>,
        is_root: bool,
        module: &Documentation,
    ) -> Result<String, handlebars::RenderError> {
        let mut flatten_items = Vec::default();

        for item in &module.items {
            match item {
                Item::Function { metadata, .. } => {
                    for m in metadata {
                        let definition = m.generate_function_definition();
                        let serialized = definition.display();
                        let ty = definition.type_to_str();
                        let color = match ty {
                            "op" => GLOSSARY_COLOR_OP,
                            "get/set" => GLOSSARY_COLOR_GETSET,
                            "index get/set" => GLOSSARY_COLOR_INDEX,
                            _ => GLOSSARY_COLOR_FN,
                        };

                        flatten_items.push(json!({
                            "color": color,
                            "type": ty,
                            "definition": serialized.trim_start_matches(ty).trim(),
                            "heading_id": item.heading_id(),
                        }));
                    }
                }
                Item::CustomType { metadata, .. } => {
                    flatten_items.push(json!({
                        "color": GLOSSARY_COLOR_FN,
                        "type": "type",
                        "definition": metadata.display_name,
                        "heading_id": item.heading_id(),
                    }));
                }
            }
        }

        let data = json!({
            "title": module.name,
            "root": is_root,
            "slug": self.slug.clone().unwrap_or_default(),
            "items": flatten_items,
        });

        let mut glossary = hbs.render("docusaurus-glossary", &data)?;

        for module in &module.sub_modules {
            glossary += self.generate_inner(hbs, false, module)?.as_str();
        }

        Ok(glossary)
    }
}

/// Create a new builder to generate a function glossary for docusaurus from a [`super::module::Documentation`] object.
#[must_use]
pub fn docusaurus_glossary() -> DocusaurusGlossaryOptions {
    DocusaurusGlossaryOptions::default()
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
                include_str!("handlebars/mdbook/module.hbs"),
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
