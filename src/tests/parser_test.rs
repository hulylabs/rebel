// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parser::{Collector, Parser, ParserError, WordKind};

/// Simple collector for parser tests that records all tokens in a single list
/// which makes it easier to verify expectations in tests
#[derive(PartialEq, Debug, Default)]
pub struct SimpleCollector {
    /// Collected tokens as formatted strings
    pub tokens: Vec<String>,
}

impl Collector for SimpleCollector {
    type Error = ();

    fn string(&mut self, string: &str) -> Result<(), Self::Error> {
        self.tokens.push(format!("String: {}", string));
        Ok(())
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Result<(), Self::Error> {
        let kind_str = match kind {
            WordKind::Word => "Word",
            WordKind::SetWord => "SetWord",
            WordKind::GetWord => "GetWord",
        };
        self.tokens.push(format!("{}: {}", kind_str, word));
        Ok(())
    }

    fn integer(&mut self, value: i32) -> Result<(), Self::Error> {
        self.tokens.push(format!("Integer: {}", value));
        Ok(())
    }

    fn begin_block(&mut self) -> Result<(), Self::Error> {
        self.tokens.push("BeginBlock".to_string());
        Ok(())
    }

    fn end_block(&mut self) -> Result<(), Self::Error> {
        self.tokens.push("EndBlock".to_string());
        Ok(())
    }

    fn begin_path(&mut self) -> Result<(), Self::Error> {
        self.tokens.push("BeginPath".to_string());
        Ok(())
    }

    fn end_path(&mut self) -> Result<(), Self::Error> {
        self.tokens.push("EndPath".to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a parser and run the parse operation
    fn parse(input: &str) -> Result<SimpleCollector, ParserError<()>> {
        let mut collector = SimpleCollector::default();
        Parser::parse_str(input, &mut collector)?;
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
            collector.tokens,
            vec![
                "BeginBlock",
                "Word: word1",
                "String: string",
                "Integer: 123",
                "Word: word2",
                "EndBlock"
            ]
        );
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
            collector.tokens,
            vec![
                "BeginBlock",
                "String: Hello\nWorld",
                "String: Tab\tCharacter",
                "String: Quotes: \"quoted\"",
                "String: Backslash: \\",
                "String: Carriage Return: \r",
                "String: Mixed: \t\r\n\"\\",
                "EndBlock"
            ]
        );
    }

    #[test]
    fn test_string_with_escaped_quotes() {
        let input = r#"["This string has \"escaped quotes\""]"#;

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "String: This string has \"escaped quotes\"",
                "EndBlock"
            ]
        );
    }

    #[test]
    fn test_string_with_escaped_newlines() {
        let input = r#"["Line1\nLine2\nLine3"]"#;

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.tokens,
            vec!["BeginBlock", "String: Line1\nLine2\nLine3", "EndBlock"]
        );
    }

    #[test]
    fn test_integers() {
        let input = "[123 -456 0 +789]";

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "Integer: 123",
                "Integer: -456",
                "Integer: 0",
                "Integer: 789",
                "EndBlock"
            ]
        );
    }

    #[test]
    fn test_words() {
        let input = "[word set-word: :get-word]";

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "Word: word",
                "SetWord: set-word",
                "GetWord: get-word",
                "EndBlock"
            ]
        );
    }

    #[test]
    fn test_nested_blocks() {
        let input = "[outer [inner1 [deep]] [inner2]]";

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "Word: outer",
                "BeginBlock",
                "Word: inner1",
                "BeginBlock",
                "Word: deep",
                "EndBlock",
                "EndBlock",
                "BeginBlock",
                "Word: inner2",
                "EndBlock",
                "EndBlock"
            ]
        );
    }

    #[test]
    fn test_paths() {
        let input = "[word/path/item word/item]";

        let collector = parse(input).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "BeginPath",
                "Word: word",
                "Word: path",
                "Word: item",
                "EndPath",
                "BeginPath",
                "Word: word",
                "Word: item",
                "EndPath",
                "EndBlock"
            ]
        );
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

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "Word: word1",
                "Integer: 123",
                "String: string",
                "SetWord: word2",
                "Integer: -456",
                "BeginBlock",
                "Word: nested",
                "EndBlock",
                "GetWord: get-word",
                "String: multi\n            line",
                "EndBlock"
            ]
        );
    }

    #[test]
    fn test_empty_input() {
        let input = "[]";

        let collector = parse(input).unwrap();

        assert_eq!(collector.tokens, vec!["BeginBlock", "EndBlock"]);
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

    // Static parse methods test from parser.rs
    #[test]
    fn test_parse_str_method() {
        // Test the parse_str method with a simple input
        let input = r#"[word 123 "string"]"#;

        let mut collector = SimpleCollector::default();
        Parser::parse_str(input, &mut collector).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "Word: word",
                "Integer: 123",
                "String: string",
                "EndBlock",
            ]
        );
    }

    #[test]
    fn test_parse_block_method() {
        // Test the parse_block method with a simple input (not in brackets)
        let input = r#"word 123 "string""#;

        let mut collector = SimpleCollector::default();
        Parser::parse_block(input, &mut collector).unwrap();

        assert_eq!(
            collector.tokens,
            vec![
                "BeginBlock",
                "Word: word",
                "Integer: 123",
                "String: string",
                "EndBlock",
            ]
        );
    }
}
