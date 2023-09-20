#![doc = include_str!("../README.md")]

pub mod function;
pub mod glossary;
pub mod module;

pub use module::ModuleDocumentation;

/// NOTE: mdbook handles this automatically, but other
///       markdown processors might not.
/// Remove lines of code that starts with the '#' token,
/// which are removed on rust docs automatically.
fn remove_test_code(doc_comments: &str) -> String {
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

/// Remove doc comments identifiers.
fn fmt_doc_comments(dc: String) -> String {
    dc.replace("/// ", "")
        .replace("///", "")
        .replace("/**", "")
        .replace("**/", "")
        .replace("**/", "")
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_remove_test_code_simple() {
        pretty_assertions::assert_eq!(
            remove_test_code(
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
            remove_test_code(
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
            remove_test_code(
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
