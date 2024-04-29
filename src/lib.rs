#![doc = include_str!("../README.md")]

mod custom_types;
pub mod doc_item;
pub mod export;
mod function;
pub mod generate;
pub mod glossary;
pub mod module;

pub use glossary::ModuleGlossary;
pub use module::ModuleDocumentation;
