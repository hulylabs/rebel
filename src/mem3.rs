// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, WordKind};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {}

pub type Word = u32;

#[derive(Debug, Clone, Copy)]
pub struct Addr<T>(Word, PhantomData<T>);

impl<T> Addr<T>
where
    T: Default + Copy,
{
    pub fn new(address: Word) -> Self {
        Self(address, PhantomData)
    }

    pub fn address(self, cap: Word) -> Option<usize> {
        if self.0 >= cap {
            None
        } else {
            Some(self.0 as usize)
        }
    }

    pub fn range(self, len: Word, cap: Word) -> Option<Range<usize>> {
        let start = self.0;
        let end = start + len;
        if end > cap {
            None
        } else {
            Some(start as usize..end as usize)
        }
    }

    pub fn prev(self, n: Word) -> Option<Self> {
        self.0.checked_sub(n).map(Self::new)
    }

    pub fn next(self, n: Word) -> Option<Self> {
        self.0.checked_add(n).map(Self::new)
    }

    pub fn capped_next(self, n: Word, cap: Word) -> Option<Self> {
        self.next(n).and_then(|next| next.verify(cap))
    }

    pub fn verify(self, cap: Word) -> Option<Self> {
        if self.0 < cap { Some(self) } else { None }
    }

    // pub fn deref<'a>(self, domain: &'a Domain<T>) -> Option<T> {
    //     domain.get_item(self)
    // }
}

#[derive(Debug, Clone, Copy)]
pub enum VmValue {
    None,
    Int(i32),
    Block(Addr<Block<VmValue>>),
    String(Addr<Block<u8>>),
    Word(Addr<Block<u8>>),
    SetWord(Addr<Block<u8>>),
    GetWord(Addr<Block<u8>>),
    Path(Addr<Block<VmValue>>),
}

impl Default for VmValue {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Block<T> {
    cap: Word,
    len: Word,
    data: Addr<T>,
}

impl<T> Block<T>
where
    T: Default + Copy,
{
    pub fn len(&self) -> Word {
        self.len
    }

    pub fn get_item<'a>(&self, index: Word, domain: &'a Domain<T>) -> Option<&'a T> {
        domain.get_item(self.data.capped_next(index, self.len)?)
    }

    pub fn push(&mut self, item: T, domain: &mut Domain<T>) -> Option<()> {
        domain
            .get_item_mut(self.data.capped_next(self.len, self.cap)?)
            .map(|slot| {
                *slot = item;
            })?;
        self.len += 1;
        Some(())
    }

    pub fn push_all(&mut self, items: &[T], domain: &mut Domain<T>) -> Option<()> {
        let addr = self.data.capped_next(self.len, self.cap)?;
        let len = items.len() as Word;
        domain.get_mut(addr, len).map(|slot| {
            slot.copy_from_slice(items);
        })?;
        self.len += len;
        Some(())
    }

    pub fn pop_all<'a>(&mut self, offset: Word, domain: &'a mut Domain<T>) -> Option<&'a [T]> {
        let items = self.len.checked_sub(offset)?;
        domain.get(self.data.capped_next(offset, self.cap)?, items)
    }

    pub fn move_to(&mut self, dest: &Block<T>, items: Word, domain: &mut Domain<T>) -> Option<()> {
        let from_new_len = self.len.checked_sub(items)?;
        let from = self.data.capped_next(from_new_len, self.cap)?;
        let to = dest.data.capped_next(dest.len, dest.cap)?;

        domain.move_items(from, to, items)?;
        self.len = from_new_len;
        Some(())
    }

    pub fn pop(&mut self, domain: &mut Domain<T>) -> Option<T> {
        self.len = self.len.checked_sub(1)?;
        domain
            .get_item(self.data.capped_next(self.len, self.cap)?)
            .copied()
    }
}

impl<T> Default for Block<T>
where
    T: Default + Copy,
{
    fn default() -> Self {
        Self {
            cap: 0,
            len: 0,
            data: Addr::new(0),
        }
    }
}

pub struct Domain<T> {
    items: Box<[T]>,
    len: Word,
}

impl<T> Domain<T>
where
    T: Default + Copy,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            items: vec![T::default(); capacity].into_boxed_slice(),
            len: 0,
        }
    }

    pub fn get_item(&self, addr: Addr<T>) -> Option<&T> {
        self.items.get(addr.address(self.len)?)
    }

    pub fn get(&self, addr: Addr<T>, len: Word) -> Option<&[T]> {
        self.items.get(addr.range(len, self.len)?)
    }

    pub fn get_item_mut(&mut self, addr: Addr<T>) -> Option<&mut T> {
        self.items.get_mut(addr.0 as usize)
    }

    pub fn get_mut(&mut self, addr: Addr<T>, len: Word) -> Option<&mut [T]> {
        self.items.get_mut(addr.range(len, self.len)?)
    }

    pub fn push_all(&mut self, items: &[T]) -> Option<Addr<T>> {
        let addr = self.len;
        let begin = addr as usize;
        let end = begin + items.len();
        self.items.get_mut(begin..end).map(|slot| {
            slot.copy_from_slice(items);
        })?;
        self.len = end as Word;
        Some(Addr::new(addr))
    }

    pub fn push(&mut self, item: T) -> Option<Addr<T>> {
        let addr = self.len;
        self.items.get_mut(addr as usize).map(|slot| {
            *slot = item;
        })?;
        self.len += 1;
        Some(Addr::new(addr))
    }

    pub fn alloc(&mut self, items: Word) -> Option<Addr<T>> {
        let addr = self.len;
        let new_addr = addr + items;
        if new_addr > self.items.len() as Word {
            None
        } else {
            self.len = new_addr;
            Some(Addr::new(addr))
        }
    }

    pub fn move_items(&mut self, from: Addr<T>, to: Addr<T>, items: Word) -> Option<()> {
        let from = from.address(self.len)?;
        let to = to.address(self.len)?;
        let items = items as usize;

        if from + items > self.items.len() || to + items > self.items.len() {
            return None;
        }

        for i in 0..items {
            self.items[to + i] = self.items[from + i];
        }

        Some(())
    }
}

