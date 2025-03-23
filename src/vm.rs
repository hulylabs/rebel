// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Block, MemValue, Memory, Offset, Word};

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
//     block: Block<MemValue>,
//     ip: Word,
// }

pub struct Process(Block<MemValue>);

impl Process {
    const BLOCK: Offset = 0;
    const IP: Offset = 1;

    pub fn alloc(memory: &mut Memory) -> Option<Process> {
        let block = MemValue::none();
        let ip = MemValue::none();

        Some(Process(
            memory.get_heap()?.alloc_block(&[block, ip], memory)?,
        ))
    }

    pub fn get_block(&self, memory: &Memory) -> Option<Block<MemValue>> {
        None
    }
}
