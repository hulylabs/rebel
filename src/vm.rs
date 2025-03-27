// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{
    Addr, AnyBlock, Block, BlockStorage, Domain, Memory, MemoryError, Symbol, VmValue, Word,
};

#[derive(Debug, Clone, Copy)]
enum OpKind {
    Halt,
    SetWord(Symbol),
}

#[derive(Debug, Clone, Copy)]
struct Op {
    kind: OpKind,
    bp: Word,
    arity: Word,
}

impl Op {
    fn new(kind: OpKind, bp: Word, arity: Word) -> Self {
        Self { kind, bp, arity }
    }
}

impl Default for Op {
    fn default() -> Self {
        Self {
            kind: OpKind::Halt,
            bp: 0,
            arity: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Process {
    block: Addr<Block<VmValue>>,
    ip: Word,
    stack: Addr<Block<VmValue>>,
    op_stack: Addr<Block<Op>>,
}

// You can add process-specific methods if needed
impl Process {
    fn resolve(&mut self, value: &VmValue, vm: &Vm) -> Result<VmValue, MemoryError> {
        match value {
            VmValue::Word(symbol) => vm.memory.get_value(*symbol),
            _ => Ok(*value),
        }
    }

    fn do_op(&mut self, op: Op, vm: &mut Vm) -> Result<(), MemoryError> {
        match op.kind {
            OpKind::Halt => {
                // Halt the process
                self.block = Addr::new(0);
                self.ip = 0;
                self.stack = Addr::new(0);
                self.op_stack = Addr::new(0);
            }
            OpKind::SetWord(symbol) => {
                let value = self.stack.pop(&mut vm.memory)?;
                vm.memory.set_value(symbol, value)?;
            }
        }
        Ok(())
    }

    fn next_op(&mut self, vm: &mut Vm) -> Result<OpKind, MemoryError> {
        loop {
            // Check pending operations
            if let Some(&op) = self.op_stack.peek(vm)? {
                let sp = self.stack.len(&vm.memory)?;
                if sp == op.bp + op.arity {
                    self.op_stack.drop(vm)?;
                    return Ok(op.kind);
                }
            }

            // read next value
            if let Some(val) = self.block.peek_at(self.ip, &vm.memory)? {
                self.ip += 1;
                let value = self.resolve(val, vm)?;
                match value {
                    VmValue::SetWord(symbol) => self.op_stack.push(
                        Op::new(OpKind::SetWord(symbol), self.stack.len(&vm.memory)?, 1),
                        vm,
                    )?,
                    _ => self.stack.push(value, &mut vm.memory)?,
                }
            }
        }
    }
}

impl Default for Process {
    fn default() -> Self {
        Self {
            block: Addr::new(0),
            ip: 0,
            stack: Addr::new(0),
            op_stack: Addr::new(0),
        }
    }
}

//

pub struct Vm {
    memory: Memory,
    ops: Domain<Op>,
    proc_domain: Domain<Process>,
    processes: Block<Process>,
}

impl Vm {
    const MAX_PROCESSES: Word = 256;

    pub fn new(memory: Memory) -> Result<Self, MemoryError> {
        let mut proc_domain = Domain::<Process>::new(Self::MAX_PROCESSES as usize);
        let proc_values = proc_domain.alloc(Self::MAX_PROCESSES)?;
        let processes = Block::new(Self::MAX_PROCESSES, Self::MAX_PROCESSES, proc_values);
        Ok(Self {
            memory,
            proc_domain,
            ops: Domain::<Op>::new(0x10000),
            processes,
        })
    }

    // pub fn spawn(&mut self, stack_size: Word) -> Result<ProcessId, MemoryError> {
    //     let values = self.memory.alloc_values(stack_size)?;
    //     let stack = Block::new(stack_size, stack_size, values);

    //     let ops = self.ops.alloc(256)?;
    //     let op_stack = Block::new(256, 256, ops);

    //     let process = Process {
    //         block: Addr::new(0),
    //         ip: 0,
    //         stack,
    //         op_stack,
    //     };

    //     self.processes.push(process);
    //     Ok(self.processes.len() as Word - 1) // Return the process ID (index)
    // }

    // Execute the next operation for a specific process
    // fn execute_next_op(&mut self, pid: ProcessId) -> Result<OpKind, MemoryError> {
    //     if let Some(process) = self.processes.get_mut(pid) {
    //         return Self::process_next_op(process, &mut self.ops);
    //     }
    //     Ok(OpKind::None) // Process not found
    // }
}

//

impl<'a> BlockStorage<'a, Op> for Vm {
    fn access_block(
        &self,
        addr: Addr<Block<Op>>,
    ) -> Result<(&Block<Op>, &Domain<Op>), MemoryError> {
        let typeless = self.memory.blocks.get_item(Addr::new(addr.0))?;
        let ptr = typeless as *const AnyBlock;
        let block = unsafe { &*ptr.cast::<Block<Op>>() };
        Ok((block, &self.ops))
    }

    fn access_block_mut(
        &mut self,
        addr: Addr<Block<Op>>,
    ) -> Result<(&mut Block<Op>, &mut Domain<Op>), MemoryError> {
        let typeless = self.memory.blocks.get_item_mut(Addr::new(addr.0))?;
        let ptr = typeless as *mut AnyBlock;
        let block = unsafe { &mut *ptr.cast::<Block<Op>>() };
        Ok((block, &mut self.ops))
    }
}
