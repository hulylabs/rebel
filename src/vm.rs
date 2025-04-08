// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Memory, MemoryError, Offset, Series, Value};
use crate::parse::{Collector, WordKind};

pub struct Process<'a> {
    memory: &'a mut Memory,
    stack: Series<Value>,
    pos_stack: Series<Offset>,
}

impl<'a> Collector for Process<'a> {
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

    /// Called at the start of a block
    fn begin_block(&mut self) -> Result<(), Self::Error> {
        self.memory
            .push(self.pos_stack, self.memory.len(self.stack)?)
    }

    /// Called at the end of a block
    fn end_block(&mut self) -> Result<(), Self::Error> {
        let block_series = self.drain()?;
        let block = Value::block(block_series);
        self.stack.push(block, &mut self.memory)
    }

    /// Called at the start of a path
    fn begin_path(&mut self) -> Result<(), Self::Error> {
        let len = self.stack.len(self.memory)?;
        self.pos_stack.push(len, &mut self.memory)
    }

    /// Called at the end of a path
    fn end_path(&mut self) -> Result<(), Self::Error> {
        let block = self.drain().map(Value::path)?;
        self.stack.push(block, &mut self.memory)
    }
}
