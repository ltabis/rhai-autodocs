pub mod error;
pub mod options;

use self::{error::AutodocsError, options::Options};
use crate::custom_types::CustomTypesMetadata;
use crate::doc_item::DocItem;
use crate::function::FunctionMetadata;
use serde::{Deserialize, Serialize};

/// Rhai module documentation in markdown format.
#[derive(Debug)]
pub struct ModuleDocumentation {
    /// Complete path to the module.
    pub namespace: String,
    /// Name of the module.
    pub name: String,
    /// Sub modules.
    pub sub_modules: Vec<ModuleDocumentation>,
    /// Module documentation as raw text.
    pub documentation: String,
    /// Documentation items found in the module.
    pub items: Vec<DocItem>,
}

/// Intermediatory representation of the documentation.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModuleMetadata {
    /// Optional documentation for the module.
    pub doc: Option<String>,
    /// Functions metadata, if any.
    pub functions: Option<Vec<FunctionMetadata>>,
    /// Custom types metadata, if any.
    pub custom_types: Option<Vec<CustomTypesMetadata>>,
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
) -> Result<ModuleDocumentation, AutodocsError> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    let metadata = serde_json::from_str::<ModuleMetadata>(&json_fns)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    generate_module_documentation_inner(options, None, "global", &metadata)
}

fn generate_module_documentation_inner(
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: &ModuleMetadata,
) -> Result<ModuleDocumentation, AutodocsError> {
    let name = name.into();
    let namespace = namespace.map_or(name.clone(), |namespace| namespace);
    // Format the module doc comments to make them
    // readable markdown.
    let documentation = metadata
        .doc
        .clone()
        .map(|dc| DocItem::remove_test_code(&DocItem::fmt_doc_comments(dc)))
        .unwrap_or_default();

    let mut md = ModuleDocumentation {
        namespace: namespace.clone(),
        name,
        documentation,
        sub_modules: vec![],
        items: vec![],
    };

    let mut items = vec![];

    if let Some(types) = &metadata.custom_types {
        for ty in types {
            items.push(DocItem::new_custom_type(ty.clone(), options)?);
        }
    }

    if let Some(functions) = &metadata.functions {
        for (name, polymorphisms) in group_functions(functions) {
            if let Ok(doc_item) = DocItem::new_function(&polymorphisms[..], &name, options) {
                items.push(doc_item);
            }
        }
    }

    // Remove ignored documentation.
    let items = items.into_iter().flatten().collect::<Vec<DocItem>>();

    md.items = options.items_order.order_items(items);

    // Generate documentation for each submodule. (if any)
    if let Some(sub_modules) = &metadata.modules {
        for (sub_module, value) in sub_modules {
            md.sub_modules.push(generate_module_documentation_inner(
                options,
                Some(format!("{}/{}", namespace, sub_module)),
                sub_module,
                &serde_json::from_value::<ModuleMetadata>(value.clone())
                    .map_err(|error| AutodocsError::Metadata(error.to_string()))?,
            )?);
        }
    }

    Ok(md)
}

pub(crate) fn group_functions(
    functions: &[FunctionMetadata],
) -> std::collections::HashMap<String, Vec<FunctionMetadata>> {
    let mut function_groups = std::collections::HashMap::<String, Vec<FunctionMetadata>>::default();

    // Rhai function can be polymorphes, so we group them by name.
    functions.iter().for_each(|metadata| {
        // Remove getter/setter prefixes to group them and indexers.
        let name = metadata.generate_function_definition().name();

        match function_groups.get_mut(&name) {
            Some(polymorphisms) => polymorphisms.push(metadata.clone()),
            None => {
                function_groups.insert(name.to_string(), vec![metadata.clone()]);
            }
        };
    });

    function_groups
}

#[cfg(test)]
mod test {
    use crate::{export, generate, module::options::ItemsOrder};

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
        pub fn add(a: rhai::INT, b: rhai::INT) -> rhai::INT {
            a + b
        }

        /// This ust be hidden.
        #[rhai_fn(global)]
        pub fn hide(a: rhai::INT, b: rhai::INT) -> rhai::INT {
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

        let docs = generate::docusaurus().build(&docs).unwrap();

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
