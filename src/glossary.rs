use crate::module::{error::AutodocsError, ModuleMetadata};

/// Glossary of all function for a module and it's submodules.
#[derive(Debug)]
pub struct ModuleGlossary {
    /// Formated function signatures by submodules.
    pub content: String,
}

/// Generate documentation based on an engine instance and a glossary of all functions.
/// Make sure all the functions, operators, plugins, etc. are registered inside this instance.
///
/// # Result
/// * A vector of documented modules.
///
/// # Errors
/// * Failed to generate function metadata as json.
/// * Failed to parse module metadata.
fn generate_glossary(
    engine: &rhai::Engine,
    options: &Options,
) -> Result<(ModuleDocumentation, ModuleGlossary), AutodocsError> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    let metadata = serde_json::from_str::<ModuleMetadata>(&json_fns)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    Ok((
        generate_module_documentation(engine, &options, None, "global", &metadata)?,
        generate_module_glossary(engine, options, None, "global", &metadata)?,
    ))
}

fn generate_module_glossary(
    engine: &rhai::Engine,
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: &ModuleMetadata,
) -> Result<ModuleGlossary, AutodocsError> {
    let name = name.into();
    let namespace = namespace.map_or(name.clone(), |namespace| namespace);

    let signatures = if let Some(functions) = &metadata.functions {
        let fn_groups = group_functions(options, &namespace, functions)?;
        let mut signatures = String::default();

        for (_, polymorphisms) in fn_groups {
            for p in polymorphisms {
                let root_definition = generate_function_definition(engine, p);
                // FIXME: this only works for docusaurus.
                // TODO: customize colors.
                signatures += &if root_definition.starts_with("op ") {
                    format!(
                        "- <Highlight color=\"#25c2a0\">op</Highlight> <code>{{\"{}\"}}</code>\n",
                        root_definition.trim_start_matches("op ")
                    )
                } else if root_definition.starts_with("fn get ") {
                    format!(
                        "- <Highlight color=\"#25c2a0\">get</Highlight> <code>{{\"{}\"}}</code>\n",
                        root_definition.trim_start_matches("fn get ")
                    )
                } else if root_definition.starts_with("fn set ") {
                    format!(
                        "- <Highlight color=\"#25c2a0\">set</Highlight> <code>{{\"{}\"}}</code>\n",
                        root_definition.trim_start_matches("fn set ")
                    )
                } else {
                    format!(
                        "- <Highlight color=\"#25c2a0\">fn</Highlight> <code>{{\"{}\"}}</code>\n",
                        root_definition.trim_start_matches("fn ")
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
                let mg = generate_module_glossary(
                    engine,
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
