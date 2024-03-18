use crate::{
    custom_types::CustomTypesMetadata,
    function::FunctionMetadata,
    module::{
        error::AutodocsError,
        options::{Options, RHAI_ITEM_INDEX_PATTERN},
    },
    ItemsOrder,
};

/// Generic representation of documentation for a specific item. (a function, a custom type, etc.)
#[derive(Debug, Clone)]
pub enum DocItem {
    Function {
        root_metadata: FunctionMetadata,
        metadata: Vec<FunctionMetadata>,
        name: String,
        index: usize,
    },
    CustomType {
        metadata: CustomTypesMetadata,
        index: usize,
    },
}

use serde::ser::SerializeStruct;

impl serde::Serialize for DocItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            DocItem::Function {
                root_metadata,
                name,
                metadata,
                ..
            } => {
                let mut state = serializer.serialize_struct("item", 4)?;
                state.serialize_field(
                    "type",
                    root_metadata.generate_function_definition().type_to_str(),
                )?;
                state.serialize_field("name", name)?;
                state.serialize_field(
                    "signatures",
                    metadata
                        .iter()
                        .map(|metadata| metadata.generate_function_definition().display())
                        .collect::<Vec<_>>()
                        .join("\n")
                        .as_str(),
                )?;
                state.serialize_field("sections", {
                    &Section::extract_sections(
                        &root_metadata
                            .doc_comments
                            .clone()
                            .unwrap_or_default()
                            .join("\n"),
                    )
                })?;
                state.end()
            }
            DocItem::CustomType { metadata, .. } => {
                let mut state = serializer.serialize_struct("item", 2)?;
                state.serialize_field("name", &metadata.display_name)?;
                state.serialize_field(
                    "sections",
                    &Section::extract_sections(
                        &metadata.doc_comments.clone().unwrap_or_default().join("\n"),
                    ),
                )?;
                state.end()
            }
        }
    }
}

#[derive(Default, Clone, serde::Serialize)]
struct Section {
    pub name: String,
    pub body: String,
}

impl Section {
    fn extract_sections(docs: &str) -> Vec<Section> {
        let mut sections = vec![];
        let mut current_name = "Description".to_string();
        let mut current_body = vec![];

        // Start by extracting all sections from markdown comments.
        docs.lines().fold(true, |first, line| {
            if let Some((_prefix, name)) = line.split_once("# ") {
                if !first {
                    sections.push(Section {
                        name: std::mem::take(&mut current_name),
                        body: DocItem::format_comments(&current_body[..]),
                    });
                }

                current_name = name.to_string();
                current_body = vec![];
            } else {
                current_body.push(format!("{line}\n"));
            }

            false
        });

        sections
    }
}

impl DocItem {
    pub fn new_function(
        metadata: &[FunctionMetadata],
        name: &str,
        namespace: &str,
        options: &Options,
    ) -> Result<Self, AutodocsError> {
        // Takes the first valid comments found for a function group.
        let root = metadata
            .iter()
            .find(|metadata| metadata.doc_comments.is_some());

        match root {
            // Anonymous functions are ignored.
            Some(root) if !name.starts_with("anon$") => {
                // let root_definition = root.generate_function_definition();
                let index = if matches!(options.items_order, ItemsOrder::ByIndex) {
                    Self::find_index(
                        name,
                        namespace,
                        root.doc_comments.as_ref().unwrap_or(&vec![]),
                    )?
                } else {
                    0
                };

                Ok(Self::Function {
                    root_metadata: root.clone(),
                    metadata: metadata.to_vec(),
                    name: name.to_string(),
                    index,
                })
            }
            _ => Err(AutodocsError::Metadata(format!(
                "No documentation found for function item {namespace}/{name}"
            ))),
        }
    }

    pub fn new_custom_type(
        metadata: CustomTypesMetadata,
        namespace: &str,
        options: &Options,
    ) -> Result<Self, AutodocsError> {
        let index = if matches!(options.items_order, ItemsOrder::ByIndex) {
            Self::find_index(
                &metadata.display_name,
                namespace,
                metadata.doc_comments.as_ref().unwrap_or(&vec![]),
            )?
        } else {
            0
        };

        Ok(Self::CustomType { metadata, index })
    }

    pub fn index(&self) -> usize {
        match self {
            DocItem::CustomType { index, .. } | DocItem::Function { index, .. } => *index,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DocItem::CustomType { metadata, .. } => metadata.display_name.as_str(),
            DocItem::Function { name, .. } => name,
        }
    }

    /// Find the order index of the item by searching for the index pattern.
    pub fn find_index(
        name: &str,
        namespace: &str,
        doc_comments: &[String],
    ) -> Result<usize, AutodocsError> {
        if let Some((_, index)) = doc_comments
            .iter()
            .find_map(|line| line.rsplit_once(RHAI_ITEM_INDEX_PATTERN))
        {
            index.parse::<usize>().map_err(|err| {
                AutodocsError::PreProcessing(format!("failed to parsed order metadata: {err}"))
            })
        } else {
            Err(AutodocsError::PreProcessing(format!(
                "missing order metadata in item {}/{}",
                namespace, name
            )))
        }
    }

    /// Format the function doc comments to make them
    /// into readable markdown.
    pub fn format_comments(doc_comments: &[String]) -> String {
        let doc_comments = doc_comments.to_vec();
        let removed_extra_tokens = Self::remove_extra_tokens(doc_comments).join("\n");
        let remove_comments = Self::fmt_doc_comments(removed_extra_tokens);

        Self::remove_test_code(&remove_comments)
    }

    /// Remove crate specific comments, like `rhai-autodocs:index`.
    pub fn remove_extra_tokens(dc: Vec<String>) -> Vec<String> {
        dc.into_iter()
            .map(|s| {
                s.lines()
                    .filter(|l| !l.contains(RHAI_ITEM_INDEX_PATTERN))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
    }

    /// Remove doc comments identifiers.
    pub fn fmt_doc_comments(dc: String) -> String {
        dc.replace("/// ", "")
            .replace("///", "")
            .replace("/**", "")
            .replace("**/", "")
            .replace("**/", "")
    }

    /// NOTE: mdbook handles this automatically, but other
    ///       markdown processors might not.
    /// Remove lines of code that starts with the '#' token,
    /// which are removed on rust docs automatically.
    pub fn remove_test_code(doc_comments: &str) -> String {
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
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_remove_test_code_simple() {
        pretty_assertions::assert_eq!(
            DocItem::remove_test_code(
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
            DocItem::remove_test_code(
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
            DocItem::remove_test_code(
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
}
