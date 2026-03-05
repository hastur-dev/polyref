/// Produces bigram shingle strings from `text`.
/// Algorithm:
///   1. Lowercase the input.
///   2. Split on runs of non-alphanumeric characters -> word tokens.
///   3. Filter tokens shorter than 2 characters.
///   4. Emit overlapping pairs: "word1 word2" for each adjacent pair.
pub fn bigrams(text: &str) -> Vec<String> {
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .collect();

    if words.len() < 2 {
        return Vec::new();
    }

    words
        .windows(2)
        .map(|pair| format!("{} {}", pair[0], pair[1]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigrams_basic() {
        let result = bigrams("hello world foo");
        assert_eq!(result, vec!["hello world", "world foo"]);
    }

    #[test]
    fn test_bigrams_single_word() {
        assert!(bigrams("hello").is_empty());
    }

    #[test]
    fn test_bigrams_empty() {
        assert!(bigrams("").is_empty());
    }

    #[test]
    fn test_bigrams_punctuation_stripped() {
        let result = bigrams("foo, bar. baz");
        assert_eq!(result, vec!["foo bar", "bar baz"]);
    }

    #[test]
    fn test_bigrams_case_normalized() {
        let result = bigrams("Hello World");
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn test_bigrams_short_tokens_filtered() {
        let result = bigrams("a b foo bar");
        assert_eq!(result, vec!["foo bar"]);
    }
}
