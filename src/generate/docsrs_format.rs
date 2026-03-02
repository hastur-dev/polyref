use crate::generate::docsrs::{ItemKind, ItemRef, ScrapedCrate, ItemDetail};

/// Format a scraped crate into a reference file matching the existing format
pub fn format_scraped_crate(scraped: &ScrapedCrate, version: &str) -> String {
    let crate_name = &scraped.name;
    let underscored = crate_name.replace('-', "_");

    // Collect top-level item names for the use statement
    let top_level_names: Vec<&str> = scraped
        .items
        .iter()
        .filter(|(item, _)| {
            item.module_prefix.is_none()
                && matches!(
                    item.kind,
                    ItemKind::Struct
                        | ItemKind::Enum
                        | ItemKind::Trait
                        | ItemKind::TypeAlias
                        | ItemKind::Constant
                )
        })
        .map(|(item, _)| item.name.as_str())
        .collect();

    let imports = if top_level_names.is_empty() {
        "*".to_string()
    } else {
        top_level_names.join(", ")
    };

    let mut output = String::new();

    // Header
    output.push_str(&format!("// {} Reference\n", crate_name));
    output.push_str(&format!(
        "// Cargo.toml: {} = \"{}\"\n",
        crate_name, version
    ));
    output.push_str(&format!(
        "// Usage: use {}::{{{}}};\n",
        underscored,
        if imports.len() > 80 {
            "*".to_string()
        } else {
            imports.clone()
        }
    ));
    output.push('\n');

    // Use statement with all top-level items
    if !top_level_names.is_empty() {
        // Split into chunks of 6 to keep lines readable
        for chunk in top_level_names.chunks(6) {
            output.push_str(&format!(
                "use {}::{{{}}};\n",
                underscored,
                chunk.join(", ")
            ));
        }
        output.push('\n');
    }

    // Group items by section (ItemKind)
    let mut grouped: std::collections::BTreeMap<u8, Vec<&(ItemRef, ItemDetail)>> =
        std::collections::BTreeMap::new();

    for pair in &scraped.items {
        let order = pair.0.kind.sort_order();
        grouped.entry(order).or_default().push(pair);
    }

    for items in grouped.values() {
        if items.is_empty() {
            continue;
        }

        let section_name = items[0].0.kind.section_name();
        output.push_str(&crate::generate::templates::section_header(section_name));
        output.push('\n');
        output.push('\n');

        for (item, detail) in items.iter() {
            format_item(&mut output, &underscored, item, detail);
        }

        output.push('\n');
    }

    output
}

