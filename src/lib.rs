#![doc = include_str!("../README.md")]

pub mod error;
pub mod function;
pub mod module;
pub mod options;

use function::FunctionMetadata;
pub use module::ModuleDocumentation;
use module::ModuleMetadata;
use options::Options;
pub use options::{options, FunctionOrder, SectionFormat};

use error::AutodocsError;

/// NOTE: mdbook handles this automatically, but other
///       markdown processors might not.
/// Remove lines of code that starts with the '#' token,
/// which are removed on rust docs automatically.
fn remove_test_code(doc_comments: &str) -> String {
    let mut formatted = vec![];
    let mut in_code_block = false;
    for line in doc_comments.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            formatted.push(line);
            continue;
        }

        if !(in_code_block && line.starts_with("# ")) {
            formatted.push(line);
        }
    }

    formatted.join("\n")
}

/// Remove doc comments identifiers.
fn fmt_doc_comments(dc: String) -> String {
    dc.replace("/// ", "")
        .replace("///", "")
        .replace("/**", "")
        .replace("**/", "")
        .replace("**/", "")
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
fn generate_documentation(
    engine: &rhai::Engine,
    options: Options,
) -> Result<ModuleDocumentation, AutodocsError> {
    let json_fns = engine
        .gen_fn_metadata_to_json(options.include_standard_packages)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    let metadata = serde_json::from_str::<ModuleMetadata>(&json_fns)
        .map_err(|error| AutodocsError::Metadata(error.to_string()))?;

    generate_module_documentation(engine, &options, None, "global", metadata)
}

fn generate_module_documentation(
    engine: &rhai::Engine,
    options: &Options,
    namespace: Option<String>,
    name: impl Into<String>,
    metadata: ModuleMetadata,
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

    if let Some(functions) = metadata.functions {
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
            .order_function_groups(&namespace, function_groups)?;

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
                engine,
                options,
                &name.replace("get$", "").replace("set$", ""),
                &polymorphisms[..],
            ) {
                md.documentation += &fn_doc;
            }
        }
    }

    // Generate documentation for each submodule. (if any)
    if let Some(sub_modules) = metadata.modules {
        for (sub_module, value) in sub_modules {
            md.sub_modules.push(generate_module_documentation(
                engine,
                options,
                Some(format!("{}/{}", namespace, sub_module)),
                &sub_module,
                serde_json::from_value::<ModuleMetadata>(value)
                    .map_err(|error| AutodocsError::Metadata(error.to_string()))?,
            )?);
        }
    }

    Ok(md)
}

