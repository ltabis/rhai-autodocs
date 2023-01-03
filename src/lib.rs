use serde::{Deserialize, Serialize};

/// Rhai module documentation in markdown format.
pub struct ModuleDocumentation {
    /// Name of the module.
    pub name: String,
    /// Sub modules.
    pub sub_modules: Vec<ModuleDocumentation>,
    /// Raw text documentation in markdown.
    pub documentation: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ModuleMetadata {
    doc: Option<String>,
    functions: Option<Vec<FunctionMetadata>>,
    modules: Option<serde_json::Map<String, serde_json::Value>>,
}

impl ModuleMetadata {
    pub fn fmt_doc_comments(&self) -> Option<String> {
        self.doc
            .clone()
            .map(|dc| remove_test_code(&fmt_doc_comments(dc)))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct FunctionMetadata {
    pub access: String,
    pub base_hash: u128,
    pub full_hash: u128,
    pub name: String,
    pub namespace: String,
    pub num_params: usize,
    pub params: Option<Vec<std::collections::HashMap<String, String>>>,
    pub signature: String,
    pub return_type: Option<String>,
    pub doc_comments: Option<Vec<String>>,
}

impl FunctionMetadata {
    pub fn fmt_doc_comments(&self) -> Option<String> {
        self.doc_comments
            .clone()
            .map(|dc| remove_test_code(&fmt_doc_comments(dc.join("\n"))))
    }
}
/// Remove doc comments identifiers.
fn fmt_doc_comments(dc: String) -> String {
    dc.replace("/// ", "")
        .replace("///", "")
        .replace("/**", "")
        .replace("**/", "")
        .replace("**/", "")
}

/// NOTE: might be useless because mdbook seems to handle this.
///       to keep in case we migrate to another md processor.
fn remove_test_code(doc_comments: &str) -> String {
    let mut formatted = vec![];
    let mut in_code_block = false;
    for line in doc_comments.lines() {
        if line.trim() == "```" {
            in_code_block = !in_code_block;
            formatted.push(line);
            continue;
        }

        if !(in_code_block && line.starts_with('#') && !line.starts_with("#{")) {
            formatted.push(line);
        }
    }

    formatted.join("\n")
}

/// Generate documentation based on an engine instance.
/// Make sure all the functions, operators, plugins, etc. are registered inside this instance.
///
/// # Result
/// * A vector of documented modules.
///
/// # Errors
/// *
pub fn generate_documentation(
    engine: &rhai::Engine,
    include_standard_packages: bool,
) -> Result<ModuleDocumentation, String> {
    let json_fns = engine
        .gen_fn_metadata_to_json(include_standard_packages)
        .map_err(|error| error.to_string())?;

    let metadata =
        serde_json::from_str::<ModuleMetadata>(&json_fns).map_err(|error| error.to_string())?;

    generate_module_documentation(&engine, "global", metadata)
}

fn generate_module_documentation(
    engine: &rhai::Engine,
    namespace: &str,
    metadata: ModuleMetadata,
) -> Result<ModuleDocumentation, String> {
    let mut md = ModuleDocumentation {
        name: namespace.to_owned(),
        sub_modules: vec![],
        documentation: format!(
            "# {namespace}\n\n{}",
            metadata
                .fmt_doc_comments()
                .map_or_else(String::default, |doc| format!("{doc}\n\n"))
        ),
    };

    if let Some(functions) = metadata.functions {
        let mut fn_groups = std::collections::HashMap::<String, Vec<&FunctionMetadata>>::default();

        functions.iter().for_each(|metadata| {
            match fn_groups.get_mut(&metadata.name) {
                Some(polymorphisms) => polymorphisms.push(metadata),
                None => {
                    fn_groups.insert(metadata.name.clone(), vec![metadata]);
                }
            };
        });

        let mut fn_groups = fn_groups
            .iter()
            .map(|(name, polymorphisms)| (name, polymorphisms))
            .collect::<Vec<_>>();
        fn_groups.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (name, polymorphisms) in fn_groups {
            if let Some(fn_doc) = generate_function_documentation(
                engine,
                &name.replace("get$", "").replace("set$", ""),
                &polymorphisms[..],
            ) {
                md.documentation += &fn_doc;
            }
        }
    }

    if let Some(sub_modules) = metadata.modules {
        for (sub_module, value) in sub_modules {
            md.sub_modules.push(generate_module_documentation(
                engine,
                &format!("{namespace}::{sub_module}"),
                serde_json::from_value::<ModuleMetadata>(value)
                    .map_err(|error| error.to_string())?,
            )?);
        }
    }

    Ok(md)
}

fn generate_function_documentation(
    engine: &rhai::Engine,
    name: &str,
    polymorphisms: &[&FunctionMetadata],
) -> Option<String> {
    let metadata = polymorphisms.first().expect("will never be empty");
    let root_definition = generate_function_definition(engine, metadata);

    if !name.starts_with("anon$") {
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
                .fmt_doc_comments()
                .map(|doc| format!(
                    r#"
<details>
<summary markdown="span"> details </summary>

{doc}
</details>
"#
                ))
                .unwrap_or_default()
        ))
    } else {
        None
    }
}

fn is_operator(name: &str) -> bool {
    ["==", "!=", ">", ">=", "<", "<=", "in"]
        .into_iter()
        .any(|op| op == name)
}

fn generate_function_definition(engine: &rhai::Engine, metadata: &FunctionMetadata) -> String {
    let mut definition = if is_operator(&metadata.name) {
        String::from("op ")
    } else {
        String::from("fn ")
    };

    if let Some(name) = metadata.name.strip_prefix("get$") {
        definition += &format!("get {name}(");
    } else if let Some(name) = metadata.name.strip_prefix("set$") {
        definition += &format!("set {name}(");
    } else {
        definition += &format!("{}(", metadata.name);
    }

    let mut first = true;

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

    if let Some(return_type) = &metadata.return_type {
        definition + format!(") -> {}", def_type_name(return_type, engine)).as_str()
    } else {
        definition + ")"
    }
}

/// We have to transform some of the types.
///
/// This is highly inefficient and is currently based on trial and error with the core packages.
///
/// It tries to flatten types, removing `&` and `&mut`, and paths, while keeping generics.
///
/// Associated generic types are also rewritten into regular generic type parameters.
fn def_type_name<'a>(ty: &'a str, _: &'a rhai::Engine) -> std::borrow::Cow<'a, str> {
    // let ty = engine.format_type_name(ty).replace("crate::", "");
    let ty = ty.strip_prefix("&mut").unwrap_or(ty).trim();
    let ty = remove_result(ty);
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
        ty.strip_suffix(", Box<EvalAltResult>>")
    } else if let Some(ty) = ty.strip_prefix("EngineResult<") {
        ty.strip_suffix('>')
    } else if let Some(ty) = ty.strip_prefix("RhaiResultOf<") {
        ty.strip_suffix('>')
    } else {
        None
    }
    .map_or(ty, str::trim)
}
