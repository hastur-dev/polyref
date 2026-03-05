use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// A generated reference entry from rustdoc JSON.
#[derive(Debug, Clone)]
pub struct GenEntry {
    pub name: String,
    pub kind: EntryKind,
    pub parent: Option<String>,
    pub signature: String,
    pub description: String,
    pub arg_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryKind {
    Function,
    Method,
    AssociatedFunction,
    Struct,
    Enum,
    Field,
    Variant,
}

/// Parsed rustdoc JSON output.
pub struct RustdocOutput {
    pub crate_name: String,
    pub crate_version: String,
    pub entries: Vec<GenEntry>,
}

/// Parse a rustdoc JSON file and extract reference entries.
pub fn parse_rustdoc_json(path: &Path) -> anyhow::Result<RustdocOutput> {
    let content = std::fs::read_to_string(path)?;
    parse_rustdoc_json_str(&content)
}

/// Parse rustdoc JSON from a string.
pub fn parse_rustdoc_json_str(content: &str) -> anyhow::Result<RustdocOutput> {
    let doc: Value = serde_json::from_str(content)?;
    let index = doc["index"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("missing index in rustdoc JSON"))?;

    let root_id = doc["root"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing root"))?;
    let root = &index[root_id];
    let crate_name = root["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let crate_version = doc["crate_version"]
        .as_str()
        .unwrap_or("0.0.0")
        .to_string();

    // Build a map from item ID to the impl's "for" type name (for methods)
    let mut impl_parent_map: HashMap<String, String> = HashMap::new();
    for (_id, item) in index {
        if let Some(impl_obj) = item["inner"].get("impl") {
            let for_type = extract_type_name(&impl_obj["for_"]);
            if let Some(items) = impl_obj["items"].as_array() {
                for item_id in items {
                    if let Some(id_str) = item_id.as_str() {
                        impl_parent_map.insert(id_str.to_string(), for_type.clone());
                    }
                }
            }
        }
    }

    let mut entries = Vec::new();

    for (id, item) in index {
        let vis = item["visibility"].as_str().unwrap_or("private");
        if vis != "public" {
            continue;
        }

        let name = match item["name"].as_str() {
            Some(n) => n.to_string(),
            None => continue, // Skip unnamed items (like impl blocks)
        };

        let docs = item["docs"].as_str().unwrap_or("").to_string();
        let inner = &item["inner"];

        if let Some(func) = inner.get("function") {
            let parent = impl_parent_map.get(id.as_str()).cloned();
            let (sig, arg_count, kind) = extract_function_info(&name, func, &parent);
            entries.push(GenEntry {
                name,
                kind,
                parent,
                signature: sig,
                description: first_line(&docs),
                arg_count,
            });
        } else if inner.get("struct").is_some() {
            entries.push(GenEntry {
                name,
                kind: EntryKind::Struct,
                parent: None,
                signature: String::new(),
                description: first_line(&docs),
                arg_count: 0,
            });
        } else if inner.get("enum").is_some() {
            entries.push(GenEntry {
                name,
                kind: EntryKind::Enum,
                parent: None,
                signature: String::new(),
                description: first_line(&docs),
                arg_count: 0,
            });
        } else if inner.get("struct_field").is_some() {
            let parent = impl_parent_map.get(id.as_str()).cloned();
            entries.push(GenEntry {
                name,
                kind: EntryKind::Field,
                parent,
                signature: String::new(),
                description: first_line(&docs),
                arg_count: 0,
            });
        } else if inner.get("variant").is_some() {
            entries.push(GenEntry {
                name,
                kind: EntryKind::Variant,
                parent: None,
                signature: String::new(),
                description: first_line(&docs),
                arg_count: 0,
            });
        }
    }

    // Sort for deterministic output
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

/// Generate a .rs reference file from parsed rustdoc output.
pub fn generate_ref_file(output: &RustdocOutput) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "// Version: {}",
        output.crate_version
    ));
    lines.push(format!(
        "// Auto-generated reference for {}",
        output.crate_name
    ));
    lines.push(String::new());

    // Group by parent
    let mut by_parent: HashMap<Option<&str>, Vec<&GenEntry>> = HashMap::new();
    for entry in &output.entries {
        by_parent
            .entry(entry.parent.as_deref())
            .or_default()
            .push(entry);
    }

    // Free functions first
    if let Some(fns) = by_parent.remove(&None) {
        let mut types: Vec<&&GenEntry> = Vec::new();
        let mut funcs: Vec<&&GenEntry> = Vec::new();
        for entry in fns.iter() {
            if matches!(entry.kind, EntryKind::Struct | EntryKind::Enum | EntryKind::Variant | EntryKind::Field) {
                types.push(entry);
            } else {
                funcs.push(entry);
            }
        }

        for entry in &types {
            match entry.kind {
                EntryKind::Struct => {
                    lines.push(format!("// struct {} — {}", entry.name, entry.description));
                }
                EntryKind::Enum => {
                    lines.push(format!("// enum {} — {}", entry.name, entry.description));
                }
                _ => {}
            }
        }
        if !types.is_empty() {
            lines.push(String::new());
        }

        for entry in &funcs {
            lines.push(format!(
                "fn {}({}) // {} [min_args={}, max_args={}]",
                entry.name,
                entry.signature,
                entry.description,
                entry.arg_count,
                entry.arg_count,
            ));
        }
        if !funcs.is_empty() {
            lines.push(String::new());
        }
    }

    // Then impl blocks
    let mut parents: Vec<&str> = by_parent.keys().filter_map(|k| *k).collect();
    parents.sort();

    for parent in parents {
        if let Some(methods) = by_parent.get(&Some(parent)) {
            lines.push(format!("impl {} {{", parent));
            for entry in methods {
                let kind_label = match entry.kind {
                    EntryKind::AssociatedFunction => "fn",
                    EntryKind::Method => "fn",
                    _ => "fn",
                };
                lines.push(format!(
                    "    {} {}({}) // {} [min_args={}, max_args={}]",
                    kind_label,
                    entry.name,
                    entry.signature,
                    entry.description,
                    entry.arg_count,
                    entry.arg_count,
                ));
            }
            lines.push("}".to_string());
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

fn extract_function_info(
    _name: &str,
    func: &Value,
    parent: &Option<String>,
) -> (String, usize, EntryKind) {
    let sig = &func["sig"];
    let inputs = sig["inputs"].as_array();

    let mut args = Vec::new();
    let mut has_self = false;
    let mut real_arg_count = 0;

    if let Some(inputs) = inputs {
        for input in inputs {
            if let Some(arr) = input.as_array() {
                if !arr.is_empty() {
                    let param_name = arr[0].as_str().unwrap_or("_");
                    if param_name == "self" {
                        has_self = true;
                        args.push("&self".to_string());
                    } else {
                        real_arg_count += 1;
                        let type_name = if arr.len() >= 2 {
                            extract_type_name(&arr[1])
                        } else {
                            "_".to_string()
                        };
                        args.push(format!("{}: {}", param_name, type_name));
                    }
                }
            }
        }
    }

    let sig_str = args.join(", ");
    let kind = if parent.is_some() {
        if has_self {
            EntryKind::Method
        } else {
            EntryKind::AssociatedFunction
        }
    } else {
        EntryKind::Function
    };

    (sig_str, real_arg_count, kind)
}

fn extract_type_name(ty: &Value) -> String {
    if let Some(p) = ty.get("primitive") {
        return p.as_str().unwrap_or("_").to_string();
    }
    if let Some(rp) = ty.get("resolved_path") {
        return rp["name"].as_str().unwrap_or("_").to_string();
    }
    if let Some(g) = ty.get("generic") {
        return g.as_str().unwrap_or("T").to_string();
    }
    if ty.get("borrowed_ref").is_some() {
        let inner = &ty["borrowed_ref"]["type_"];
        let inner_name = extract_type_name(inner);
        let mutable = ty["borrowed_ref"]["mutable"].as_bool().unwrap_or(false);
        if mutable {
            return format!("&mut {}", inner_name);
        }
        return format!("&{}", inner_name);
    }
    "_".to_string()
}

fn first_line(s: &str) -> String {
    s.lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}
