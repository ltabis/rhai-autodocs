use serde::{Deserialize, Serialize};

use crate::{fmt_doc_comments, remove_test_code};

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
                .filter(|l| !l.contains(crate::options::RHAI_FUNCTION_INDEX_PATTERN))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
}

impl FunctionMetadata {
    /// Format the function doc comments to make them
    /// into readable markdown.
    pub fn fmt_doc_comments(&self, section_format: &crate::SectionFormat) -> Option<String> {
        self.doc_comments.clone().map(|dc| {
            let removed_extra_tokens = remove_extra_tokens(dc).join("\n");
            let remove_comments = fmt_doc_comments(removed_extra_tokens);
            let remove_test_code = remove_test_code(&remove_comments);

            section_format.fmt_sections(&self.name, remove_test_code)
        })
    }
}