/// Format a single item entry
fn format_item(output: &mut String, crate_module: &str, item: &ItemRef, detail: &ItemDetail) {
    let full_name = if let Some(prefix) = &item.module_prefix {
        format!("{}::{}", prefix, item.name)
    } else {
        item.name.clone()
    };

    let comment = if detail.description.is_empty() {
        String::new()
    } else {
        format!("// {}", detail.description)
    };

    match item.kind {
        ItemKind::Struct => {
            let sig = if detail.signature.is_empty() {
                format!("pub struct {} {{ }}", item.name)
            } else {
                simplify_struct_sig(&detail.signature)
            };
            if comment.is_empty() {
                output.push_str(&format!("{}\n", sig));
            } else {
                output.push_str(&format!("{:<40} {}\n", sig, comment));
            }
        }
        ItemKind::Enum => {
            let sig = if detail.signature.is_empty() {
                format!("pub enum {} {{ }}", item.name)
            } else {
                simplify_enum_sig(&detail.signature)
            };
            if comment.is_empty() {
                output.push_str(&format!("{}\n", sig));
            } else {
                output.push_str(&format!("{:<40} {}\n", sig, comment));
            }
        }
        ItemKind::Trait => {
            let sig = if detail.signature.is_empty() {
                format!("pub trait {} {{ }}", item.name)
            } else {
                simplify_trait_sig(&detail.signature)
            };
            if comment.is_empty() {
                output.push_str(&format!("{}\n", sig));
            } else {
                output.push_str(&format!("{:<40} {}\n", sig, comment));
            }
        }
        ItemKind::Function => {
            let sig = if detail.signature.is_empty() {
                format!("pub fn {}()", item.name)
            } else {
                simplify_fn_sig(&detail.signature)
            };
            if comment.is_empty() {
                output.push_str(&format!("{}\n", sig));
            } else {
                output.push_str(&format!("{:<40} {}\n", sig, comment));
            }
        }
        ItemKind::Macro | ItemKind::DeriveMacro | ItemKind::AttrMacro => {
            let entry = format!("{}!(...)", item.name);
            if comment.is_empty() {
                output.push_str(&format!("{}\n", entry));
            } else {
                output.push_str(&format!("{:<40} {}\n", entry, comment));
            }
        }
        ItemKind::TypeAlias => {
            let sig = if detail.signature.is_empty() {
                format!("pub type {} = ...;", item.name)
            } else {
                detail.signature.clone()
            };
            if comment.is_empty() {
                output.push_str(&format!("{}\n", sig));
            } else {
                output.push_str(&format!("{:<40} {}\n", sig, comment));
            }
        }
        ItemKind::Constant => {
            let sig = if detail.signature.is_empty() {
                format!("pub const {}: ... = ...;", item.name)
            } else {
                detail.signature.clone()
            };
            if comment.is_empty() {
                output.push_str(&format!("{}\n", sig));
            } else {
                output.push_str(&format!("{:<40} {}\n", sig, comment));
            }
        }
    }

    // Format methods
    let var_name = to_snake_case(&full_name);
    let _crate_module = crate_module; // used for context if needed

    for method in &detail.methods {
        let method_comment = if method.description.is_empty() {
            String::new()
        } else {
            format!("// {}", method.description)
        };

        let method_sig = simplify_method_sig(&method.signature);
        let args = extract_method_args(&method_sig);
        let entry = format!("{}.{}({})", var_name, method.name, args);

        if method_comment.is_empty() {
            output.push_str(&format!("{}\n", entry));
        } else {
            output.push_str(&format!("{:<40} {}\n", entry, method_comment));
        }
    }

    if !detail.methods.is_empty() {
        output.push('\n');
    }
}

/// Convert a PascalCase name to snake_case
fn to_snake_case(name: &str) -> String {
    // Handle module paths by taking the last component
    let base = name.rsplit("::").next().unwrap_or(name);
    let mut result = String::new();
    for (i, ch) in base.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    result
}

/// Simplify a struct signature to just "pub struct Name { }"
fn simplify_struct_sig(sig: &str) -> String {
    // Extract just the struct name and generic params
    if let Some(idx) = sig.find("struct ") {
        let rest = &sig[idx + 7..];
        let end = rest.find('{').or_else(|| rest.find("where")).unwrap_or(rest.len());
        let name_part = rest[..end].trim();
        format!("pub struct {} {{ }}", name_part)
    } else {
        sig.to_string()
    }
}

/// Simplify an enum signature
fn simplify_enum_sig(sig: &str) -> String {
    if let Some(idx) = sig.find("enum ") {
        let rest = &sig[idx + 5..];
        let end = rest.find('{').unwrap_or(rest.len());
        let name_part = rest[..end].trim();
        format!("pub enum {} {{ }}", name_part)
    } else {
        sig.to_string()
    }
}

/// Simplify a trait signature
fn simplify_trait_sig(sig: &str) -> String {
    if let Some(idx) = sig.find("trait ") {
        let rest = &sig[idx + 6..];
        let end = rest.find('{').unwrap_or(rest.len());
        let name_part = rest[..end].trim();
        format!("pub trait {} {{ }}", name_part)
    } else {
        sig.to_string()
    }
}

