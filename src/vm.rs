// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Addr, Block, Domain, Memory, MemoryError, VmValue, Word};

#[derive(Debug, Clone, Copy)]
enum OpKind {
    None,
    SetWord,
}

#[derive(Debug, Clone, Copy)]
struct Op {
    kind: OpKind,
    bp: Word,
    arity: Word,
}

impl Default for Op {
    fn default() -> Self {
        Self {
            kind: OpKind::None,
            bp: 0,
            arity: 0,
        }
    }
}

pub struct Process {
    block: Addr<Block<VmValue>>,
    ip: Word,

    op_domain: Domain<Op>,

    stack: Block<VmValue>,
    op_stack: Block<Op>,
}

impl Process {
    pub fn new(memory: &mut Memory, stack_size: Word) -> Result<Self, MemoryError> {
        let values = memory.alloc_values(stack_size)?;
        let stack = Block::new(stack_size, stack_size, values);

        let mut op_domain = Domain::<Op>::new(256);
        let ops = op_domain.alloc(256)?;
        let op_stack = Block::new(256, 256, ops);

        Ok(Self {
            block: Addr::new(0),
            ip: 0,
            op_domain,
            stack,
            op_stack,
        })
    }

    fn next_op(&self) -> Result<(), MemoryError> {
        loop {
            // Check pending operations
            if let Some(op) = self.op_stack.peek() {
                let sp = self.stack.len();
                if sp == bp + arity {
                    let [op, word, _, _] = self.op_stack.pop()?;
                    return Ok((op, word));
                }
            }
        }
    }
}
