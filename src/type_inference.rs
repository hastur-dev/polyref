use std::collections::HashMap;

use regex::Regex;
use std::sync::LazyLock;

use crate::generate::{EntryKind, ReferenceEntry};

/// Holds inferred type bindings for variables in a source file.
#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    pub bindings: HashMap<String, String>, // var_name → type_name
}

/// Result of checking a method call with type context
#[derive(Debug, PartialEq)]
pub enum MethodCheckResult {
    Valid,
    Invalid { suggestion: Option<String> },
    Unknown, // type not inferrable, caller decides
}

static EXPLICIT_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"let\s+(?:mut\s+)?([a-z_][a-z0-9_]*)\s*:\s*([A-Z][a-zA-Z0-9_]*)").expect("valid regex")
});

static CONSTRUCTOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"let\s+(?:mut\s+)?([a-z_][a-z0-9_]*)\s*=\s*([A-Z][a-zA-Z0-9_]*)::").expect("valid regex")
});

/// Infer type from explicit annotation: `let var: Type = ...`
pub fn infer_explicit_type_binding(line: &str) -> Option<(String, String)> {
    let cap = EXPLICIT_TYPE_RE.captures(line)?;
    let var_name = cap[1].to_string();
    let type_name = cap[2].to_string();

    debug_assert!(!var_name.is_empty(), "var_name must be non-empty");
    debug_assert!(
        type_name.starts_with(|c: char| c.is_uppercase()),
        "type_name must start with uppercase"
    );

    Some((var_name, type_name))
}

/// Infer type from constructor call: `let var = Type::...`
pub fn infer_constructor_binding(line: &str) -> Option<(String, String)> {
    let cap = CONSTRUCTOR_RE.captures(line)?;
    let var_name = cap[1].to_string();
    let type_name = cap[2].to_string();

    debug_assert!(
        type_name.starts_with(|c: char| c.is_uppercase()),
        "type_name must start with uppercase"
    );
    debug_assert!(
        !var_name.contains(char::is_whitespace),
        "var_name must not contain whitespace"
    );

    Some((var_name, type_name))
}

/// Build a type context from all lines in a source file.
pub fn build_type_context(source_lines: &[&str]) -> TypeContext {
    let mut ctx = TypeContext::default();

    for line in source_lines {
        if let Some((var, typ)) = infer_explicit_type_binding(line) {
            ctx.bindings.insert(var, typ);
        } else if let Some((var, typ)) = infer_constructor_binding(line) {
            ctx.bindings.insert(var, typ);
        }
    }

    debug_assert!(
        ctx.bindings.keys().all(|k| !k.is_empty()),
        "no empty keys"
    );
    debug_assert!(
        ctx.bindings.values().all(|v| !v.is_empty()),
        "no empty values"
    );

    ctx
}

/// Resolve the type of a receiver variable from the context.
pub fn resolve_receiver_type<'a>(
    receiver: &str,
    ctx: &'a TypeContext,
) -> Option<&'a str> {
    if receiver.is_empty() {
        return None;
    }
    ctx.bindings.get(receiver).map(|s| s.as_str())
}

