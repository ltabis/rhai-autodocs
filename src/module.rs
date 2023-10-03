pub mod error;
pub mod options;

use serde::{Deserialize, Serialize};

use crate::function::FunctionMetadata;
use crate::{fmt_doc_comments, remove_test_code};

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
    /// Raw text documentation in markdown.
    pub documentation: String,
}

/// Intermediatory representation of the documentation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ModuleMetadata {
    /// Optional documentation for the module.
    pub doc: Option<String>,
    /// Functions metadata, if any.
    pub functions: Option<Vec<FunctionMetadata>>,
    /// Sub-modules, if any, stored as raw json values.
    pub modules: Option<serde_json::Map<String, serde_json::Value>>,
}

impl ModuleMetadata {
    /// Format the module doc comments to make them
    /// readable markdown.
    pub fn fmt_doc_comments(&self) -> Option<String> {
        self.doc
            .clone()
            .map(|dc| remove_test_code(&fmt_doc_comments(dc)))
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

    let documentation = match options.markdown_processor {
        options::MarkdownProcessor::MdBook => {
            format!(
                r#"# {}

```Namespace: {}```

{}"#,
                &name,
                &namespace,
                metadata
                    .fmt_doc_comments()
                    .map_or_else(String::default, |doc| format!("{doc}\n\n"))
            )
        }
        options::MarkdownProcessor::Docusaurus => {
            format!(
                r#"---
title: {}
slug: /{}
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

```Namespace: {}```

{}"#,
                &name,
                &namespace,
                &namespace,
                metadata
                    .fmt_doc_comments()
                    .map_or_else(String::default, |doc| format!("{doc}\n\n"))
            )
        }
    };

    let mut md = ModuleDocumentation {
        namespace: namespace.clone(),
        name,
        sub_modules: vec![],
        documentation,
    };

    if let Some(functions) = &metadata.functions {
        let fn_groups = group_functions(options, &namespace, functions)?;

        // Generate a clean documentation for each functions.
        // Functions that share the same name will keep only
        // one documentation, the others will be dropped.
        //
        // This means that:
        // ```rust
        // /// doc 1
        // fn my_func(a: int)`
        // ```
        // and
        // ```rust
        // /// doc 2
        // fn my_func(a: int, b: int)`
        // ```
        // will be written as the following:
        // ```rust
        // /// doc 1
        // fn my_func(a: int);
        // fn my_func(a: int, b: int);
        // ```
        for (name, polymorphisms) in fn_groups {
            if let Some(fn_doc) = generate_function_documentation(
                options,
                &name.replace("get$", "").replace("set$", ""),
                &polymorphisms[..],
            ) {
                md.documentation += &fn_doc;
            }
        }
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
            )?);
        }
    }

    Ok(md)
}

pub(crate) fn group_functions<'meta>(
    options: &Options,
    namespace: &str,
    functions: &'meta [FunctionMetadata],
) -> Result<Vec<(String, Vec<&'meta FunctionMetadata>)>, AutodocsError> {
    let mut function_groups =
        std::collections::HashMap::<String, Vec<&FunctionMetadata>>::default();

    // Rhai function can be polymorphes, so we group them by name.
    functions.iter().for_each(|metadata| {
        match function_groups.get_mut(&metadata.name) {
            Some(polymorphisms) => polymorphisms.push(metadata),
            None => {
                function_groups.insert(metadata.name.clone(), vec![metadata]);
            }
        };
    });

    let function_groups = function_groups
        .into_iter()
        .map(|(name, polymorphisms)| (name, polymorphisms))
        .collect::<Vec<_>>();

    let fn_groups = options
        .functions_order
        .order_function_groups(namespace, function_groups)?;

    Ok(fn_groups)
}

/// Generate markdown/html documentation for a function.
/// TODO: Add other word processors.
fn generate_function_documentation(
    options: &Options,
    name: &str,
    polymorphisms: &[&FunctionMetadata],
) -> Option<String> {
    // Takes the first valid comments found for a function group.
    let metadata = polymorphisms
        .iter()
        .find(|metadata| metadata.doc_comments.is_some())?;
    let root_definition = metadata.generate_function_definition();

    // Anonymous functions are ignored.
    if !name.starts_with("anon$") {
        match options.markdown_processor {
            options::MarkdownProcessor::MdBook => {
                Some(format!(
                    r#"
<div markdown="span" style='box-shadow: 0 4px 8px 0 rgba(0,0,0,0.2); padding: 15px; border-radius: 5px;'>

<h2 class="func-name"> <code>{}</code> {} </h2>

```rust,ignore
{}
```
{}
</div>
</br>
"#,
                    // Add a specific prefix for the function type documented.
                    if root_definition.starts_with("op") {
                        "op"
                    } else if root_definition.starts_with("fn get ") {
                        "get"
                    } else if root_definition.starts_with("fn set ") {
                        "set"
                    } else {
                        "fn"
                    },
                    name,
                    polymorphisms
                        .iter()
                        .map(|metadata| metadata.generate_function_definition())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    &metadata
                        .fmt_doc_comments(&options.sections_format, &options.markdown_processor)
                        .unwrap_or_default()
                ))
            }
            options::MarkdownProcessor::Docusaurus => {
                Some(format!(
                    r#"## <code>{}</code> {}
```js
{}
```
{}
"#,
                    // Add a specific prefix for the function type documented.
                    if root_definition.starts_with("op") {
                        "op"
                    } else if root_definition.starts_with("fn get ") {
                        "get"
                    } else if root_definition.starts_with("fn set ") {
                        "set"
                    } else {
                        "fn"
                    },
                    name,
                    polymorphisms
                        .iter()
                        .map(|metadata| metadata.generate_function_definition())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    &metadata
                        .fmt_doc_comments(&options.sections_format, &options.markdown_processor)
                        .unwrap_or_default()
                ))
            }
        }
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::module::options::FunctionOrder;
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
            .order_functions_with(FunctionOrder::ByIndex)
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
