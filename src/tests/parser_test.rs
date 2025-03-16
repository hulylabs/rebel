// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parser::{Collector, Parser, ParserError, WordKind};

/// Test collector implementation for parser tests
#[derive(PartialEq, Debug, Default)]
pub struct TestCollector {
    pub strings: Vec<String>,
    pub words: Vec<(WordKind, String)>,
    pub integers: Vec<i32>,
    pub block_depth: i32,
    pub path_depth: i32,
}

impl Collector for TestCollector {
    type Error = ();

    fn string(&mut self, string: &str) -> Result<(), Self::Error> {
        self.strings.push(string.to_string());
        Ok(())
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Result<(), Self::Error> {
        self.words.push((kind, word.to_string()));
        Ok(())
    }

    fn integer(&mut self, value: i32) -> Result<(), Self::Error> {
        self.integers.push(value);
        Ok(())
    }

    fn begin_block(&mut self) -> Result<(), Self::Error> {
        self.block_depth += 1;
        Ok(())
    }

    fn end_block(&mut self) -> Result<(), Self::Error> {
        self.block_depth -= 1;
        Ok(())
    }

    fn begin_path(&mut self) -> Result<(), Self::Error> {
        self.path_depth += 1;
        Ok(())
    }

    fn end_path(&mut self) -> Result<(), Self::Error> {
        self.path_depth -= 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a parser and run the parse operation
    fn parse(input: &str) -> Result<TestCollector, ParserError<()>> {
        let mut collector = TestCollector::default();
        Parser::parse(input, &mut collector)?;
        Ok(collector)
    }

    #[test]
    fn test_comments_are_ignored() {
        let input = r#"[
                ; this is a comment
                word1 ; this is a comment
                "string" ; another comment
                123 ; numeric comment
                ; full line comment
                word2
            ]"#;

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.words,
            vec![
                (WordKind::Word, "word1".to_string()),
                (WordKind::Word, "word2".to_string()),
            ]
        );
        assert_eq!(collector.strings, vec!["string"]);
        assert_eq!(collector.integers, vec![123]);
        assert_eq!(collector.block_depth, 0); // Should be balanced
    }

    #[test]
    fn test_escaped_characters_in_strings() {
        let input = r#"[
            "Hello\nWorld"
            "Tab\tCharacter"
            "Quotes: \"quoted\""
            "Backslash: \\"
            "Carriage Return: \r"
            "Mixed: \t\r\n\"\\"
        ]"#;

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.strings,
            vec![
                "Hello\nWorld",
                "Tab\tCharacter",
                "Quotes: \"quoted\"",
                "Backslash: \\",
                "Carriage Return: \r",
                "Mixed: \t\r\n\"\\"
            ]
        );
    }

    #[test]
    fn test_string_with_escaped_quotes() {
        let input = r#"["This string has \"escaped quotes\""]"#;

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.strings,
            vec!["This string has \"escaped quotes\""]
        );
    }

    #[test]
    fn test_string_with_escaped_newlines() {
        let input = r#"["Line1\nLine2\nLine3"]"#;

        let collector = parse(input).unwrap();

        assert_eq!(collector.strings, vec!["Line1\nLine2\nLine3"]);
    }

    #[test]
    fn test_integers() {
        let input = "[123 -456 0 +789]";

        let collector = parse(input).unwrap();

        assert_eq!(collector.integers, vec![123, -456, 0, 789]);
    }

    #[test]
    fn test_words() {
        let input = "[word set-word: :get-word]";

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.words,
            vec![
                (WordKind::Word, "word".to_string()),
                (WordKind::SetWord, "set-word".to_string()),
                (WordKind::GetWord, "get-word".to_string()),
            ]
        );
    }

    #[test]
    fn test_nested_blocks() {
        let input = "[outer [inner1 [deep]] [inner2]]";

        let collector = parse(input).unwrap();

        // We won't be able to verify the nesting structure directly with our simple collector,
        // but we can verify the words were collected
        assert_eq!(
            collector.words,
            vec![
                (WordKind::Word, "outer".to_string()),
                (WordKind::Word, "inner1".to_string()),
                (WordKind::Word, "deep".to_string()),
                (WordKind::Word, "inner2".to_string()),
            ]
        );

        // Block depth should be balanced at the end
        assert_eq!(collector.block_depth, 0);
    }

    #[test]
    fn test_paths() {
        let input = "[word/path/item word/item]";

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.words,
            vec![
                (WordKind::Word, "word".to_string()),
                (WordKind::Word, "path".to_string()),
                (WordKind::Word, "item".to_string()),
                (WordKind::Word, "word".to_string()),
                (WordKind::Word, "item".to_string()),
            ]
        );

        // Path depth should be balanced at the end
        assert_eq!(collector.path_depth, 0);
    }

    #[test]
    fn test_mixed_tokens() {
        let input = r#"[
            word1 123 "string" 
            word2: -456 [nested] 
            :get-word "multi
            line"
        ]"#;

        let collector = parse(input).unwrap();

        assert_eq!(collector.integers, vec![123, -456]);
        assert_eq!(
            collector.words,
            vec![
                (WordKind::Word, "word1".to_string()),
                (WordKind::SetWord, "word2".to_string()),
                (WordKind::Word, "nested".to_string()),
                (WordKind::GetWord, "get-word".to_string()),
            ]
        );
        // When parsing multi-line strings, the parser preserves indentation
        assert_eq!(collector.strings, vec!["string", "multi\n            line"]);
    }

    #[test]
    fn test_empty_input() {
        let input = "[]";

        let collector = parse(input).unwrap();

        assert_eq!(collector.words.len(), 0);
        assert_eq!(collector.integers.len(), 0);
        assert_eq!(collector.strings.len(), 0);
    }

    #[test]
    fn test_error_conditions() {
        // Invalid escape sequence
        assert!(parse(r#"["invalid \z escape"]"#).is_err());

        // Unclosed string
        assert!(parse(r#"["unclosed string]"#).is_err());

        // Empty word (error)
        assert!(parse("[:]").is_err());

        // Integer overflow (if we try to parse a number larger than i32::MAX)
        assert!(parse("[99999999999]").is_err());
    }
}
