// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

//! Parser for REBOL-inspired language
//!
//! This module provides a parser for a REBOL-inspired language, which is a flexible,
//! lightweight, and dynamic language with minimal syntax.
//!
//! The parser handles:
//! - Strings with escape sequences
//! - Different word types (regular words, set-words, get-words)
//! - Integer literals
//! - Block structures (nested blocks)
//! - Path notation
//! - Comments (semicolon style)

use std::str::CharIndices;
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParserError<E> {
    /// Input ended unexpectedly
    #[error("end of input")]
    EndOfInput,
    /// An unexpected character was encountered
    #[error("unexpected character: `{0}`")]
    UnexpectedChar(char),
    /// Integer value exceeds the range of i32
    #[error("integer overflow")]
    IntegerOverflow,
    /// An unexpected error occurred
    #[error("unexpected error")]
    UnexpectedError,
    /// Attempted to parse an empty word
    #[error("empty word")]
    EmptyWord,
    /// Error propagated from the collector
    #[error(transparent)]
    CollectorError(#[from] E),
}

/// Types of word tokens
#[derive(Debug, PartialEq)]
pub enum WordKind {
    /// Regular word (e.g., `word`)
    Word,
    /// Set-word with trailing colon (e.g., `word:`)
    SetWord,
    /// Get-word with leading colon (e.g., `:word`)
    GetWord,
}

/// Interface for collecting parsed tokens
///
/// Implementors receive callbacks for each parsed token
/// and can build their own representation of the parsed input.
pub trait Collector {
    /// Error type returned by the collector
    type Error;

    /// Called when a string is parsed
    fn string(&mut self, string: &str) -> Result<(), Self::Error>;

    /// Called when a word is parsed
    fn word(&mut self, kind: WordKind, word: &str) -> Result<(), Self::Error>;

    /// Called when an integer is parsed
    fn integer(&mut self, value: i32) -> Result<(), Self::Error>;

    /// Called at the start of a block
    fn begin_block(&mut self) -> Result<(), Self::Error>;

    /// Called at the end of a block
    fn end_block(&mut self) -> Result<(), Self::Error>;

    /// Called at the start of a path
    fn begin_path(&mut self) -> Result<(), Self::Error>;

    /// Called at the end of a path
    fn end_path(&mut self) -> Result<(), Self::Error>;
}

/// Parser for REBOL-inspired language tokens
pub struct Parser<'a, C>
where
    C: Collector,
{
    input: &'a str,
    cursor: CharIndices<'a>,
    collector: &'a mut C,
    in_path: bool,
}

impl<'a, C> Parser<'a, C>
where
    C: Collector,
{
    /// Creates a new parser
    ///
    /// # Parameters
    ///
    /// * `input` - The input string to parse
    /// * `collector` - The collector that will receive parsed tokens
    ///
    /// # Example
    ///
    /// ```
    /// # use rebel::parser::{Collector, WordKind, Parser};
    /// # struct MyCollector;
    /// # impl Collector for MyCollector {
    /// #     type Error = ();
    /// #     fn string(&mut self, _: &str) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn integer(&mut self, _: i32) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn begin_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn end_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn begin_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn end_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// let input = "[word 123 \"string\"]";
    /// let mut parser = Parser::new(input, &mut collector);
    /// ```
    pub fn new(input: &'a str, collector: &'a mut C) -> Self {
        Self {
            input,
            collector,
            cursor: input.char_indices(),
            in_path: false,
        }
    }

    /// Parse a block from the input
    ///
    /// This method will parse the input until the end, treating it as a block.
    /// It calls the collector's `begin_block` and `end_block` methods, along
    /// with methods for each parsed token.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if parsing succeeded
    /// * `Err(ParserError)` if parsing failed
    ///
    /// # Example
    ///
    /// ```
    /// # use rebel::parser::{Collector, WordKind, Parser};
    /// # struct MyCollector;
    /// # impl Collector for MyCollector {
    /// #     type Error = ();
    /// #     fn string(&mut self, _: &str) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn integer(&mut self, _: i32) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn begin_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn end_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn begin_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn end_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// let input = "[word 123 \"string\"]";
    /// let mut parser = Parser::new(input, &mut collector);
    /// parser.parse_block().expect("Failed to parse block");
    /// ```
    pub fn parse_block(&mut self) -> Result<(), ParserError<C::Error>> {
        self.collector
            .begin_block()
            .map_err(ParserError::CollectorError)?;
        self.parse_tokens()?;
        self.collector
            .end_block()
            .map_err(ParserError::CollectorError)
    }

    /// Parse input directly with a collector
    ///
    /// This static method provides a convenient way to parse input without
    /// needing to create a parser instance manually. Unlike `parse_block`, this does not
    /// automatically wrap the input in a block - it parses the input exactly as provided.
    ///
    /// # Parameters
    ///
    /// * `input` - The input string to parse
    /// * `collector` - The collector that will receive parsed tokens
    ///
    /// # Returns
    ///
    /// * `Ok(())` if parsing succeeded
    /// * `Err(ParserError)` if parsing failed
    ///
    /// # Example
    ///
    /// ```
    /// # use rebel::parser::{Collector, WordKind, Parser};
    /// # struct MyCollector;
    /// # impl Collector for MyCollector {
    /// #     type Error = ();
    /// #     fn string(&mut self, _: &str) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn integer(&mut self, _: i32) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn begin_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn end_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn begin_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn end_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// let input = "[word 123 \"string\"]";
    /// Parser::parse(input, &mut collector).expect("Failed to parse");
    /// ```
    pub fn parse(input: &'a str, collector: &'a mut C) -> Result<(), ParserError<C::Error>> {
        let mut parser = Self::new(input, collector);
        parser.parse_tokens()
    }

    fn skip_whitespace(&mut self) -> Option<(usize, char)> {
        while let Some((pos, char)) = self.cursor.next() {
            if char.is_ascii_whitespace() {
                continue;
            } else if char == ';' {
                // Skip comment until newline
                for (_, c) in self.cursor.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            } else {
                return Some((pos, char));
            }
        }
        None
    }

    fn parse_string(&mut self, pos: usize) -> Result<Option<char>, ParserError<C::Error>> {
        let _start_pos = pos + 1; // Skip the opening quote
        let mut result = String::new();
        let mut escaped = false;

        while let Some((_, char)) = self.cursor.next() {
            if escaped {
                // Handle escape sequences
                let escaped_char = match char {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    _ => return Err(ParserError::UnexpectedChar(char)),
                };
                result.push(escaped_char);
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                // End of string
                return self
                    .collector
                    .string(&result)
                    .map(|()| None)
                    .map_err(ParserError::CollectorError);
            } else {
                result.push(char);
            }
        }

        // If we get here, we never found the closing quote
        Err(ParserError::EndOfInput)
    }

    fn collect_word(
        &mut self,
        symbol: &str,
        kind: WordKind,
        consumed: Option<char>,
    ) -> Result<Option<char>, C::Error> {
        if let Some('/') = consumed {
            if self.in_path == false {
                self.in_path = true;
                self.collector.begin_path()?;
            }
        }
        self.collector.word(kind, symbol).map(|_| consumed)
    }

    fn parse_word(&mut self, start_pos: usize) -> Result<Option<char>, ParserError<C::Error>> {
        // Determine word type and content based on initial character and subsequent parsing
        if self.input.as_bytes().get(start_pos) == Some(&b':') {
            // This is a get-word starting with ':'
            self.cursor.next(); // Skip the colon

            // Parse the word part (after the colon)
            let word_start = start_pos + 1;
            let mut end_pos = word_start;

            let terminator = loop {
                match self.cursor.next() {
                    Some((pos, c)) => {
                        if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '?' {
                            end_pos = pos + 1;
                        } else if c == ']' || c == '/' || c.is_ascii_whitespace() {
                            end_pos = pos;
                            break Some(c);
                        } else {
                            return Err(ParserError::UnexpectedChar(c));
                        }
                    }
                    None => {
                        break None;
                    }
                }
            };

            if end_pos <= word_start {
                return Err(ParserError::EmptyWord);
            }

            let word = self
                .input
                .get(word_start..end_pos)
                .ok_or(ParserError::UnexpectedError)?;
            self.collector
                .word(WordKind::GetWord, word)
                .map_err(ParserError::CollectorError)?;

            Ok(terminator)
        } else {
            // Regular word or set-word
            let mut end_pos = start_pos;
            let mut is_set_word = false;

            let terminator = loop {
                match self.cursor.next() {
                    Some((pos, c)) => {
                        if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '?' {
                            end_pos = pos + 1;
                        } else if c == ':' {
                            is_set_word = true;
                            end_pos = pos;
                            break None; // Colon consumed, no terminator to return
                        } else if c == ']' || c == '/' || c.is_ascii_whitespace() {
                            end_pos = pos;
                            break Some(c);
                        } else {
                            return Err(ParserError::UnexpectedChar(c));
                        }
                    }
                    None => {
                        break None;
                    }
                }
            };

            if end_pos <= start_pos {
                return Err(ParserError::EmptyWord);
            }

            let word = self
                .input
                .get(start_pos..end_pos)
                .ok_or(ParserError::UnexpectedError)?;
            let kind = if is_set_word {
                WordKind::SetWord
            } else {
                WordKind::Word
            };

            self.collector
                .word(kind, word)
                .map_err(ParserError::CollectorError)?;

            Ok(terminator)
        }
    }

    fn parse_number(&mut self, char: char) -> Result<Option<char>, ParserError<C::Error>> {
        let mut value: i32 = 0;
        let mut is_negative = false;
        let mut has_digits = false;
        let mut consumed = None;

        match char {
            '+' => {}
            '-' => {
                is_negative = true;
            }
            c if c.is_ascii_digit() => {
                value = c.to_digit(10).ok_or(ParserError::UnexpectedError)? as i32;
                has_digits = true;
            }
            _ => return Err(ParserError::UnexpectedChar(char)),
        }

        for (_, char) in self.cursor.by_ref() {
            match char {
                c if c.is_ascii_digit() => {
                    has_digits = true;
                    let digit = c.to_digit(10).ok_or(ParserError::UnexpectedError)? as i32;
                    value = value
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(digit))
                        .ok_or(ParserError::IntegerOverflow)?;
                }
                ']' => {
                    consumed = Some(char);
                    break;
                }
                _ => break,
            }
        }
        if !has_digits {
            return Err(ParserError::EndOfInput);
        }
        if is_negative {
            value = value.checked_neg().ok_or(ParserError::IntegerOverflow)?;
        }
        self.collector
            .integer(value)
            .map(|_| consumed)
            .map_err(ParserError::CollectorError)
    }

    fn process_block_end(&mut self, consumed: Option<char>) -> Result<(), C::Error> {
        match consumed {
            Some('/') => {}
            _ => {
                if self.in_path {
                    self.in_path = false;
                    self.collector.end_path()?;
                }
            }
        }
        if let Some(']') = consumed {
            self.collector.end_block()?;
        }
        Ok(())
    }

    fn parse_tokens(&mut self) -> Result<(), ParserError<C::Error>> {
        while let Some((pos, char)) = self.skip_whitespace() {
            let consumed = match char {
                '[' => self
                    .collector
                    .begin_block()
                    .map(|()| None)
                    .map_err(ParserError::CollectorError)?,
                ']' => Some(char),
                '"' => self.parse_string(pos)?,
                ':' => self.parse_word(pos)?, // Special handling for get-words
                c if c.is_ascii_alphabetic() => self.parse_word(pos)?,
                c if c.is_ascii_digit() || c == '+' || c == '-' => self.parse_number(c)?,
                _ => return Err(ParserError::UnexpectedChar(char)),
            };
            self.process_block_end(consumed)
                .map_err(ParserError::CollectorError)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Collector, Parser, WordKind};

    #[derive(PartialEq, Debug, Default)]
    struct SimpleCollector {
        tokens: Vec<String>,
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

    #[test]
    fn test_static_parse_method() {
        // Test the static parse method with a simple input
        let input = r#"[word 123 "string"]"#;

        let mut collector = SimpleCollector::default();
        Parser::parse(input, &mut collector).unwrap();

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
