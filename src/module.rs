use crate::function;
use crate::item::Item;
use crate::{custom_types, export::Options};
use serde::{Deserialize, Serialize};

/// rhai-autodocs failed to export documentation for a module.
#[derive(Debug)]
pub enum Error {
    /// Something went wrong when parsing the `# rhai-autodocs:index` preprocessor.
    ParseOrderMetadata(std::num::ParseIntError),
    /// Something went wrong during the parsing of the module metadata.
    ParseModuleMetadata(serde_json::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ParseOrderMetadata(error) =>
                    format!("failed to parse function ordering: {error}"),
                Self::ParseModuleMetadata(error) =>
                    format!("failed to parse function or module metadata: {error}"),
            }
        )
    }
}

/// Rhai module documentation parsed from a definitions exported by a rhai engine.
#[derive(Debug)]
pub struct Documentation {
    /// Complete path to the module.
    pub namespace: String,
    /// Name of the module.
    pub name: String,
    /// Sub modules.
    pub sub_modules: Vec<Documentation>,
    /// Module documentation as raw text.
    pub documentation: String,
    /// Documentation items found in the module.
    pub items: Vec<Item>,
}

/// Intermediatory representation of the documentation.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModuleMetadata {
    /// Optional documentation for the module.
    pub doc: Option<String>,
    /// Functions metadata, if any.
    pub functions: Option<Vec<function::Metadata>>,
    /// Custom types metadata, if any.
    pub custom_types: Option<Vec<custom_types::Metadata>>,
    /// Sub-modules, if any, stored as raw json values.
    pub modules: Option<serde_json::Map<String, serde_json::Value>>,
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
pub(crate) fn generate_module_documentation(
    engine: &rhai::Engine,
    options: &Options,
) -> Result<Documentation, Error> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(Error::ParseModuleMetadata)?;

    let metadata =
        serde_json::from_str::<ModuleMetadata>(&json_fns).map_err(Error::ParseModuleMetadata)?;

    generate_module_documentation_inner(options, None, "global", &metadata)
}

fn generate_module_documentation_inner(
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: &ModuleMetadata,
) -> Result<Documentation, Error> {
    let name = name.into();
    let namespace = namespace.map_or(name.clone(), |namespace| namespace);
    // Format the module doc comments to make them
    // readable markdown.
    let documentation = metadata
        .doc
        .clone()
        .map(|dc| Item::remove_test_code(&Item::fmt_doc_comments(&dc)))
        .unwrap_or_default();

    let mut md = Documentation {
        namespace: namespace.clone(),
        name,
        documentation,
        sub_modules: vec![],
        items: vec![],
    };

    let mut items = vec![];

    if let Some(types) = &metadata.custom_types {
        for ty in types {
            items.push(Item::new_custom_type(ty.clone(), options)?);
        }
    }

    if let Some(functions) = &metadata.functions {
        for (name, polymorphisms) in group_functions(functions) {
            if let Ok(doc_item) = Item::new_function(&polymorphisms[..], &name, options) {
                items.push(doc_item);
            }
        }
    }

    // Remove ignored documentation.
    let items = items.into_iter().flatten().collect::<Vec<Item>>();

    md.items = options.items_order.order_items(items);

    // Generate documentation for each submodule. (if any)
    if let Some(sub_modules) = &metadata.modules {
        for (sub_module, value) in sub_modules {
            md.sub_modules.push(generate_module_documentation_inner(
                options,
                Some(format!("{namespace}/{sub_module}")),
                sub_module,
                &serde_json::from_value::<ModuleMetadata>(value.clone())
                    .map_err(Error::ParseModuleMetadata)?,
            )?);
        }
    }

    Ok(md)
}

pub(crate) fn group_functions(
    functions: &[function::Metadata],
) -> std::collections::HashMap<String, Vec<function::Metadata>> {
    let mut function_groups =
        std::collections::HashMap::<String, Vec<function::Metadata>>::default();

    // Rhai function can be polymorphes, so we group them by name.
    for metadata in functions {
        // Remove getter/setter prefixes to group them and indexers.
        let name = metadata.generate_function_definition().name();

        match function_groups.get_mut(&name) {
            Some(polymorphisms) => polymorphisms.push(metadata.clone()),
            None => {
                function_groups.insert(name.to_string(), vec![metadata.clone()]);
            }
        };
    }

    function_groups
}

/// Glossary of all function for a module and it's submodules.
#[derive(Debug)]
pub struct Glossary {
    /// Formatted function signatures by submodules.
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
pub(crate) fn generate_module_glossary(
    engine: &rhai::Engine,
    options: &Options,
) -> Result<Glossary, Error> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(Error::ParseModuleMetadata)?;

