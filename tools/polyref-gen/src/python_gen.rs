use std::path::Path;

/// A generated Python reference entry.
#[derive(Debug, Clone)]
pub struct PyGenEntry {
    pub name: String,
    pub kind: PyEntryKind,
    pub parent: Option<String>,
    pub signature: String,
    pub description: String,
    pub arg_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PyEntryKind {
    Function,
    Method,
    Class,
    StaticMethod,
}

/// Parsed Python stub output.
pub struct PythonStubOutput {
    pub module_name: String,
    pub version: String,
    pub entries: Vec<PyGenEntry>,
}

/// Parse a .pyi stub file to extract reference entries.
pub fn parse_pyi_stub(path: &Path) -> anyhow::Result<PythonStubOutput> {
    let content = std::fs::read_to_string(path)?;
    parse_pyi_stub_str(&content, path)
}

/// Parse .pyi stub content from a string.
pub fn parse_pyi_stub_str(content: &str, path: &Path) -> anyhow::Result<PythonStubOutput> {
    let module_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut entries = Vec::new();
    let mut current_class: Option<String> = None;
    let mut version = String::from("0.0.0");

    for line in content.lines() {
        let trimmed = line.trim();

        // Extract version from comment
        if let Some(rest) = trimmed.strip_prefix("# Version:") {
            version = rest.trim().to_string();
            continue;
        }

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Detect class definition
        if trimmed.starts_with("class ") {
            let class_name = extract_class_name(trimmed);
            current_class = Some(class_name.clone());
            entries.push(PyGenEntry {
                name: class_name,
                kind: PyEntryKind::Class,
                parent: None,
                signature: String::new(),
                description: String::new(),
                arg_count: 0,
            });
            continue;
        }

        // Un-indent resets class context
        if !line.starts_with(' ') && !line.starts_with('\t')
            && trimmed.starts_with("def ")
        {
            current_class = None;
        }

        // Detect function/method
        if trimmed.starts_with("def ") || trimmed.starts_with("@staticmethod") {
            let is_static = trimmed.starts_with("@staticmethod");
            let def_line = if is_static {
                // The def should be the next non-decorator line, but in single-line parsing
                // we handle both separately. Skip this decorator line.
                continue;
            } else {
                trimmed
            };

            let (name, sig, arg_count) = extract_def_info(def_line);

            let is_in_class = line.starts_with("    ") || line.starts_with('\t');
            let parent = if is_in_class {
                current_class.clone()
            } else {
                None
            };

            let kind = if parent.is_some() {
                if sig.starts_with("self") || sig.starts_with("self,") {
                    PyEntryKind::Method
                } else {
                    PyEntryKind::StaticMethod
                }
            } else {
                PyEntryKind::Function
            };

            // For methods, subtract self/cls from arg count
            let real_arg_count = if matches!(kind, PyEntryKind::Method | PyEntryKind::StaticMethod)
                && arg_count > 0
            {
                arg_count - 1
            } else {
                arg_count
            };

            entries.push(PyGenEntry {
                name,
                kind,
                parent,
                signature: sig,
                description: String::new(),
                arg_count: real_arg_count,
            });
        }
    }

    Ok(PythonStubOutput {
        module_name,
        version,
        entries,
    })
}

/// Generate a .polyref file from parsed Python stub output.
pub fn generate_polyref_file(output: &PythonStubOutput) -> String {
    let mut lines = Vec::new();
    lines.push("@lang python".to_string());
    lines.push(format!("@module {}", output.module_name));
    lines.push(format!("@version {}", output.version));
    lines.push(String::new());

    // Group by parent
    let mut current_class: Option<&str> = None;

    for entry in &output.entries {
        match entry.kind {
            PyEntryKind::Class => {
                if current_class.is_some() {
                    lines.push(String::new());
                }
                lines.push(format!("@class {}", entry.name));
                current_class = Some(&entry.name);
            }
            PyEntryKind::Method | PyEntryKind::StaticMethod => {
                lines.push(format!(
                    "@method {} args={} // {}",
                    entry.name, entry.arg_count, entry.signature
                ));
            }
            PyEntryKind::Function => {
                if current_class.is_some() {
                    lines.push(String::new());
                    current_class = None;
                }
                lines.push(format!(
                    "@fn {} args={} // {}",
                    entry.name, entry.arg_count, entry.signature
                ));
            }
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

fn extract_class_name(line: &str) -> String {
    let after_class = line.strip_prefix("class ").unwrap_or(line);
    let name = after_class
        .split(['(', ':', ' '])
        .next()
        .unwrap_or("Unknown");
    name.to_string()
}

fn extract_def_info(line: &str) -> (String, String, usize) {
    let after_def = line.strip_prefix("def ").unwrap_or(line);
    let paren_start = after_def.find('(');
    let paren_end = after_def.rfind(')');

    let name = match paren_start {
        Some(pos) => after_def[..pos].trim().to_string(),
        None => after_def.trim().to_string(),
    };

    let sig = match (paren_start, paren_end) {
        (Some(start), Some(end)) if end > start => after_def[start + 1..end].trim().to_string(),
        _ => String::new(),
    };

    let arg_count = if sig.is_empty() {
        0
    } else {
        count_args(&sig)
    };

    (name, sig, arg_count)
}

fn count_args(sig: &str) -> usize {
    if sig.trim().is_empty() {
        return 0;
    }
    // Simple comma counting, accounting for nested brackets
    let mut depth = 0;
    let mut count = 1;
    for ch in sig.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}
