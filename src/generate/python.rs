use crate::detect::{Dependency, Language};
use crate::generate::{EntryKind, Generator, ReferenceEntry, ReferenceFile};
use std::path::Path;

pub struct PythonGenerator;

impl Generator for PythonGenerator {
    fn language(&self) -> Language {
        Language::Python
    }

    fn generate(
        &self,
        dep: &Dependency,
        output_dir: &Path,
        global_refs_dir: Option<&Path>,
    ) -> anyhow::Result<ReferenceFile> {
        let file_name = format!("lib_{}.py", dep.name.replace('-', "_"));
        let file_path = output_dir.join("python").join(&file_name);

        // Check for existing user-provided reference file in project refs dir
        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            let entries = parse_python_reference(&content);
            return Ok(ReferenceFile {
                library_name: dep.name.clone(),
                version: dep.version.clone(),
                language: Language::Python,
                entries,
                raw_content: content,
                file_path,
            });
        }

        // Check global refs dir (flat layout: global_refs_dir/lib_*.py)
        if let Some(global_dir) = global_refs_dir {
            let global_path = global_dir.join(&file_name);
            if global_path.exists() {
                let content = std::fs::read_to_string(&global_path)?;
                let entries = parse_python_reference(&content);
                return Ok(ReferenceFile {
                    library_name: dep.name.clone(),
                    version: dep.version.clone(),
                    language: Language::Python,
                    entries,
                    raw_content: content,
                    file_path: global_path,
                });
            }
        }

        // Generate stub
        let content = generate_stub_python(dep);
        let entries = parse_python_reference(&content);

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, &content)?;

        Ok(ReferenceFile {
            library_name: dep.name.clone(),
            version: dep.version.clone(),
            language: Language::Python,
            entries,
            raw_content: content,
            file_path,
        })
    }
}

fn generate_stub_python(dep: &Dependency) -> String {
    let header =
        crate::generate::templates::file_header_python(&dep.name, &dep.version, &dep.name);
    format!(
        "{}\n# NOTE: This is a stub reference file. Populate with actual API documentation.\n",
        header
    )
}

