use crate::detect::{Dependency, Language};
use crate::generate::{EntryKind, Generator, ReferenceEntry, ReferenceFile};
use std::path::Path;

pub struct RustGenerator;

impl Generator for RustGenerator {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn generate(
        &self,
        dep: &Dependency,
        output_dir: &Path,
        global_refs_dir: Option<&Path>,
    ) -> anyhow::Result<ReferenceFile> {
        let file_name = format!("lib_{}.rs", dep.name.replace('-', "_"));
        let file_path = output_dir.join("rust").join(&file_name);

        // Check for existing user-provided reference file in project refs dir
        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            let entries = parse_rust_reference(&content);
            return Ok(ReferenceFile {
                library_name: dep.name.clone(),
                version: dep.version.clone(),
                language: Language::Rust,
                entries,
                raw_content: content,
                file_path,
            });
        }

        // Check global refs dir (flat layout: global_refs_dir/lib_*.rs)
        if let Some(global_dir) = global_refs_dir {
            let global_path = global_dir.join(&file_name);
            if global_path.exists() {
                let content = std::fs::read_to_string(&global_path)?;
                let entries = parse_rust_reference(&content);
                return Ok(ReferenceFile {
                    library_name: dep.name.clone(),
                    version: dep.version.clone(),
                    language: Language::Rust,
                    entries,
                    raw_content: content,
                    file_path: global_path,
                });
            }
        }

        // Try scraping docs.rs, fall back to stub on failure or if disabled
        let content = if std::env::var("POLYREF_NO_FETCH").is_err() {
            match crate::generate::docsrs::scrape_crate(&dep.name) {
                Ok(scraped) => {
                    crate::generate::docsrs_format::format_scraped_crate(&scraped, &dep.version)
                }
                Err(_) => generate_stub_rust(dep),
            }
        } else {
            generate_stub_rust(dep)
        };
        let entries = parse_rust_reference(&content);

        // Write the file
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, &content)?;

        Ok(ReferenceFile {
            library_name: dep.name.clone(),
            version: dep.version.clone(),
            language: Language::Rust,
            entries,
            raw_content: content,
            file_path,
        })
    }
}

fn generate_stub_rust(dep: &Dependency) -> String {
    let header = crate::generate::templates::file_header_rust(&dep.name, &dep.version, "*");
    format!(
        "{}\n// NOTE: This is a stub reference file. Populate with actual API documentation.\n// See docs.rs/{}/{} for the full API.\n",
        header, dep.name, dep.version
    )
}

