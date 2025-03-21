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
//! - Block structures with nested blocks (e.g., `[outer [inner]]`)
//! - Path notation (e.g., `word/path/item`)
//! - Comments using semicolons (e.g., `; comment`)

use std::str::CharIndices;
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParserError {
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
    #[error("collector error")]
    CollectorError,
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
    fn string(&mut self, string: &str) -> Option<()>;

    /// Called when a word is parsed
    fn word(&mut self, kind: WordKind, word: &str) -> Option<()>;

    /// Called when an integer is parsed
    fn integer(&mut self, value: i32) -> Option<()>;

    /// Called at the start of a block
    fn begin_block(&mut self) -> Option<()>;

    /// Called at the end of a block
    fn end_block(&mut self) -> Option<()>;

    /// Called at the start of a path
    fn begin_path(&mut self) -> Option<()>;

    /// Called at the end of a path
    fn end_path(&mut self) -> Option<()>;
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
    /// #     fn string(&mut self, _: &str) -> Option<()> { Some(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Option<()> { Some(()) }
    /// #     fn integer(&mut self, _: i32) -> Option<()> { Some(()) }
    /// #     fn begin_block(&mut self) -> Option<()> { Some(()) }
    /// #     fn end_block(&mut self) -> Option<()> { Some(()) }
    /// #     fn begin_path(&mut self) -> Option<()> { Some(()) }
    /// #     fn end_path(&mut self) -> Option<()> { Some(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// // Note: Input content isn't inside brackets, but will be treated as a block
    /// let input = "word 123 \"string\"";
    /// Parser::parse_block(input, &mut collector).expect("Failed to parse block");
    /// ```
    pub fn parse_block(input: &'a str, collector: &'a mut C) -> Result<(), ParserError> {
        let mut parser = Self::new(input, collector);
        parser
            .collector
            .begin_block()
            .ok_or(ParserError::CollectorError)?;
        parser.do_parse()?;
        parser
            .collector
            .end_block()
            .ok_or(ParserError::CollectorError)
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
    /// #     fn string(&mut self, _: &str) -> Option<()> { Some(()) }
    /// #     fn word(&mut self, _: WordKind, _: &str) -> Option<()> { Some(()) }
    /// #     fn integer(&mut self, _: i32) -> Option<()> { Some(()) }
    /// #     fn begin_block(&mut self) -> Option<()> { Some(()) }
    /// #     fn end_block(&mut self) -> Option<()> { Some(()) }
    /// #     fn begin_path(&mut self) -> Option<()> { Some(()) }
    /// #     fn end_path(&mut self) -> Option<()> { Some(()) }
    /// # }
    /// # let mut collector = MyCollector;
    /// let input = "[word 123 \"string\"]";
    /// Parser::parse(input, &mut collector).expect("Failed to parse");
    /// ```
    pub fn parse(input: &'a str, collector: &'a mut C) -> Result<(), ParserError> {
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

    fn parse_string(&mut self, pos: usize) -> Result<Option<char>, ParserError> {
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
                return self
                    .collector
                    .string(&result)
                    .map(|()| None)
                    .ok_or(ParserError::CollectorError);
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
    ) -> Option<Option<char>> {
        if let Some('/') = consumed {
            if !self.in_path {
                self.in_path = true;
                self.collector.begin_path()?
            }
        }
        self.collector.word(kind, symbol).map(|_| consumed)
    }

    fn parse_word(&mut self, start_pos: usize) -> Result<Option<char>, ParserError> {
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
            .ok_or(ParserError::CollectorError)
    }

    fn parse_number(&mut self, char: char) -> Result<Option<char>, ParserError> {
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
            .ok_or(ParserError::CollectorError)
    }

    fn process_block_end(&mut self, consumed: Option<char>) -> Option<()> {
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
        Some(())
    }

    fn do_parse(&mut self) -> Result<(), ParserError> {
        while let Some((pos, char)) = self.skip_whitespace() {
            let consumed = match char {
                '[' => self
                    .collector
                    .begin_block()
                    .map(|()| None)
                    .ok_or(ParserError::CollectorError)?,
                ']' => Some(char),
                '"' => self.parse_string(pos)?,
                ':' => self.parse_word(pos)?, // Special handling for get-words
                c if c.is_ascii_alphabetic() => self.parse_word(pos)?,
                c if c.is_ascii_digit() || c == '+' || c == '-' => self.parse_number(c)?,
                _ => return Err(ParserError::UnexpectedChar(char)),
            };
            self.process_block_end(consumed)
                .ok_or(ParserError::CollectorError)?;
        }
        Ok(())
    }
}