/// Parse a Python reference file into structured entries
pub fn parse_python_reference(content: &str) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();
    let mut current_section = String::new();
    let mut _current_class: Option<String> = None;
    let mut in_class = false;
    let mut class_indent = 0usize;

    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect section headers (# ==== lines)
        if trimmed.starts_with("# ====") {
            // Next non-empty, non-==== line is the section name
            if let Some(next_line) = lines.get(i + 1) {
                let next_trimmed = next_line.trim();
                if next_trimmed.starts_with("# ") && !next_trimmed.starts_with("# ====") {
                    current_section = next_trimmed
                        .trim_start_matches("# ")
                        .trim()
                        .to_string();
                }
            }
            continue;
        }

        // Track class context by indentation
        let indent = line.len() - line.trim_start().len();
        if in_class && indent == 0 && !trimmed.is_empty() && !trimmed.starts_with('#') {
            in_class = false;
            _current_class = None;
        }

        // Parse import statements
        if trimmed.starts_with("import ") && !trimmed.starts_with("import ") {
            continue; // Skip bare imports for entry purposes
        }
        if trimmed.starts_with("from ") && trimmed.contains(" import ") {
            if let Some(import_idx) = trimmed.find(" import ") {
                let items_str = &trimmed[import_idx + 8..];
                for item in items_str.split(',') {
                    let name = item.trim().to_string();
                    if !name.is_empty() && name != "*" {
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
            continue;
        }

        // Parse class declarations
        if trimmed.starts_with("class ") {
            let name = extract_python_class_name(trimmed);
            if let Some(name) = name {
                in_class = true;
                class_indent = indent;
                _current_class = Some(name.clone());
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

        // Check for @property or @staticmethod / @classmethod before a def
        let is_property = i > 0 && lines.get(i.wrapping_sub(1)).is_some_and(|prev| prev.trim() == "@property");
        let is_decorator = trimmed.starts_with('@') && !trimmed.starts_with("@property") && !trimmed.starts_with("@staticmethod") && !trimmed.starts_with("@classmethod");

        // Parse decorators
        if is_decorator {
            let decorator_name = trimmed.trim_start_matches('@');
            let decorator_name = if let Some(paren) = decorator_name.find('(') {
                &decorator_name[..paren]
            } else {
                decorator_name
            };
            entries.push(ReferenceEntry {
                name: decorator_name.trim().to_string(),
                kind: EntryKind::Decorator,
                signature: trimmed.to_string(),
                description: String::new(),
                section: current_section.clone(),
            });
            continue;
        }

        // Parse function/method definitions
        if trimmed.starts_with("def ") {
            if let Some(sig) = extract_python_function_sig(trimmed) {
                let kind = if in_class && indent > class_indent {
                    if is_property {
                        EntryKind::Property
                    } else {
                        EntryKind::Method
                    }
                } else {
                    EntryKind::Function
                };

                entries.push(ReferenceEntry {
                    name: sig.name.clone(),
                    kind,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
            }
            continue;
        }

        // Parse class attributes / constants: NAME: type = value or NAME: type
        if in_class && indent > class_indent {
            if let Some(attr) = parse_python_attribute(trimmed) {
                entries.push(ReferenceEntry {
                    name: attr,
                    kind: EntryKind::Constant,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
                continue;
            }
        }

        // Top-level constants: UPPER_CASE: type = value
        if indent == 0 && !in_class {
            if let Some(const_name) = parse_python_constant(trimmed) {
                entries.push(ReferenceEntry {
                    name: const_name,
                    kind: EntryKind::Constant,
                    signature: trimmed.to_string(),
                    description: String::new(),
                    section: current_section.clone(),
                });
            }
        }
    }

    entries
}

#[derive(Debug)]
pub struct PythonFunctionSig {
    pub name: String,
    pub params: Vec<PythonParam>,
    pub return_type: Option<String>,
}

#[derive(Debug)]
pub struct PythonParam {
    pub name: String,
    pub type_hint: Option<String>,
    pub default: Option<String>,
    pub is_kwargs: bool,
    pub is_args: bool,
}

pub fn extract_python_function_sig(line: &str) -> Option<PythonFunctionSig> {
    let trimmed = line.trim();
    if !trimmed.starts_with("def ") {
        return None;
    }

    let after_def = &trimmed[4..];
    let paren_start = after_def.find('(')?;
    let name = after_def[..paren_start].trim().to_string();

    // Find matching closing paren
    let rest = &after_def[paren_start + 1..];
    let paren_end = find_matching_paren(rest)?;
    let params_str = &rest[..paren_end];

    let params = parse_python_params(params_str);

    // Extract return type
    let after_paren = &rest[paren_end + 1..];
    let return_type = if let Some(arrow_idx) = after_paren.find("->") {
        let ret = after_paren[arrow_idx + 2..].trim();
        let ret = ret.trim_end_matches("...").trim().trim_end_matches(':').trim();
        if ret.is_empty() {
            None
        } else {
            Some(ret.to_string())
        }
    } else {
        None
    };

    Some(PythonFunctionSig {
        name,
        params,
        return_type,
    })
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn parse_python_params(params_str: &str) -> Vec<PythonParam> {
    let mut params = Vec::new();
    if params_str.trim().is_empty() {
        return params;
    }

    // Split on commas, respecting nested brackets
    let parts = split_python_params(params_str);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let is_kwargs = part.starts_with("**");
        let is_args = !is_kwargs && part.starts_with('*');
        let part = part.trim_start_matches("**").trim_start_matches('*');

        // Split on = for default
        let (name_type, default) = if let Some(eq_idx) = part.find('=') {
            let d = part[eq_idx + 1..].trim().to_string();
            (part[..eq_idx].trim(), Some(d))
        } else {
            (part.trim(), None)
        };

        // Split on : for type hint
        let (name, type_hint) = if let Some(colon_idx) = name_type.find(':') {
            let t = name_type[colon_idx + 1..].trim().to_string();
            (
                name_type[..colon_idx].trim().to_string(),
                if t.is_empty() { None } else { Some(t) },
            )
        } else {
            (name_type.to_string(), None)
        };

        if !name.is_empty() {
            params.push(PythonParam {
                name,
                type_hint,
                default,
                is_kwargs,
                is_args,
            });
        }
    }

    params
}

fn split_python_params(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for c in s.chars() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                parts.push(current.clone());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
}

fn extract_python_class_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("class ") {
        return None;
    }
    let rest = &trimmed[6..];
    let end = rest
        .find(|c: char| c == '(' || c == ':' || c.is_whitespace())
        .unwrap_or(rest.len());
    let name = rest[..end].trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn parse_python_attribute(line: &str) -> Option<String> {
    let trimmed = line.trim();
    // Pattern: name: type or name: type = value
    if trimmed.starts_with("def ")
        || trimmed.starts_with('@')
        || trimmed.starts_with('#')
        || trimmed.is_empty()
    {
        return None;
    }
    if let Some(colon_idx) = trimmed.find(':') {
        let name = trimmed[..colon_idx].trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Some(name.to_string());
        }
    }
    None
}

fn parse_python_constant(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with("def ") || trimmed.starts_with("class ") || trimmed.starts_with("import ") || trimmed.starts_with("from ") {
        return None;
    }
    // UPPER_CASE: type = value
    if let Some(colon_idx) = trimmed.find(':') {
        let name = trimmed[..colon_idx].trim();
        if !name.is_empty() && name.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric()) {
            return Some(name.to_string());
        }
    }
    None
}
