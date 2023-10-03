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
    pub fn generate_function_definition(&self, engine: &rhai::Engine) -> String {
        // Add the operator / function prefix.
        let mut definition = if is_operator(&self.name) {
            String::from("op ")
        } else {
            String::from("fn ")
        };

        // Add getter and setter prefix + the name of the function.
        if let Some(name) = self.name.strip_prefix("get$") {
            definition += &format!("get {name}(");
        } else if let Some(name) = self.name.strip_prefix("set$") {
            definition += &format!("set {name}(");
        } else {
            definition += &format!("{}(", self.name);
        }

        let mut first = true;

        // Add params with their types.
        for i in 0..self.num_params {
            if !first {
                definition += ", ";
            }
            first = false;

            let (param_name, param_type) = self
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
        definition
            + match self.return_type.as_deref() {
                Some("()") | None => ")".to_string(),
                Some(t) => format!(") -> {}", def_type_name(t, engine)),
            }
            .as_str()
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
