use crate::module::{error::AutodocsError, group_functions, options::Options, ModuleMetadata};

/// Glossary of all function for a module and it's submodules.
#[derive(Debug)]
pub struct ModuleGlossary {
    /// Formated function signatures by submodules.
    pub content: String,
}

/// Generate documentation based on an engine instance and a glossary of all functions.
/// Make sure all the functions, operators, plugins, etc. are registered inside this instance.
///
/// # CAUTION
///
/// This only works for docusaurus at the moment.
///
/// # Result
/// * A vector of documented modules.
///
/// # Errors
/// * Failed to generate function metadata as json.
/// * Failed to parse module metadata.
pub fn generate_module_glossary(
    engine: &rhai::Engine,
    options: &Options,
) -> Result<ModuleGlossary, AutodocsError> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    let metadata = serde_json::from_str::<ModuleMetadata>(&json_fns)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    generate_child_module_glossary(options, None, "global", &metadata)
}

fn generate_child_module_glossary(
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: &ModuleMetadata,
) -> Result<ModuleGlossary, AutodocsError> {
    fn make_highlight(color: &str, item_type: &str, definition: &str) -> String {
        format!("- <Highlight color=\"{color}\">{item_type}</Highlight> <code>{{\"{definition}\"}}</code>\n",)
    }

    let name = name.into();
    let namespace = namespace.map_or(name.clone(), |namespace| namespace);

    let signatures = if let Some(functions) = &metadata.functions {
        let fn_groups = group_functions(options, &namespace, functions)?;
        let mut signatures = String::default();

        for (_, polymorphisms) in fn_groups {
            for fn_metadata in polymorphisms {
                let root_definition = fn_metadata.generate_function_definition();

                let serialized = root_definition.display();
                // FIXME: this only works for docusaurus.
                // TODO: customize colors.
                signatures += &if serialized.starts_with("op ") {
                    make_highlight(
                        "#16c6f3",
                        root_definition.type_to_str(),
                        serialized.trim_start_matches("op "),
                    )
                } else if serialized.starts_with("get ") {
                    make_highlight(
                        "#25c2a0",
                        root_definition.type_to_str(),
                        serialized.trim_start_matches("get "),
                    )
                } else if serialized.starts_with("set ") {
                    make_highlight(
                        "#25c2a0",
                        root_definition.type_to_str(),
                        serialized.trim_start_matches("set "),
                    )
                } else if serialized.starts_with("index get ") {
                    make_highlight(
                        "#25c2a0",
                        root_definition.type_to_str(),
                        serialized.trim_start_matches("index get "),
                    )
                } else if serialized.starts_with("index set ") {
                    make_highlight(
                        "#25c2a0",
                        root_definition.type_to_str(),
                        serialized.trim_start_matches("index set "),
                    )
                } else {
                    make_highlight(
                        "#C6cacb",
                        root_definition.type_to_str(),
                        serialized.trim_start_matches("fn "),
                    )
                };
            }
        }

        signatures
    } else {
        String::default()
    };

    // FIXME: this only works for docusaurus.
    let mut mg = ModuleGlossary {
        content: if name == "global" {
            format!(
                "{} \n\n### {}\n{}",
                include_str!("components/highlight.js"),
                name,
                signatures
            )
        } else {
            format!("### {}\n{}", name, signatures)
        },
    };

    // Generate signatures for each submodule. (if any)
    if let Some(sub_modules) = &metadata.modules {
        for (sub_module, value) in sub_modules {
            mg.content.push_str(&{
                let mg = generate_child_module_glossary(
                    options,
                    Some(format!("{}/{}", namespace, sub_module)),
                    sub_module,
                    &serde_json::from_value::<ModuleMetadata>(value.clone())
                        .map_err(|error| AutodocsError::Metadata(error.to_string()))?,
                )?;

                mg.content
            });
        }
    }

    Ok(mg)
}
