// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, WordKind};
use smol_str::SmolStr;
use thiserror::Error;

/// Errors that can occur during value collection
#[derive(Debug, Error)]
pub enum ValueCollectorError {
    /// Attempted to end a block without a matching begin_block
    #[error("unmatched end_block")]
    UnmatchedEndBlock,

    /// Attempted to end a path without a matching begin_path
    #[error("unmatched end_path")]
    UnmatchedEndPath,

    /// Attempted to end a path when the current context is a block
    #[error("end path without matching begin_path")]
    PathBlockMismatch,
}

/// Represents a Rebel value in memory
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Represents no value
    None,
    /// Integer value
    Int(i32),
    /// Boolean value
    Bool(bool),
    /// Ordered collection of values
    Block(Box<[Value]>),
    /// String value
    String(SmolStr),
    /// Word identifier
    Word(SmolStr),
    /// Word with assignment semantics
    SetWord(SmolStr),
    /// Word with lookup semantics
    GetWord(SmolStr),
    /// Key-value pairs representing an object
    Context(Box<[(SmolStr, Value)]>),
    /// Path expression
    Path(Box<[Value]>),
}

impl Value {
    /// Converts the value to its string representation
    pub fn form(&self) -> String {
        match self {
            Value::None => "none".into(),
            Value::Int(n) => n.to_string(),
            Value::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Value::String(s) => s.to_string(),
            Value::Word(w) => w.to_string(),
            Value::SetWord(w) => {
                let mut result = w.to_string();
                result.push(':');
                result
            }
            Value::GetWord(w) => {
                let mut result = ":".to_string();
                result.push_str(&w);
                result
            }
            Value::Block(block) => {
                let mut result = String::new();
                let mut first = true;
                for item in block.iter() {
                    if !first {
                        result.push(' ');
                    }
                    first = false;
                    result.push_str(&item.form());
                }
                result
            }
            Value::Context(pairs) => {
                let mut result = "make object! [".to_string();
                let mut first = true;
                for (key, value) in pairs.iter() {
                    if !first {
                        result.push(' ');
                    }
                    first = false;
                    result.push_str(&key);
                    result.push(':');
                    result.push_str(&value.form());
                }
                result.push(']');
                result
            }
            Value::Path(path) => {
                let mut result = String::new();
                let mut first = true;
                for segment in path.iter() {
                    if !first {
                        result.push('/');
                    }
                    first = false;
                    result.push_str(&segment.form());
                }
                result
            }
        }
    }

    /// Create a None value
    pub fn none() -> Self {
        Value::None
    }

    /// Create an Int value
    pub fn int(value: i32) -> Self {
        Value::Int(value)
    }

    /// Create a boolean value (as an Int with value 1 or 0)
    pub fn boolean(value: bool) -> Self {
        Value::Bool(value)
    }

    /// Create a String value
    pub fn string<S: Into<SmolStr>>(value: S) -> Self {
        Value::String(value.into())
    }

    /// Create a Word value
    pub fn word<S: Into<SmolStr>>(value: S) -> Self {
        Value::Word(value.into())
    }

    /// Create a SetWord value
    pub fn set_word<S: Into<SmolStr>>(value: S) -> Self {
        Value::SetWord(value.into())
    }

