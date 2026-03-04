// Enhanced reference file parser - Task 2
// Placeholder: will be fully implemented in Task 2

use regex::Regex;
use std::sync::LazyLock;

use crate::generate::{EntryKind, ReferenceEntry};

/// A re-export entry parsed from `pub use path::item;`
#[derive(Debug, Clone, PartialEq)]
pub struct ReExport {
    pub original_path: String,
    pub exported_as: String,
}

static IMPL_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"impl\s+([A-Z][a-zA-Z0-9_]*)\s*\{").expect("valid regex")
});

static PUB_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"pub\s+(?:async\s+)?fn\s+([a-z_][a-zA-Z0-9_]*)").expect("valid regex")
});

static ENUM_VARIANT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*([A-Z][a-zA-Z0-9_]*)").expect("valid regex")
});

static PUB_FIELD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"pub\s+([a-z_][a-zA-Z0-9_]*)\s*:").expect("valid regex")
});

// Reserved for future use with complex re-export patterns
#[allow(dead_code)]
static REEXPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"pub\s+use\s+([\w:]+)(?:::\{[^}]+\})?;").expect("valid regex")
});

/// Parse an impl block's text to extract methods and associated functions.
pub fn parse_impl_block(
    block_text: &str,
    type_name: &str,
) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();

    for line in block_text.lines() {
        let trimmed = line.trim();
        if let Some(cap) = PUB_FN_RE.captures(trimmed) {
            let fn_name = cap[1].to_string();
            let is_method = trimmed.contains("&self") || trimmed.contains("&mut self");
            let kind = if is_method {
                EntryKind::Method
            } else {
                EntryKind::AssociatedFn
            };

            let min_args = parse_arg_count_from_sig(trimmed);

            entries.push(ReferenceEntry {
                name: fn_name,
                kind,
                signature: trimmed.to_string(),
                type_context: Some(type_name.to_string()),
                min_args: min_args.0,
                max_args: min_args.1,
                ..Default::default()
            });
        }
    }

    debug_assert!(
        entries.iter().all(|e| e.type_context.as_deref() == Some(type_name)),
        "all entries must have correct type_context"
    );

    entries
}

/// Parse enum variants from an enum block.
pub fn parse_enum_variants(
    enum_text: &str,
    enum_name: &str,
) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();

    for line in enum_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed == "{"
            || trimmed == "}"
            || trimmed.starts_with("pub enum")
            || trimmed.starts_with("enum")
        {
            continue;
        }

        if let Some(cap) = ENUM_VARIANT_RE.captures(trimmed) {
            let variant_name = cap[1].to_string();
            debug_assert!(
                !variant_name.contains(char::is_whitespace),
                "variant name must not contain whitespace"
            );
            debug_assert!(!variant_name.is_empty(), "variant name must be non-empty");

            entries.push(ReferenceEntry {
                name: variant_name,
                kind: EntryKind::EnumVariant,
                parent: Some(enum_name.to_string()),
                signature: trimmed.to_string(),
                ..Default::default()
            });
        }
    }

    entries
}

/// Parse pub struct fields from a struct block.
pub fn parse_struct_fields(
    struct_text: &str,
    struct_name: &str,
) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();

    for line in struct_text.lines() {
        let trimmed = line.trim();
        if let Some(cap) = PUB_FIELD_RE.captures(trimmed) {
            let field_name = cap[1].to_string();
            debug_assert!(
                field_name.starts_with(|c: char| c.is_ascii_lowercase()),
                "field name must start with lowercase"
            );

            entries.push(ReferenceEntry {
                name: field_name,
                kind: EntryKind::StructField,
                parent: Some(struct_name.to_string()),
                signature: trimmed.to_string(),
                ..Default::default()
            });
        }
    }

    entries
}

