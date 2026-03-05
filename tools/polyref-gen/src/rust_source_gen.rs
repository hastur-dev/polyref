use std::path::Path;

use crate::rustdoc_gen::{EntryKind, GenEntry, RustdocOutput};

/// Parse Rust source files directly (no rustdoc JSON needed).
/// Walks the `src/` directory of a Rust project and extracts public API items
/// using regex-based parsing.
pub fn parse_rust_project(project_dir: &Path) -> anyhow::Result<RustdocOutput> {
    let cargo_toml = project_dir.join("Cargo.toml");
    let (crate_name, crate_version) = if cargo_toml.exists() {
        parse_cargo_toml(&cargo_toml)?
    } else {
        (
            project_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            "0.0.0".to_string(),
        )
    };

    let mut entries = Vec::new();

    // Try src/ first, then crates/*/src/, then workspace member dirs
    let src_dir = project_dir.join("src");
    if src_dir.exists() {
        collect_rs_files(&src_dir, &mut entries)?;
    }

    // Also check crates/ directory (workspace-style projects)
    let crates_dir = project_dir.join("crates");
    if crates_dir.exists() && crates_dir.is_dir() {
        collect_workspace_crates(&crates_dir, &mut entries)?;
    }

    // Check for workspace subcrate directories (e.g., pingora-core/src/)
    if entries.is_empty() {
        collect_workspace_subcrates(project_dir, &mut entries)?;
    }

    // If no src/ or crates/ found at all, bail
    if !src_dir.exists() && !crates_dir.exists() {
        // Check workspace subcrates
        let has_subcrate_src = std::fs::read_dir(project_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| {
                        let p = e.path();
                        p.is_dir() && p.join("Cargo.toml").exists() && p.join("src").exists()
                    })
            })
            .unwrap_or(false);
        if !has_subcrate_src {
            anyhow::bail!("No Rust source directories found in {}", project_dir.display());
        }
    }

    entries.sort_by(|a, b| {
        a.parent
            .as_deref()
            .unwrap_or("")
            .cmp(b.parent.as_deref().unwrap_or(""))
            .then(a.name.cmp(&b.name))
    });

    Ok(RustdocOutput {
        crate_name,
        crate_version,
        entries,
    })
}

fn parse_cargo_toml(path: &Path) -> anyhow::Result<(String, String)> {
    let content = std::fs::read_to_string(path)?;
    let name = extract_toml_value(&content, "name").unwrap_or_else(|| "unknown".to_string());
    let version = extract_toml_value(&content, "version").unwrap_or_else(|| "0.0.0".to_string());
    Ok((name, version))
}

fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    let in_package = find_section_lines(content, "[package]");
    for line in in_package {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let val = rest.trim().trim_matches('"');
                return Some(val.to_string());
            }
        }
    }
    None
}

fn find_section_lines(content: &str, section: &str) -> Vec<String> {
    let mut in_section = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == section {
            in_section = true;
            continue;
        }
        if in_section {
            if trimmed.starts_with('[') {
                break;
            }
            lines.push(line.to_string());
        }
    }
    lines
}

fn collect_rs_files(dir: &Path, entries: &mut Vec<GenEntry>) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            // Skip target directories
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "target" || name.starts_with('.') {
                continue;
            }
            collect_rs_files(&path, entries)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            // Skip non-UTF-8 files gracefully
            match std::fs::read_to_string(&path) {
                Ok(content) => parse_rs_source(&content, entries),
                Err(e) => {
                    eprintln!(
                        "  Warning: skipping {} ({})",
                        path.display(),
                        e
                    );
                }
            }
        }
    }
    Ok(())
}

fn collect_workspace_crates(crates_dir: &Path, entries: &mut Vec<GenEntry>) -> anyhow::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(crates_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            let src = path.join("src");
            if src.exists() {
                collect_rs_files(&src, entries)?;
            }
        }
    }
    Ok(())
}

fn collect_workspace_subcrates(
    project_dir: &Path,
    entries: &mut Vec<GenEntry>,
) -> anyhow::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(project_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "target" || name.starts_with('.') {
                continue;
            }
            // Check if this subdirectory has its own Cargo.toml + src/
            if path.join("Cargo.toml").exists() && path.join("src").exists() {
                collect_rs_files(&path.join("src"), entries)?;
            }
        }
    }
    Ok(())
}

