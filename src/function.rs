use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{
    fmt_doc_comments,
    module::options::{MarkdownProcessor, SectionFormat, RHAI_FUNCTION_INDEX_PATTERN},
    remove_test_code,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FunctionMetadata {
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

/// Remove crate specific comments, like `rhai-autodocs:index`.
fn remove_extra_tokens(dc: Vec<String>) -> Vec<String> {
    dc.into_iter()
        .map(|s| {
            s.lines()
                .filter(|l| !l.contains(RHAI_FUNCTION_INDEX_PATTERN))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
}

impl FunctionMetadata {
    /// Format the function doc comments to make them
    /// into readable markdown.
    pub fn fmt_doc_comments(
        &self,
        section_format: &SectionFormat,
        markdown_processor: &MarkdownProcessor,
    ) -> Option<String> {
        self.doc_comments.clone().map(|dc| {
            let removed_extra_tokens = remove_extra_tokens(dc).join("\n");
            let remove_comments = fmt_doc_comments(removed_extra_tokens);
            let remove_test_code = remove_test_code(&remove_comments);

            section_format.fmt_sections(&self.name, markdown_processor, remove_test_code)
        })
    }

    /// Generate a pseudo-Rust definition of a rhai function.
    /// e.g. `fn my_func(a: int) -> ()`
    pub fn generate_function_definition(&self) -> String {
        // Add getter and setter prefix + the name of the function.
        let definition = Definition::new(&self.name, self.params.as_ref().unwrap_or(&vec![]));

        definition.display(
            &self.name,
            self.return_type
                .as_deref()
                .and_then(|ty| def_type_name(ty).map(|s| s.to_string())),
        )
    }
}

fn is_operator(name: &str) -> bool {
    ["==", "!=", ">", ">=", "<", "<=", "in"]
        .into_iter()
        .any(|op| op == name)
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
fn def_type_name(ty: &str) -> Option<std::borrow::Cow<'_, str>> {
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

    if ty == "()" {
        None
    } else {
        Some(ty.into())
    }
}

/// Remove the result wrapper for a return type since it can be confusing in the documentation
/// NOTE: should we replace the wrapper by a '!' character or a tag on the function definition ?
fn remove_result(ty: &str) -> &str {
    if let Some(ty) = ty.strip_prefix("Result<") {
        ty.strip_suffix(",Box<EvalAltResult>>")
            .or_else(|| ty.strip_suffix(",Box<rhai::EvalAltResult>>"))
            .or_else(|| ty.strip_suffix(", Box<EvalAltResult>>"))
            .or_else(|| ty.strip_suffix(", Box<rhai::EvalAltResult>>"))
            .or_else(|| ty.strip_suffix('>'))
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

struct Arg {
    name: String,
    ty: String,
}

impl Arg {
    fn unknown() -> Self {
        Self {
            name: "_".to_string(),
            ty: "?".into(),
        }
    }
}

impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.ty)
    }
}

enum Definition {
    Function { args: Vec<Arg> },
    Operator { arg1: Arg, arg2: Arg },
    Get { target: Arg, index: Arg },
    Set { target: Arg, index: Arg, value: Arg },
    IndexGet { target: Arg, index: Arg },
    IndexSet { target: Arg, index: Arg, value: Arg },
}

impl Definition {
    fn new(name: &str, args: &[std::collections::HashMap<String, String>]) -> Self {
        fn get_arg(args: &[std::collections::HashMap<String, String>], index: usize) -> Arg {
            args.get(index).map_or(Arg::unknown(), |def| Arg {
                name: def
                    .get("name")
                    .map(|n| n.as_str())
                    .unwrap_or("_")
                    .to_string(),
                ty: def
                    .get("type")
                    .map_or(std::borrow::Cow::Borrowed("?"), |ty| {
                        def_type_name(ty).unwrap_or("?".into())
                    })
                    .to_string(),
            })
        }

        if is_operator(name) {
            Self::Operator {
                arg1: get_arg(args, 0),
                arg2: get_arg(args, 1),
            }
        } else if let Some(name) = name.strip_prefix("get$") {
            Self::Get {
                target: get_arg(args, 0),
                index: Arg {
                    name: name.to_string(),
                    ty: "_".to_string(),
                },
            }
        } else if let Some(name) = name.strip_prefix("set$") {
            Self::Set {
                target: get_arg(args, 0),
                index: Arg {
                    name: name.to_string(),
                    ty: "_".to_string(),
                },
                value: get_arg(args, 1),
            }
        } else if name.strip_prefix("index$get$").is_some() {
            Self::IndexGet {
                target: get_arg(args, 0),
                index: get_arg(args, 1),
            }
        } else if name.strip_prefix("index$set$").is_some() {
            Self::IndexSet {
                target: get_arg(args, 0),
                index: get_arg(args, 1),
                value: get_arg(args, 2),
            }
        } else {
            Self::Function {
                args: args
                    .iter()
                    .enumerate()
                    .map(|(index, _)| get_arg(args, index))
                    .collect::<Vec<Arg>>(),
            }
        }
    }

    fn display(&self, name: &str, return_type: Option<String>) -> String {
        match self {
            Definition::Function { args } => {
                format!(
                    "fn {}({})",
                    name,
                    args.iter()
                        .map(|arg| arg.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ) + return_type
                    .map_or(String::default(), |rt| format!(" -> {rt}"))
                    .as_str()
            }
            Definition::Operator { arg1, arg2 } => {
                format!("op {} {} {}", arg1.ty, name, arg2.ty)
                    + return_type
                        .map_or(")".to_string(), |rt| format!(" -> {rt}"))
                        .as_str()
            }
            Definition::Get { target, index } => {
                format!("get {}.{}", target.ty, index.name)
                    + return_type
                        .map_or(")".to_string(), |rt| format!(" -> {rt}"))
                        .as_str()
            }
            Definition::Set {
                target,
                index,
                value,
            } => {
                format!("set {}.{} = {}", target.ty, index.name, value.ty)
            }
            Definition::IndexGet { target, index } => {
                format!("index get {}[{}]", target.ty, index)
                    + return_type
                        .map_or(")".to_string(), |rt| format!(" -> {rt}"))
                        .as_str()
            }
            Definition::IndexSet {
                target,
                index,
                value,
            } => format!("index set {}[{}] = {}", target.ty, index, value.ty),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
}