/// Simplify a function signature to one line
fn simplify_fn_sig(sig: &str) -> String {
    // Already cleaned up by clean_signature, just ensure it's reasonable length
    if sig.len() > 120 {
        // Truncate at closing paren if possible
        if let Some(paren_idx) = sig.find(')') {
            let rest = &sig[paren_idx + 1..];
            if let Some(arrow_end) = rest.find(['{', ';']) {
                return format!("{}{}", &sig[..paren_idx + 1], &rest[..arrow_end]).trim().to_string();
            }
            return sig[..paren_idx + 1].to_string();
        }
    }
    sig.to_string()
}

/// Simplify a method signature and extract just the relevant parts
fn simplify_method_sig(sig: &str) -> String {
    sig.to_string()
}

/// Extract argument names from a method signature
fn extract_method_args(sig: &str) -> String {
    // Find the parenthesized args section
    if let Some(open) = sig.find('(') {
        if let Some(close) = sig.rfind(')') {
            let args_str = &sig[open + 1..close];
            let args: Vec<&str> = args_str
                .split(',')
                .map(|a| a.trim())
                .filter(|a| !a.is_empty() && *a != "self" && *a != "&self" && *a != "&mut self")
                .map(|a| {
                    // Extract just the parameter name (before the colon)
                    if let Some(colon) = a.find(':') {
                        a[..colon].trim()
                    } else {
                        a
                    }
                })
                .collect();
            return args.join(", ");
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::docsrs::{MethodDetail, ScrapedCrate, ItemRef, ItemDetail, ItemKind};

    fn make_item(name: &str, kind: ItemKind) -> ItemRef {
        ItemRef {
            name: name.to_string(),
            kind,
            path: format!("{}.{}.html", kind_to_prefix(kind), name),
            module_prefix: None,
        }
    }

    fn kind_to_prefix(kind: ItemKind) -> &'static str {
        match kind {
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Trait => "trait",
            ItemKind::Function => "fn",
            ItemKind::Macro | ItemKind::DeriveMacro | ItemKind::AttrMacro => "macro",
            ItemKind::TypeAlias => "type",
            ItemKind::Constant => "constant",
        }
    }

    fn make_detail(sig: &str, desc: &str) -> ItemDetail {
        ItemDetail {
            signature: sig.to_string(),
            description: desc.to_string(),
            methods: Vec::new(),
        }
    }

    #[test]
    fn test_format_scraped_crate_header() {
        let scraped = ScrapedCrate {
            name: "tokio".to_string(),
            items: vec![(
                make_item("Runtime", ItemKind::Struct),
                make_detail("pub struct Runtime { }", "An async runtime"),
            )],
        };

        let output = format_scraped_crate(&scraped, "1.0");
        assert!(output.contains("// tokio Reference"));
        assert!(output.contains("Cargo.toml: tokio = \"1.0\""));
        assert!(output.contains("use tokio::"));
    }

    #[test]
    fn test_format_scraped_crate_groups_by_section() {
        let scraped = ScrapedCrate {
            name: "mycrate".to_string(),
            items: vec![
                (
                    make_item("Config", ItemKind::Struct),
                    make_detail("pub struct Config { }", "Configuration"),
                ),
                (
                    make_item("Error", ItemKind::Enum),
                    make_detail("pub enum Error { }", "Error types"),
                ),
                (
                    make_item("run", ItemKind::Function),
                    make_detail("pub fn run()", "Run the app"),
                ),
            ],
        };

        let output = format_scraped_crate(&scraped, "0.1.0");
        assert!(output.contains("STRUCTS"));
        assert!(output.contains("ENUMS"));
        assert!(output.contains("FUNCTIONS"));

        // Verify section order: structs before enums before functions
        let structs_pos = output.find("STRUCTS").unwrap();
        let enums_pos = output.find("ENUMS").unwrap();
        let functions_pos = output.find("FUNCTIONS").unwrap();
        assert!(structs_pos < enums_pos);
        assert!(enums_pos < functions_pos);
    }

    #[test]
    fn test_format_scraped_crate_method_entries() {
        let scraped = ScrapedCrate {
            name: "mycrate".to_string(),
            items: vec![(
                make_item("Builder", ItemKind::Struct),
                ItemDetail {
                    signature: "pub struct Builder { }".to_string(),
                    description: "A builder pattern".to_string(),
                    methods: vec![
                        MethodDetail {
                            name: "new".to_string(),
                            signature: "pub fn new() -> Builder".to_string(),
                            description: "Create new builder".to_string(),
                        },
                        MethodDetail {
                            name: "build".to_string(),
                            signature: "pub fn build(self) -> Config".to_string(),
                            description: "Build the config".to_string(),
                        },
                    ],
                },
            )],
        };

        let output = format_scraped_crate(&scraped, "1.0");
        assert!(output.contains("builder.new()"), "output was:\n{}", output);
        assert!(output.contains("builder.build()"), "output was:\n{}", output);
    }

    #[test]
    fn test_format_scraped_crate_import_line() {
        let scraped = ScrapedCrate {
            name: "serde".to_string(),
            items: vec![
                (
                    make_item("Serialize", ItemKind::Trait),
                    make_detail("pub trait Serialize { }", "Serialize trait"),
                ),
                (
                    make_item("Deserialize", ItemKind::Trait),
                    make_detail("pub trait Deserialize { }", "Deserialize trait"),
                ),
            ],
        };

        let output = format_scraped_crate(&scraped, "1.0");
        // Should have a use statement with the trait names
        assert!(output.contains("use serde::{Serialize, Deserialize}"));
    }

    #[test]
    fn test_format_scraped_crate_empty() {
        let scraped = ScrapedCrate {
            name: "empty".to_string(),
            items: vec![],
        };

        let output = format_scraped_crate(&scraped, "0.1.0");
        assert!(output.contains("// empty Reference"));
        assert!(output.contains("Cargo.toml"));
        // Should still be valid output even with no items
        assert!(!output.is_empty());
    }

    #[test]
    fn test_format_roundtrip_parsed_by_parse_rust_reference() {
        let scraped = ScrapedCrate {
            name: "testcrate".to_string(),
            items: vec![
                (
                    make_item("Config", ItemKind::Struct),
                    ItemDetail {
                        signature: "pub struct Config { }".to_string(),
                        description: "Main config".to_string(),
                        methods: vec![MethodDetail {
                            name: "new".to_string(),
                            signature: "pub fn new() -> Config".to_string(),
                            description: "Create config".to_string(),
                        }],
                    },
                ),
                (
                    make_item("run", ItemKind::Function),
                    make_detail("pub fn run(config: Config) -> Result<()>", "Run it"),
                ),
                (
                    make_item("log", ItemKind::Macro),
                    make_detail("macro_rules! log", "Logging macro"),
                ),
            ],
        };

        let formatted = format_scraped_crate(&scraped, "1.0");
        let entries = crate::generate::rust::parse_rust_reference(&formatted);

        // Should find struct, function, macro, and method entries
        assert!(
            entries.iter().any(|e| e.kind == crate::generate::EntryKind::Struct && e.name == "Config"),
            "entries: {:?}", entries
        );
        assert!(
            entries.iter().any(|e| e.kind == crate::generate::EntryKind::Method && e.name == "new"),
            "entries: {:?}", entries
        );
        assert!(
            entries.iter().any(|e| e.kind == crate::generate::EntryKind::Macro && e.name == "log!"),
            "entries: {:?}", entries
        );
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HashMap"), "hash_map");
        assert_eq!(to_snake_case("BTreeMap"), "b_tree_map");
        assert_eq!(to_snake_case("Config"), "config");
        assert_eq!(to_snake_case("collections::Entry"), "entry");
    }

    #[test]
    fn test_extract_method_args() {
        assert_eq!(extract_method_args("pub fn new() -> Self"), "");
        assert_eq!(extract_method_args("pub fn get(&self, key: K) -> V"), "key");
        assert_eq!(
            extract_method_args("pub fn insert(&mut self, key: K, value: V) -> Option<V>"),
            "key, value"
        );
    }
}
