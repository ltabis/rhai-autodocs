#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CustomTypesMetadata {
    pub type_name: String,
    pub display_name: String,
    pub doc_comments: Vec<String>,
}