/// Parse a single .rs file and extract public API items.
pub fn parse_rs_source(content: &str, entries: &mut Vec<GenEntry>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut current_impl: Option<String> = None;
    let mut impl_brace_depth: i32 = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Track impl blocks
        if let Some(impl_type) = parse_impl_line(trimmed) {
            current_impl = Some(impl_type);
            impl_brace_depth = 0;
            for ch in trimmed.chars() {
                match ch {
                    '{' => impl_brace_depth += 1,
                    '}' => impl_brace_depth -= 1,
                    _ => {}
                }
            }
            i += 1;
            continue;
        }

        // Track brace depth inside impl
        if current_impl.is_some() {
            for ch in trimmed.chars() {
                match ch {
                    '{' => impl_brace_depth += 1,
                    '}' => impl_brace_depth -= 1,
                    _ => {}
                }
            }
            if impl_brace_depth <= 0 {
                current_impl = None;
            }
        }

        // pub struct
        if trimmed.starts_with("pub struct ") || trimmed.starts_with("pub(crate) struct ") {
            let name = extract_item_name(trimmed, "struct");
            if let Some(name) = name {
                let desc = gather_doc_comment(&lines, i);
                if !entries.iter().any(|e| e.name == name && e.kind == EntryKind::Struct) {
                    entries.push(GenEntry {
                        name,
                        kind: EntryKind::Struct,
                        parent: None,
                        signature: String::new(),
                        description: desc,
                        arg_count: 0,
                    });
                }
            }
        }

        // pub enum
        if trimmed.starts_with("pub enum ") || trimmed.starts_with("pub(crate) enum ") {
            let name = extract_item_name(trimmed, "enum");
            if let Some(name) = name {
                let desc = gather_doc_comment(&lines, i);
                if !entries.iter().any(|e| e.name == name && e.kind == EntryKind::Enum) {
                    entries.push(GenEntry {
                        name,
                        kind: EntryKind::Enum,
                        parent: None,
                        signature: String::new(),
                        description: desc,
                        arg_count: 0,
                    });
                }
            }
        }

        // pub fn / pub async fn
        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(crate) async fn ")
            || trimmed.starts_with("pub const fn ")
            || trimmed.starts_with("pub unsafe fn ")
        {
            // Gather multi-line signature
            let full_sig = gather_full_signature(&lines, i);
            let (name, sig, arg_count, has_self) = parse_fn_signature(&full_sig);
            if let Some(name) = name {
                let desc = gather_doc_comment(&lines, i);
                let parent = current_impl.clone();
                let kind = if parent.is_some() {
                    if has_self {
                        EntryKind::Method
                    } else {
                        EntryKind::AssociatedFunction
                    }
                } else {
                    EntryKind::Function
                };
                entries.push(GenEntry {
                    name,
                    kind,
                    parent,
                    signature: sig,
                    description: desc,
                    arg_count,
                });
            }
        }

        i += 1;
    }
}

