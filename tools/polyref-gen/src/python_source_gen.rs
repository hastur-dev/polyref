use std::path::Path;

use crate::python_gen::{PyEntryKind, PyGenEntry, PythonStubOutput};

/// Parse a Python project directory and extract all public API items.
/// Walks the directory for `.py` files and parses classes, functions, and methods.
pub fn parse_python_project(project_dir: &Path) -> anyhow::Result<PythonStubOutput> {
    let module_name = project_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let version = detect_version(project_dir).unwrap_or_else(|| "0.0.0".to_string());

    let mut entries = Vec::new();
    collect_py_files(project_dir, &mut entries)?;

    Ok(PythonStubOutput {
        module_name,
        version,
        entries,
    })
}

fn detect_version(project_dir: &Path) -> Option<String> {
    // Try pyproject.toml
    let pyproject = project_dir.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if let Some(v) = extract_toml_version(&content) {
                return Some(v);
            }
        }
    }

    // Try setup.py
    let setup_py = project_dir.join("setup.py");
    if setup_py.exists() {
        if let Ok(content) = std::fs::read_to_string(&setup_py) {
            if let Some(v) = extract_setup_version(&content) {
                return Some(v);
            }
        }
    }

    None
}

fn extract_toml_version(content: &str) -> Option<String> {
    let mut in_project = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[project]" || trimmed == "[tool.poetry]" {
            in_project = true;
            continue;
        }
        if in_project {
            if trimmed.starts_with('[') {
                in_project = false;
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("version") {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('=') {
                    let val = rest.trim().trim_matches('"').trim_matches('\'');
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn extract_setup_version(content: &str) -> Option<String> {
    // Look for version="x.y.z" in setup() call
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") {
            if let Some(pos) = trimmed.find('=') {
                let val = trimmed[pos + 1..]
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'' || c == ',');
                if !val.is_empty() && val.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn collect_py_files(
    dir: &Path,
    entries: &mut Vec<PyGenEntry>,
) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    // Skip common non-source directories
    let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(
        dir_name,
        "__pycache__"
            | ".git"
            | ".venv"
            | "venv"
            | "env"
            | ".env"
            | "node_modules"
            | ".tox"
            | ".pytest_cache"
            | ".mypy_cache"
            | "dist"
            | "build"
            | "egg-info"
    ) || dir_name.ends_with(".egg-info")
    {
        return Ok(());
    }

    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect_py_files(&path, entries)?;
        } else if path.extension().is_some_and(|ext| ext == "py") {
            // Skip test files and setup files
            let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if filename.starts_with("test_")
                || filename == "setup.py"
                || filename == "conftest.py"
                || filename.starts_with("_")
                    && filename != "__init__.py"
            {
                continue;
            }
            let content = std::fs::read_to_string(&path)?;
            parse_py_source(&content, entries);
        }
    }
    Ok(())
}

/// Parse a single .py file and extract public API items.
pub fn parse_py_source(content: &str, entries: &mut Vec<PyGenEntry>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut current_class: Option<String> = None;
    let mut class_indent: usize = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();

        // Skip empty lines, comments, decorators (handled below)
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        // If we're in a class and we see something at class_indent or less, leave the class
        if current_class.is_some()
            && indent <= class_indent
            && !trimmed.starts_with('@')
            && (!trimmed.starts_with("def ") || indent < class_indent)
        {
            current_class = None;
        }

        // Class definition
        if trimmed.starts_with("class ") && !trimmed.starts_with("class _") {
            let name = extract_py_class_name(trimmed);
            if !name.starts_with('_') {
                current_class = Some(name.clone());
                class_indent = indent;
                // Check for duplicate
                if !entries.iter().any(|e| e.name == name && e.kind == PyEntryKind::Class) {
                    let desc = gather_py_docstring(&lines, i);
                    entries.push(PyGenEntry {
                        name,
                        kind: PyEntryKind::Class,
                        parent: None,
                        signature: String::new(),
                        description: desc,
                        arg_count: 0,
                    });
                }
            }
            i += 1;
            continue;
        }

        // Function/method definition
        if trimmed.starts_with("def ") && !trimmed.starts_with("def _") || trimmed.starts_with("def __init__") {
            let full_sig = gather_py_full_signature(&lines, i);
            let (name, sig, arg_count) = parse_py_def(&full_sig);

            // Skip private methods (except __init__)
            if name.starts_with('_') && name != "__init__" {
                i += 1;
                continue;
            }

            let is_in_class = indent > 0 && current_class.is_some();
            let parent = if is_in_class {
                current_class.clone()
            } else {
                None
            };

            // Check decorator for staticmethod/classmethod
            let is_static = i > 0 && {
                let prev = lines[i - 1].trim();
                prev == "@staticmethod" || prev == "@classmethod"
            };

            let kind = if parent.is_some() {
                if is_static {
                    PyEntryKind::StaticMethod
                } else if sig.starts_with("self") || sig.starts_with("self,") {
                    PyEntryKind::Method
                } else {
                    PyEntryKind::StaticMethod
                }
            } else {
                PyEntryKind::Function
            };

            // Subtract self/cls from arg count
            let real_arg_count = if matches!(kind, PyEntryKind::Method) && arg_count > 0 {
                arg_count - 1
            } else if matches!(kind, PyEntryKind::StaticMethod) && !is_static && arg_count > 0 {
                // If it has 'cls' as first param
                if sig.starts_with("cls") {
                    arg_count - 1
                } else {
                    arg_count
                }
            } else {
                arg_count
            };

            let desc = gather_py_docstring(&lines, i);

            entries.push(PyGenEntry {
                name,
                kind,
                parent,
                signature: sig,
                description: desc,
                arg_count: real_arg_count,
            });
        }

        i += 1;
    }
}

fn extract_py_class_name(line: &str) -> String {
    let after_class = line.strip_prefix("class ").unwrap_or(line);
    after_class
        .split(['(', ':', ' '])
        .next()
        .unwrap_or("Unknown")
        .to_string()
}

fn gather_py_full_signature(lines: &[&str], start: usize) -> String {
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
    }
    sig
}

