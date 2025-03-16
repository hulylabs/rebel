// RebelDB™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Int(i32),
    Bool(bool),
    Block(Box<[Value]>),
    String(SmolStr),
    Word(SmolStr),
    SetWord(SmolStr),
    GetWord(SmolStr),
    Context(Box<[(SmolStr, Value)]>),
    Path(Box<[Value]>),
}

impl Value {
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
}
