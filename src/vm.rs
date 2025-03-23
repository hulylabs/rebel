// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Block, Memory, Offset, VmValue};

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

pub struct Process(Block<VmValue>);

impl Process {
    const BLOCK: Offset = 0;
    const IP: Offset = 1;

    pub fn alloc(memory: &mut Memory) -> Option<Process> {
        let block = VmValue::None;
        let ip = VmValue::None;

        Some(Process(
            memory.get_heap()?.alloc_block(&[block, ip], memory)?,
        ))
    }

    pub fn get_block(&self, _memory: &Memory) -> Option<Block<VmValue>> {
        None
    }
}
