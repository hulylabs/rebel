// RebelDB™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parser::Parser;
use crate::value::{Value, ValueCollector};

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to parse input with ValueCollector and return the result
    fn parse_to_value(input: &str) -> Option<Value> {
        let mut collector = ValueCollector::new();
        if Parser::parse(input, &mut collector).is_ok() {
            collector.value()
        } else {
            None
        }
    }

    // Helper function to parse input as a block with ValueCollector
    fn parse_block_to_value(input: &str) -> Option<Value> {
        let mut collector = ValueCollector::new();
        if Parser::parse_block(input, &mut collector).is_ok() {
            collector.value()
        } else {
            None
        }
    }

    #[test]
    fn test_collector_with_basic_values() {
        // Test with a string
        let value = parse_to_value(r#"["Hello world"]"#).unwrap();
        assert_eq!(value, Value::block([Value::string("Hello world")]));

        // Test with an integer
        let value = parse_to_value("[42]").unwrap();
        assert_eq!(value, Value::block([Value::int(42)]));

        // Test with a word
        let value = parse_to_value("[hello]").unwrap();
        assert_eq!(value, Value::block([Value::word("hello")]));

        // Test with different word types
        let value = parse_to_value("[word set-word: :get-word]").unwrap();
        assert_eq!(
            value,
            Value::block([
                Value::word("word"),
                Value::set_word("set-word"),
                Value::get_word("get-word"),
            ])
        );
    }

    #[test]
    fn test_collector_with_nested_blocks() {
        let value = parse_to_value("[[nested [deeply [nested]]]]").unwrap();

        // Using direct value comparison for the entire nested structure
        assert_eq!(
            value,
            Value::block([Value::block([
                Value::word("nested"),
                Value::block([Value::word("deeply"), Value::block([Value::word("nested")])])
            ])])
        );
    }

    #[test]
    fn test_collector_with_mixed_values() {
        let input = r#"[
            42 
            "string" 
            word 
            [nested block] 
            true
            none
        ]"#;

        let value = parse_to_value(input).unwrap();
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
    }

    #[test]
    fn test_collector_with_paths() {
        let value = parse_to_value("[object/property/value]").unwrap();

        assert_eq!(
            value,
            Value::block([Value::path([
                Value::word("object"),
                Value::word("property"),
                Value::word("value")
            ])])
        );
    }

    #[test]
    fn test_parse_block_method() {
        // Test the parse_block method which automatically wraps input in a block
        let value = parse_block_to_value(r#"word 123 "string""#).unwrap();

        assert_eq!(
            value,
            Value::block([
                Value::word("word"),
                Value::int(123),
                Value::string("string")
            ])
        );
    }

    #[test]
    fn test_empty_block() {
        let value = parse_to_value("[]").unwrap();
        assert_eq!(value, Value::block([]));
    }

    #[test]
    fn test_multiple_nested_paths() {
        let value = parse_to_value("[system/console/write system/console/read-line]").unwrap();

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
    }

    #[test]
    fn test_form_method() {
        // Test that form produces the expected string representation
        let value = parse_to_value(r#"[1 "hello" word]"#).unwrap();
        assert_eq!(value.form(), "1 hello word");

        // Test with nested structure
        let value = parse_to_value(r#"[nested [values "here"]]"#).unwrap();
        assert_eq!(value.form(), "nested values here");
    }

    #[test]
    fn test_error_handling() {
        // Test with unclosed string
        assert!(parse_to_value(r#"["unclosed]"#).is_none());

        // Test with bad escape sequence
        assert!(parse_to_value(r#"["bad \z escape"]"#).is_none());

        // Test with integer overflow
        assert!(parse_to_value("[99999999999]").is_none());
    }
}
