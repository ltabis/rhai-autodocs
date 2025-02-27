use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// Metadata exposed by Rhai for functions.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
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

impl Metadata {
    /// Generate a pseudo-Rust definition of a rhai function.
    /// e.g. `fn my_func(a: int) -> ()`
    pub fn generate_function_definition(&self) -> Definition {
        Definition::new(
            &self.name,
            self.params.as_ref().unwrap_or(&vec![]),
            self.return_type.as_deref(),
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
fn def_type_name(ty: &str) -> Option<String> {
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

    #[cfg(not(feature = "no_float"))]
    let ty = ty.replace(std::any::type_name::<rhai::FLOAT>(), "float");
    #[cfg(not(feature = "no_index"))]
    let ty = ty.replace(std::any::type_name::<rhai::Array>(), "Array");
    #[cfg(not(feature = "no_index"))]
    let ty = ty.replace(std::any::type_name::<rhai::Blob>(), "Blob");
    #[cfg(not(feature = "no_object"))]
    let ty = ty.replace(std::any::type_name::<rhai::Map>(), "Map");
    #[cfg(not(feature = "no_time"))]
    let ty = ty.replace(std::any::type_name::<rhai::Instant>(), "Instant");
    #[cfg(not(feature = "no_time"))]
    let ty = ty.replace(std::any::type_name::<rhai::FnPtr>(), "FnPtr");

    if ty == "()" {
        None
    } else {
        Some(ty)
    }
}

/// Remove the result wrapper for a return type since it can be confusing in the documentation
/// NOTE: should we replace the wrapper by a '!' character or a tag on the function definition ?
fn remove_result(ty: &str) -> &str {
    let without_result = ty
        .split_once("Result<")
        .or_else(|| ty.split_once("RhaiResultOf<"))
        .map(|(_, ty)| ty);

    without_result
        .and_then(|ty| {
            ty.split_once(',')
                .or_else(|| ty.split_once('>'))
                .map(|(ty, _)| ty)
        })
        .map_or(ty, str::trim)
}

pub struct Arg {
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

pub enum Definition {
    Function {
        name: String,
        args: Vec<Arg>,
        return_type: Option<String>,
    },
    Operator {
        name: String,
        arg1: Arg,
        arg2: Arg,
        return_type: Option<String>,
    },
    Get {
        target: Arg,
        index: Arg,
        return_type: Option<String>,
    },
    Set {
        target: Arg,
        index: Arg,
        value: Arg,
    },
    IndexGet {
        target: Arg,
        index: Arg,
        return_type: Option<String>,
    },
    IndexSet {
        target: Arg,
        index: Arg,
        value: Arg,
    },
}

impl Definition {
    pub fn new(
        name: &str,
        args: &[std::collections::HashMap<String, String>],
        return_type: Option<&str>,
    ) -> Self {
        fn get_arg(args: &[std::collections::HashMap<String, String>], index: usize) -> Arg {
            args.get(index).map_or_else(Arg::unknown, |def| Arg {
                name: def
                    .get("name")
                    .map_or("_", std::string::String::as_str)
                    .to_string(),
                ty: def
                    .get("type")
                    .and_then(|ty| def_type_name(ty))
                    .unwrap_or_else(|| "?".to_string()),
            })
        }

        let return_type = return_type.and_then(def_type_name);

        if is_operator(name) {
            Self::Operator {
                name: name.to_string(),
                arg1: get_arg(args, 0),
                arg2: get_arg(args, 1),
                return_type,
            }
        } else if let Some(name) = name.strip_prefix("get$") {
            Self::Get {
                target: get_arg(args, 0),
                index: Arg {
                    name: name.to_string(),
                    ty: "_".to_string(),
                },
                return_type,
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
                return_type,
            }
        } else if name.strip_prefix("index$set$").is_some() {
            Self::IndexSet {
                target: get_arg(args, 0),
                index: get_arg(args, 1),
                value: get_arg(args, 2),
            }
        } else {
            Self::Function {
                name: name.to_string(),
                args: args
                    .iter()
                    .enumerate()
                    .map(|(index, _)| get_arg(args, index))
                    .collect::<Vec<Arg>>(),
                return_type,
            }
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Function {
                name,
                args,
                return_type,
            } => {
                format!(
                    "fn {}({})",
                    name,
                    args.iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<String>>()
                        .join(", ")
                ) + return_type
                    .as_ref()
                    .map_or_else(String::default, |rt| format!(" -> {rt}"))
                    .as_str()
            }
            Self::Operator {
                name,
                arg1,
                arg2,
                return_type,
            } => {
                format!("op {} {} {}", arg1.ty, name, arg2.ty)
                    + return_type
                        .as_ref()
                        .map_or(")".to_string(), |rt| format!(" -> {rt}"))
                        .as_str()
            }
            Self::Get {
                target,
                index,
                return_type,
            } => {
                format!("get {}.{}", target.ty, index.name)
                    + return_type
                        .as_ref()
                        .map_or(")".to_string(), |rt| format!(" -> {rt}"))
                        .as_str()
            }
            Self::Set {
                target,
                index,
                value,
            } => {
                format!("set {}.{} = {}", target.ty, index.name, value.ty)
            }
            Self::IndexGet {
                target,
                index,
                return_type,
            } => {
                format!("index get {}[{}]", target.ty, index)
                    + return_type
                        .as_ref()
                        .map_or(")".to_string(), |rt| format!(" -> {rt}"))
                        .as_str()
            }
            Self::IndexSet {
                target,
                index,
                value,
            } => format!("index set {}[{}] = {}", target.ty, index, value.ty),
        }
    }

    /// Return the function type of the definition as a string.
    pub const fn type_to_str(&self) -> &'static str {
        match self {
            Self::Function { .. } => "fn",
            Self::Operator { .. } => "op",
            Self::Get { .. } | Self::Set { .. } => "get/set",
            Self::IndexGet { .. } | Self::IndexSet { .. } => "index get/set",
        }
    }

    /// Full name of the definition.
    pub fn name(&self) -> String {
        match self {
            Self::Function { name, .. } | Self::Operator { name, .. } => name.clone(),
            Self::Set { target, index, .. } | Self::Get { target, index, .. } => {
                format!("{}.{}", target.ty, index.name)
            }
            Self::IndexGet { target, index, .. } | Self::IndexSet { target, index, .. } => {
                format!("{}.{}", target.ty, index.ty)
            }
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