/// Generate markdown/html documentation for a function.
/// TODO: Add other word processors.
fn generate_function_documentation(
    engine: &rhai::Engine,
    options: &Options,
    name: &str,
    polymorphisms: &[&FunctionMetadata],
) -> Option<String> {
    // Takes the first valid comments found for a function group.
    let metadata = polymorphisms
        .iter()
        .find(|metadata| metadata.doc_comments.is_some())?;
    let root_definition = generate_function_definition(engine, metadata);

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
                        .map(|metadata| generate_function_definition(engine, metadata))
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
                        .map(|metadata| generate_function_definition(engine, metadata))
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

fn is_operator(name: &str) -> bool {
    ["==", "!=", ">", ">=", "<", "<=", "in"]
        .into_iter()
        .any(|op| op == name)
}

/// Generate a pseudo-Rust definition of a rhai function.
/// e.g. `fn my_func(a: int) -> ()`
fn generate_function_definition(engine: &rhai::Engine, metadata: &FunctionMetadata) -> String {
    // Add the operator / function prefix.
    let mut definition = if is_operator(&metadata.name) {
        String::from("op ")
    } else {
        String::from("fn ")
    };

    // Add getter and setter prefix + the name of the function.
    if let Some(name) = metadata.name.strip_prefix("get$") {
        definition += &format!("get {name}(");
    } else if let Some(name) = metadata.name.strip_prefix("set$") {
        definition += &format!("set {name}(");
    } else {
        definition += &format!("{}(", metadata.name);
    }

    let mut first = true;

    // Add params with their types.
    for i in 0..metadata.num_params {
        if !first {
            definition += ", ";
        }
        first = false;

        let (param_name, param_type) = metadata
            .params
            .as_ref()
            .expect("metadata.num_params does not match the number of parameters")
            .get(i)
            .map_or(("_", "?".into()), |s| {
                (
                    s.get("name").map(|s| s.as_str()).unwrap_or("_"),
                    s.get("type").map_or(std::borrow::Cow::Borrowed("?"), |ty| {
                        def_type_name(ty, engine)
                    }),
                )
            });

        definition += &format!("{param_name}: {param_type}");
    }

    // Add an eventual return type.
    if let Some(return_type) = &metadata.return_type {
        definition + format!(") -> {}", def_type_name(return_type, engine)).as_str()
    } else {
        definition + ")"
    }
}

/// This is the code a private function in the rhai crate. It is used to map
/// "Rust" types to a more user readable format. Here is the documentation of the
/// original function:
///
/// """
/// We have to transform some of the types.
///
/// This is highly inefficient and is currently based on trial and error with the core packages.
///
/// It tries to flatten types, removing `&` and `&mut`, and paths, while keeping generics.
///
/// Associated generic types are also rewritten into regular generic type parameters.
/// """
fn def_type_name<'a>(ty: &'a str, _: &'a rhai::Engine) -> std::borrow::Cow<'a, str> {
    let ty = ty.strip_prefix("&mut").unwrap_or(ty).trim();
    let ty = remove_result(ty);
    // Removes namespaces for the type.
    let ty = ty.split("::").last().unwrap();

    let ty = ty
        .replace("Iterator<Item=", "Iterator<")
        .replace("Dynamic", "?")
        .replace("INT", "int")
        .replace(std::any::type_name::<rhai::INT>(), "int")
        .replace("FLOAT", "float")
        .replace("&str", "String")
        .replace("ImmutableString", "String");

    let ty = ty.replace(std::any::type_name::<rhai::FLOAT>(), "float");
    let ty = ty.replace(std::any::type_name::<rhai::Array>(), "Array");
    let ty = ty.replace(std::any::type_name::<rhai::Blob>(), "Blob");
    let ty = ty.replace(std::any::type_name::<rhai::Map>(), "Map");
    let ty = ty.replace(std::any::type_name::<rhai::Instant>(), "Instant");
    let ty = ty.replace(std::any::type_name::<rhai::FnPtr>(), "FnPtr");

    ty.into()
}

/// Remove the result wrapper for a return type since it can be confusing in the documentation
/// NOTE: should we replace the wrapper by a '!' character or a tag on the function definition ?
fn remove_result(ty: &str) -> &str {
    if let Some(ty) = ty.strip_prefix("Result<") {
        ty.strip_suffix(",Box<EvalAltResult>>")
            .or_else(|| ty.strip_suffix(",Box<rhai::EvalAltResult>>"))
            .or_else(|| ty.strip_suffix(", Box<EvalAltResult>>"))
            .or_else(|| ty.strip_suffix(", Box<rhai::EvalAltResult>>"))
    } else if let Some(ty) = ty.strip_prefix("EngineResult<") {
        ty.strip_suffix('>')
    } else if let Some(ty) = ty
        .strip_prefix("RhaiResultOf<")
        .or_else(|| ty.strip_prefix("rhai::RhaiResultOf<"))
    {
        ty.strip_suffix('>')
    } else {
        None
    }
    .map_or(ty, str::trim)
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_remove_test_code_simple() {
        pretty_assertions::assert_eq!(
            remove_test_code(
                r#"
# Not removed.
```
fn my_func(a: int) -> () {}
do stuff ...
# Please hide this.
do something else ...
# Also this.
```
# Not removed either.
"#,
            ),
            r#"
# Not removed.
```
fn my_func(a: int) -> () {}
do stuff ...
do something else ...
```
# Not removed either."#,
        )
    }

    #[test]
    fn test_remove_test_code_multiple_blocks() {
        pretty_assertions::assert_eq!(
            remove_test_code(
                r#"
```ignore
block 1
# Please hide this.
```

# A title

```
block 2
# Please hide this.
john
doe
# To hide.
```
"#,
            ),
            r#"
```ignore
block 1
```

# A title

```
block 2
john
doe
```"#,
        )
    }

    #[test]
    fn test_remove_test_code_with_rhai_map() {
        pretty_assertions::assert_eq!(
            remove_test_code(
                r#"
```rhai
#{
    "a": 1,
    "b": 2,
    "c": 3,
};
# Please hide this.
```

# A title

```
# Please hide this.
let map = #{
    "hello": "world"
# To hide.
};
# To hide.
```
"#,
            ),
            r#"
```rhai
#{
    "a": 1,
    "b": 2,
    "c": 3,
};
```

# A title

```
let map = #{
    "hello": "world"
};
```"#,
        )
    }

    #[test]
    fn test_remove_result() {
        assert_eq!("Cache", remove_result("Result<Cache, Box<EvalAltResult>>"));
        assert_eq!("Cache", remove_result("Result<Cache,Box<EvalAltResult>>"));
        assert_eq!(
            "&mut Cache",
            remove_result("Result<&mut Cache, Box<EvalAltResult>>")
        );
        assert_eq!(
            "Cache",
            remove_result("Result<Cache, Box<rhai::EvalAltResult>>")
        );
        assert_eq!(
            "Cache",
            remove_result("Result<Cache,Box<rhai::EvalAltResult>>")
        );
        assert_eq!("Stuff", remove_result("EngineResult<Stuff>"));
        assert_eq!("Stuff", remove_result("RhaiResultOf<Stuff>"));
        assert_eq!("Stuff", remove_result("rhai::RhaiResultOf<Stuff>"));
    }

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

        engine.register_static_module("my_module", exported_module!(my_module).into());

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
