// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Parser, ParserError};
use crate::value::{Value, ValueCollector};

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to parse input with ValueCollector and return the result
    fn parse_to_value(input: &str) -> Result<Value, ParserError> {
        let mut collector = ValueCollector::new();
        Parser::parse(input, &mut collector)?;
        collector.value().ok_or(ParserError::UnexpectedError)
    }

    // Helper function to parse input as a block with ValueCollector
    fn parse_block_to_value(input: &str) -> Result<Value, ParserError> {
        let mut collector = ValueCollector::new();
        Parser::parse_block(input, &mut collector)?;
        collector.value().ok_or(ParserError::UnexpectedError)
    }

    #[test]
    fn test_collector_with_basic_values() -> Result<(), ParserError> {
        // Test with a string
        let value = parse_to_value(r#"["Hello world"]"#)?;
        assert_eq!(value, Value::block([Value::string("Hello world")]));

        // Test with an integer
        let value = parse_to_value("[42]")?;
        assert_eq!(value, Value::block([Value::int(42)]));

        // Test with a word
        let value = parse_to_value("[hello]")?;
        assert_eq!(value, Value::block([Value::word("hello")]));

        // Test with different word types
        let value = parse_to_value("[word set-word: :get-word]")?;
        assert_eq!(
            value,
            Value::block([
                Value::word("word"),
                Value::set_word("set-word"),
                Value::get_word("get-word"),
            ])
        );

        Ok(())
    }

    #[test]
    fn test_collector_with_nested_blocks() -> Result<(), ParserError> {
        let value = parse_to_value("[[nested [deeply [nested]]]]")?;

        // Using direct value comparison for the entire nested structure
        assert_eq!(
            value,
            Value::block([Value::block([
                Value::word("nested"),
                Value::block([Value::word("deeply"), Value::block([Value::word("nested")])])
            ])])
        );

        Ok(())
    }

    #[test]
    fn test_collector_with_mixed_values() -> Result<(), ParserError> {
        let input = r#"[
            42 
            "string" 
            word 
            [nested block] 
            true
            none
        ]"#;

        let value = parse_to_value(input)?;
        assert_eq!(
            value,
            Value::block([
                Value::int(42),
                Value::string("string"),
                Value::word("word"),
                Value::block([Value::word("nested"), Value::word("block")]),
                Value::word("true"),
                Value::word("none"),
            ])
        );

        Ok(())
    }

    #[test]
    fn test_collector_with_paths() -> Result<(), ParserError> {
        let value = parse_to_value("[object/property/value]")?;

        assert_eq!(
            value,
            Value::block([Value::path([
                Value::word("object"),
                Value::word("property"),
                Value::word("value")
            ])])
        );

        Ok(())
    }

    #[test]
    fn test_parse_block_method() -> Result<(), ParserError> {
        // Test the parse_block method which automatically wraps input in a block
        let value = parse_block_to_value(r#"word 123 "string""#)?;

        assert_eq!(
            value,
            Value::block([
                Value::word("word"),
                Value::int(123),
                Value::string("string")
            ])
        );

        Ok(())
    }

    #[test]
    fn test_empty_block() -> Result<(), ParserError> {
        let value = parse_to_value("[]")?;
        assert_eq!(value, Value::block([]));

        Ok(())
    }

    #[test]
    fn test_multiple_nested_paths() -> Result<(), ParserError> {
        let value = parse_to_value("[system/console/write system/console/read-line]")?;

        assert_eq!(
            value,
            Value::block([
                Value::path([
                    Value::word("system"),
                    Value::word("console"),
                    Value::word("write")
                ]),
                Value::path([
                    Value::word("system"),
                    Value::word("console"),
                    Value::word("read-line")
                ])
            ])
        );

        Ok(())
    }

    #[test]
    fn test_form_method() -> Result<(), ParserError> {
        // Test that form produces the expected string representation
        let value = parse_to_value(r#"[1 "hello" word]"#)?;
        assert_eq!(value.form(), "1 hello word");

        // Test with nested structure
        let value = parse_to_value(r#"[nested [values "here"]]"#)?;
        assert_eq!(value.form(), "nested values here");

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        // Test with unclosed string
        match parse_to_value(r#"["unclosed]"#) {
            Err(ParserError::EndOfInput) => (), // Expected error
            Err(e) => panic!("Expected EndOfInput error, got: {:?}", e),
            Ok(_) => panic!("Expected error, got success"),
        }

        // Test with bad escape sequence
        match parse_to_value(r#"["bad \z escape"]"#) {
            Err(ParserError::UnexpectedChar('z')) => (), // Expected error
            Err(e) => panic!("Expected UnexpectedChar error, got: {:?}", e),
            Ok(_) => panic!("Expected error, got success"),
        }

        // Test with integer overflow
        match parse_to_value("[99999999999]") {
            Err(ParserError::IntegerOverflow) => (), // Expected error
            Err(e) => panic!("Expected IntegerOverflow error, got: {:?}", e),
            Ok(_) => panic!("Expected error, got success"),
        }
    }
}
