// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Memory, MemoryError, Offset, Series, Type, Value};
use crate::parse::{Collector, Parser, WordKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    ParserError(#[from] crate::parse::ParserError<MemoryError>),
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
}

pub struct Process<'a> {
    memory: &'a mut Memory,
    stack: Series<Value>,
    pos_stack: Series<Offset>,
}

impl<'a> Process<'a> {
    pub fn new(memory: &'a mut Memory) -> Result<Self, MemoryError> {
        let stack = memory.alloc::<Value>(64)?;
        let pos_stack = memory.alloc::<Offset>(64)?;
        Ok(Self {
            memory,
            stack,
            pos_stack,
        })
    }

    pub fn parse_block(&mut self, input: &str) -> Result<Value, VmError> {
        Parser::parse_block(input, self)?;
        self.memory.pop(self.stack).map_err(Into::into)
    }

    fn begin(&mut self) -> Result<(), MemoryError> {
        self.memory
            .push(self.pos_stack, self.memory.len(self.stack)?)
    }

    fn end(&mut self, kind: Type) -> Result<(), MemoryError> {
        let pos = self.memory.pop(self.pos_stack)?;
        let block = self.memory.drain(self.stack, pos)?;
        self.memory
            .push(self.stack, Value::new(kind, block.address()))
    }
}

impl Collector for Process<'_> {
    type Error = MemoryError;

    /// Called when a string is parsed
    fn string(&mut self, string: &str) -> Result<(), Self::Error> {
        let string = self.memory.alloc_string(string).map(Value::string)?;
        self.memory.push(self.stack, string)
    }

    /// Called when a word is parsed
    fn word(&mut self, kind: WordKind, symbol: &str) -> Result<(), Self::Error> {
        let symbol = self.memory.alloc_string(symbol)?;
        self.memory.push(self.stack, Value::any_word(kind, symbol))
    }

    /// Called when an integer is parsed
    fn integer(&mut self, value: i32) -> Result<(), Self::Error> {
        self.memory.push(self.stack, Value::int(value))
    }
    
    /// Called when a float is parsed
    fn float(&mut self, value: f32) -> Result<(), Self::Error> {
        self.memory.push(self.stack, Value::float(value))
    }

    /// Called at the start of a block
    fn begin_block(&mut self) -> Result<(), Self::Error> {
        self.begin()
    }

    /// Called at the end of a block
    fn end_block(&mut self) -> Result<(), Self::Error> {
        self.end(Value::BLOCK)
    }

    /// Called at the start of a path
    fn begin_path(&mut self) -> Result<(), Self::Error> {
        self.begin()
    }

    /// Called at the end of a path
    fn end_path(&mut self) -> Result<(), Self::Error> {
        self.end(Value::PATH)
    }
}
