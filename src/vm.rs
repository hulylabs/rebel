// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Block, CapAddress, LenAddress, MemValue, Memory, Offset, Stack, Word};

pub struct Vm<'a> {
    memory: &'a mut Memory<'a>,
}

impl<'a> Vm<'a> {
    pub fn new(memory: &'a mut Memory<'a>) -> Vm<'a> {
        Vm { memory }
    }
}
// pub struct Process<'a> {
//     vm: &'a mut Vm<'a>,
//     block: Block<VmValue>,
//     ip: Word,
// }

pub struct Process(Block<Word>); // struct

impl Process {
    const BLOCK: Offset = 0;
    const IP: Offset = 1;
    const STACK: Offset = 2;
    const OP_STACK: Offset = 3;

    pub fn alloc(memory: &mut Memory) -> Option<Process> {
        let stack = memory.get_heap()?.alloc_stack::<MemValue>(memory, 256)?;
        let op_stack = memory.get_heap()?.alloc_stack::<Word>(memory, 256)?;
        Some(Process(memory.get_heap()?.alloc_block(
            &[0, 0, stack.address(), op_stack.address()],
            memory,
        )?))
    }

    pub fn get_current_block(&self, memory: &Memory) -> Option<Block<MemValue>> {
        self.0
            .get(Self::BLOCK, memory)
            .map(LenAddress)
            .map(Block::new)
    }

    pub fn set_current_block(&mut self, block: Block<Word>, memory: &mut Memory) {
        self.0.set(Self::BLOCK, block.address(), memory);
    }

    pub fn get_ip(&self, memory: &Memory) -> Option<Word> {
        self.0.get(Self::IP, memory)
    }

    pub fn set_ip(&mut self, ip: Word, memory: &mut Memory) {
        self.0.set(Self::IP, ip, memory);
    }

    pub fn get_stack(&self, memory: &Memory) -> Option<Stack<MemValue>> {
        self.0
            .get(Self::STACK, memory)
            .map(CapAddress)
            .map(Stack::new)
    }

    pub fn get_op_stack(&self, memory: &Memory) -> Option<Stack<Word>> {
        self.0
            .get(Self::OP_STACK, memory)
            .map(CapAddress)
            .map(Stack::new)
    }

    //
}
