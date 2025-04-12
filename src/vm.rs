// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use std::mem::zeroed;

use crate::mem::{
    Address, Block, Memory, MemoryError, NativeFunc, Offset, Series, Type, Value, Word,
};
use crate::parse::{Collector, Parser, ParserError, WordKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    ParserError(#[from] ParserError<MemoryError>),
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
    #[error("Invalid code")]
    InvalidCode,
    #[error("Integer overflow")]
    IntegerOverflow,
}

//

type Op = u8;

pub struct Code;

impl Code {
    // const HALT: Op = 0;
    const CONST: Op = 1;
    const NONE: Op = 2;
    const WORD: Op = 3;
    const SET_WORD: Op = 4;
    const LEAVE: Op = 5;
    pub const ADD: Op = 6;
    // const EITHER: Op = 7;
}

//

#[derive(Debug, Clone, Copy)]
enum Call {
    SetWord(Address),
}

#[derive(Debug, Clone, Copy)]
pub struct Defer {
    call: Call,
    bp: Offset,
    arity: Word,
}

impl Defer {
    fn new(call: Call, bp: Offset, arity: Word) -> Self {
        Defer { call, bp, arity }
    }
}

//

type NativeFn = fn(&mut Process) -> Result<(), VmError>;

pub struct NativeDescriptor {
    name: &'static str,
    description: &'static str,
    func: NativeFn,
    arity: usize,
}

impl NativeDescriptor {
    pub const fn new(
        name: &'static str,
        description: &'static str,
        func: NativeFn,
        arity: usize,
    ) -> Self {
        Self {
            name,
            description,
            func,
            arity,
        }
    }
}

pub struct Vm {
    memory: Memory,
    natives: Vec<NativeFn>,
}

impl Vm {
    pub fn new(memory: Memory) -> Result<Self, MemoryError> {
        let descs = crate::stdlib::NATIVES;
        let mut vm = Self {
            memory,
            natives: Vec::<NativeFn>::with_capacity(descs.len()),
        };
        let natives = vm.memory.alloc::<NativeFunc>(descs.len())?;

        for native in descs {
            let symbol = vm.memory.get_or_add_symbol(native.name)?;
            let description = vm.memory.alloc_string(native.description)?;
            let id = vm.natives.len();
            vm.natives.push(native.func);
            let native = NativeFunc::new(id, native.arity, description);
            let address = vm.memory.push(natives, native)?;
            vm.memory
                .set_word(symbol.address(), Value::native(address))?;
        }

        Ok(vm)
    }

    pub fn parse_block(&mut self, input: &str) -> Result<Value, VmError> {
        let mut collector = ParseCollector::new(&mut self.memory);
        Parser::parse_block(input, &mut collector)?;
        collector.stack.pop().map_err(Into::into)
    }

    pub fn start(&mut self) -> Result<(), MemoryError> {
        let descs = crate::stdlib::NATIVES;
        let natives = self.memory.alloc::<NativeFunc>(descs.len())?;

        for native in descs {
            let symbol = self.memory.get_or_add_symbol(native.name)?;
            let description = self.memory.alloc_string(native.description)?;
            let id = self.natives.len();
            self.natives.push(native.func);
            let native = NativeFunc::new(id, native.arity, description);
            let address = self.memory.push(natives, native)?;
            self.memory
                .set_word(symbol.address(), Value::native(address))?;
        }

        Ok(())
    }
}

//

pub struct ArrayStack<T, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T, const N: usize> ArrayStack<T, N>
where
    T: Copy,
{
    fn new() -> Self {
        Self {
            data: unsafe { zeroed() },
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), MemoryError> {
        self.data
            .get_mut(self.len)
            .map(|slot| {
                *slot = value;
                self.len += 1;
            })
            .ok_or(MemoryError::StackOverflow)
    }

    fn pop(&mut self) -> Result<T, MemoryError> {
        if self.len > 0 {
            self.len -= 1;
            self.data
                .get(self.len)
                .copied()
                .ok_or(MemoryError::StackUnderflow)
        } else {
            Err(MemoryError::StackUnderflow)
        }
    }

    fn extend<const L: usize>(&mut self, values: &[T; L]) -> Result<(), MemoryError> {
        self.data
            .get_mut(self.len..self.len + L)
            .map(|slice| {
                slice.copy_from_slice(values);
                self.len += L;
            })
            .ok_or(MemoryError::StackOverflow)
    }

    fn drop(&mut self) -> Result<(), MemoryError> {
        self.len = self.len.checked_sub(1).ok_or(MemoryError::StackUnderflow)?;
        Ok(())
    }

    fn last(&self) -> Option<&T> {
        self.len.checked_sub(1).and_then(|i| self.data.get(i))
    }

    fn as_slice(&self) -> Result<&[T], MemoryError> {
        self.data.get(..self.len).ok_or(MemoryError::OutOfBounds)
    }

    fn drain(&mut self, pos: usize) -> Result<&[T], MemoryError> {
        let len = self.len;
        self.len = pos;
        self.data.get(pos..len).ok_or(MemoryError::OutOfBounds)
    }

    fn nip_opt(&mut self, n: usize) -> Option<()> {
        if n == 0 {
            return None;
        }
        let last = self.len.checked_sub(1)?;
        let new_last = self.len.checked_sub(n)?;
        let last_value = self.data.get(last).copied()?;
        let dst = self.data.get_mut(new_last)?;
        *dst = last_value;
        self.len = new_last + 1;
        Some(())
    }

    fn nip(&mut self, n: usize) -> Result<(), MemoryError> {
        self.nip_opt(n).ok_or(MemoryError::StackUnderflow)
    }

    pub fn pop_n<const M: usize>(&mut self) -> Result<&[T; M], MemoryError> {
        let new_len = self.len.checked_sub(M).ok_or(MemoryError::StackUnderflow)?;
        let result = self
            .data
            .get(new_len..self.len)
            .ok_or(MemoryError::StackUnderflow)?;
        self.len = new_len;
        result.try_into().map_err(Into::into)
    }
}

pub type ByteCode = ArrayStack<u8, 1024>;

struct InstructionPointer(usize);

impl InstructionPointer {
    fn jmp(&mut self, code: Series<u8>) {
        self.0 = code.address() as usize + std::mem::size_of::<Block>();
    }

    fn read_code(&mut self, memory: &Memory) -> Option<u8> {
        let result = memory.get_u8(self.0);
        self.0 += 1;
        result
    }

    fn read_u8(&mut self, memory: &Memory) -> Result<u8, MemoryError> {
        let result = memory.get_u8(self.0).ok_or(MemoryError::OutOfBounds)?;
        self.0 += 1;
        Ok(result)
    }

    fn read_u32(&mut self, memory: &Memory) -> Result<u32, MemoryError> {
        let result = memory.get_u32_ne(self.0).ok_or(MemoryError::OutOfBounds)?;
        self.0 += 4;
        Ok(result)
    }
}

//

type Stack = ArrayStack<Value, 64>;

pub struct Process<'a> {
    vm: &'a mut Vm,
    ip: InstructionPointer,
    stack: Stack,
}

impl<'a> Process<'a> {
    pub fn new(vm: &'a mut Vm) -> Self {
        Self {
            vm,
            stack: ArrayStack::new(),
            ip: InstructionPointer(0),
        }
    }

    pub fn get_stack_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }

    pub fn compile(&mut self, block: Series<Value>) -> Result<Series<u8>, MemoryError> {
        let mut defer_stack = ArrayStack::<Defer, 64>::new();
        let mut code_stack = ByteCode::new();

        let len = self.vm.memory.len(block)?;
        let mut ip = block.address() + Block::SIZE;
        let end = ip + len * Value::SIZE;
        let mut stack_len = 0;

        while ip < end {
            while let Some(defer) = defer_stack.last() {
                if stack_len == defer.bp + defer.arity {
                    stack_len -= defer.arity;
                    stack_len += 1;
                    match defer.call {
                        Call::SetWord(symbol) => {
                            code_stack.push(Code::SET_WORD)?;
                            code_stack.extend(&u32::to_ne_bytes(symbol))?;
                        }
                    }
                    defer_stack.drop()?;
                } else {
                    break;
                }
            }

            let value = self.vm.memory.get::<Value>(ip)?;
            match value.kind() {
                Value::WORD => {
                    code_stack.push(Code::WORD)?;
                    code_stack.extend(&u32::to_ne_bytes(value.data()))?;
                    stack_len += 1;
                }
                Value::SET_WORD => {
                    let defer = Defer::new(Call::SetWord(value.data()), stack_len, 1);
                    defer_stack.push(defer)?;
                }
                _ => {
                    code_stack.extend(&[Code::CONST, value.kind() as u8])?;
                    code_stack.extend(&u32::to_ne_bytes(value.data()))?;
                    stack_len += 1;
                }
            }
            ip += Value::SIZE;
        }
        // fix stack
        match stack_len {
            0 => code_stack.push(Code::NONE)?,
            1 => {}
            n => code_stack.extend(&[Code::LEAVE, n as u8])?,
        }
        // code_stack.push(Code::HALT)?;

        self.vm.memory.alloc_items(code_stack.as_slice()?)
    }

    pub fn exec(&mut self, code_block: Series<u8>) -> Result<Value, VmError> {
        // let mut ip = InstructionPointer::new(code_block);
        self.ip.jmp(code_block);

        // let code = self.vm.memory.get_bytes(code_block.address())?;
        // let mut reader = CodeReader::new(code);

        while let Some(op) = self.ip.read_code(&self.vm.memory) {
            match op {
                Code::CONST => {
                    let kind = self.ip.read_u8(&self.vm.memory)? as Word;
                    self.stack
                        .push(Value::new(kind, self.ip.read_u32(&self.vm.memory)?))?;
                }
                Code::WORD => {
                    let symbol = self.ip.read_u32(&self.vm.memory)?;
                    let value = self.vm.memory.get_word(symbol)?;
                    self.stack.push(value)?;
                }
                Code::SET_WORD => {
                    let symbol = self.ip.read_u32(&self.vm.memory)?;
                    let value = self
                        .stack
                        .last()
                        .copied()
                        .ok_or(MemoryError::StackUnderflow)?;
                    self.vm.memory.set_word(symbol, value)?;
                }
                Code::LEAVE => {
                    let drop = self.ip.read_u8(&self.vm.memory)? as usize;
                    self.stack.nip(drop)?;
                    break;
                }
                Code::NONE => self.stack.push(Value::new(Value::NONE, 0))?,
                Code::ADD => {
                    let [va, vb] = self.stack.pop_n()?;
                    let a = va.as_int()?;
                    let b = vb.as_int()?;
                    let result = a.checked_add(b).ok_or(VmError::IntegerOverflow)?;
                    self.stack.push(Value::int(result))?;
                }
                _ => {
                    return Err(VmError::InvalidCode);
                }
            }
        }
        self.stack.pop().map_err(Into::into)
    }
}

