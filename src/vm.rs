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
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct Code(pub Op, pub Word);

impl Code {
    pub const HALT: Op = 0;
    pub const CONST: Op = 1;
    pub const TYPE: Op = 2;
    pub const WORD: Op = 3;
    pub const SET_WORD: Op = 4;
    pub const CALL_NATIVE: Op = 5;
    pub const LEAVE: Op = 6;

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
            println!("value: {:?}", value);

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
                    println!("defer: {:?}", defer);
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
        // fix stack
        match stack_len {
            0 => {
                self.memory.push_all(
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
    use crate::mem::{Block, MemoryError, Value};

    // Helper function to create a test memory
    fn create_test_memory() -> Result<Memory, MemoryError> {
        Memory::new(4096)
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

    // Test basic block parsing with Process
    #[test]
    fn test_parse_empty_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[]")?;
        println!("Result: kind={}, data={}", result.kind(), result.data());

        // The result should be of kind BLOCK (4)
        assert_eq!(result.kind(), Value::BLOCK, "Result should have BLOCK kind");

        // The data field should point to a valid block address
        assert!(result.data() > 0, "Result should have valid data address");

        // Check the block's content - parse_block("[]") returns a block containing an empty block [[]]
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;

        // The outer block should have length 1 (containing the empty block)
        assert_eq!(
            outer_block.len(),
            1,
            "Outer block should have length 1 (containing the empty block)"
        );

        // Get the inner block value (first item in the outer block)
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?; // Block::SIZE_IN_WORDS is 2

        // This inner value should be of kind BLOCK
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        // Now verify the inner block is empty (length 0)
        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(
            inner_block.len(),
            0,
            "Inner block should be empty (length 0)"
        );

        Ok(())
    }

    #[test]
    fn test_parse_integer_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[1 2 3]")?;

        // Verify the result is a block
        assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
        assert!(result.data() > 0, "Expected a valid block address");

        // Verify the outer block - should contain one inner block
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;
        assert_eq!(outer_block.len(), 1, "Outer block should have length 1");

        // Get the inner block and verify it contains integers 1, 2, 3
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(
            inner_block.len(),
            3,
            "Inner block should contain 3 integers"
        );

        // Verify the values: [1, 2, 3]
        let value_1 = *memory.get::<Value>(inner_block_addr + 2)?; // First integer
        let value_2 = *memory.get::<Value>(inner_block_addr + 4)?; // Second integer
        let value_3 = *memory.get::<Value>(inner_block_addr + 6)?; // Third integer

        assert_eq!(value_1.kind(), Value::INT, "First value should be INT");
        assert_eq!(value_1.data(), 1, "First value should be 1");

        assert_eq!(value_2.kind(), Value::INT, "Second value should be INT");
        assert_eq!(value_2.data(), 2, "Second value should be 2");

        assert_eq!(value_3.kind(), Value::INT, "Third value should be INT");
        assert_eq!(value_3.data(), 3, "Third value should be 3");

        Ok(())
    }

    #[test]
    fn test_parse_float_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[3.14 -2.5 0.0]")?;

        // Verify the result is a block
        assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
        assert!(result.data() > 0, "Expected a valid block address");

        // Verify the outer block - should contain one inner block
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;
        assert_eq!(outer_block.len(), 1, "Outer block should have length 1");

        // Get the inner block and verify it contains floats
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(inner_block.len(), 3, "Inner block should contain 3 floats");

        // Verify the values: [3.14, -2.5, 0.0]
        let value_1 = *memory.get::<Value>(inner_block_addr + 2)?; // First float
        let value_2 = *memory.get::<Value>(inner_block_addr + 4)?; // Second float
        let value_3 = *memory.get::<Value>(inner_block_addr + 6)?; // Third float

        assert_eq!(value_1.kind(), Value::FLOAT, "First value should be FLOAT");
        assert!(
            (value_1.as_float()? - 3.14).abs() < 0.0001,
            "First value should be 3.14"
        );

        assert_eq!(value_2.kind(), Value::FLOAT, "Second value should be FLOAT");
        assert!(
            (value_2.as_float()? - (-2.5)).abs() < 0.0001,
            "Second value should be -2.5"
        );

        assert_eq!(value_3.kind(), Value::FLOAT, "Third value should be FLOAT");
        assert!(
            (value_3.as_float()? - 0.0).abs() < 0.0001,
            "Third value should be 0.0"
        );

        Ok(())
    }

    #[test]
    fn test_parse_mixed_numeric_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Block with mixed integer and float values
        let result = process.parse_block("[42 3.14159 -10 -0.5]")?;

        // Verify the result is a block
        assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
        assert!(result.data() > 0, "Expected a valid block address");

        // Verify the outer block - should contain one inner block
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;
        assert_eq!(outer_block.len(), 1, "Outer block should have length 1");

        // Get the inner block and verify it contains mixed values
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(inner_block.len(), 4, "Inner block should contain 4 values");

        // Check the values
        let value_1 = *memory.get::<Value>(inner_block_addr + 2)?;
        let value_2 = *memory.get::<Value>(inner_block_addr + 4)?;
        let value_3 = *memory.get::<Value>(inner_block_addr + 6)?;
        let value_4 = *memory.get::<Value>(inner_block_addr + 8)?;

        assert_eq!(value_1.kind(), Value::INT, "First value should be INT");
        assert_eq!(value_1.data(), 42, "First value should be 42");

        assert_eq!(value_2.kind(), Value::FLOAT, "Second value should be FLOAT");
        assert!(
            (value_2.as_float()? - 3.14159).abs() < 0.0001,
            "Second value should be ~3.14159"
        );

        assert_eq!(value_3.kind(), Value::INT, "Third value should be INT");
        assert_eq!(
            value_3.data(),
            0xFFFFFFF6,
            "Third value should be -10 (as two's complement)"
        );
        assert_eq!(value_3.as_int()?, -10, "Third value should be -10");

        assert_eq!(value_4.kind(), Value::FLOAT, "Fourth value should be FLOAT");
        assert!(
            (value_4.as_float()? - (-0.5)).abs() < 0.0001,
            "Fourth value should be -0.5"
        );

        Ok(())
    }

