/// Model output ingestor: extract code from markdown fences.
///
/// Handles raw model output that wraps code in markdown fences (```lang ... ```).
/// Useful for processing LLM-generated code that may include explanations.

use regex::Regex;
use std::sync::OnceLock;

/// Regex pattern for markdown fenced code blocks: ```lang\ncode\n```
/// Group 1: language hint (e.g., "rust", "python"), can be empty
/// Group 2: code content (non-greedy, matches across newlines)
fn fence_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"```(\w*)\n([\s\S]*?)```").expect("fence regex is valid")
    })
}

/// Extract code from markdown fenced blocks in model output.
///
/// # Strategy
/// 1. If `lang_hint` is non-empty, prefer the first fenced block whose language tag matches
///    (case-insensitive).
/// 2. If no matching block found (or lang_hint is empty), use the first fenced block.
/// 3. If no fenced blocks found, return the raw content as-is (fallback).
///
/// # Arguments
/// - `content`: Raw model output (may include explanations, fenced code, etc.)
/// - `lang_hint`: Optional language hint (e.g., "rust", "python").
///   If provided and a fenced block matches, it will be preferred.
///
/// # Returns
/// Extracted code block, or raw content if no fences found.
pub fn extract_code_from_model_output(content: &str, lang_hint: &str) -> String {
    assert!(!content.is_empty(), "content must be non-empty");

    let regex = fence_regex();
    let matches: Vec<_> = regex.find_iter(content).collect();

    if matches.is_empty() {
        // No fenced blocks found; return raw content
        return content.to_string();
    }

    // If lang_hint is provided, prefer matching block
    if !lang_hint.is_empty() {
        for m in &matches {
            let caps = regex.captures(m.as_str()).expect("valid capture");
            let lang_tag = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if lang_tag.eq_ignore_ascii_case(lang_hint) {
                let code = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();
                assert!(
                    !code.is_empty(),
                    "extracted block is empty (shouldn't happen with regex)"
                );
                assert!(!code.contains("```"), "extracted code contains backticks");
                return code.to_string();
            }
        }
    }

    // Fall back to the first fenced block
    if let Some(first) = matches.first() {
        let caps = regex
            .captures(first.as_str())
            .expect("valid first capture");
        let code = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();
        assert!(
            !code.is_empty(),
            "extracted block is empty (shouldn't happen with regex)"
        );
        assert!(!code.contains("```"), "extracted code contains backticks");
        return code.to_string();
    }

    // Should not reach here (matches is non-empty)
    unreachable!("matches checked but somehow empty");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_raw_code_no_fences() {
        let input = "let x = 1;";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_raw_code_no_fences_with_hint() {
        let input = "let x = 1;";
        let result = extract_code_from_model_output(input, "rust");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_rust_fenced_block() {
        let input = "```rust\nlet x = 1;\n```";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_generic_fence() {
        let input = "```\nlet x = 1;\n```";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_prefers_lang_hint() {
        let input = "```python\ndef foo():\n    pass\n```\n\n```rust\nfn foo() {}\n```";
        let result = extract_code_from_model_output(input, "rust");
        assert_eq!(result, "fn foo() {}");
    }

    #[test]
    fn test_extract_fallback_to_first_block() {
        let input = "```python\ndef foo():\n    pass\n```\n\n```rust\nfn foo() {}\n```";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(result, "def foo():\n    pass");
    }

    #[test]
    fn test_extract_multiple_blocks_first_wins_no_hint() {
        let input = "```\ncode1\n```\n\n```\ncode2\n```";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(result, "code1");
    }

    #[test]
    fn test_extract_with_surrounding_text() {
        let input = "Here's the code:\n\n```rust\nlet x = 1;\n```\n\nLet me explain...";
        let result = extract_code_from_model_output(input, "rust");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_multiline_code() {
        let input = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(
            result,
            "fn main() {\n    println!(\"hello\");\n}"
        );
    }

    #[test]
    fn test_extract_trims_whitespace() {
        let input = "```rust\n   \n  let x = 1;\n   \n  ```";
        let result = extract_code_from_model_output(input, "");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_case_insensitive_lang_hint() {
        let input = "```RUST\nlet x = 1;\n```";
        let result = extract_code_from_model_output(input, "rust");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_extract_case_insensitive_lang_hint_mismatch() {
        let input = "```python\ndef foo():\n    pass\n```";
        let result = extract_code_from_model_output(input, "RUST");
        // lang_hint is "RUST" (uppered in hint), but only "python" block exists
        // Should fall back to first block
        assert_eq!(result, "def foo():\n    pass");
    }

    #[test]
    #[should_panic(expected = "content must be non-empty")]
    fn test_extract_empty_content_panics() {
        extract_code_from_model_output("", "rust");
    }

    #[test]
    fn test_extract_lang_hint_with_numbers() {
        let input = "```python3\nprint('hello')\n```";
        let result = extract_code_from_model_output(input, "python3");
        assert_eq!(result, "print('hello')");
    }
}
