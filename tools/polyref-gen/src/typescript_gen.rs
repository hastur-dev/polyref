use std::path::Path;

/// A generated TypeScript reference entry.
#[derive(Debug, Clone)]
pub struct TsGenEntry {
    pub name: String,
    pub kind: TsEntryKind,
    pub parent: Option<String>,
    pub signature: String,
    pub description: String,
    pub arg_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TsEntryKind {
    Function,
    Method,
    Class,
    Interface,
    Field,
}

/// Parsed TypeScript declaration output.
pub struct TypeScriptDeclOutput {
    pub module_name: String,
    pub version: String,
    pub entries: Vec<TsGenEntry>,
}

/// Parse a .d.ts declaration file to extract reference entries.
pub fn parse_dts_file(path: &Path) -> anyhow::Result<TypeScriptDeclOutput> {
    let content = std::fs::read_to_string(path)?;
    parse_dts_str(&content, path)
}

/// Parse .d.ts content from a string.
pub fn parse_dts_str(content: &str, path: &Path) -> anyhow::Result<TypeScriptDeclOutput> {
    let module_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut entries = Vec::new();
    let mut current_parent: Option<String> = None;
    let mut version = String::from("0.0.0");
    let mut brace_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Extract version
        if let Some(rest) = trimmed.strip_prefix("// Version:") {
            version = rest.trim().to_string();
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        // Track brace depth
        for ch in trimmed.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if brace_depth <= 0 {
                        current_parent = None;
                        brace_depth = 0;
                    }
                }
                _ => {}
            }
        }

        // Detect export/declare class/interface
        let clean = trimmed
            .trim_start_matches("export ")
            .trim_start_matches("declare ");

        if clean.starts_with("class ") || clean.starts_with("interface ") {
            let (keyword, rest) = if clean.starts_with("class ") {
                ("class", clean.strip_prefix("class ").unwrap_or(""))
            } else {
                ("interface", clean.strip_prefix("interface ").unwrap_or(""))
            };
            let name = rest
                .split([' ', '{', '<', '('])
                .next()
                .unwrap_or("Unknown")
                .to_string();
            current_parent = Some(name.clone());
            let kind = if keyword == "class" {
                TsEntryKind::Class
            } else {
                TsEntryKind::Interface
            };
            entries.push(TsGenEntry {
                name,
                kind,
                parent: None,
                signature: String::new(),
                description: String::new(),
                arg_count: 0,
            });
            continue;
        }

        // Detect function declarations
        if clean.starts_with("function ") {
            let after = clean.strip_prefix("function ").unwrap_or("");
            let (name, sig, arg_count) = extract_ts_func_info(after);
            entries.push(TsGenEntry {
                name,
                kind: TsEntryKind::Function,
                parent: None,
                signature: sig,
                description: String::new(),
                arg_count,
            });
            continue;
        }

        // Detect methods inside class/interface
        if current_parent.is_some() && brace_depth > 0 {
            let method_line = clean
                .trim_start_matches("public ")
                .trim_start_matches("private ")
                .trim_start_matches("protected ")
                .trim_start_matches("static ")
                .trim_start_matches("readonly ")
                .trim_start_matches("abstract ");

            if method_line.contains('(') && !method_line.starts_with("new ") {
                let (name, sig, arg_count) = extract_ts_func_info(method_line);
                if !name.is_empty() && name != "constructor" {
                    entries.push(TsGenEntry {
                        name,
                        kind: TsEntryKind::Method,
                        parent: current_parent.clone(),
                        signature: sig,
                        description: String::new(),
                        arg_count,
                    });
                }
            } else if method_line.contains(':') && !method_line.contains('(') {
                // Field declaration
                let name = method_line
                    .split([':',  '?'])
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    entries.push(TsGenEntry {
                        name,
                        kind: TsEntryKind::Field,
                        parent: current_parent.clone(),
                        signature: String::new(),
                        description: String::new(),
                        arg_count: 0,
                    });
                }
            }
        }
    }

    Ok(TypeScriptDeclOutput {
        module_name,
        version,
        entries,
    })
}

/// Generate a .polyref file from parsed TypeScript declaration output.
pub fn generate_polyref_file(output: &TypeScriptDeclOutput) -> String {
    let mut lines = Vec::new();
    lines.push("@lang typescript".to_string());
    lines.push(format!("@module {}", output.module_name));
    lines.push(format!("@version {}", output.version));
    lines.push(String::new());

    let mut current_class: Option<&str> = None;

    for entry in &output.entries {
        match entry.kind {
            TsEntryKind::Class | TsEntryKind::Interface => {
                if current_class.is_some() {
                    lines.push(String::new());
                }
                lines.push(format!("@class {}", entry.name));
                current_class = Some(&entry.name);
            }
            TsEntryKind::Method => {
                lines.push(format!("@method {} args={}", entry.name, entry.arg_count));
            }
            TsEntryKind::Field => {
                lines.push(format!("@field {}", entry.name));
            }
            TsEntryKind::Function => {
                if current_class.is_some() {
                    lines.push(String::new());
                    current_class = None;
                }
                lines.push(format!("@fn {} args={}", entry.name, entry.arg_count));
            }
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

fn extract_ts_func_info(line: &str) -> (String, String, usize) {
    let paren_start = line.find('(');
    let name = match paren_start {
        Some(pos) => line[..pos].trim().to_string(),
        None => line.trim().to_string(),
    };

    // Find matching closing paren
    let sig = match paren_start {
        Some(start) => {
            let mut depth = 0;
            let mut end = start;
            for (i, ch) in line[start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            line[start + 1..end].trim().to_string()
        }
        None => String::new(),
    };

    let arg_count = if sig.is_empty() {
        0
    } else {
        count_ts_args(&sig)
    };

    (name, sig, arg_count)
}

fn count_ts_args(sig: &str) -> usize {
    if sig.trim().is_empty() {
        return 0;
    }
    let mut depth = 0;
    let mut count = 1;
    for ch in sig.chars() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}