/// Check a method call with type context.
///
/// If the receiver type is known, only checks methods for that type.
/// If unknown, returns `Unknown` for the caller to handle.
pub fn check_method_with_type_context(
    method: &str,
    receiver: &str,
    ctx: &TypeContext,
    refs: &[ReferenceEntry],
) -> MethodCheckResult {
    if method.is_empty() || receiver.is_empty() {
        return MethodCheckResult::Unknown;
    }

    let resolved_type = match resolve_receiver_type(receiver, ctx) {
        Some(t) => t,
        None => return MethodCheckResult::Unknown,
    };

    // Check if method exists for this type
    let type_methods: Vec<&ReferenceEntry> = refs
        .iter()
        .filter(|r| {
            r.kind == EntryKind::Method
                && r.type_context.as_deref() == Some(resolved_type)
        })
        .collect();

    // If no methods registered for this type, return Unknown
    if type_methods.is_empty() {
        return MethodCheckResult::Unknown;
    }

    // Exact match
    if type_methods.iter().any(|r| r.name == method) {
        return MethodCheckResult::Valid;
    }

    // Fuzzy suggestion
    let method_names: Vec<&str> = type_methods.iter().map(|r| r.name.as_str()).collect();
    let mut best_score = 0.0;
    let mut best_name: Option<String> = None;

    for &candidate in &method_names {
        let score = strsim::jaro_winkler(method, candidate);
        if score > best_score {
            best_score = score;
            best_name = Some(candidate.to_string());
        }
    }

    let suggestion = if best_score >= 0.35 { best_name } else { None };

    MethodCheckResult::Invalid { suggestion }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::ReferenceEntry;

    fn make_method_ref(type_name: &str, method_name: &str) -> ReferenceEntry {
        ReferenceEntry {
            name: method_name.to_string(),
            kind: EntryKind::Method,
            type_context: Some(type_name.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_infer_explicit_type_binding() {
        let result = infer_explicit_type_binding(
            "let handle: JoinHandle = rt.spawn(async { 42 });",
        );
        assert!(result.is_some());
        let (var, typ) = result.unwrap();
        assert_eq!(var, "handle");
        assert_eq!(typ, "JoinHandle");
    }

    #[test]
    fn test_infer_explicit_type_no_match() {
        let result = infer_explicit_type_binding("let x = 42;");
        assert!(result.is_none());
    }

    #[test]
    fn test_infer_constructor_binding() {
        let result = infer_constructor_binding("let rt = Runtime::new().unwrap();");
        assert!(result.is_some());
        let (var, typ) = result.unwrap();
        assert_eq!(var, "rt");
        assert_eq!(typ, "Runtime");
    }

    #[test]
    fn test_infer_constructor_binding_no_match() {
        let result = infer_constructor_binding("let x = some_function();");
        assert!(result.is_none());
    }

    #[test]
    fn test_build_type_context_multiple_bindings() {
        let lines = vec![
            "let rt = Runtime::new();",
            "let handle: JoinHandle = rt.spawn(task);",
            "let set = JoinSet::new();",
        ];
        let ctx = build_type_context(&lines);
        assert_eq!(ctx.bindings.len(), 3);
    }

    #[test]
    fn test_build_type_context_overwrites_reassignment() {
        let lines = vec![
            "let x = A::new();",
            "let x = B::new();",
        ];
        let ctx = build_type_context(&lines);
        assert_eq!(ctx.bindings.get("x").unwrap(), "B");
    }

    #[test]
    fn test_resolve_known_receiver() {
        let mut ctx = TypeContext::default();
        ctx.bindings.insert("rt".to_string(), "Runtime".to_string());
        assert_eq!(resolve_receiver_type("rt", &ctx), Some("Runtime"));
    }

    #[test]
    fn test_resolve_unknown_receiver() {
        let ctx = TypeContext::default();
        assert_eq!(resolve_receiver_type("rt", &ctx), None);
    }

    #[test]
    fn test_method_check_with_known_type_valid() {
        let refs = vec![make_method_ref("Runtime", "block_on")];
        let mut ctx = TypeContext::default();
        ctx.bindings.insert("rt".to_string(), "Runtime".to_string());

        let result = check_method_with_type_context("block_on", "rt", &ctx, &refs);
        assert_eq!(result, MethodCheckResult::Valid);
    }

    #[test]
    fn test_method_check_with_known_type_invalid() {
        let refs = vec![make_method_ref("Runtime", "block_on")];
        let mut ctx = TypeContext::default();
        ctx.bindings.insert("rt".to_string(), "Runtime".to_string());

        let result = check_method_with_type_context("block_forever", "rt", &ctx, &refs);
        assert!(matches!(result, MethodCheckResult::Invalid { .. }));
    }

    #[test]
    fn test_method_check_with_unknown_type_returns_unknown() {
        let refs = vec![make_method_ref("Runtime", "block_on")];
        let ctx = TypeContext::default();

        let result = check_method_with_type_context("block_on", "unknown_var", &ctx, &refs);
        assert_eq!(result, MethodCheckResult::Unknown);
    }

    #[test]
    fn test_build_context_no_empty_keys() {
        let lines = vec!["let rt = Runtime::new();", "let x = 42;"];
        let ctx = build_type_context(&lines);
        assert!(ctx.bindings.keys().all(|k| !k.is_empty()));
    }

    #[test]
    fn test_infer_extracts_base_type_not_generic() {
        let result = infer_explicit_type_binding("let h: JoinHandle<i32> = rt.spawn(task);");
        assert!(result.is_some());
        let (_, typ) = result.unwrap();
        assert_eq!(typ, "JoinHandle");
        assert!(!typ.contains('<'));
    }
}