fn parse_py_def(full_sig: &str) -> (String, String, usize) {
    let after_def = match full_sig.find("def ") {
        Some(pos) => &full_sig[pos + 4..],
        None => return (String::new(), String::new(), 0),
    };

    let paren_start = after_def.find('(');
    let name = match paren_start {
        Some(pos) => after_def[..pos].trim().to_string(),
        None => after_def.trim().to_string(),
    };

    let sig = match paren_start {
        Some(start) => {
            let mut depth = 0;
            let mut end = start;
            for (i, ch) in after_def[start..].char_indices() {
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
            after_def[start + 1..end].trim().to_string()
        }
        None => String::new(),
    };

    let arg_count = if sig.is_empty() {
        0
    } else {
        count_py_args(&sig)
    };

    (name, sig, arg_count)
}

fn count_py_args(sig: &str) -> usize {
    if sig.trim().is_empty() {
        return 0;
    }
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

fn gather_py_docstring(lines: &[&str], def_line: usize) -> String {
    // Look for docstring on line after def/class (after the colon)
    // Find the line with the colon first (may be multi-line def)
    let mut colon_line = def_line;
    for (i, line) in lines.iter().enumerate().take(lines.len().min(def_line + 10)).skip(def_line) {
        if line.contains(':') {
            colon_line = i;
            if line.trim().ends_with(':') {
                break;
            }
        }
    }

    let doc_start = colon_line + 1;
    if doc_start >= lines.len() {
        return String::new();
    }

    let trimmed = lines[doc_start].trim();
    if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        let quote = &trimmed[..3];
        // Single-line docstring
        if trimmed.len() > 6 && trimmed.ends_with(quote) {
            return trimmed[3..trimmed.len() - 3].trim().to_string();
        }
        // Multi-line: take first line
        let first = trimmed[3..].trim().to_string();
        if !first.is_empty() {
            return first;
        }
        // Look at the next line
        if doc_start + 1 < lines.len() {
            return lines[doc_start + 1].trim().to_string();
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_py_source_basic() {
        let source = r#"
class Greeter:
    """A greeter class."""

    def __init__(self, name: str):
        self.name = name

    def greet(self) -> str:
        """Return greeting."""
        return f"Hello, {self.name}"

def helper(x: int, y: int) -> int:
    """Add two numbers."""
    return x + y
"#;
        let mut entries = Vec::new();
        parse_py_source(source, &mut entries);

        let class = entries.iter().find(|e| e.name == "Greeter").unwrap();
        assert_eq!(class.kind, PyEntryKind::Class);
        assert_eq!(class.description, "A greeter class.");

        let init = entries.iter().find(|e| e.name == "__init__").unwrap();
        assert_eq!(init.kind, PyEntryKind::Method);
        assert_eq!(init.parent, Some("Greeter".to_string()));
        assert_eq!(init.arg_count, 1); // self excluded

        let greet = entries.iter().find(|e| e.name == "greet").unwrap();
        assert_eq!(greet.kind, PyEntryKind::Method);
        assert_eq!(greet.arg_count, 0);

        let helper = entries.iter().find(|e| e.name == "helper").unwrap();
        assert_eq!(helper.kind, PyEntryKind::Function);
        assert_eq!(helper.arg_count, 2);
    }

    #[test]
    fn test_parse_py_source_static_method() {
        let source = r#"
class Factory:
    @staticmethod
    def create(name: str) -> "Factory":
        return Factory()
"#;
        let mut entries = Vec::new();
        parse_py_source(source, &mut entries);

        let create = entries.iter().find(|e| e.name == "create").unwrap();
        assert_eq!(create.kind, PyEntryKind::StaticMethod);
        assert_eq!(create.arg_count, 1);
    }

    #[test]
    fn test_parse_py_source_skips_private() {
        let source = r#"
def public_fn():
    pass

def _private_fn():
    pass

class _PrivateClass:
    pass

class PublicClass:
    def _private_method(self):
        pass
    def public_method(self):
        pass
"#;
        let mut entries = Vec::new();
        parse_py_source(source, &mut entries);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"public_fn"));
        assert!(!names.contains(&"_private_fn"));
        assert!(!names.contains(&"_PrivateClass"));
        assert!(names.contains(&"PublicClass"));
        assert!(!names.contains(&"_private_method"));
        assert!(names.contains(&"public_method"));
    }

    #[test]
    fn test_extract_toml_version() {
        let content = r#"
[project]
name = "my-package"
version = "2.1.0"

[build-system]
requires = ["setuptools"]
"#;
        assert_eq!(extract_toml_version(content), Some("2.1.0".to_string()));
    }

    #[test]
    fn test_extract_toml_version_poetry() {
        let content = r#"
[tool.poetry]
name = "my-package"
version = "3.0.1"
"#;
        assert_eq!(extract_toml_version(content), Some("3.0.1".to_string()));
    }

    #[test]
    fn test_extract_setup_version() {
        let content = r#"
from setuptools import setup

setup(
    name="my-package",
    version="1.5.0",
    packages=find_packages(),
)
"#;
        assert_eq!(extract_setup_version(content), Some("1.5.0".to_string()));
    }

    #[test]
    fn test_count_py_args() {
        assert_eq!(count_py_args(""), 0);
        assert_eq!(count_py_args("self"), 1);
        assert_eq!(count_py_args("self, name: str"), 2);
        assert_eq!(count_py_args("x: Dict[str, int], y: int"), 2);
    }

    #[test]
    fn test_multiline_def() {
        let source = r#"
def long_function(
    param1: str,
    param2: int,
    param3: float
) -> bool:
    return True
"#;
        let mut entries = Vec::new();
        parse_py_source(source, &mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "long_function");
        assert_eq!(entries[0].arg_count, 3);
    }

    #[test]
    fn test_docstring_extraction() {
        let lines = vec![
            "def foo():",
            "    \"\"\"This is the doc.\"\"\"",
            "    pass",
        ];
        let doc = gather_py_docstring(&lines, 0);
        assert_eq!(doc, "This is the doc.");
    }

    #[test]
    fn test_docstring_multiline() {
        let lines = vec![
            "def foo():",
            "    \"\"\"",
            "    Multi-line docstring.",
            "    \"\"\"",
            "    pass",
        ];
        let doc = gather_py_docstring(&lines, 0);
        assert_eq!(doc, "Multi-line docstring.");
    }
}
