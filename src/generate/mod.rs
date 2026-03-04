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
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct ReferenceEntry {
    pub name: String,
    pub kind: EntryKind,
    pub signature: String,
    pub description: String,
    pub section: String,
    /// The owning type for methods/associated fns (e.g., "Runtime" for Runtime::new)
    pub type_context: Option<String>,
    /// Parent name for enum variants or struct fields
    pub parent: Option<String>,
    /// Minimum required arguments (excluding self)
    pub min_args: Option<usize>,
    /// Maximum allowed arguments (None = variadic)
    pub max_args: Option<usize>,
    /// Original path for re-exports
    pub original_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum EntryKind {
    #[default]
    Function,
    Method,
    AssociatedFn,
    Class,
    Struct,
    Trait,
    Interface,
    TypeAlias,
    Enum,
    EnumVariant,
    StructField,
    Constant,
    Decorator,
    Macro,
    Hook,
    Component,
    Property,
    Module,
    ReExport,
}

impl ReferenceEntry {
    /// Create a basic reference entry with all optional fields set to None
    pub fn basic(
        name: String,
        kind: EntryKind,
        signature: String,
        description: String,
        section: String,
    ) -> Self {
        Self {
            name,
            kind,
            signature,
            description,
            section,
            type_context: None,
            parent: None,
            min_args: None,
            max_args: None,
            original_path: None,
        }
    }
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
