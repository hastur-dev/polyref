use crate::detect::{Dependency, Language};
use crate::generate::{EntryKind, Generator, ReferenceEntry, ReferenceFile};
use std::path::Path;

pub struct TypeScriptGenerator;

impl Generator for TypeScriptGenerator {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn generate(
        &self,
        dep: &Dependency,
        output_dir: &Path,
        global_refs_dir: Option<&Path>,
    ) -> anyhow::Result<ReferenceFile> {
        let file_name = format!("lib_{}.ts", dep.name.replace('-', "_").replace('@', "").replace('/', "_"));
        let file_path = output_dir.join("typescript").join(&file_name);

        // Check for existing user-provided reference file in project refs dir
        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            let entries = parse_typescript_reference(&content);
            return Ok(ReferenceFile {
                library_name: dep.name.clone(),
                version: dep.version.clone(),
                language: Language::TypeScript,
                entries,
                raw_content: content,
                file_path,
            });
        }

        // Check global refs dir (flat layout: global_refs_dir/lib_*.ts)
        if let Some(global_dir) = global_refs_dir {
            let global_path = global_dir.join(&file_name);
            if global_path.exists() {
                let content = std::fs::read_to_string(&global_path)?;
                let entries = parse_typescript_reference(&content);
                return Ok(ReferenceFile {
                    library_name: dep.name.clone(),
                    version: dep.version.clone(),
                    language: Language::TypeScript,
                    entries,
                    raw_content: content,
                    file_path: global_path,
                });
            }
        }

        // Generate stub
        let content = generate_stub_typescript(dep);
        let entries = parse_typescript_reference(&content);

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, &content)?;

        Ok(ReferenceFile {
            library_name: dep.name.clone(),
            version: dep.version.clone(),
            language: Language::TypeScript,
            entries,
            raw_content: content,
            file_path,
        })
    }
}

fn generate_stub_typescript(dep: &Dependency) -> String {
    let header =
        crate::generate::templates::file_header_typescript(&dep.name, &dep.version, &dep.name);
    format!(
        "{}\n// NOTE: This is a stub reference file. Populate with actual API documentation.\n",
        header
    )
}

/// Parse a TypeScript reference file into structured entries
pub fn parse_typescript_reference(content: &str) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();
    let mut current_section = String::new();
    let mut in_class = false;
    let mut in_interface = false;
    let mut brace_depth = 0usize;

    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track brace depth for class/interface body
        for c in trimmed.chars() {
            match c {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                    if brace_depth == 0 {
                        in_class = false;
                        in_interface = false;
                    }
                }
                _ => {}
            }
        }

        // Detect section headers
        if trimmed.starts_with("// ====") {
            if let Some(next_line) = lines.get(i + 1) {
                let next_trimmed = next_line.trim();
                if next_trimmed.starts_with("// ") && !next_trimmed.starts_with("// ====") {
                    current_section = next_trimmed
                        .trim_start_matches("// ")
                        .trim()
                        .to_string();
                }
            }
            continue;
        }

        // Skip comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Parse import statements
        if trimmed.starts_with("import ") {
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
                            });
                        }
                    }
                }
            }
            continue;
        }

        // Parse function declarations
        if trimmed.starts_with("function ")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("declare function ")
        {
            if let Some(entry) = parse_ts_function(trimmed, &current_section) {
                entries.push(entry);
            }
            continue;
        }

        // Parse class declarations
        if trimmed.starts_with("class ")
            || trimmed.starts_with("export class ")
            || trimmed.starts_with("declare class ")
            || trimmed.starts_with("abstract class ")
        {
            if let Some(name) = extract_ts_identifier(trimmed, "class ") {
                in_class = true;
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Class,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
            }
            continue;
        }

        // Parse interface declarations
        if trimmed.starts_with("interface ")
            || trimmed.starts_with("export interface ")
            || trimmed.starts_with("declare interface ")
        {
            if let Some(name) = extract_ts_identifier(trimmed, "interface ") {
                in_interface = true;
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Interface,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
            }
            continue;
        }

        // Parse type aliases
        if trimmed.starts_with("type ")
            || trimmed.starts_with("export type ")
            || trimmed.starts_with("declare type ")
        {
            if let Some(name) = extract_ts_identifier(trimmed, "type ") {
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::TypeAlias,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
            }
            continue;
        }

        // Parse enum declarations
        if trimmed.starts_with("enum ")
            || trimmed.starts_with("export enum ")
            || trimmed.starts_with("declare enum ")
            || trimmed.starts_with("const enum ")
        {
            if let Some(name) = extract_ts_identifier(trimmed, "enum ") {
                entries.push(ReferenceEntry {
                    name,
                    kind: EntryKind::Enum,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
            }
            continue;
        }

        // Parse const declarations
        if trimmed.starts_with("const ")
            || trimmed.starts_with("export const ")
            || trimmed.starts_with("declare const ")
        {
            // Skip destructuring patterns like `const [a, b] = ...`
            if !trimmed.contains('[') && !trimmed.contains('{') {
                if let Some(name) = extract_ts_identifier(trimmed, "const ") {
                    entries.push(ReferenceEntry {
                        name,
                        kind: EntryKind::Constant,
                        signature: trimmed.to_string(),
                        description: String::new(),
                        section: current_section.clone(),
                    });
                }
            }
            continue;
        }

        // Parse methods inside class/interface
        if (in_class || in_interface) && brace_depth > 0 {
            if let Some(entry) = parse_ts_member(trimmed, &current_section) {
                entries.push(entry);
            }
        }
    }

    entries
}