fn parse_impl_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    // Match: impl TypeName { or impl TypeName<...> { or impl Trait for TypeName {
    if !trimmed.starts_with("impl ") && !trimmed.starts_with("impl<") {
        return None;
    }

    let after_impl = if trimmed.starts_with("impl<") {
        // Skip generic params
        let mut depth = 0;
        let mut end = 4; // after "impl"
        for (i, ch) in trimmed[4..].char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = 4 + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        trimmed[end..].trim()
    } else {
        trimmed.strip_prefix("impl ").unwrap_or("")
    };

    // Check for "Trait for Type" pattern
    let type_part = if let Some(pos) = after_impl.find(" for ") {
        after_impl[pos + 5..].trim()
    } else {
        after_impl
    };

    // Extract the type name (before any < or { or where)
    let name = type_part
        .split(['<', '{', ' '])
        .next()
        .unwrap_or("")
        .trim();

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_item_name(line: &str, keyword: &str) -> Option<String> {
    let pattern = format!("{} ", keyword);
    let pos = line.find(&pattern)?;
    let after = &line[pos + pattern.len()..];
    let name = after
        .split(['<', '{', '(', ' ', ';'])
        .next()
        .unwrap_or("")
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn gather_doc_comment(lines: &[&str], item_line: usize) -> String {
    let mut doc_lines = Vec::new();
    let mut j = item_line;
    while j > 0 {
        j -= 1;
        let trimmed = lines[j].trim();
        if let Some(doc) = trimmed.strip_prefix("///") {
            doc_lines.push(doc.trim().to_string());
        } else if trimmed.starts_with("#[") || trimmed.is_empty() {
            // Attributes or blank lines between doc comments and item — keep looking
            if trimmed.is_empty() {
                break;
            }
        } else {
            break;
        }
    }
    doc_lines.reverse();
    doc_lines.first().cloned().unwrap_or_default()
}

fn gather_full_signature(lines: &[&str], start: usize) -> String {
    let mut sig = String::new();
    let mut depth = 0i32;
    let mut found_open = false;
    for line in &lines[start..] {
        let line = line.trim();
        sig.push_str(line);
        sig.push(' ');
        for ch in line.chars() {
            match ch {
                '(' => {
                    found_open = true;
                    depth += 1;
                }
                ')' => depth -= 1,
                _ => {}
            }
        }
        if found_open && depth <= 0 {
            break;
        }
        // Also break on { if we've seen the closing paren
        if line.contains('{') && found_open && depth <= 0 {
            break;
        }
    }
    sig
}

fn parse_fn_signature(full_sig: &str) -> (Option<String>, String, usize, bool) {
    // Find the function name: after "fn "
    let fn_pos = match full_sig.find("fn ") {
        Some(p) => p,
        None => return (None, String::new(), 0, false),
    };
    let after_fn = &full_sig[fn_pos + 3..];

    // Name is everything before the first ( or <
    let name_end = after_fn
        .find(['(', '<'])
        .unwrap_or(after_fn.len());
    let name = after_fn[..name_end].trim().to_string();
    if name.is_empty() {
        return (None, String::new(), 0, false);
    }

    // Extract params between the outermost ()
    let paren_start = match full_sig[fn_pos..].find('(') {
        Some(p) => fn_pos + p,
        None => return (Some(name), String::new(), 0, false),
    };
    let mut depth = 0;
    let mut paren_end = paren_start;
    for (i, ch) in full_sig[paren_start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    paren_end = paren_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let params_str = full_sig[paren_start + 1..paren_end].trim();
    let has_self = params_str.contains("self");

    // Count actual arguments (not &self/&mut self/self)
    let mut arg_count = 0;
    if !params_str.is_empty() {
        let mut depth = 0;
        let mut in_arg = true;
        let mut current_arg = String::new();
        for ch in params_str.chars() {
            match ch {
                '(' | '[' | '{' | '<' => {
                    depth += 1;
                    current_arg.push(ch);
                }
                ')' | ']' | '}' | '>' => {
                    depth -= 1;
                    current_arg.push(ch);
                }
                ',' if depth == 0 => {
                    let trimmed = current_arg.trim();
                    if !is_self_param(trimmed) && !trimmed.is_empty() {
                        arg_count += 1;
                    }
                    current_arg.clear();
                    in_arg = true;
                }
                _ => {
                    if in_arg {
                        current_arg.push(ch);
                    }
                }
            }
        }
        let trimmed = current_arg.trim();
        if !trimmed.is_empty() && !is_self_param(trimmed) {
            arg_count += 1;
        }
    }

    let sig = build_simplified_sig(params_str);
    (Some(name), sig, arg_count, has_self)
}

fn is_self_param(s: &str) -> bool {
    let s = s.trim();
    s == "self" || s == "&self" || s == "&mut self" || s == "mut self" || s == "self:"
        || s.starts_with("&self,") || s.starts_with("&mut self,")
}

fn build_simplified_sig(params: &str) -> String {
    if params.is_empty() {
        return String::new();
    }
    // Split on commas at depth 0 and rebuild
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for ch in params.chars() {
        match ch {
            '(' | '[' | '{' | '<' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' | '>' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_impl_line() {
        assert_eq!(parse_impl_line("impl Foo {"), Some("Foo".to_string()));
        assert_eq!(parse_impl_line("impl<T> Foo<T> {"), Some("Foo".to_string()));
        assert_eq!(
            parse_impl_line("impl Display for Foo {"),
            Some("Foo".to_string())
        );
        assert_eq!(
            parse_impl_line("impl<T: Clone> Iterator for MyIter<T> {"),
            Some("MyIter".to_string())
        );
        assert_eq!(parse_impl_line("fn something()"), None);
    }

    #[test]
    fn test_extract_item_name() {
        assert_eq!(
            extract_item_name("pub struct Foo {", "struct"),
            Some("Foo".to_string())
        );
        assert_eq!(
            extract_item_name("pub struct Bar<T> {", "struct"),
            Some("Bar".to_string())
        );
        assert_eq!(
            extract_item_name("pub enum Color {", "enum"),
            Some("Color".to_string())
        );
    }

    #[test]
    fn test_parse_fn_signature_free_fn() {
        let sig = "pub fn hello(name: &str, age: u32) -> String { ";
        let (name, _sig, count, has_self) = parse_fn_signature(sig);
        assert_eq!(name, Some("hello".to_string()));
        assert_eq!(count, 2);
        assert!(!has_self);
    }

    #[test]
    fn test_parse_fn_signature_method() {
        let sig = "pub fn greet(&self, name: &str) -> String { ";
        let (name, _sig, count, has_self) = parse_fn_signature(sig);
        assert_eq!(name, Some("greet".to_string()));
        assert_eq!(count, 1);
        assert!(has_self);
    }

    #[test]
    fn test_parse_fn_signature_no_args() {
        let sig = "pub fn new() -> Self { ";
        let (name, _sig, count, has_self) = parse_fn_signature(sig);
        assert_eq!(name, Some("new".to_string()));
        assert_eq!(count, 0);
        assert!(!has_self);
    }

    #[test]
    fn test_parse_fn_signature_generic() {
        let sig = "pub fn process<T: Clone>(items: Vec<T>, count: usize) -> Vec<T> { ";
        let (name, _sig, count, has_self) = parse_fn_signature(sig);
        assert_eq!(name, Some("process".to_string()));
        assert_eq!(count, 2);
        assert!(!has_self);
    }

    #[test]
    fn test_parse_rs_source_basic() {
        let source = r#"
/// A sample struct.
pub struct Foo {
    pub name: String,
}

impl Foo {
    /// Create new Foo.
    pub fn new(name: String) -> Self {
        Foo { name }
    }

    /// Get name reference.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Free function.
pub fn do_stuff(x: i32, y: i32) -> i32 {
    x + y
}
"#;
        let mut entries = Vec::new();
        parse_rs_source(source, &mut entries);

        let struct_entry = entries.iter().find(|e| e.name == "Foo").unwrap();
        assert_eq!(struct_entry.kind, EntryKind::Struct);
        assert_eq!(struct_entry.description, "A sample struct.");

        let new_entry = entries.iter().find(|e| e.name == "new").unwrap();
        assert_eq!(new_entry.kind, EntryKind::AssociatedFunction);
        assert_eq!(new_entry.parent, Some("Foo".to_string()));
        assert_eq!(new_entry.arg_count, 1);

        let name_entry = entries.iter().find(|e| e.name == "name").unwrap();
        assert_eq!(name_entry.kind, EntryKind::Method);
        assert_eq!(name_entry.arg_count, 0);

        let do_stuff = entries.iter().find(|e| e.name == "do_stuff").unwrap();
        assert_eq!(do_stuff.kind, EntryKind::Function);
        assert_eq!(do_stuff.arg_count, 2);
    }

    #[test]
    fn test_parse_rs_source_async_fn() {
        let source = r#"
pub async fn fetch_data(url: &str) -> Result<String, Error> {
    todo!()
}
"#;
        let mut entries = Vec::new();
        parse_rs_source(source, &mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "fetch_data");
        assert_eq!(entries[0].arg_count, 1);
    }

    #[test]
    fn test_parse_rs_source_enum() {
        let source = r#"
/// Color options.
pub enum Color {
    Red,
    Green,
    Blue,
}
"#;
        let mut entries = Vec::new();
        parse_rs_source(source, &mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Color");
        assert_eq!(entries[0].kind, EntryKind::Enum);
    }

    #[test]
    fn test_parse_rs_source_trait_impl() {
        let source = r#"
pub struct MyType;

impl Display for MyType {
    pub fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "MyType")
    }
}
"#;
        let mut entries = Vec::new();
        parse_rs_source(source, &mut entries);

        let fmt = entries.iter().find(|e| e.name == "fmt");
        assert!(fmt.is_some());
        assert_eq!(fmt.unwrap().parent, Some("MyType".to_string()));
    }

    #[test]
    fn test_gather_doc_comment() {
        let lines = vec![
            "/// First line of docs.",
            "/// Second line.",
            "pub fn foo() {}",
        ];
        let doc = gather_doc_comment(&lines, 2);
        assert_eq!(doc, "First line of docs.");
    }

    #[test]
    fn test_is_self_param() {
        assert!(is_self_param("self"));
        assert!(is_self_param("&self"));
        assert!(is_self_param("&mut self"));
        assert!(!is_self_param("name: String"));
        assert!(!is_self_param("self_name: String"));
    }

    #[test]
    fn test_cargo_toml_parsing() {
        let content = r#"
[package]
name = "my-crate"
version = "1.2.3"
edition = "2021"

[dependencies]
serde = "1"
"#;
        let name = extract_toml_value(content, "name");
        assert_eq!(name, Some("my-crate".to_string()));
        let version = extract_toml_value(content, "version");
        assert_eq!(version, Some("1.2.3".to_string()));
    }
}