/// Parse a Rust reference file into structured entries
pub fn parse_rust_reference(content: &str) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headers
        if trimmed.starts_with("// ====") {
            continue;
        }
        // Section name comes after a line of ====
        if trimmed.starts_with("// ")
            && !trimmed.starts_with("// -")
            && !trimmed.contains("Reference")
            && !trimmed.contains("Cargo.toml")
            && !trimmed.contains("Usage:")
            && !trimmed.contains("NOTE:")
            && !trimmed.contains("docs.rs")
            && !trimmed.contains("See ")
        {
            // Check if this looks like a section header (all caps or title case, no code)
            let text = trimmed.trim_start_matches("// ").trim();
            if !text.is_empty()
                && !text.contains('(')
                && !text.contains('=')
                && !text.contains(';')
                && text.len() < 80
            {
                // Could be a section name — only treat as section if the previous line was ====
                // For simplicity, track any comment-only line that looks like a title
                let is_section = text.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c == '&' || c == '/' || c == '-' || c == '_');
                if is_section && text.len() > 2 {
                    current_section = text.to_string();
                    continue;
                }
            }
        }

        // Parse use statements
        if trimmed.starts_with("use ") && trimmed.contains("::") {
            // Extract imported items
            if let Some(brace_start) = trimmed.find('{') {
                if let Some(brace_end) = trimmed.find('}') {
                    let items = &trimmed[brace_start + 1..brace_end];
                    for item in items.split(',') {
                        let name = item.trim().to_string();
                        if !name.is_empty() {
                            entries.push(ReferenceEntry {
                                name,
                                kind: EntryKind::Module,
                                signature: trimmed.to_string(),
                                description: String::new(),
                                section: current_section.clone(),
                            ..Default::default()
                            });
                        }
                    }
                }
            }
            continue;
        }

        // Parse function declarations: fn name(...)
        if let Some(fn_entry) = parse_fn_line(trimmed, &current_section) {
            entries.push(fn_entry);
            continue;
        }

        // Parse struct declarations
        if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
            let name = extract_identifier(trimmed, "struct ");
            if let Some(name) = name {
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Struct,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                            ..Default::default()
                });
            }
            continue;
        }

        // Parse enum declarations
        if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
            let name = extract_identifier(trimmed, "enum ");
            if let Some(name) = name {
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Enum,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                            ..Default::default()
                });
            }
            continue;
        }

        // Parse trait declarations
        if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
            let name = extract_identifier(trimmed, "trait ");
            if let Some(name) = name {
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Trait,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                            ..Default::default()
                });
            }
            continue;
        }

        // Parse const declarations
        if trimmed.starts_with("pub const ") || trimmed.starts_with("const ") {
            let name = extract_identifier(trimmed, "const ");
            if let Some(name) = name {
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Constant,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                            ..Default::default()
                });
            }
            continue;
        }

        // Parse macro invocations: name!(...)
        if let Some(excl_idx) = trimmed.find("!(") {
            let macro_name = trimmed[..excl_idx].trim().trim_start_matches("let ").trim();
            // Only treat as macro if the name is a simple identifier
            let macro_name = macro_name.rsplit(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
            if !macro_name.is_empty() && macro_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                entries.push(ReferenceEntry {
                    name: format!("{}!", macro_name),
                    kind: EntryKind::Macro,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                            ..Default::default()
                });
            }
            continue;
        }

        // Parse method calls: type.method(...)
        if trimmed.contains('.') && trimmed.contains('(') && !trimmed.starts_with("//") && !trimmed.starts_with('#') {
            if let Some(method_entry) = parse_method_call(trimmed, &current_section) {
                entries.push(method_entry);
            }
        }
    }

    entries
}

fn parse_fn_line(line: &str, section: &str) -> Option<ReferenceEntry> {
    let trimmed = line.trim();

    // Match lines starting with fn, pub fn, async fn, pub async fn
    let fn_prefix = if trimmed.starts_with("pub async fn ") {
        Some("pub async fn ")
    } else if trimmed.starts_with("pub fn ") {
        Some("pub fn ")
    } else if trimmed.starts_with("async fn ") {
        Some("async fn ")
    } else if trimmed.starts_with("fn ") {
        Some("fn ")
    } else {
        None
    };

    let prefix = fn_prefix?;
    let rest = &trimmed[prefix.len()..];
    let name_end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    Some(ReferenceEntry {
        name,
        kind: EntryKind::Function,
        signature: trimmed.to_string(),
        description: String::new(),
        section: section.to_string(),
                            ..Default::default()
    })
}

fn extract_identifier(line: &str, keyword: &str) -> Option<String> {
    let idx = line.find(keyword)? + keyword.len();
    let rest = &line[idx..];
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
    let name = rest[..name_end].to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn parse_method_call(line: &str, section: &str) -> Option<ReferenceEntry> {
    let trimmed = line.trim();
    // Find pattern: identifier.method(
    let dot_idx = trimmed.find('.')?;
    let after_dot = &trimmed[dot_idx + 1..];
    let paren_idx = after_dot.find('(')?;
    let method_name = after_dot[..paren_idx].trim();

    if method_name.is_empty() || method_name.contains(' ') || method_name.contains('.') {
        return None;
    }

    // Verify the method name is a valid identifier
    if !method_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    Some(ReferenceEntry {
        name: method_name.to_string(),
        kind: EntryKind::Method,
        signature: trimmed.to_string(),
        description: String::new(),
        section: section.to_string(),
                            ..Default::default()
    })
}
