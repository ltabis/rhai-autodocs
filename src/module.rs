pub mod error;
pub mod options;

use serde::{Deserialize, Serialize};

use crate::custom_types::CustomTypesMetadata;
use crate::doc_item::DocItem;
use crate::function::FunctionMetadata;

use self::error::AutodocsError;
use self::options::Options;

pub use self::options::options;

#[derive(Debug)]
/// Rhai module documentation in markdown format.
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

impl ModuleMetadata {
    /// Format the module doc comments to make them
    /// readable markdown.
    pub fn fmt_doc_comments(&self) -> Option<String> {
        self.doc
            .clone()
            .map(|dc| DocItem::remove_test_code(&DocItem::fmt_doc_comments(dc)))
    }
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
pub fn generate_module_documentation(
    engine: &rhai::Engine,
    options: &Options,
    hbs_registry: &handlebars::Handlebars,
) -> Result<ModuleDocumentation, AutodocsError> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    let metadata = serde_json::from_str::<ModuleMetadata>(&json_fns)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    generate_module_documentation_inner(options, None, "global", &metadata, hbs_registry)
}

fn generate_module_documentation_inner(
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: &ModuleMetadata,
    hbs_registry: &handlebars::Handlebars,
) -> Result<ModuleDocumentation, AutodocsError> {
    let name = name.into();
    let namespace = namespace.map_or(name.clone(), |namespace| namespace);

    let documentation = match options.markdown_processor {
        options::MarkdownProcessor::MdBook => hbs_registry
            .render(
                "mdbook-header",
                &std::collections::BTreeMap::from_iter([
                    ("title".to_string(), name.clone()),
                    ("namespace".to_string(), namespace.clone()),
                    (
                        "body".to_string(),
                        metadata
                            .fmt_doc_comments()
                            .map_or_else(String::default, |doc| format!("{doc}\n\n")),
                    ),
                ]),
            )
            .unwrap(),
        options::MarkdownProcessor::Docusaurus => hbs_registry
            .render(
                "docusaurus-header",
                &std::collections::BTreeMap::from_iter([
                    ("title".to_string(), name.clone()),
                    ("slug".to_string(), namespace.clone()),
                    ("namespace".to_string(), namespace.clone()),
                    (
                        "body".to_string(),
                        metadata
                            .fmt_doc_comments()
                            .map_or_else(String::default, |doc| format!("{doc}\n\n")),
                    ),
                ]),
            )
            .unwrap(),
    };

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
            items.push(DocItem::new_custom_type(
                ty.clone(),
                &namespace,
                options,
                hbs_registry,
            )?);
        }
    }

    if let Some(functions) = &metadata.functions {
        for (name, polymorphisms) in group_functions(functions) {
            if let Ok(doc_item) = DocItem::new_function(
                &polymorphisms[..],
                name.replace("get$", "").replace("set$", "").as_str(),
                &namespace,
                options,
                hbs_registry,
            ) {
                items.push(doc_item);
            }
        }
    }

    md.items = options.items_order.order_items(items);

    for items in &md.items {
        md.documentation += items.docs();
    }

    // Generate documentation for each submodule. (if any)
    if let Some(sub_modules) = &metadata.modules {
        for (sub_module, value) in sub_modules {
            md.sub_modules.push(generate_module_documentation_inner(
                options,
                Some(format!("{}/{}", namespace, sub_module)),
                sub_module,
                &serde_json::from_value::<ModuleMetadata>(value.clone())
                    .map_err(|error| AutodocsError::Metadata(error.to_string()))?,
                hbs_registry,
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
        match function_groups.get_mut(&metadata.name) {
            Some(polymorphisms) => polymorphisms.push(metadata.clone()),
            None => {
                function_groups.insert(metadata.name.clone(), vec![metadata.clone()]);
            }
        };
    });

    function_groups
}

#[cfg(test)]
mod test {
    use crate::module::options::ItemsOrder;

    use super::*;
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
    }

    #[test]
    fn test_order_by_index() {
        let mut engine = rhai::Engine::new();

        engine.register_static_module("my_module", rhai::exported_module!(my_module).into());

        // register custom functions and types ...
        let docs = options::options()
            .include_standard_packages(false)
            .order_items_with(ItemsOrder::ByIndex)
            .for_markdown_processor(options::MarkdownProcessor::MdBook)
            .generate(&engine)
            .expect("failed to generate documentation");

        assert_eq!(docs.name, "global");
        assert_eq!(
            docs.documentation,
            "# global\n\n```Namespace: global```\n\n"
        );

        let my_module = &docs.sub_modules[0];

        assert_eq!(my_module.name, "my_module");
        pretty_assertions::assert_eq!(
            my_module.documentation,
            r#"# my_module

```Namespace: global/my_module```

My own module.

<div markdown="span" style='box-shadow: 0 4px 8px 0 rgba(0,0,0,0.2); padding: 15px; border-radius: 5px;'>
    <h2 class="func-name"> <code>fn</code> hello_world </h2>

    ```rust,ignore
    fn hello_world()
    ```

    <details>
    <summary markdown="span"> details </summary>

    A function that prints to stdout.
</details>
</div>
</br>
<div markdown="span" style='box-shadow: 0 4px 8px 0 rgba(0,0,0,0.2); padding: 15px; border-radius: 5px;'>
    <h2 class="func-name"> <code>fn</code> add </h2>

    ```rust,ignore
    fn add(a: int, b: int) -> int
    ```

    <details>
    <summary markdown="span"> details </summary>

    A function that adds two integers together.
</details>
</div>
</br>
"#
        );
    }
}
