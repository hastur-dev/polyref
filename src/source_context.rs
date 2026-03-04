use std::collections::HashMap;

use regex::Regex;
use std::sync::LazyLock;

use crate::generate::ReferenceFile;
use crate::type_inference::{build_type_context, TypeContext};

/// Contextual information gathered from a source file before per-line checking.
#[derive(Debug, Clone)]
pub struct SourceContext {
    pub imported_crates: Vec<String>,
    pub imported_items: HashMap<String, String>, // item → full path
    pub type_context: TypeContext,
    pub active_ref_files: Vec<String>,
}

static USE_CRATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"use\s+([a-z_][a-z0-9_]*)::").expect("valid regex")
});

static USE_ITEM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"use\s+([\w:]+)::([A-Za-z_]\w*)\s*;").expect("valid regex")
});

static USE_BRACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"use\s+([\w:]+)::\{([^}]+)\}\s*;").expect("valid regex")
});

/// Extract top-level crate names from `use crate_name::...` statements.
pub fn extract_imported_crates(content: &str) -> Vec<String> {
    let mut crates: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if let Some(cap) = USE_CRATE_RE.captures(trimmed) {
            let crate_name = cap[1].to_string();
            if !crates.contains(&crate_name) {
                crates.push(crate_name);
            }
        }
    }

    debug_assert!(
        crates.iter().all(|c| !c.is_empty()),
        "no empty crate names"
    );
    debug_assert!(
        {
            let mut sorted = crates.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len() == crates.len()
        },
        "no duplicates"
    );

    crates
}

/// Extract individually imported items from `use` statements.
pub fn extract_imported_items(content: &str) -> HashMap<String, String> {
    let mut items = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || !trimmed.starts_with("use ") {
            continue;
        }

        // Handle `use path::Item;`
        if let Some(cap) = USE_ITEM_RE.captures(trimmed) {
            let path = cap[1].to_string();
            let item = cap[2].to_string();
            let full_path = format!("{}::{}", path, item);
            if full_path.contains("::") {
                items.insert(item, full_path);
            }
        }

        // Handle `use path::{Item1, Item2};`
        if let Some(cap) = USE_BRACE_RE.captures(trimmed) {
            let base_path = cap[1].to_string();
            let brace_content = &cap[2];
            for part in brace_content.split(',') {
                let item = part.trim().to_string();
                if !item.is_empty() && item != "self" {
                    let full_path = format!("{}::{}", base_path, item);
                    items.insert(item, full_path);
                }
            }
        }
    }

    debug_assert!(
        items.keys().all(|k| !k.is_empty()),
        "no empty keys"
    );

    items
}

/// Build a complete source context from file content.
pub fn build_source_context(content: &str) -> SourceContext {
    let imported_crates = extract_imported_crates(content);
    let imported_items = extract_imported_items(content);
    let lines: Vec<&str> = content.lines().collect();
    let type_context = build_type_context(&lines);

    SourceContext {
        imported_crates,
        imported_items,
        type_context,
        active_ref_files: Vec::new(),
    }
}

/// Select only reference files whose crate name matches imported crates.
///
/// Falls back to all refs if no imports are detected (backward compatible).
pub fn select_relevant_ref_files<'a>(
    ctx: &SourceContext,
    all_refs: &'a [ReferenceFile],
) -> Vec<&'a ReferenceFile> {
    if ctx.imported_crates.is_empty() {
        return all_refs.iter().collect();
    }

    let selected: Vec<&'a ReferenceFile> = all_refs
        .iter()
        .filter(|rf| {
            let lib_name = rf.library_name.replace('-', "_");
            ctx.imported_crates.contains(&lib_name)
                || ctx.imported_crates.contains(&rf.library_name)
        })
        .collect();

    debug_assert!(selected.len() <= all_refs.len());

    // If nothing matched, fall back to all
    if selected.is_empty() {
        return all_refs.iter().collect();
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::Language;
    use crate::generate::ReferenceFile;
    use std::path::PathBuf;

    fn make_ref_file(name: &str) -> ReferenceFile {
        ReferenceFile {
            library_name: name.to_string(),
            version: "1.0".to_string(),
            language: Language::Rust,
            entries: vec![],
            raw_content: String::new(),
            file_path: PathBuf::from(format!("refs/rust/lib_{}.rs", name)),
        }
    }

    #[test]
    fn test_extract_imported_crates_single() {
        let content = "use tokio::runtime::Runtime;";
        let crates = extract_imported_crates(content);
        assert_eq!(crates, vec!["tokio"]);
    }

    #[test]
    fn test_extract_imported_crates_multiple() {
        let content = "use tokio::runtime::Runtime;\nuse crossterm::event;\nuse std::io;";
        let crates = extract_imported_crates(content);
        assert!(crates.contains(&"tokio".to_string()));
        assert!(crates.contains(&"crossterm".to_string()));
        assert!(crates.contains(&"std".to_string()));
    }

    #[test]
    fn test_extract_imported_crates_no_duplicates() {
        let content = "use tokio::runtime::Runtime;\nuse tokio::task::JoinHandle;";
        let crates = extract_imported_crates(content);
        assert_eq!(crates.iter().filter(|c| *c == "tokio").count(), 1);
    }

    #[test]
    fn test_extract_imported_items_single() {
        let content = "use tokio::runtime::Runtime;";
        let items = extract_imported_items(content);
        assert_eq!(
            items.get("Runtime"),
            Some(&"tokio::runtime::Runtime".to_string())
        );
    }

    #[test]
    fn test_extract_imported_items_multi_brace() {
        let content = "use tokio::task::{spawn, JoinHandle};";
        let items = extract_imported_items(content);
        assert_eq!(
            items.get("spawn"),
            Some(&"tokio::task::spawn".to_string())
        );
        assert_eq!(
            items.get("JoinHandle"),
            Some(&"tokio::task::JoinHandle".to_string())
        );
    }

    #[test]
    fn test_build_source_context_populates_all_fields() {
        let content = "use tokio::runtime::Runtime;\nlet rt = Runtime::new();";
        let ctx = build_source_context(content);
        assert!(!ctx.imported_crates.is_empty());
        assert!(!ctx.type_context.bindings.is_empty());
    }

    #[test]
    fn test_select_relevant_refs_filters_correctly() {
        let refs = vec![make_ref_file("tokio"), make_ref_file("crossterm")];
        let content = "use tokio::runtime::Runtime;";
        let ctx = build_source_context(content);
        let selected = select_relevant_ref_files(&ctx, &refs);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].library_name, "tokio");
    }

    #[test]
    fn test_select_relevant_refs_empty_imports_returns_all() {
        let refs = vec![make_ref_file("tokio"), make_ref_file("crossterm")];
        let content = "fn main() {}";
        let ctx = build_source_context(content);
        let selected = select_relevant_ref_files(&ctx, &refs);
        assert_eq!(selected.len(), refs.len());
    }

    #[test]
    fn test_extract_items_no_empty_keys() {
        let content = "use tokio::task::spawn;\nuse std::io;";
        let items = extract_imported_items(content);
        assert!(items.keys().all(|k| !k.is_empty()));
    }

    #[test]
    fn test_extract_crates_no_empty_strings() {
        let content = "use tokio::task::spawn;";
        let crates = extract_imported_crates(content);
        assert!(crates.iter().all(|c| !c.is_empty()));
    }
}