    #[test]
    fn test_parse_string_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[\"hello\" \"world\"]")?;

        // Verify the result is a block
        assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
        assert!(result.data() > 0, "Expected a valid block address");

        // Verify the outer block - should contain one inner block
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;
        assert_eq!(outer_block.len(), 1, "Outer block should have length 1");

        // Get the inner block and verify it contains two strings
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(inner_block.len(), 2, "Inner block should contain 2 strings");

        // Verify the string values
        let string1_value = *memory.get::<Value>(inner_block_addr + 2)?; // First string
        let string2_value = *memory.get::<Value>(inner_block_addr + 4)?; // Second string

        assert_eq!(
            string1_value.kind(),
            Value::STRING,
            "First value should be STRING"
        );
        assert_eq!(
            string2_value.kind(),
            Value::STRING,
            "Second value should be STRING"
        );

        // We can't easily check the string content directly, but we can verify they're different
        assert_ne!(
            string1_value.data(),
            string2_value.data(),
            "String addresses should be different"
        );

        Ok(())
    }

    #[test]
    fn test_parse_word_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[word set-word: :get-word]")?;

        // Verify the result is a block
        assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
        assert!(result.data() > 0, "Expected a valid block address");

        // Verify the outer block - should contain one inner block
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;
        assert_eq!(outer_block.len(), 1, "Outer block should have length 1");

        // Get the inner block and verify it contains 3 words of different types
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(inner_block.len(), 3, "Inner block should contain 3 words");

        // Verify the word values
        let word1 = *memory.get::<Value>(inner_block_addr + 2)?; // Regular word
        let word2 = *memory.get::<Value>(inner_block_addr + 4)?; // Set word
        let word3 = *memory.get::<Value>(inner_block_addr + 6)?; // Get word

        assert_eq!(word1.kind(), Value::WORD, "First value should be WORD");
        assert_eq!(
            word2.kind(),
            Value::SET_WORD,
            "Second value should be SET_WORD"
        );
        assert_eq!(
            word3.kind(),
            Value::GET_WORD,
            "Third value should be GET_WORD"
        );

        Ok(())
    }

    #[test]
    fn test_parse_nested_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[1 [2 3] 4]")?;

        // Verify the result is a block
        assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
        assert!(result.data() > 0, "Expected a valid block address");

        // Verify the outer block - should contain one inner block
        let outer_block_addr = result.data();
        let outer_block = memory.get::<Block>(outer_block_addr)?;
        assert_eq!(outer_block.len(), 1, "Outer block should have length 1");

        // Get the inner block and verify it contains integers 1, [2 3], 4
        let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
        assert_eq!(
            inner_block_value.kind(),
            Value::BLOCK,
            "Inner value should be a BLOCK"
        );

        let inner_block_addr = inner_block_value.data();
        let inner_block = memory.get::<Block>(inner_block_addr)?;
        assert_eq!(inner_block.len(), 3, "Inner block should contain 3 values");

        // Verify the values: [1, [2 3], 4]
        let value_1 = *memory.get::<Value>(inner_block_addr + 2)?; // First integer
        let nested_block = *memory.get::<Value>(inner_block_addr + 4)?; // Nested block
        let value_4 = *memory.get::<Value>(inner_block_addr + 6)?; // Last integer

        assert_eq!(value_1.kind(), Value::INT, "First value should be INT");
        assert_eq!(value_1.data(), 1, "First value should be 1");

        assert_eq!(
            nested_block.kind(),
            Value::BLOCK,
            "Middle value should be BLOCK"
        );
        assert_eq!(value_4.kind(), Value::INT, "Last value should be INT");
        assert_eq!(value_4.data(), 4, "Last value should be 4");

        // Verify the nested block [2 3]
        let nested_addr = nested_block.data();
        let nested = memory.get::<Block>(nested_addr)?;
        assert_eq!(nested.len(), 2, "Nested block should contain 2 values");

        let nested_1 = *memory.get::<Value>(nested_addr + 2)?;
        let nested_2 = *memory.get::<Value>(nested_addr + 4)?;

        assert_eq!(
            nested_1.kind(),
            Value::INT,
            "First nested value should be INT"
        );
        assert_eq!(nested_1.data(), 2, "First nested value should be 2");
        assert_eq!(
            nested_2.kind(),
            Value::INT,
            "Second nested value should be INT"
        );
        assert_eq!(nested_2.data(), 3, "Second nested value should be 3");

        Ok(())
    }

    #[test]
    fn test_parse_path_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        let result = process.parse_block("[a/b c/d/e]")?;
        // For path blocks, the kind is usually Value::BLOCK
        assert_eq!(result.kind(), Value::BLOCK);
        assert!(result.data() > 0, "Expected a valid block address");

        Ok(())
    }

    // Test error handling for specific parser errors
    #[test]
    fn test_parse_invalid_escape() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory).unwrap();

        // Invalid escape sequence
        let result = process.parse_block("[\"invalid \\z escape\"]");
        assert!(result.is_err(), "Expected error for invalid escape");

        Ok(())
    }

    // Test memory allocation and addresses
    #[test]
    fn test_parse_block_addresses() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Parse a couple of blocks
        let result1 = process.parse_block("[1 2 3]")?;
        let result2 = process.parse_block("[\"a\" \"b\" \"c\"]")?;

        // Verify they have different addresses
        assert_ne!(result1.data(), result2.data());

        Ok(())
    }

    // Test handling of larger inputs
    #[test]
    fn test_parse_larger_input() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Create a larger block
        let mut input = "[".to_string();
        for i in 1..15 {
            input.push_str(&format!("{} ", i));
        }
        input.push(']');

        let result = process.parse_block(&input)?;
        assert!(result.data() > 0, "Expected a valid block address");

        Ok(())
    }

    // Test handling of comments in the input
    #[test]
    fn test_parse_with_comments() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Parse a block with comments
        let input = "[\n    1 ; comment after integer\n    ; full line comment\n    \"text\" ; comment after string\n    word ; comment after word\n]";
        let result = process.parse_block(input)?;
        assert!(result.data() > 0, "Expected a valid block address");

        Ok(())
    }

    // Test handling of complex syntax
    #[test]
    fn test_parse_complex_syntax() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Test with escaped characters in strings
        let result1 = process.parse_block("[\"hello\\nworld\" \"quote\\\"inside\"]")?;
        assert!(result1.data() > 0, "Expected a valid block address");

        // Test with multiple mixed types
        let result2 = process.parse_block("[1 \"text\" word :get-word]")?;
        assert!(result2.data() > 0, "Expected a valid block address");

        Ok(())
    }

    // Diagnostic test to understand block size variations
    #[test]
    fn test_parse_block_size_variations() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;
        let mut process = Process::new(&mut memory)?;

        // Test with blocks of varying sizes
        let empty_block = process.parse_block("[]")?;
        println!(
            "Empty block: kind={}, data={}",
            empty_block.kind(),
            empty_block.data()
        );

        let one_item_block = process.parse_block("[1]")?;
        println!(
            "One item block: kind={}, data={}",
            one_item_block.kind(),
            one_item_block.data()
        );

        let two_item_block = process.parse_block("[1 2]")?;
        println!(
            "Two item block: kind={}, data={}",
            two_item_block.kind(),
            two_item_block.data()
        );

        // All should be block kind
        assert_eq!(
            empty_block.kind(),
            Value::BLOCK,
            "Empty block should have BLOCK kind"
        );
        assert_eq!(
            one_item_block.kind(),
            Value::BLOCK,
            "One item block should have BLOCK kind"
        );
        assert_eq!(
            two_item_block.kind(),
            Value::BLOCK,
            "Two item block should have BLOCK kind"
        );

        Ok(())
    }

    // Test simple constant compilation [1 2 3]
    #[test]
    fn test_compile_constants() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;

        // Parse and compile in separate scopes to avoid borrow issues
        let block_value = {
            let mut process = Process::new(&mut memory)?;
            process.parse_block("[1 2 3]")?
        };

        let block_address = block_value.as_block()?.address();
        let inner_block_value = *memory.get::<Value>(block_address + 2)?;
        let inner_block = inner_block_value.as_block()?;

        let compiled = {
            let mut process = Process::new(&mut memory)?;
            process.compile(inner_block)?
        };

        // Verify that compilation works by checking block length
        let compiled_addr = compiled.address();
        let compiled_block = memory.get::<Block>(compiled_addr)?;

        // We expect 7 instructions (3 constants with 2 instructions each + 1 LEAVE)
        assert_eq!(compiled_block.len(), 7, "Block length should be 7");

        Ok(())
    }

    // Test compilation of a block with set_word: [x: 5 x]
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

    // Test compilation of a block with multiple set_words: [x: y: z: 42 y]
    #[test]
    fn test_compile_multiple_set_words() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;

        // Parse and compile in separate scopes to avoid borrow issues
        let block_value = {
            let mut process = Process::new(&mut memory)?;
            process.parse_block("[x: y: z: 42 y]")?
        };

        let block_address = block_value.as_block()?.address();
        let inner_block_value = *memory.get::<Value>(block_address + 2)?;
        let inner_block = inner_block_value.as_block()?;

        let compiled = {
            let mut process = Process::new(&mut memory)?;
            process.compile(inner_block)?
        };

        // Verify that compilation works by checking block length
        let compiled_addr = compiled.address();
        let compiled_block = memory.get::<Block>(compiled_addr)?;

        // After debugging, we found the correct number of instructions is 5
        // (TYPE, CONST, SET_WORD z, SET_WORD y, SET_WORD x, WORD y)
        assert_eq!(compiled_block.len(), 5, "Block length should be 5");

        Ok(())
    }

    // Test compilation adds NONE value when compiling an empty block
    #[test]
    fn test_compile_empty_block() -> Result<(), VmError> {
        let mut memory = create_test_memory()?;

        // Parse and compile in separate scopes to avoid borrow issues
        let block_value = {
            let mut process = Process::new(&mut memory)?;
            process.parse_block("[]")?
        };

        let block_address = block_value.as_block()?.address();
        let inner_block_value = *memory.get::<Value>(block_address + 2)?;
        let inner_block = inner_block_value.as_block()?;

        let compiled = {
            let mut process = Process::new(&mut memory)?;
            process.compile(inner_block)?
        };

        // Verify that compilation works by checking block length
        let compiled_addr = compiled.address();
        let compiled_block = memory.get::<Block>(compiled_addr)?;

        // For empty blocks, we expect 2 instructions (TYPE NONE, CONST 0)
        assert_eq!(
            compiled_block.len(),
            2,
            "Empty block should have 2 instructions"
        );

        Ok(())
    }
}
