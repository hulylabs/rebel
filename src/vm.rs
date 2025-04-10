// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Memory, MemoryError, Offset, Series, Type, Value, Word};
use crate::parse::{Collector, Parser, WordKind};
use bytemuck::{Pod, Zeroable};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    ParserError(#[from] crate::parse::ParserError<MemoryError>),
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
}

//

pub type Op = Word;
pub type CodeBlock = Series<Code>;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Code(Op, Word);

impl Code {
    const HALT: Op = 0;
    const CONST: Op = 1;
    const TYPE: Op = 2;
    const WORD: Op = 3;
    const SET_WORD: Op = 4;
    const CALL_NATIVE: Op = 5;

    pub fn new(op: Op, data: Word) -> Self {
        Code(op, data)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Defer {
    code: Code,
    bp: Offset,
    arity: Word,
}

impl Defer {
    fn new(code: Code, bp: Offset, arity: Word) -> Self {
        Defer { code, bp, arity }
    }
}

pub struct Process<'a> {
    memory: &'a mut Memory,
    stack: Series<Value>,
    pos_stack: Series<Offset>,
    defer_stack: Series<Defer>,
    code_stack: Series<Code>,
}

impl<'a> Process<'a> {
    pub fn new(memory: &'a mut Memory) -> Result<Self, MemoryError> {
        let stack = memory.alloc::<Value>(64)?;
        let pos_stack = memory.alloc::<Offset>(64)?;
        let defer_stack = memory.alloc::<Defer>(64)?;
        let code_stack = memory.alloc::<Code>(64)?;

        Ok(Self {
            memory,
            stack,
            pos_stack,
            defer_stack,
            code_stack,
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

    pub fn compile(&mut self, block: Series<Value>) -> Result<Series<Code>, MemoryError> {
        let mut ip = block.address();
        let end = ip + self.memory.len(block)? * Value::SIZE_IN_WORDS;
        let mut stack_len = 0;

        while ip < end {
            while let Some(defer) = self.memory.peek(self.defer_stack)? {
                if stack_len == defer.bp + defer.arity {
                    stack_len -= defer.arity;
                    stack_len += 1;
                    self.memory.push(self.code_stack, defer.code)?;
                    self.memory.pop(self.defer_stack)?;
                } else {
                    break;
                }
            }

            let value = self.memory.get::<Value>(ip)?;
            match value.kind() {
                Value::WORD => {
                    let code = Code::new(Code::WORD, value.data());
                    self.memory.push(self.code_stack, code)?;
                    stack_len += 1;
                }
                Value::SET_WORD => {
                    let code = Code::new(Code::SET_WORD, value.data());
                    let defer = Defer::new(code, stack_len, 1);
                    self.memory.push(self.defer_stack, defer)?;
                }
                _ => {
                    let data = value.data();
                    self.memory
                        .push(self.code_stack, Code::new(Code::TYPE, value.kind()))?;
                    self.memory
                        .push(self.code_stack, Code::new(Code::CONST, data))?;
                    stack_len += 1;
                }
            }
            ip += Value::SIZE_IN_WORDS;
        }
        Ok(self.memory.drain(self.code_stack, 0)?)
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
