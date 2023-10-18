#![doc = include_str!("../README.md")]

pub mod custom_types;
pub mod doc_item;
pub mod function;
pub mod glossary;
pub mod module;

pub use glossary::ModuleGlossary;
pub use module::{
    options::{options, ItemsOrder, MarkdownProcessor, SectionFormat},
    ModuleDocumentation,
};
