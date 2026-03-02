pub mod cache;
pub mod docsrs;
pub mod docsrs_format;
pub mod python;
pub mod rust;
pub mod templates;
pub mod typescript;

use crate::detect::{Dependency, Language};
use std::path::{Path, PathBuf};

/// A single entry in a reference file (function, class, type, etc.)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ReferenceEntry {
    pub name: String,
    pub kind: EntryKind,
    pub signature: String,
    pub description: String,
    pub section: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum EntryKind {
    Function,
    Method,
    Class,
    Struct,
    Trait,
    Interface,
    TypeAlias,
    Enum,
    Constant,
    Decorator,
    Macro,
    Hook,
    Component,
    Property,
    Module,
}

/// A complete reference file for one library
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReferenceFile {
    pub library_name: String,
    pub version: String,
    pub language: Language,
    pub entries: Vec<ReferenceEntry>,
    pub raw_content: String,
    pub file_path: PathBuf,
}

/// Trait that all language generators implement
pub trait Generator {
    fn generate(
        &self,
        dep: &Dependency,
        output_dir: &Path,
        global_refs_dir: Option<&Path>,
    ) -> anyhow::Result<ReferenceFile>;
    fn language(&self) -> Language;
}