/// Parse `pub use path::item;` re-exports.
pub fn parse_reexports(content: &str) -> Vec<ReExport> {
    let mut exports = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("pub use ") {
            continue;
        }

        let path = trimmed
            .trim_start_matches("pub use ")
            .trim_end_matches(';')
            .trim();

        if !path.contains("::") {
            continue;
        }

        let exported_as = path.rsplit("::").next().unwrap_or("").to_string();
        if exported_as.is_empty() || exported_as.contains('{') {
            continue;
        }

        debug_assert!(!exported_as.is_empty(), "exported_as must be non-empty");
        debug_assert!(path.contains("::"), "original_path must contain ::");

        exports.push(ReExport {
            original_path: path.to_string(),
            exported_as,
        });
    }

    exports
}

/// Parse a function signature to determine argument count (excluding self).
pub fn parse_arg_count_from_sig(
    sig: &str,
) -> (Option<usize>, Option<usize>) {
    let paren_start = match sig.find('(') {
        Some(idx) => idx,
        None => return (None, None),
    };

    let rest = &sig[paren_start + 1..];
    let paren_end = find_matching_close_paren(rest);
    let params_str = match paren_end {
        Some(idx) => rest[..idx].trim(),
        None => return (None, None),
    };

    if params_str.is_empty() {
        return (Some(0), Some(0));
    }

    let parts: Vec<&str> = split_params(params_str);
    let count = parts
        .iter()
        .filter(|p| {
            let t = p.trim();
            t != "&self" && t != "&mut self" && t != "self" && t != "mut self"
        })
        .count();

    let result = (Some(count), Some(count));
    debug_assert!(
        result.0.unwrap() <= result.1.unwrap(),
        "min_args must be <= max_args"
    );
    result
}

fn find_matching_close_paren(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev = '\0';

    for (i, c) in s.char_indices() {
        if in_string {
            if c == string_char && prev != '\\' {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    string_char = c;
                }
                '(' | '[' | '{' | '<' => depth += 1,
                ')' => {
                    if depth == 0 {
                        return Some(i);
                    }
                    depth -= 1;
                }
                ']' | '}' | '>' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }
        prev = c;
    }

    None
}