fn parse_ts_function(line: &str, section: &str) -> Option<ReferenceEntry> {
    let trimmed = line.trim();

    // Find "function " and extract name
    let fn_idx = trimmed.find("function ")? + 9;
    let rest = &trimmed[fn_idx..];
    let name_end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    // Detect hooks and components
    let kind = if name.starts_with("use") && name.len() > 3 && name.chars().nth(3).is_some_and(|c| c.is_uppercase()) {
        EntryKind::Hook
    } else if name.chars().next().is_some_and(|c| c.is_uppercase()) && line.contains("JSX.Element") {
        EntryKind::Component
    } else {
        EntryKind::Function
    };

    Some(ReferenceEntry {
        name,
        kind,
        signature: trimmed.to_string(),
        description: String::new(),
        section: section.to_string(),
    })
}

fn extract_ts_identifier(line: &str, keyword: &str) -> Option<String> {
    let idx = line.find(keyword)? + keyword.len();
    let rest = &line[idx..];
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
        .unwrap_or(rest.len());
    let name = rest[..name_end].to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn parse_ts_member(line: &str, section: &str) -> Option<ReferenceEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") || trimmed == "{" || trimmed == "}" {
        return None;
    }

    // Method: name(params): ReturnType;
    if let Some(paren_idx) = trimmed.find('(') {
        let before_paren = trimmed[..paren_idx].trim();
        let name = before_paren
            .split_whitespace()
            .last()?
            .trim_start_matches("readonly ")
            .trim_start_matches("static ")
            .trim_start_matches("async ")
            .to_string();

        if name.is_empty() || name == "constructor" {
            if name == "constructor" {
                return Some(ReferenceEntry {
                    name: "constructor".to_string(),
                    kind: EntryKind::Method,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: section.to_string(),
                });
            }
            return None;
        }

        return Some(ReferenceEntry {
            name,
            kind: EntryKind::Method,
            signature: trimmed.to_string(),
            description: String::new(),
            section: section.to_string(),
        });
    }

    // Property: name: Type;  or  name?: Type;
    if trimmed.contains(':') {
        let colon_idx = trimmed.find(':')?;
        let name = trimmed[..colon_idx]
            .trim()
            .trim_end_matches('?')
            .trim_start_matches("readonly ")
            .trim()
            .to_string();
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
            return Some(ReferenceEntry {
                name,
                kind: EntryKind::Property,
                signature: trimmed.to_string(),
                description: String::new(),
                section: section.to_string(),
            });
        }
    }

    None
}