trait DomainProvider<'a, T> {
    fn domain(&'a self) -> &'a Domain<T>;
}

pub struct Memory {
    values: Domain<VmValue>,
    blocks: Domain<Block<VmValue>>,
    strings: Domain<Block<u8>>,
    bytes: Domain<u8>,
    words: Domain<Word>,
    //
    symbols: HashMap<SmolStr, Addr<Block<u8>>>,
    stack: Block<VmValue>,
    op_stack: Block<Word>,
}

impl Memory {
    fn new() -> Self {
        Self {
            bytes: Domain::new(0x10000),
            words: Domain::new(0x10000),
            values: Domain::new(0x10000),
            blocks: Domain::new(1024),
            strings: Domain::new(1024),
            //
            symbols: HashMap::new(),
            stack: Block::default(),
            op_stack: Block::default(),
        }
    }

    fn init(&mut self) -> Option<()> {
        let space = self.values.alloc(256)?;
        self.stack = Block {
            cap: 256,
            len: 0,
            data: space,
        };
        Some(())
    }

    pub fn alloc_empty_block(&mut self, cap: Word) -> Option<Addr<Block<VmValue>>> {
        self.blocks.push(Block {
            cap,
            len: 0,
            data: Addr(0, std::marker::PhantomData),
        })
    }

    pub fn alloc_block(&mut self, items: &[VmValue]) -> Option<Addr<Block<VmValue>>> {
        let data = self.values.push_all(items)?;
        let len = items.len() as Word;
        self.blocks.push(Block {
            cap: len,
            len,
            data,
        })
    }

    pub fn alloc_string(&mut self, s: &str) -> Option<Addr<Block<u8>>> {
        let bytes = s.as_bytes();
        let len = bytes.len() as Word;
        self.strings.push(Block {
            cap: len,
            len,
            data: self.bytes.push_all(bytes)?,
        })
    }

    pub fn get_symbol(&mut self, string: &str) -> Option<Addr<Block<u8>>> {
        let symbol = self.symbols.get(string).copied();
        if symbol.is_none() {
            let new_symbol = self.alloc_string(string)?;
            self.symbols.insert(string.into(), new_symbol);
            Some(new_symbol)
        } else {
            symbol
        }
    }

    // P A R S E  H E L P E R S

    pub fn begin(&mut self) -> Option<()> {
        self.op_stack.push(self.stack.len(), &mut self.words)
    }

    pub fn end(&mut self) -> Option<Addr<Block<VmValue>>> {
        let offset = { self.op_stack.pop(&mut self.words)? };
        let items = self.stack.len().checked_sub(offset)?;
        let block = self.alloc_empty_block(items)?;
        let to = self.blocks.get_item(block)?;
        self.stack.move_to(to, items, &mut self.values)?;
        Some(block)
    }
}

impl<'a> DomainProvider<'a, Block<VmValue>> for Memory {
    fn domain(&'a self) -> &'a Domain<Block<VmValue>> {
        &self.blocks
    }
}

// P A R S E  C O L L E C T O R

impl Collector for Memory {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Option<()> {
        let string = VmValue::String(self.alloc_string(string)?);
        self.stack.push(string, &mut self.values)
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Option<()> {
        let symbol = self.get_symbol(word)?;
        let value = match kind {
            WordKind::Word => VmValue::Word(symbol),
            WordKind::SetWord => VmValue::SetWord(symbol),
            WordKind::GetWord => VmValue::GetWord(symbol),
        };
        self.stack.push(value, &mut self.values)
    }

    fn integer(&mut self, value: i32) -> Option<()> {
        self.stack.push(VmValue::Int(value), &mut self.values)
    }

    fn begin_block(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_block(&mut self) -> Option<()> {
        let block = self.end().map(VmValue::Block)?;
        self.stack.push(block, &mut self.values)
    }

    fn begin_path(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_path(&mut self) -> Option<()> {
        let block = self.end().map(VmValue::Path)?;
        self.stack.push(block, &mut self.values)
    }
}

//

pub fn alloc_block_3(memory: &mut Memory, cap: Word) -> Option<Addr<Block<VmValue>>> {
    memory.alloc_empty_block(cap)
}

pub fn alloc_string_3(memory: &mut Memory, string: &str) -> Option<Addr<Block<u8>>> {
    memory.alloc_string(string)
}

pub fn parse_block_3(memory: &mut Memory, input: &str) -> Option<()> {
    crate::parse::Parser::parse(input, memory).ok()
}

pub fn push_3(memory: &mut Memory, block: Addr<Block<VmValue>>, value: VmValue) -> Option<()> {
    memory
        .blocks
        .get_item_mut(block)?
        .push(value, &mut memory.values)
}
