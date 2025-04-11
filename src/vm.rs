// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Block, Memory, MemoryError, Offset, Series, Type, Value, Word};
use crate::parse::{Collector, Parser, WordKind};
use bytemuck::{Pod, Zeroable};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    ParserError(#[from] crate::parse::ParserError<MemoryError>),
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
    #[error("Invalid code")]
    InvalidCode,
}

//

type Op = Word;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct Code(Op, Word);

impl Code {
    const SIZE_IN_WORDS: Offset = 2;

    // const HALT: Op = 0;
    const CONST: Op = 1;
    const TYPE: Op = 2;
    const WORD: Op = 3;
    const SET_WORD: Op = 4;
    // const CALL_NATIVE: Op = 5;
    const LEAVE: Op = 6;

    pub fn new(op: Op, data: Word) -> Self {
        Code(op, data)
    }

    pub fn op(&self) -> Op {
        self.0
    }

    pub fn data(&self) -> Word {
        self.1
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
        let mut ip = block.address() + Value::SIZE_IN_WORDS;
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
                    self.memory.push_n(
                        self.code_stack,
                        &[
                            Code::new(Code::TYPE, value.kind()),
                            Code::new(Code::CONST, value.data()),
                        ],
                    )?;
                    stack_len += 1;
                }
            }
            ip += Value::SIZE_IN_WORDS;
        }
        // fix stack
        match stack_len {
            0 => {
                self.memory.push_n(
                    self.code_stack,
                    &[
                        Code::new(Code::TYPE, Value::NONE),
                        Code::new(Code::CONST, 0),
                    ],
                )?;
            }
            1 => {}
            n => {
                self.memory
                    .push(self.code_stack, Code::new(Code::LEAVE, n - 1))?;
            }
        }
        Ok(self.memory.drain(self.code_stack, 0)?)
    }

    pub fn exec(&mut self, code: Series<Code>) -> Result<Value, VmError> {
        let mut ip = code.address() + Block::SIZE_IN_WORDS;
        let end = ip + self.memory.len(code)? * Code::SIZE_IN_WORDS;

        let mut kind = Value::NONE;

        while ip < end {
            let code = *self.memory.get::<Code>(ip)?;
            match code {
                Code(Code::TYPE, typ) => kind = typ,
                Code(Code::CONST, value) => {
                    self.memory.push(self.stack, Value::new(kind, value))?
                }
                Code(Code::WORD, symbol) => {
                    let value = self.memory.get_word(symbol)?;
                    self.memory.push(self.stack, value)?;
                }
                Code(Code::SET_WORD, symbol) => {
                    let value = self.memory.peek(self.stack)?.copied();
                    let value = value.ok_or(MemoryError::StackUnderflow)?;
                    self.memory.set_word(symbol, value)?;
                }
                Code(Code::LEAVE, drop) => {
                    let value = self.memory.pop(self.stack)?;
                    self.memory.drop(self.stack, drop)?;
                    self.memory.push(self.stack, value)?;
                }
                _ => {
                    return Err(VmError::InvalidCode);
                }
            }
            ip += Code::SIZE_IN_WORDS;
        }
        self.memory.pop(self.stack).map_err(Into::into)
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
        // let symbol = self.memory.alloc_string(symbol)?;
        let symbol = self.memory.get_or_add_symbol(symbol)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::{MemoryError, Value};

    // Helper function to create a test memory
    fn create_test_memory() -> Result<Memory, MemoryError> {
        Memory::new(65536)
    }

    // Test basic block parsing with Process
    #[test]
    fn test_parse_1() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("x: 5 x")?;
        let block = memory.peek_at(result.as_block()?, 0)?;

        match block {
            [
                Value(Value::SET_WORD, x),
                Value(Value::INT, 5),
                Value(Value::WORD, y),
            ] => {
                assert_eq!(x, y, "Expected x to be equal to y");
            }
            _ => panic!("Unexpected block structure"),
        }

        Ok(())
    }

    // Test empty block parsing
    #[test]
    fn test_parse_empty_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("")?;

        assert_eq!(result.kind(), Value::BLOCK);
        assert!(result.data() > 0, "Block address should be valid");

        let block = result.as_block()?;
        assert_eq!(memory.len(block)?, 0, "Block should be empty");

        Ok(())
    }

    // Test integer block parsing
    #[test]
    fn test_parse_integer_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("1 2 3")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 3, "Block should contain 3 integers");

        match values {
            [
                Value(Value::INT, 1),
                Value(Value::INT, 2),
                Value(Value::INT, 3),
            ] => {}
            _ => panic!("Unexpected block structure"),
        }

        Ok(())
    }

    // Test float block parsing
    #[test]
    fn test_parse_float_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("3.14 -2.5 0.0")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 3, "Block should contain 3 floats");

        // Check types
        assert_eq!(values[0].kind(), Value::FLOAT);
        assert_eq!(values[1].kind(), Value::FLOAT);
        assert_eq!(values[2].kind(), Value::FLOAT);

        // Check values with approximate comparison
        assert!((values[0].as_float()? - 3.14).abs() < 0.0001);
        assert!((values[1].as_float()? - (-2.5)).abs() < 0.0001);
        assert!((values[2].as_float()? - 0.0).abs() < 0.0001);

        Ok(())
    }

    // Test mixed numeric values
    #[test]
    fn test_parse_mixed_numeric_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("42 3.14159 -10 -0.5")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 4, "Block should contain 4 values");

        assert_eq!(values[0].kind(), Value::INT);
        assert_eq!(values[0].data(), 42);

        assert_eq!(values[1].kind(), Value::FLOAT);
        assert!((values[1].as_float()? - 3.14159).abs() < 0.0001);

        assert_eq!(values[2].kind(), Value::INT);
        assert_eq!(values[2].as_int()?, -10);

        assert_eq!(values[3].kind(), Value::FLOAT);
        assert!((values[3].as_float()? - (-0.5)).abs() < 0.0001);

        Ok(())
    }

    // Test string block parsing
    #[test]
    fn test_parse_string_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("\"hello\" \"world\"")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 2, "Block should contain 2 strings");

        assert_eq!(values[0].kind(), Value::STRING);
        assert_eq!(values[1].kind(), Value::STRING);

        // String addresses should be different
        assert_ne!(values[0].data(), values[1].data());

        Ok(())
    }

    // Test word types
    #[test]
    fn test_parse_word_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("word set-word: :get-word")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 3, "Block should contain 3 values");

        match values {
            [
                Value(Value::WORD, _),
                Value(Value::SET_WORD, _),
                Value(Value::GET_WORD, _),
            ] => {}
            _ => panic!("Unexpected block structure"),
        }

        Ok(())
    }

    // Test nested blocks
    #[test]
    fn test_parse_nested_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("1 [2 3] 4")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 3, "Block should contain 3 values");

        // Check outer structure
        assert_eq!(values[0].kind(), Value::INT);
        assert_eq!(values[0].data(), 1);

        assert_eq!(values[1].kind(), Value::BLOCK);

        assert_eq!(values[2].kind(), Value::INT);
        assert_eq!(values[2].data(), 4);

        // Verify nested block
        let nested_block = values[1].as_block()?;
        let nested_values = memory.peek_at(nested_block, 0)?;

        assert_eq!(
            nested_values.len(),
            2,
            "Nested block should contain 2 values"
        );
        assert_eq!(nested_values[0].kind(), Value::INT);
        assert_eq!(nested_values[0].data(), 2);
        assert_eq!(nested_values[1].kind(), Value::INT);
        assert_eq!(nested_values[1].data(), 3);

        Ok(())
    }

    // Test path notation
    #[test]
    fn test_parse_path() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("a/b c/d/e")?;
        let block = result.as_block()?;
        let values = memory.peek_at(block, 0)?;

        assert_eq!(values.len(), 2, "Block should contain 2 paths");

        // Both should be PATH type
        assert_eq!(values[0].kind(), Value::PATH);
        assert_eq!(values[1].kind(), Value::PATH);

        // First path should have 2 elements, second should have 3
        let path1 = values[0].as_path()?;
        let path2 = values[1].as_path()?;

        assert_eq!(memory.len(path1)?, 2);
        assert_eq!(memory.len(path2)?, 3);

        Ok(())
    }

    // Test error handling
    #[test]
    fn test_parse_errors() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Invalid escape sequence
        let result = process.parse_block("\"invalid \\z escape\"");
        assert!(result.is_err(), "Should error on invalid escape sequence");

        Ok(())
    }

    // Test compilation of constants
    #[test]
    fn test_compile_constants() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let block = process.parse_block("1 2 3")?;
        let code_block = process.compile(block.as_block()?)?;
        let code = memory.peek_at(code_block, 0)?;

        assert_eq!(code.len(), 7, "Should generate 7 instructions");

        match code {
            [
                Code(Code::TYPE, Value::INT),
                Code(Code::CONST, 1),
                Code(Code::TYPE, Value::INT),
                Code(Code::CONST, 2),
                Code(Code::TYPE, Value::INT),
                Code(Code::CONST, 3),
                Code(Code::LEAVE, 2),
            ] => {}
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    // Test compilation with set-word
    #[test]
    fn test_compile_set_word_and_use() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let block = process.parse_block("x: 5 x")?;
        let code_block = process.compile(block.as_block()?)?;
        let code = memory.peek_at(code_block, 0)?;

        match code {
            [
                Code(Code::TYPE, Value::INT),
                Code(Code::CONST, 5),
                Code(Code::SET_WORD, x),
                Code(Code::WORD, y),
                Code(Code::LEAVE, 1),
            ] => {
                assert_eq!(x, y, "x should be same symbol")
            }
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    // Test compilation with multiple set-words
    #[test]
    fn test_compile_multiple_set_words() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let block = process.parse_block("x: y: z: 42 y")?;
        let code_block = process.compile(block.as_block()?)?;
        let code = memory.peek_at(code_block, 0)?;

        match code {
            [
                Code(Code::TYPE, Value::INT),
                Code(Code::CONST, 42),
                Code(Code::SET_WORD, x),
                Code(Code::SET_WORD, y),
                Code(Code::SET_WORD, z),
                Code(Code::WORD, m),
                Code(Code::LEAVE, 1),
            ] => {
                assert_eq!(m, y, "y should be same symbol");
                assert_ne!(x, y, "x should be different from y");
                assert_ne!(x, z, "x should be different from z");
                assert_ne!(y, z, "y should be different from z");
            }
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    // Test compilation of empty block
    #[test]
    fn test_compile_empty_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let block = process.parse_block("")?;
        let code_block = process.compile(block.as_block()?)?;
        let code = memory.peek_at(code_block, 0)?;

        assert_eq!(code.len(), 2, "Empty block should have 2 instructions");

        match code {
            [Code(Code::TYPE, Value::NONE), Code(Code::CONST, 0)] => {}
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    #[test]
    fn test_exec_1() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let block = process.parse_block("1 2 3")?;
        let code_block = process.compile(block.as_block()?)?;

        let result = process.exec(code_block)?;
        assert_eq!(result, Value::int(3), "Expected result to be 3");

        Ok(())
    }

    #[test]
    fn test_exec_2() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let block = process.parse_block("x: y: 42 z: 5 y")?;
        let code_block = process.compile(block.as_block()?)?;

        let result = process.exec(code_block)?;
        assert_eq!(result, Value::int(42), "Expected result to be 3");

        Ok(())
    }
}