    let metadata =
        serde_json::from_str::<ModuleMetadata>(&json_fns).map_err(Error::ParseModuleMetadata)?;

    generate_module_glossary_inner(options, None, "global", &metadata)
}

#[allow(clippy::too_many_lines)]
fn generate_module_glossary_inner(
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: &ModuleMetadata,
) -> Result<Glossary, Error> {
    fn make_highlight(color: &str, item_type: &str, definition: &str) -> String {
        format!("- <Highlight color=\"{color}\">{item_type}</Highlight> <code>{{\"{definition}\"}}</code>\n",)
    }

    let name = name.into();
    let namespace = namespace.map_or(name.clone(), |namespace| namespace);
    let mut items = if let Some(types) = &metadata.custom_types {
        types
            .iter()
            .map(|metadata| Item::new_custom_type(metadata.clone(), options))
            .collect::<Result<Vec<_>, Error>>()?
    } else {
        vec![]
    };

    items.extend(if let Some(functions) = &metadata.functions {
        let groups = group_functions(functions);
        groups
            .iter()
            .map(|(name, metadata)| Item::new_function(metadata, name, options))
            .collect::<Result<Vec<_>, Error>>()?
    } else {
        vec![]
    });

    // Remove ignored documentation.
    let items = items.into_iter().flatten().collect::<Vec<Item>>();

    let items = options.items_order.order_items(items);

    let signatures = {
        let mut signatures = String::default();

        for item in &items {
            match item {
                Item::Function { metadata, .. } => {
                    for m in metadata {
                        let root_definition = m.generate_function_definition();

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
                        }
                    }
                }
                Item::CustomType { metadata, .. } => {
                    signatures += &make_highlight("#C6cacb", "type", &metadata.display_name);
                }
            }
        }

        signatures
    };

    // FIXME: this only works for docusaurus.
    let mut mg = Glossary {
        content: if name == "global" {
            format!(
                "{} \n\n### {}\n{}",
                include_str!("components/highlight.js"),
                name,
                signatures
            )
        } else {
            format!("### {name}\n{signatures}")
        },
    };

    // Generate signatures for each submodule. (if any)
    if let Some(sub_modules) = &metadata.modules {
        for (sub_module, value) in sub_modules {
            mg.content.push_str(&{
                let mg = generate_module_glossary_inner(
                    options,
                    Some(format!("{namespace}/{sub_module}")),
                    sub_module,
                    &serde_json::from_value::<ModuleMetadata>(value.clone())
                        .map_err(Error::ParseModuleMetadata)?,
                )?;

                mg.content
            });
        }
    }

    Ok(mg)
}

#[cfg(test)]
mod test {
    use crate::export::{self, ItemsOrder};

    use rhai::plugin::*;

    /// My own module.
    #[export_module]
    mod my_module {
        /// A function that prints to stdout.
        ///
        /// # rhai-autodocs:index:1
        #[rhai_fn(global)]
        pub fn hello_world() {
            println!("Hello, World!");
        }

        /// A function that adds two integers together.
        ///
        /// # rhai-autodocs:index:2
        #[rhai_fn(global)]
        pub const fn add(a: rhai::INT, b: rhai::INT) -> rhai::INT {
            a + b
        }

        /// This ust be hidden.
        #[rhai_fn(global)]
        pub const fn hide(a: rhai::INT, b: rhai::INT) -> rhai::INT {
            a + b
        }
    }

    #[test]
    fn test_order_by_index() {
        let mut engine = rhai::Engine::new();

        // register custom functions and types ...
        engine.register_static_module("my_module", rhai::exported_module!(my_module).into());

        // export documentation with option.
        let docs = export::options()
            .include_standard_packages(false)
            .order_items_with(ItemsOrder::ByIndex)
            .export(&engine)
            .expect("failed to generate documentation");

        let docs = crate::generate::docusaurus().generate(&docs).unwrap();

        pretty_assertions::assert_eq!(
                docs.get("global")
                .unwrap(),
            "---\ntitle: global\nslug: /global\n---\n\nimport Tabs from '@theme/Tabs';\nimport TabItem from '@theme/TabItem';\n\n```Namespace: global```\n\n\n\n"
        );

        pretty_assertions::assert_eq!(
            docs.get("my_module").unwrap(),
            r#"---
title: my_module
slug: /my_module
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

```Namespace: global/my_module```

My own module.


## <code>fn</code> hello_world

```js
fn hello_world()
```

<Tabs>
    <TabItem value="Description" default>

        A function that prints to stdout.
    </TabItem>
</Tabs>

## <code>fn</code> add

```js
fn add(a: int, b: int) -> int
```

<Tabs>
    <TabItem value="Description" default>

        A function that adds two integers together.
    </TabItem>
</Tabs>
"#
        );
    }
}
