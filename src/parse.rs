// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

//! Parser for REBOL-inspired language
//!
//! This module provides a parser for a REBOL-inspired language, which is a flexible,
//! lightweight, and dynamic language with minimal syntax.
//!
//! The parser offers two primary ways to process input:
//! - `parse`: Parses the input exactly as provided
//! - `parse_block`: Automatically wraps the input in a block
//!
//! The parser handles the following REBOL-inspired syntax elements:
//! - Strings with escape sequences (e.g., `"Hello\nWorld"`)
//! - Different word types:
//!   - Regular words (e.g., `word`)
//!   - Set-words with trailing colon (e.g., `word:`)
//!   - Get-words with leading colon (e.g., `:word`)
//! - Integer literals (e.g., `123`, `-456`, `+789`)
//! - Float literals (e.g., `3.14`, `-2.5`, `+10.0`)
//! - Block structures with nested blocks (e.g., `[outer [inner]]`)
//! - Path notation (e.g., `word/path/item`)
//! - Comments using semicolons (e.g., `; comment`)

use std::str::CharIndices;
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParserError<C> {
    /// Input ended unexpectedly
    #[error("end of input")]
    EndOfInput,
    /// An unexpected character was encountered
    #[error("unexpected character: `{0}`")]
    UnexpectedChar(char),
    /// Integer value exceeds the range of i32
    #[error("integer overflow")]
    IntegerOverflow,
    /// Float value exceeds the range of f32
    #[error("float overflow")]
    FloatOverflow,
    /// An unexpected error occurred
    #[error("unexpected error")]
    UnexpectedError,
    /// Attempted to parse an empty word
    #[error("empty word")]
    EmptyWord,
    /// Error propagated from the collector
    #[error("collector error")]
    CollectorError(#[from] C),
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
    
    /// Called when a float is parsed
    fn float(&mut self, value: f32) -> Result<(), Self::Error>;

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
    // Internal constructor
    fn new(input: &'a str, collector: &'a mut C) -> Self {
        Self {
            input,
            collector,
            cursor: input.char_indices(),
            in_path: false,
        }
    }

    /// Parse input as a block
    ///
    /// This method parses the input text and automatically wraps it in a block,
    /// calling the collector's `begin_block` and `end_block` methods to surround the content.
    /// This is useful when you want to parse content and treat it as if it were
    /// enclosed in square brackets, even if it's not.
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
    /// # use rebel::parse::{Collector, WordKind, Parser};
    /// # struct MyCollector;
    /// # impl Collector for MyCollector {
    /// #     type Error = ();
    /// #     fn string(&mut self, _: &str) -> Result<(), ()> { Ok(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Result<(), ()> { Ok(()) }
    /// #     fn integer(&mut self, _: i32) -> Result<(), ()> { Ok(()) }
    /// #     fn float(&mut self, _: f32) -> Result<(), ()> { Ok(()) }
    /// #     fn begin_block(&mut self) -> Result<(), ()> { Ok(()) }
    /// #     fn end_block(&mut self) -> Result<(), ()> { Ok(()) }
    /// #     fn begin_path(&mut self) -> Result<(), ()> { Ok(()) }
    /// #     fn end_path(&mut self) -> Result<(), ()> { Ok(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// // Note: Input content isn't inside brackets, but will be treated as a block
    /// let input = "word 123 \"string\"";
    /// Parser::parse_block(input, &mut collector).expect("Failed to parse block");
    /// ```
    pub fn parse_block(input: &'a str, collector: &'a mut C) -> Result<(), ParserError<C::Error>> {
        let mut parser = Self::new(input, collector);
        parser.collector.begin_block()?;
        parser.do_parse()?;
        parser.collector.end_block().map_err(Into::into)
    }

    /// Parse input directly with a collector
    ///
    /// This method parses the input exactly as provided without adding any wrappers.
    /// Unlike `parse_block`, which automatically wraps the input in a block,
    /// `parse` processes the input exactly as given.
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
    /// # use rebel::parse::{Collector, WordKind, Parser};
    /// # struct MyCollector;
    /// # impl Collector for MyCollector {
    /// #     type Error = ();
    /// #     fn string(&mut self, _: &str) -> Result<(), ()> { Ok(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Result<(), ()> { Ok(()) }
    /// #     fn integer(&mut self, _: i32) -> Result<(), ()> { Ok(()) }
    /// #     fn float(&mut self, _: f32) -> Result<(), ()> { Ok(()) }
    /// #     fn begin_block(&mut self) -> Result<(), ()> { Ok(()) }
    /// #     fn end_block(&mut self) -> Result<(), ()> { Ok(()) }
    /// #     fn begin_path(&mut self) -> Result<(), ()> { Ok(()) }
    /// #     fn end_path(&mut self) -> Result<(), ()> { Ok(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// let input = "[word 123 \"string\"]";
    /// Parser::parse(input, &mut collector).expect("Failed to parse");
    /// ```
    pub fn parse(input: &'a str, collector: &'a mut C) -> Result<(), ParserError<C::Error>> {
        let mut parser = Self::new(input, collector);
        parser.do_parse()
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

        for (_, char) in self.cursor.by_ref() {
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
                return Ok(self.collector.string(&result).map(|_| None)?);
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
    ) -> Result<Option<char>, ParserError<C::Error>> {
        if let Some('/') = consumed {
            if !self.in_path {
                self.in_path = true;
                self.collector.begin_path()?
            }
        }
        Ok(self.collector.word(kind, symbol).map(|_| consumed)?)
    }

    fn parse_word(&mut self, start_pos: usize) -> Result<Option<char>, ParserError<C::Error>> {
        let mut kind = WordKind::Word;
        let mut word_start = start_pos;

        // Special handling for get-words starting with a colon
        if self.input.as_bytes().get(start_pos) == Some(&b':') {
            kind = WordKind::GetWord;
            word_start = start_pos + 1; // Skip the colon for get-words
        }

        let consumed = loop {
            match self.cursor.next() {
                Some((pos, char)) => match char {
                    ':' => {
                        if pos != start_pos {
                            // Not at the beginning (already handled)
                            kind = WordKind::SetWord;
                            break Some(char);
                        }
                    }
                    ']' | '/' => break Some(char),
                    c if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '?' => {}
                    c if c.is_ascii_whitespace() => break Some(char),
                    _ => return Err(ParserError::UnexpectedChar(char)),
                },
                None => break None,
            }
        };

        let pos = self.cursor.offset() - if consumed.is_some() { 1 } else { 0 };
        if pos <= word_start {
            return Err(ParserError::EmptyWord);
        }
        let symbol = self
            .input
            .get(word_start..pos)
            .ok_or(ParserError::UnexpectedError)?;

        self.collect_word(symbol, kind, consumed)
    }

    fn parse_number(&mut self, char: char) -> Result<Option<char>, ParserError<C::Error>> {
        let mut int_value: i32 = 0;
        let mut float_value: f32 = 0.0;
        let mut is_negative = false;
        let mut has_digits = false;
        let mut is_float = false;
        let mut decimal_position = 0.1;
        let mut consumed = None;

        match char {
            '+' => {}
            '-' => {
                is_negative = true;
            }
            c if c.is_ascii_digit() => {
                int_value = c.to_digit(10).ok_or(ParserError::UnexpectedError)? as i32;
                has_digits = true;
            }
            _ => return Err(ParserError::UnexpectedChar(char)),
        }

        for (_, char) in self.cursor.by_ref() {
            match char {
                '.' if !is_float => {
                    is_float = true;
                    float_value = int_value as f32;
                }
                c if c.is_ascii_digit() => {
                    has_digits = true;
                    let digit = c.to_digit(10).ok_or(ParserError::UnexpectedError)? as i32;
                    
                    if is_float {
                        let digit_float = digit as f32 * decimal_position;
                        float_value += digit_float;
                        decimal_position *= 0.1;
                    } else {
                        // Still parsing integer part
                        int_value = int_value
                            .checked_mul(10)
                            .and_then(|v| v.checked_add(digit))
                            .ok_or(ParserError::IntegerOverflow)?;
                    }
                }
                ']' => {
                    consumed = Some(char);
                    break;
                }
                c if c.is_ascii_whitespace() => {
                    break;
                }
                _ => {
                    return Err(ParserError::UnexpectedChar(char));
                }
            }
        }
        
        if !has_digits {
            return Err(ParserError::EndOfInput);
        }
        
        if is_float {
            if is_negative {
                float_value = -float_value;
            }
            self.collector
                .float(float_value)
                .map(|_| consumed)
                .map_err(Into::into)
        } else {
            if is_negative {
                int_value = int_value.checked_neg().ok_or(ParserError::IntegerOverflow)?;
            }
            self.collector
                .integer(int_value)
                .map(|_| consumed)
                .map_err(Into::into)
        }
    }

    fn process_block_end(&mut self, consumed: Option<char>) -> Result<(), ParserError<C::Error>> {
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

    fn do_parse(&mut self) -> Result<(), ParserError<C::Error>> {
        while let Some((pos, char)) = self.skip_whitespace() {
            let consumed = match char {
                '[' => self.collector.begin_block().map(|()| None)?,
                ']' => Some(char),
                '"' => self.parse_string(pos)?,
                ':' => self.parse_word(pos)?, // Special handling for get-words
                c if c.is_ascii_alphabetic() => self.parse_word(pos)?,
                c if c.is_ascii_digit() || c == '+' || c == '-' => self.parse_number(c)?,
                _ => return Err(ParserError::UnexpectedChar(char)),
            };
            self.process_block_end(consumed)?;
        }
        Ok(())
    }
}
