use crate::{
    custom_types::CustomTypesMetadata,
    function::FunctionMetadata,
    module::{
        error::AutodocsError,
        options::{Options, RHAI_ITEM_INDEX_PATTERN},
    },
    ItemsOrder, MarkdownProcessor,
};

/// Generic representation of documentation for a specific item. (a function, a custom type, etc.)
#[derive(Debug, Clone)]
pub enum DocItem {
    Function {
        metadata: Vec<FunctionMetadata>,
        name: String,
        index: usize,
        docs: String,
    },
    CustomType {
        metadata: CustomTypesMetadata,
        index: usize,
        docs: String,
    },
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
                let root_definition = root.generate_function_definition();
                let index = if matches!(options.items_order, ItemsOrder::ByIndex) {
                    Self::find_index(
                        name,
                        namespace,
                        root.doc_comments.as_ref().unwrap_or(&vec![]),
                    )?
                } else {
                    0
                };
                let docs = match options.markdown_processor {
                    MarkdownProcessor::MdBook => {
                        format!(
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
                            root_definition.type_to_str(),
                            root_definition.name(),
                            metadata
                                .iter()
                                .map(|metadata| metadata.generate_function_definition().display())
                                .collect::<Vec<_>>()
                                .join("\n"),
                            Self::format_comments(
                                &root.name,
                                root.doc_comments.as_ref().unwrap_or(&vec![]),
                                options
                            )
                        )
                    }
                    MarkdownProcessor::Docusaurus => {
                        format!(
                            r#"## <code>{}</code> {}
```js
{}
```
{}
"#,
                            // Add a specific prefix for the function type documented.
                            root_definition.type_to_str(),
                            root_definition.name(),
                            metadata
                                .iter()
                                .map(|metadata| metadata.generate_function_definition().display())
                                .collect::<Vec<_>>()
                                .join("\n"),
                            Self::format_comments(
                                &root.name,
                                root.doc_comments.as_ref().unwrap_or(&vec![]),
                                options
                            )
                        )
                    }
                };

                Ok(Self::Function {
                    metadata: metadata.to_vec(),
                    name: name.to_string(),
                    index,
                    docs,
                })
            }
            _ => Err(AutodocsError::Metadata(format!(
                "No documentation was found for item {namespace}/{name}"
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
        let docs = match options.markdown_processor {
            MarkdownProcessor::MdBook => {
                format!(
                    r#"<div markdown="span" style='box-shadow: 0 4px 8px 0 rgba(0,0,0,0.2); padding: 15px; border-radius: 5px;'>
    
<h2 class="func-name"> <code>type</code> {} </h2>
    
{}
</div>
</br>
"#,
                    // Add a specific prefix for the function type documented.
                    metadata.display_name,
                    Self::format_comments(
                        metadata.display_name.as_str(),
                        metadata.doc_comments.as_ref().unwrap_or(&vec![]),
                        options
                    )
                )
            }
            MarkdownProcessor::Docusaurus => {
                format!(
                    r#"## <code>type</code> {}
{}
"#,
                    // Add a specific prefix for the function type documented.
                    metadata.display_name,
                    Self::format_comments(
                        metadata.display_name.as_str(),
                        metadata.doc_comments.as_ref().unwrap_or(&vec![]),
                        options
                    )
                )
            }
        };

        Ok(Self::CustomType {
            metadata,
            index,
            docs,
        })
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

    pub fn docs(&self) -> &str {
        match self {
            DocItem::CustomType { docs, .. } | DocItem::Function { docs, .. } => docs,
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
    pub fn format_comments(name: &str, doc_comments: &[String], options: &Options) -> String {
        let doc_comments = doc_comments.to_vec();
        let removed_extra_tokens = Self::remove_extra_tokens(doc_comments).join("\n");
        let remove_comments = Self::fmt_doc_comments(removed_extra_tokens);
        let remove_test_code = Self::remove_test_code(&remove_comments);

        options
            .sections_format
            .fmt_sections(name, &options.markdown_processor, remove_test_code)
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