    /// Create a Block value from any iterable of Values
    pub fn block<I: IntoIterator<Item = Value>>(values: I) -> Self {
        Value::Block(values.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// Create a Context (object) value from any iterable of key-value pairs
    pub fn context<K: Into<SmolStr>, I: IntoIterator<Item = (K, Value)>>(pairs: I) -> Self {
        Value::Context(
            pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    /// Create a Path value from any iterable of Values
    pub fn path<I: IntoIterator<Item = Value>>(values: I) -> Self {
        Value::Path(values.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// Check if value is None
    pub fn is_none(&self) -> bool {
        matches!(self, Value::None)
    }

    /// Check if value is Int
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    /// Check if value is String
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if value is Word
    pub fn is_word(&self) -> bool {
        matches!(self, Value::Word(_))
    }

    /// Check if value is SetWord
    pub fn is_set_word(&self) -> bool {
        matches!(self, Value::SetWord(_))
    }

    /// Check if value is Block
    pub fn is_block(&self) -> bool {
        matches!(self, Value::Block(_))
    }

    /// Check if value is Context
    pub fn is_context(&self) -> bool {
        matches!(self, Value::Context(_))
    }

    /// Check if value is GetWord
    pub fn is_get_word(&self) -> bool {
        matches!(self, Value::GetWord(_))
    }

    /// Check if value is Path
    pub fn is_path(&self) -> bool {
        matches!(self, Value::Path(_))
    }

    /// Check if value represents a boolean (Int with value 0 or 1)
    pub fn is_boolean(&self) -> bool {
        match self {
            Value::Int(0 | 1) => true,
            _ => false,
        }
    }
    /// Extract an i32 value if this is an Int
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Extract a string reference if this is a String
    pub fn as_string(&self) -> Option<&SmolStr> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Extract a word reference if this is a Word
    pub fn as_word(&self) -> Option<&SmolStr> {
        match self {
            Value::Word(w) => Some(w),
            _ => None,
        }
    }

    /// Extract a setword reference if this is a SetWord
    pub fn as_set_word(&self) -> Option<&SmolStr> {
        match self {
            Value::SetWord(w) => Some(w),
            _ => None,
        }
    }

    /// Extract a boolean if this is an Int(0) or Int(1)
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Extract a block slice if this is a Block
    pub fn as_block(&self) -> Option<&[Value]> {
        match self {
            Value::Block(block) => Some(block),
            _ => None,
        }
    }

    /// Extract a mutable block slice if this is a Block
    pub fn as_block_mut(&mut self) -> Option<&mut [Value]> {
        match self {
            Value::Block(block) => Some(block),
            _ => None,
        }
    }

    /// Extract a context slice if this is a Context
    pub fn as_context(&self) -> Option<&[(SmolStr, Value)]> {
        match self {
            Value::Context(pairs) => Some(pairs),
            _ => None,
        }
    }

    /// Extract a get_word reference if this is a GetWord
    pub fn as_get_word(&self) -> Option<&SmolStr> {
        match self {
            Value::GetWord(w) => Some(w),
            _ => None,
        }
    }

    /// Extract a path slice if this is a Path
    pub fn as_path(&self) -> Option<&[Value]> {
        match self {
            Value::Path(path) => Some(path),
            _ => None,
        }
    }

    /// Create a GetWord value
    pub fn get_word<S: Into<SmolStr>>(value: S) -> Self {
        Value::GetWord(value.into())
    }
}

/// Collector that builds a Value during parsing
///
/// This collector implements the Collector trait to build a hierarchical Value
/// structure as tokens are parsed. It maintains a stack of current blocks/paths
/// to handle nesting correctly.
pub struct ValueCollector {
    /// The final collected value
    result: Option<Value>,
    /// Stack of values for nested blocks and paths
    stack: Vec<Vec<Value>>,
    /// Stack tracking whether each level is a block or path
    path_stack: Vec<bool>,
}

impl ValueCollector {
    /// Create a new ValueCollector
    pub fn new() -> Self {
        Self {
            result: None,
            stack: Vec::new(),
            path_stack: Vec::new(),
        }
    }

    /// Get the final collected value
    ///
    /// Returns the constructed Value if parsing was successful,
    /// or None if no value was built (e.g., empty input).
    pub fn value(&self) -> Option<Value> {
        self.result.clone()
    }

    /// Push a value to the current block or path being built
    fn push_value(&mut self, value: Value) {
        if let Some(current) = self.stack.last_mut() {
            current.push(value);
        } else {
            self.result = Some(value);
        }
    }
}

impl Default for ValueCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for ValueCollector {
    type Error = ValueCollectorError;

    fn string(&mut self, string: &str) -> Result<(), Self::Error> {
        self.push_value(Value::string(string));
        Ok(())
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Result<(), Self::Error> {
        let value = match kind {
            WordKind::Word => Value::word(word),
            WordKind::SetWord => Value::set_word(word),
            WordKind::GetWord => Value::get_word(word),
        };
        self.push_value(value);
        Ok(())
    }

    fn integer(&mut self, value: i32) -> Result<(), Self::Error> {
        self.push_value(Value::int(value));
        Ok(())
    }

    fn begin_block(&mut self) -> Result<(), Self::Error> {
        self.stack.push(Vec::new());
        self.path_stack.push(false);
        Ok(())
    }

    fn end_block(&mut self) -> Result<(), Self::Error> {
        if let Some(values) = self.stack.pop() {
            self.path_stack.pop();
            let block = Value::block(values);

            if self.stack.is_empty() {
                self.result = Some(block);
            } else {
                self.push_value(block);
            }
            Ok(())
        } else {
            Err(ValueCollectorError::UnmatchedEndBlock)
        }
    }

    fn begin_path(&mut self) -> Result<(), Self::Error> {
        self.stack.push(Vec::new());
        self.path_stack.push(true);
        Ok(())
    }

    fn end_path(&mut self) -> Result<(), Self::Error> {
        if let Some(values) = self.stack.pop() {
            if self.path_stack.pop() != Some(true) {
                return Err(ValueCollectorError::PathBlockMismatch);
            }

            let path = Value::path(values);

            if self.stack.is_empty() {
                self.result = Some(path);
            } else {
                self.push_value(path);
            }
            Ok(())
        } else {
            Err(ValueCollectorError::UnmatchedEndPath)
        }
    }
}
