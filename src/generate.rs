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
            module,
            "docusaurus-module",
            self.slug.as_deref(),
            &hbs_registry,
        )
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
    pub fn generate_with_glossary(
        self,
        module: &Documentation,
    ) -> Result<(String, std::collections::HashMap<String, String>), handlebars::RenderError> {
        let mut hbs_registry = handlebars::Handlebars::new();

        hbs_registry
            .register_template_string(
                "docusaurus-glossary",
                include_str!("handlebars/docusaurus/glossary.hbs"),
            )
            .expect("template is valid");

        let mut flatten_items = Vec::default();

        for item in &module.items {
            match item {
                Item::Function { metadata, .. } => {
                    for m in metadata {
                        let definition = m.generate_function_definition();
                        let serialized = definition.display();
                        let ty = definition.type_to_str();
                        let color = match ty {
                            "fn" => GLOSSARY_COLOR_FN,
                            "op" => GLOSSARY_COLOR_OP,
                            "get/set" => GLOSSARY_COLOR_GETSET,
                            "index get/set" => GLOSSARY_COLOR_INDEX,
                            _ => GLOSSARY_COLOR_FN,
                        };

                        flatten_items.push(json!({
                            "color": color,
                            "type": ty,
                            "definition": serialized.trim_start_matches(ty),
                        }));
                    }
                }
                Item::CustomType { metadata, .. } => {
                    flatten_items.push(json!({
                        "color": GLOSSARY_COLOR_FN,
                        "type": "type",
                        "definition": metadata.display_name,
                    }));
                }
            }
        }

        let data = json!({
            "title": module.name,
            "slug": self.slug.as_ref().map_or(format!("/{}/glossary", module.name), |slug| format!("{}/{}/glossary", slug, module.name)),
            "items": flatten_items,
        });

        let glossary = hbs_registry.render("docusaurus-glossary", &data)?;

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
            module,
            "docusaurus-module",
            self.slug.as_deref(),
            &hbs_registry,
        )
        .map(|docs| (glossary, docs))
    }

    // #[allow(clippy::too_many_lines)]
    // fn generate_module_glossary_inner(
    //     options: &Options,
    //     namespace: Option<String>,
    //     name: impl Into<String>,
    //     metadata: &ModuleMetadata,
    // ) -> Result<Glossary, Error> {
    //     fn make_highlight(color: &str, item_type: &str, definition: &str) -> String {
    //         format!("- <Highlight color=\"{color}\">{item_type}</Highlight> <code>{{\"{definition}\"}}</code>\n",)
    //     }

    //     let name = name.into();
    //     let namespace = namespace.map_or(name.clone(), |namespace| namespace);
    //     let mut items = if let Some(types) = &metadata.custom_types {
    //         types
    //             .iter()
    //             .map(|metadata| Item::new_custom_type(metadata.clone(), options))
    //             .collect::<Result<Vec<_>, Error>>()?
    //     } else {
    //         vec![]
    //     };

    //     items.extend(if let Some(functions) = &metadata.functions {
    //         let groups = group_functions(functions);
    //         groups
    //             .iter()
    //             .map(|(name, metadata)| Item::new_function(metadata, name, options))
    //             .collect::<Result<Vec<_>, Error>>()?
    //     } else {
    //         vec![]
    //     });

    //     // Remove ignored documentation.
    //     let items = items.into_iter().flatten().collect::<Vec<Item>>();

    //     let items = options.items_order.order_items(items);

    //     let signatures = {
    //         let mut signatures = String::default();

    //         for item in &items {
    //             match item {
    //                 Item::Function { metadata, .. } => {
    //                     for m in metadata {
    //                         let root_definition = m.generate_function_definition();

    //                         let serialized = root_definition.display();
    //                         // FIXME: this only works for docusaurus.
    //                         // TODO: customize colors.
    //                         signatures += &if serialized.starts_with("op ") {
    //                             make_highlight(
    //                                 "#16c6f3",
    //                                 root_definition.type_to_str(),
    //                                 serialized.trim_start_matches("op "),
    //                             )
    //                         } else if serialized.starts_with("get ") {
    //                             make_highlight(
    //                                 "#25c2a0",
    //                                 root_definition.type_to_str(),
    //                                 serialized.trim_start_matches("get "),
    //                             )
    //                         } else if serialized.starts_with("set ") {
    //                             make_highlight(
    //                                 "#25c2a0",
    //                                 root_definition.type_to_str(),
    //                                 serialized.trim_start_matches("set "),
    //                             )
    //                         } else if serialized.starts_with("index get ") {
    //                             make_highlight(
    //                                 "#25c2a0",
    //                                 root_definition.type_to_str(),
    //                                 serialized.trim_start_matches("index get "),
    //                             )
    //                         } else if serialized.starts_with("index set ") {
    //                             make_highlight(
    //                                 "#25c2a0",
    //                                 root_definition.type_to_str(),
    //                                 serialized.trim_start_matches("index set "),
    //                             )
    //                         } else {
    //                             make_highlight(
    //                                 "#C6cacb",
    //                                 root_definition.type_to_str(),
    //                                 serialized.trim_start_matches("fn "),
    //                             )
    //                         }
    //                     }
    //                 }
    //                 Item::CustomType { metadata, .. } => {
    //                     signatures += &make_highlight("#C6cacb", "type", &metadata.display_name);
    //                 }
    //             }
    //         }

    //         signatures
    //     };

    //     // FIXME: this only works for docusaurus.
    //     let mut mg = Glossary {
    //         content: if name == "global" {
    //             format!(
    //                 "{} \n\n### {}\n{}",
    //                 include_str!("components/highlight.js"),
    //                 name,
    //                 signatures
    //             )
    //         } else {
    //             format!("### {name}\n{signatures}")
    //         },
    //     };

    //     // Generate signatures for each submodule. (if any)
    //     if let Some(sub_modules) = &metadata.modules {
    //         for (sub_module, value) in sub_modules {
    //             mg.content.push_str(&{
    //                 let mg = generate_module_glossary_inner(
    //                     options,
    //                     Some(format!("{namespace}/{sub_module}")),
    //                     sub_module,
    //                     &serde_json::from_value::<ModuleMetadata>(value.clone())
    //                         .map_err(Error::ParseModuleMetadata)?,
    //                 )?;

    //                 mg.content
    //             });
    //         }
    //     }

    //     Ok(mg)
    // }
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