fn split_params(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

/// Full reference file parser v2 that handles impl blocks, enums, structs, re-exports.
pub fn parse_reference_file_v2(content: &str) -> Vec<ReferenceEntry> {
    let mut all_entries = Vec::new();

    // Pass 1: Extract impl blocks
    let impl_blocks = extract_braced_blocks(content, "impl");
    for (type_name, block_text) in &impl_blocks {
        let entries = parse_impl_block(block_text, type_name);
        all_entries.extend(entries);
    }

    // Pass 2: Extract enum definitions
    let enum_blocks = extract_braced_blocks(content, "enum");
    for (enum_name, block_text) in &enum_blocks {
        let entries = parse_enum_variants(block_text, enum_name);
        all_entries.extend(entries);
    }

    // Pass 3: Extract struct fields
    let struct_blocks = extract_braced_blocks(content, "struct");
    for (struct_name, block_text) in &struct_blocks {
        let entries = parse_struct_fields(block_text, struct_name);
        all_entries.extend(entries);
    }

    // Pass 4: Extract re-exports
    let reexports = parse_reexports(content);
    for re in &reexports {
        all_entries.push(ReferenceEntry {
            name: re.exported_as.clone(),
            kind: EntryKind::ReExport,
            original_path: Some(re.original_path.clone()),
            ..Default::default()
        });
    }

    // Dedup by (name, type_context, kind)
    all_entries.dedup_by(|a, b| {
        a.name == b.name && a.type_context == b.type_context && a.kind == b.kind
    });

    all_entries
}

/// Extract braced blocks like `impl TypeName { ... }` or `enum TypeName { ... }`
fn extract_braced_blocks(
    content: &str,
    keyword: &str,
) -> Vec<(String, String)> {
    let re = match keyword {
        "impl" => &*IMPL_BLOCK_RE,
        _ => {
            // Build regex for enum/struct
            let pattern = format!(
                r"(?:pub\s+)?{}\s+([A-Z][a-zA-Z0-9_]*)\s*\{{",
                regex::escape(keyword)
            );
            // Use a static approach won't work here, so parse manually
            let re = Regex::new(&pattern).expect("valid regex");
            return extract_blocks_with_regex(content, &re);
        }
    };
    extract_blocks_with_regex(content, re)
}

fn extract_blocks_with_regex(
    content: &str,
    re: &Regex,
) -> Vec<(String, String)> {
    let mut results = Vec::new();

    for mat in re.find_iter(content) {
        let cap = re.captures(&content[mat.start()..]).unwrap();
        let type_name = cap[1].to_string();
        let block_start = mat.end();

        // Find matching closing brace
        let mut depth = 1;
        let mut end = block_start;
        for (i, c) in content[block_start..].chars().enumerate() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = block_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let block_text = content[mat.start()..end].to_string();
        results.push((type_name, block_text));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_impl_block_extracts_associated_fn() {
        let block = "impl Runtime {\n    pub fn new() -> Runtime { }\n}";
        let entries = parse_impl_block(block, "Runtime");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "new");
        assert_eq!(entries[0].kind, EntryKind::AssociatedFn);
    }

    #[test]
    fn test_parse_impl_block_extracts_method() {
        let block = "impl Runtime {\n    pub fn block_on(&self, f: F) -> F::Output { }\n}";
        let entries = parse_impl_block(block, "Runtime");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, EntryKind::Method);
        assert_eq!(entries[0].type_context.as_deref(), Some("Runtime"));
    }

    #[test]
    fn test_parse_impl_block_skips_private_fns() {
        let block = "impl Runtime {\n    fn internal() { }\n    pub fn new() -> Runtime { }\n}";
        let entries = parse_impl_block(block, "Runtime");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "new");
    }

    #[test]
    fn test_parse_enum_variants_basic() {
        let entries = parse_enum_variants("pub enum Color {\n    Red,\n    Green,\n    Blue,\n}", "Color");
        assert_eq!(entries.len(), 3);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Red") && names.contains(&"Green") && names.contains(&"Blue"));
    }

    #[test]
    fn test_parse_enum_variants_skips_comments() {
        let entries = parse_enum_variants("enum Foo {\n    // A comment\n    Bar,\n    Baz,\n}", "Foo");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| !e.name.contains("comment")));
    }

    #[test]
    fn test_parse_struct_fields_pub_only() {
        let entries = parse_struct_fields("pub struct Config {\n    pub timeout: u64,\n    internal: bool,\n    pub name: String,\n}", "Config");
        assert_eq!(entries.len(), 2);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"timeout") && names.contains(&"name"));
    }

    #[test]
    fn test_parse_struct_fields_opaque_struct() {
        assert!(parse_struct_fields("pub struct Handle;", "Handle").is_empty());
    }

    #[test]
    fn test_parse_reexports_extracts_path_and_name() {
        let exports = parse_reexports("pub use tokio::task::spawn;");
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].original_path, "tokio::task::spawn");
        assert_eq!(exports[0].exported_as, "spawn");
    }

    #[test]
    fn test_parse_reexports_skips_non_pub() {
        assert!(parse_reexports("use internal::thing;").is_empty());
    }

    #[test]
    fn test_parse_reference_file_v2_full() {
        let content = "impl Runtime {\n    pub fn new() -> Runtime { }\n    pub fn block_on(&self, f: F) -> F::Output { }\n}\npub enum Color {\n    Red,\n    Green,\n    Blue,\n}\npub fn free_function() -> i32 { 0 }";
        assert!(parse_reference_file_v2(content).len() >= 5);
    }

    #[test]
    fn test_parse_reference_file_v2_empty_input() {
        assert!(parse_reference_file_v2("").is_empty());
    }

    #[test]
    fn test_parse_arg_count_zero_arg_fn() {
        assert_eq!(parse_arg_count_from_sig("pub fn new() -> Runtime;"), (Some(0), Some(0)));
    }

    #[test]
    fn test_parse_arg_count_self_only() {
        assert_eq!(parse_arg_count_from_sig("pub fn abort(&self);"), (Some(0), Some(0)));
    }

    #[test]
    fn test_parse_arg_count_one_real_arg() {
        assert_eq!(parse_arg_count_from_sig("pub fn spawn(&self, future: F) -> JoinHandle;"), (Some(1), Some(1)));
    }
}
