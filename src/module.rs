use serde::{Deserialize, Serialize};

use crate::function::FunctionMetadata;
use crate::{fmt_doc_comments, remove_test_code};

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

/// Glossary of all function for a module and it's submodules.
#[derive(Debug)]
pub struct ModuleGlossary {
    /// Formated function signatures by submodules.
    pub content: String,
}