//

struct ParseCollector<'a> {
    memory: &'a mut Memory,
    stack: ArrayStack<Value, 256>,
    pos_stack: ArrayStack<usize, 256>,
}

impl<'a> ParseCollector<'a> {
    fn new(memory: &'a mut Memory) -> Self {
        Self {
            memory,
            stack: ArrayStack::new(),
            pos_stack: ArrayStack::new(),
        }
    }

    fn begin(&mut self) -> Result<(), MemoryError> {
        self.pos_stack.push(self.stack.len)
    }

    fn end(&mut self, kind: Type) -> Result<(), MemoryError> {
        let pos = self.pos_stack.pop()?;
        let block = self.memory.alloc_items(self.stack.drain(pos)?)?;
        self.stack.push(Value::new(kind, block.address()))
    }
}

impl Collector for ParseCollector<'_> {
    type Error = MemoryError;

    /// Called when a string is parsed
    fn string(&mut self, string: &str) -> Result<(), Self::Error> {
        let string = self.memory.alloc_string(string).map(Value::string)?;
        self.stack.push(string)
    }

    /// Called when a word is parsed
    fn word(&mut self, kind: WordKind, symbol: &str) -> Result<(), Self::Error> {
        let symbol = self.memory.get_or_add_symbol(symbol)?;
        self.stack.push(Value::any_word(kind, symbol))
    }

    /// Called when an integer is parsed
    fn integer(&mut self, value: i32) -> Result<(), Self::Error> {
        self.stack.push(Value::int(value))
    }

    /// Called when a float is parsed
    fn float(&mut self, value: f32) -> Result<(), Self::Error> {
        self.stack.push(Value::float(value))
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

    const TYPE_INT: u8 = Value::INT as u8;

    // Helper function to create a test memory
    fn create_test_vm() -> Result<Vm, MemoryError> {
        Vm::new(Memory::new(65536)?)
    }

    // Test basic block parsing with Process
    #[test]
    fn test_parse_1() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("x: 5 x")?;
        let block = vm.memory.peek_at(result.as_block()?, 0)?;

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
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("")?;

        assert_eq!(result.kind(), Value::BLOCK);
        assert!(result.data() > 0, "Block address should be valid");

        let block = result.as_block()?;
        assert_eq!(vm.memory.len(block)?, 0, "Block should be empty");

        Ok(())
    }

    // Test integer block parsing
    #[test]
    fn test_parse_integer_block() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("1 2 3")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
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
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("5.14 -2.5 0.0")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
        assert_eq!(values.len(), 3, "Block should contain 3 floats");

        // Check types
        assert_eq!(values[0].kind(), Value::FLOAT);
        assert_eq!(values[1].kind(), Value::FLOAT);
        assert_eq!(values[2].kind(), Value::FLOAT);

        // Check values with approximate comparison
        assert!((values[0].as_float()? - 5.14).abs() < 0.0001);
        assert!((values[1].as_float()? - (-2.5)).abs() < 0.0001);
        assert!((values[2].as_float()? - 0.0).abs() < 0.0001);

        Ok(())
    }

    // Test mixed numeric values
    #[test]
    fn test_parse_mixed_numeric_block() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("42 5.14159 -10 -0.5")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
        assert_eq!(values.len(), 4, "Block should contain 4 values");

        assert_eq!(values[0].kind(), Value::INT);
        assert_eq!(values[0].data(), 42);

        assert_eq!(values[1].kind(), Value::FLOAT);
        assert!((values[1].as_float()? - 5.14159).abs() < 0.0001);

        assert_eq!(values[2].kind(), Value::INT);
        assert_eq!(values[2].as_int()?, -10);

        assert_eq!(values[3].kind(), Value::FLOAT);
        assert!((values[3].as_float()? - (-0.5)).abs() < 0.0001);

        Ok(())
    }

    // Test string block parsing
    #[test]
    fn test_parse_string_block() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("\"hello\" \"world\"")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
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
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("word set-word: :get-word")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
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
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("1 [2 3] 4")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
        assert_eq!(values.len(), 3, "Block should contain 3 values");

        // Check outer structure
        assert_eq!(values[0].kind(), Value::INT);
        assert_eq!(values[0].data(), 1);

        assert_eq!(values[1].kind(), Value::BLOCK);

        assert_eq!(values[2].kind(), Value::INT);
        assert_eq!(values[2].data(), 4);

        // Verify nested block
        let nested_block = values[1].as_block()?;
        let nested_values = vm.memory.peek_at(nested_block, 0)?;

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
        let mut vm = create_test_vm()?;
        let result = vm.parse_block("a/b c/d/e")?;

        let values = vm.memory.peek_at(result.as_block()?, 0)?;
        assert_eq!(values.len(), 2, "Block should contain 2 paths");

        // Both should be PATH type
        assert_eq!(values[0].kind(), Value::PATH);
        assert_eq!(values[1].kind(), Value::PATH);

        // First path should have 2 elements, second should have 3
        let path1 = values[0].as_path()?;
        let path2 = values[1].as_path()?;

        assert_eq!(vm.memory.len(path1)?, 2);
        assert_eq!(vm.memory.len(path2)?, 3);

        Ok(())
    }

    // Test error handling
    #[test]
    fn test_parse_errors() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;

        // Invalid escape sequence
        let result = vm.parse_block("\"invalid \\z escape\"");
        assert!(result.is_err(), "Should error on invalid escape sequence");

        Ok(())
    }

    // Test compilation of constants
    #[test]
    fn test_compile_constants() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let block = vm.parse_block("1 2 3")?;

        let mut process = Process::new(&mut vm);
        let code_block = process.compile(block.as_block()?)?;
        let code = process.vm.memory.get_items(code_block)?;

        match code {
            [
                Code::CONST,
                TYPE_INT,
                1,
                0,
                0,
                0,
                Code::CONST,
                TYPE_INT,
                2,
                0,
                0,
                0,
                Code::CONST,
                TYPE_INT,
                3,
                0,
                0,
                0,
                Code::LEAVE,
                3,
            ] => {}
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    // Test compilation with set-word
    #[test]
    fn test_compile_set_word_and_use() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let block = vm.parse_block("x: 5 x")?;

        let mut process = Process::new(&mut vm);
        let code_block = process.compile(block.as_block()?)?;
        let code = process.vm.memory.get_items(code_block)?;

        match code {
            [
                Code::CONST,
                TYPE_INT,
                5,
                0,
                0,
                0,
                Code::SET_WORD,
                x1,
                x2,
                x3,
                x4,
                Code::WORD,
                y1,
                y2,
                y3,
                y4,
                Code::LEAVE,
                2,
            ] => {
                assert_eq!(
                    [x1, x2, x3, x4],
                    [y1, y2, y3, y4],
                    "x should be same symbol"
                )
            }
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    // Test compilation with multiple set-words
    #[test]
    fn test_compile_multiple_set_words() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let block = vm.parse_block("x: y: z: 42 y")?;

        let mut process = Process::new(&mut vm);
        let code_block = process.compile(block.as_block()?)?;
        let code = process.vm.memory.get_items(code_block)?;

        match code {
            [
                Code::CONST,
                TYPE_INT,
                42,
                0,
                0,
                0,
                Code::SET_WORD,
                x1,
                x2,
                x3,
                x4,
                Code::SET_WORD,
                y1,
                y2,
                y3,
                y4,
                Code::SET_WORD,
                z1,
                z2,
                z3,
                z4,
                Code::WORD,
                m1,
                m2,
                m3,
                m4,
                Code::LEAVE,
                2,
            ] => {
                let x = [x1, x2, x3, x4];
                let y = [y1, y2, y3, y4];
                let z = [z1, z2, z3, z4];
                let m = [m1, m2, m3, m4];

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
        let mut vm = create_test_vm()?;
        let block = vm.parse_block("")?;

        let mut process = Process::new(&mut vm);
        let code_block = process.compile(block.as_block()?)?;
        let code = process.vm.memory.get_items(code_block)?;

        match code {
            [Code::NONE] => {}
            _ => panic!("Unexpected code sequence: {:?}", code),
        }

        Ok(())
    }

    #[test]
    fn test_exec_1() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let block = vm.parse_block("1 2 3")?;

        let mut process = Process::new(&mut vm);
        let code_block = process.compile(block.as_block()?)?;

        let result = process.exec(code_block)?;
        assert_eq!(result, Value::int(3), "Expected result to be 3");
        assert_eq!(process.stack.len, 0);

        Ok(())
    }

    #[test]
    fn test_exec_2() -> Result<(), VmError> {
        let mut vm = create_test_vm()?;
        let block = vm.parse_block("x: y: 42 z: 5 y")?;

        let mut process = Process::new(&mut vm);
        let code_block = process.compile(block.as_block()?)?;

        let result = process.exec(code_block)?;
        assert_eq!(result, Value::int(42), "Expected result to be 3");
        assert_eq!(process.stack.len, 0);

        Ok(())
    }
}
