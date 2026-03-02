use crate::detect::Language;

/// Find the closest fuzzy match for a name among candidates
pub fn fuzzy_match(name: &str, candidates: &[&str], threshold: f64) -> Option<(String, f64)> {
    let mut best: Option<(String, f64)> = None;

    for candidate in candidates {
        let score = similarity(name, candidate);
        if score >= threshold
            && best.as_ref().is_none_or(|(_, best_score)| score > *best_score) {
                best = Some((candidate.to_string(), score));
            }
    }

    best
}

/// Compute string similarity using normalized Levenshtein distance (0.0 to 1.0)
fn similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let max_len = a.len().max(b.len()) as f64;
    let dist = levenshtein_distance(a, b) as f64;
    1.0 - (dist / max_len)
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Return a "did you mean '...'?" suggestion
pub fn suggest_correction(wrong_name: &str, known_names: &[String]) -> Option<String> {
    let candidates: Vec<&str> = known_names.iter().map(|s| s.as_str()).collect();
    let result = fuzzy_match(wrong_name, &candidates, 0.5)?;
    Some(format!("did you mean '{}'?", result.0))
}

/// Extract imported names from an import statement based on language
pub fn extract_import_names(line: &str, language: Language) -> Vec<String> {
    let trimmed = line.trim();
    match language {
        Language::Rust => extract_rust_imports(trimmed),
        Language::Python => extract_python_imports(trimmed),
        Language::TypeScript => extract_ts_imports(trimmed),
    }
}

fn extract_rust_imports(line: &str) -> Vec<String> {
    if !line.starts_with("use ") {
        return vec![];
    }
    if let Some(brace_start) = line.find('{') {
        if let Some(brace_end) = line.find('}') {
            return line[brace_start + 1..brace_end]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    // use crate::Item; — single import
    if let Some(last_colon) = line.rfind("::") {
        let name = line[last_colon + 2..].trim_end_matches(';').trim().to_string();
        if !name.is_empty() {
            return vec![name];
        }
    }
    vec![]
}

fn extract_python_imports(line: &str) -> Vec<String> {
    if line.starts_with("from ") {
        if let Some(import_idx) = line.find(" import ") {
            return line[import_idx + 8..]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    if let Some(rest) = line.strip_prefix("import ") {
        return rest
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    vec![]
}

fn extract_ts_imports(line: &str) -> Vec<String> {
    if !line.starts_with("import ") {
        return vec![];
    }
    if let Some(brace_start) = line.find('{') {
        if let Some(brace_end) = line.find('}') {
            return line[brace_start + 1..brace_end]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    vec![]
}

/// Count arguments in a function call expression, handling nested parens and strings
pub fn count_arguments(call_expr: &str) -> usize {
    // Find the opening paren
    let paren_start = match call_expr.find('(') {
        Some(idx) => idx,
        None => return 0,
    };

    let inner = &call_expr[paren_start + 1..];

    // Find matching close paren
    let close_idx = match find_matching_close(inner) {
        Some(idx) => idx,
        None => inner.len(),
    };

    let args_str = inner[..close_idx].trim();
    if args_str.is_empty() {
        return 0;
    }

    // Count commas at depth 0, respecting strings
    let mut count = 1;
    let mut depth = 0;
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

    count
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

/// Check if a position in a line is inside a string literal
pub fn is_inside_string(line: &str, position: usize) -> bool {
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev = '\0';

    for (i, c) in line.chars().enumerate() {
        if i >= position {
            return in_string;
        }
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
                _ => {}
            }
        }
        prev = c;
    }

    in_string
}

/// Check if a position in a line is inside a comment
pub fn is_inside_comment(line: &str, position: usize, language: Language) -> bool {
    let before = &line[..position.min(line.len())];

    match language {
        Language::Rust | Language::TypeScript => {
            // Check for // comment
            if let Some(slash_idx) = before.find("//") {
                // Make sure // is not inside a string
                if !is_inside_string(before, slash_idx) {
                    return true;
                }
            }
            false
        }
        Language::Python => {
            // Check for # comment
            if let Some(hash_idx) = before.find('#') {
                if !is_inside_string(before, hash_idx) {
                    return true;
                }
            }
            false
        }
    }
}
