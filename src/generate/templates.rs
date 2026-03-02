/// Generate a section header banner
pub fn section_header(title: &str) -> String {
    format!(
        "// ============================================================================\n// {}\n// ============================================================================",
        title
    )
}

/// Generate a Rust reference file header
pub fn file_header_rust(name: &str, version: &str, imports: &str) -> String {
    format!(
        "// {} Reference\n// Cargo.toml: {} = \"{}\"\n// Usage: use {}::{{{}}};\n",
        name, name, version, name, imports
    )
}

/// Generate a Python reference file header
pub fn file_header_python(name: &str, version: &str, imports: &str) -> String {
    format!(
        "# {} Reference\n# pip install {}=={}\n# Usage: import {} / from {} import {}\n",
        name, name, version, name, name, imports
    )
}

/// Generate a TypeScript reference file header
pub fn file_header_typescript(name: &str, version: &str, imports: &str) -> String {
    format!(
        "// {} Reference\n// package.json: \"{}\": \"{}\"\n// Usage: import {{ {} }} from '{}';\n",
        name, name, version, imports, name
    )
}

/// Python section header
pub fn section_header_python(title: &str) -> String {
    format!(
        "# ============================================================================\n# {}\n# ============================================================================",
        title
    )
}
