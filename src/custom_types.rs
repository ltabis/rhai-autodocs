#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CustomTypesMetadata {
    /// "Real" name of the type, with rust namespaces if any.
    pub type_name: String,
    /// Simple name for Rhai documentation.
    pub display_name: String,
    /// All comments from the type.
    pub doc_comments: Option<Vec<String>>,
}
