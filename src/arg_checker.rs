use crate::generate::ReferenceEntry;

/// Issue for argument count mismatch
#[derive(Debug, Clone, PartialEq)]
pub enum ArgIssue {
    TooFewArgs {
        name: String,
        expected_min: usize,
        got: usize,
        line_number: usize,
    },
    TooManyArgs {
        name: String,
        expected_max: usize,
        got: usize,
        line_number: usize,
    },
}

/// Count the number of arguments in a call expression at top-level depth.
///
/// Returns `None` if parentheses are unbalanced (multiline call).
pub fn count_call_args(call_expr: &str) -> Option<usize> {
    let paren_start = call_expr.find('(')?;
    let rest = &call_expr[paren_start + 1..];

    let close_idx = find_matching_close(rest)?;
    let args_str = rest[..close_idx].trim();

    if args_str.is_empty() {
        return Some(0);
    }

    let mut count = 1usize;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev = '\0';

    for c in args_str.chars() {
        if in_string {
            if c == string_char && prev != '\\' {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = c;
                }
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                ',' if depth == 0 => count += 1,
                _ => {}
            }
        }
        prev = c;
    }

    debug_assert!(count >= 1, "count must be >= 1 when args_str is non-empty");
    Some(count)
}

fn find_matching_close(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev = '\0';

    for (i, c) in s.chars().enumerate() {
        if in_string {
            if c == string_char && prev != '\\' {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = c;
                }
                '(' | '[' | '{' => depth += 1,
                ')' => {
                    if depth == 0 {
                        return Some(i);
                    }
                    depth -= 1;
                }
                ']' | '}' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }
        prev = c;
    }

    None
}

/// Check if the argument count at a call site matches the reference entry's metadata.
pub fn check_arg_count(
    call_expr: &str,
    entry: &ReferenceEntry,
    line: usize,
) -> Option<ArgIssue> {
    // Only check if we have arg count metadata
    if entry.min_args.is_none() && entry.max_args.is_none() {
        return None;
    }

    let got = count_call_args(call_expr)?;

    if let Some(min) = entry.min_args {
        if got < min {
            return Some(ArgIssue::TooFewArgs {
                name: entry.name.clone(),
                expected_min: min,
                got,
                line_number: line,
            });
        }
    }

    if let Some(max) = entry.max_args {
        if got > max {
            return Some(ArgIssue::TooManyArgs {
                name: entry.name.clone(),
                expected_max: max,
                got,
                line_number: line,
            });
        }
    }

    None
}

/// Format an argument count issue as a human-readable string.
pub fn format_arg_issue(issue: &ArgIssue) -> String {
    let result = match issue {
        ArgIssue::TooFewArgs {
            name,
            expected_min,
            got,
            ..
        } => format!(
            "'{}' expects at least {} arg(s), got {} — add the missing argument(s)",
            name, expected_min, got
        ),
        ArgIssue::TooManyArgs {
            name,
            expected_max,
            got,
            ..
        } => format!(
            "'{}' expects at most {} arg(s), got {} — remove the extra argument(s)",
            name, expected_max, got
        ),
    };

    debug_assert!(!result.is_empty(), "result must be non-empty");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{EntryKind, ReferenceEntry};

    #[test]
    fn test_count_no_args() {
        assert_eq!(count_call_args("handle.abort()"), Some(0));
    }

    #[test]
    fn test_count_one_arg() {
        assert_eq!(count_call_args("rt.spawn(async { 42 })"), Some(1));
    }

    #[test]
    fn test_count_two_args() {
        assert_eq!(count_call_args("HashMap::with_capacity(10, 20)"), Some(2));
    }

    #[test]
    fn test_count_nested_calls_not_overcounted() {
        assert_eq!(count_call_args("foo(bar(1, 2), baz(3))"), Some(2));
    }

    #[test]
    fn test_count_unbalanced_parens_returns_none() {
        assert_eq!(count_call_args("foo(bar(1,"), None);
    }

    #[test]
    fn test_check_too_many_args_emits_issue() {
        let entry = ReferenceEntry {
            name: "abort".to_string(),
            kind: EntryKind::Method,
            min_args: Some(0),
            max_args: Some(0),
            ..Default::default()
        };
        let result = check_arg_count("handle.abort(true)", &entry, 8);
        assert!(matches!(result, Some(ArgIssue::TooManyArgs { .. })));
    }

    #[test]
    fn test_check_too_few_args_emits_issue() {
        let entry = ReferenceEntry {
            name: "spawn".to_string(),
            kind: EntryKind::Method,
            min_args: Some(1),
            max_args: Some(1),
            ..Default::default()
        };
        let result = check_arg_count("rt.spawn()", &entry, 10);
        assert!(matches!(result, Some(ArgIssue::TooFewArgs { .. })));
    }

    #[test]
    fn test_check_correct_args_no_issue() {
        let entry = ReferenceEntry {
            name: "new".to_string(),
            kind: EntryKind::AssociatedFn,
            min_args: Some(0),
            max_args: Some(0),
            ..Default::default()
        };
        let result = check_arg_count("Runtime::new()", &entry, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_unknown_metadata_no_issue() {
        let entry = ReferenceEntry {
            name: "something".to_string(),
            kind: EntryKind::Function,
            min_args: None,
            max_args: None,
            ..Default::default()
        };
        let result = check_arg_count("something(1, 2, 3)", &entry, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_too_many_args() {
        let issue = ArgIssue::TooManyArgs {
            name: "abort".to_string(),
            expected_max: 0,
            got: 1,
            line_number: 5,
        };
        let output = format_arg_issue(&issue);
        assert!(output.contains("abort"));
        assert!(output.contains("remove"));
    }

    #[test]
    fn test_format_too_few_args() {
        let issue = ArgIssue::TooFewArgs {
            name: "spawn".to_string(),
            expected_min: 1,
            got: 0,
            line_number: 10,
        };
        let output = format_arg_issue(&issue);
        assert!(output.contains("spawn"));
        assert!(output.contains("expects"));
    }
}
