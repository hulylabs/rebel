// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, WordKind};
use std::{marker::PhantomData, mem};
use thiserror::Error;
use xxhash_rust::const_xxh32::xxh32;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("invalid tag")]
    InvalidTag,
    #[error("out of bounds")]
    OutOfBounds,
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),
}

type Word = u32;
type Offset = Word;

pub trait Item: Sized {
    const SIZE: Offset;

    fn load(data: &[u8]) -> Option<Self>;
    fn store(self, data: &mut [u8]) -> Option<()>;
}

struct LenAddress(Offset);

impl LenAddress {
    fn drain(&self, size: Offset, memory: &mut Memory) -> Option<Offset> {
        let len = self.get_len(memory)?;
        let start = len.checked_sub(size)?;
        self.set_len(start, memory)?;
        Some(start)
    }

    fn get_len(&self, memory: &Memory) -> Option<Offset> {
        let address = self.address();
        memory
            .get(address, 4)?
            .try_into()
            .ok()
            .map(u32::from_le_bytes)
    }

    fn set_len(&self, len: Offset, memory: &mut Memory) -> Option<()> {
        let address = self.0;
        let len = len.to_le_bytes();
        memory
            .get_mut(address, 4)
            .map(|slot| slot.copy_from_slice(&len))
    }

    fn address(&self) -> Offset {
        self.0
    }

    fn data_address(&self) -> Offset {
        self.0 + 4
    }
}

#[derive(Debug, Clone, Copy)]
struct CapAddress(Offset);

impl CapAddress {
    fn init(&self, cap: Offset, memory: &mut Memory) -> Option<()> {
        let data_cap = cap.checked_sub(8)?;
        self.set_cap(data_cap, memory)?;
        self.len_address().set_len(0, memory)?;
        Some(())
    }

    fn get_cap(&self, memory: &Memory) -> Option<Word> {
        memory
            .get(self.address(), 4)
            .and_then(|slot| slot.try_into().ok())
            .map(u32::from_le_bytes)
    }

    fn set_cap(&self, cap: Offset, memory: &mut Memory) -> Option<()> {
        memory
            .get_mut(self.address(), 4)
            .map(|slot| slot.copy_from_slice(&u32::to_le_bytes(cap)))
    }

    fn reserve_slot(&self, size: Offset, memory: &mut Memory) -> Option<Offset> {
        let address = self.address();
        let header = memory.get_mut(address, 8)?;

        let cap = u32::from_le_bytes(header[0..4].try_into().ok()?);
        let len = u32::from_le_bytes(header[4..8].try_into().ok()?);

        let new_len = len + size as Offset;
        if new_len <= cap {
            header[4..8].copy_from_slice(&new_len.to_le_bytes());
            Some(self.data_address() + len)
        } else {
            None
        }
    }

    fn reserve_len(&self, size: Offset, memory: &mut Memory) -> Option<LenAddress> {
        self.reserve_slot(size + 4, memory)
            .map(LenAddress)
            .inspect(|x| {
                x.set_len(size, memory).unwrap();
            })
    }

    fn alloc_len(&self, data: &[u8], memory: &mut Memory) -> Option<LenAddress> {
        let len = data.len() as Offset;
        self.reserve_len(len, memory).inspect(|addr| {
            memory
                .get_mut(addr.data_address(), len)
                .unwrap()
                .copy_from_slice(data);
        })
    }

    fn alloc_cap(&self, size: Offset, memory: &mut Memory) -> Option<CapAddress> {
        let to_allocate = size + 8;
        let cap = self.reserve_slot(to_allocate, memory).map(CapAddress)?;
        cap.init(to_allocate, memory)?;
        Some(cap)
    }

    fn len_address(&self) -> LenAddress {
        LenAddress(self.0 + 4)
    }

    fn address(&self) -> Offset {
        self.0
    }

    fn data_address(&self) -> Offset {
        self.0 + 8
    }
}

pub struct Stack<I>(CapAddress, PhantomData<I>);

impl<I> Stack<I>
where
    I: Item,
{
    fn new(addr: CapAddress) -> Self {
        Self(addr, PhantomData)
    }

    pub fn len(&self, memory: &Memory) -> Option<Offset> {
        self.0.len_address().get_len(memory)
    }

    pub fn peek(&self, memory: &Memory) -> Option<I> {
        let len = self.0.len_address().get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let data = self.0.data_address();
        memory.get(data + start, I::SIZE).and_then(I::load)
    }

    pub fn push(&self, item: I, memory: &mut Memory) -> Option<()> {
        self.0
            .reserve_slot(I::SIZE, memory)
            .and_then(|slot| item.store(memory.get_mut(slot, I::SIZE)?))
    }

    pub fn pop(&self, memory: &mut Memory) -> Option<I> {
        self.0
            .len_address()
            .drain(I::SIZE, memory)
            .and_then(|offset| memory.get(offset, I::SIZE))
            .and_then(I::load)
    }

    fn drain(&self, to: CapAddress, items: Word, memory: &mut Memory) -> Option<Block<I>> {
        memory
            .cut_and_paste(to, self.0.len_address(), items * I::SIZE)
            .map(Block::new)
    }

    // fn drain_all(&self, to: CapAddress, memory: &mut Memory) -> Option<Block<I>> {
    //     self.drain(to, self.len(memory)?, memory)
    // }
}

pub struct Block<I>(LenAddress, PhantomData<I>);

impl<I> Block<I>
where
    I: Item,
{
    fn new(addr: LenAddress) -> Self {
        Self(addr, PhantomData)
    }

    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0.get_len(memory).map(|x| x / I::SIZE)
    }

    pub fn get(&self, index: Word, memory: &Memory) -> Option<I> {
        let len = self.len(memory)?;
        let item_start = index * I::SIZE;
        let item_end = (index + 1) * I::SIZE;
        if item_end <= len {
            let data = self.0.data_address();
            memory.get(data + item_start, I::SIZE).and_then(I::load)
        } else {
            None
        }
    }
}

pub struct Str(LenAddress);

impl Str {
    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0.get_len(memory)
    }

    pub fn as_bytes<'a>(&self, memory: &'a Memory) -> Option<&'a [u8]> {
        let len = self.len(memory)?;
        memory.get(self.0.data_address(), len)
    }
}

pub struct Arena(CapAddress);

impl Arena {
    // fn new(addr: CapAddress) -> Self {
    //     Self(addr)
    // }

    pub fn alloc_stack<I: Item>(&self, items: Word, memory: &mut Memory) -> Option<Stack<I>> {
        self.0.alloc_cap(items * I::SIZE, memory).map(Stack::new)
    }

    pub fn alloc_block<I: Item>(&self, memory: &mut Memory, data: &[u8]) -> Option<Block<I>> {
        self.0.alloc_len(data, memory).map(Block::new)
    }

    pub fn alloc_string(&self, memory: &mut Memory, string: &str) -> Option<Str> {
        self.0.alloc_len(string.as_bytes(), memory).map(Str)
    }
}

pub struct SymbolTable(CapAddress);

impl SymbolTable {
    const HASH_SEED: u32 = 0xC0FFEE;

    fn get_or_insert_symbol(
        &self,
        symbol: &str,
        heap: Arena,
        memory: &mut Memory,
    ) -> Option<LenAddress> {
        let cap_bytes = self.0.get_cap(memory)?;
        let len_address = self.0.len_address();
        let count = len_address.get_len(memory)?;

        let bytes = symbol.as_bytes();
        let hash = xxh32(bytes, Self::HASH_SEED);

        let cap = (cap_bytes - 8) / 4;

        let start = hash % cap;
        let mut index = start;
        loop {
            let entry_addr = self.0.data_address() + index * 4;
            let entry = memory.read_u32(entry_addr)?;

            if entry == 0 {
                let str = heap.alloc_string(memory, symbol)?;
                let address = str.0;
                memory.write_u32(entry_addr, address.0)?;
                len_address.set_len(count + 1, memory)?;
                return Some(address);
            }

            let stored = Str(LenAddress(entry));
            if stored.as_bytes(memory)? == bytes {
                return Some(stored.0);
            }

            index = (index + 1) % cap;
            if index == start {
                return None;
            }
        }
    }
}

pub struct Memory<'a> {
    memory: &'a mut [u8],
}

impl<'a> Memory<'a> {
    const LAYOUT_SYMBOL_TABLE: u32 = 0;
    const LAYOUT_PARSE_STACK: u32 = 1;
    const LAYOUT_PARSE_BASE: u32 = 2;
    const LAYOUT_HEAP: u32 = 3;

    const LAYOUT_REGIONS: u32 = 4;

    pub fn init(memory: &'a mut [u8], sizes: [Offset; 4]) -> Self {
        let mut addr: u32 = 0;
        for &size in sizes.iter() {
            let cap = CapAddress(addr);
            cap.set_cap(size, &mut Self { memory });
            addr += size;
        }

        Self { memory }
    }

    fn read_u32(&self, offset: Offset) -> Option<u32> {
        self.get(offset, 4)?.try_into().ok().map(u32::from_le_bytes)
    }

    fn write_u32(&mut self, offset: Offset, value: u32) -> Option<()> {
        let bytes = value.to_le_bytes();
        self.get_mut(offset, 4)?.copy_from_slice(&bytes);
        Some(())
    }

    fn copy(&mut self, dst: Offset, src: Offset, size: Offset) -> Option<()> {
        let size = size as usize;
        let dst = dst as usize;
        if dst + size > self.memory.len() {
            return None;
        }
        let src = src as usize;
        if src + size > self.memory.len() {
            return None;
        }
        for i in 0..size {
            self.memory[dst + i] = self.memory[src + i];
        }
        Some(())
    }

    fn cut_and_paste(
        &mut self,
        to: CapAddress,
        from: LenAddress,
        size: Offset,
    ) -> Option<LenAddress> {
        let src = from.drain(size, self)?;
        let dst = to.reserve_len(size, self)?;
        self.copy(dst.data_address(), src, size)?;
        Some(dst)
    }

    fn get(&self, start: Offset, len: Offset) -> Option<&[u8]> {
        let start = start as usize;
        let end = start + len as usize;
        self.memory.get(start..end)
    }

    fn get_mut(&mut self, start: Offset, len: Offset) -> Option<&mut [u8]> {
        let start = start as usize;
        let end = start + len as usize;
        self.memory.get_mut(start..end)
    }
}

//

impl Item for u8 {
    const SIZE: Offset = 1;

    fn load(data: &[u8]) -> Option<Self> {
        data.get(0).copied()
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        data.get_mut(0).map(|slot| *slot = self)
    }
}

impl Item for Word {
    const SIZE: Offset = 4;

    fn load(data: &[u8]) -> Option<Self> {
        let bytes = data.try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        let bytes = self.to_le_bytes();
        data.copy_from_slice(&bytes);
        Some(())
    }
}

//

type Tag = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemValue(Word, Tag);

impl MemValue {
    const TAG_NONE: u8 = 0;
    const TAG_INT: u8 = 1;
    const TAG_BOOL: u8 = 2;
    const TAG_BLOCK: u8 = 3;
    const TAG_CONTEXT: u8 = 4;
    const TAG_PATH: u8 = 5;
    const TAG_STRING: u8 = 6;
    const TAG_WORD: u8 = 7;
    const TAG_SET_WORD: u8 = 8;
    const TAG_GET_WORD: u8 = 9;

    pub fn string(value: Str) -> Self {
        Self(value.0.0, Self::TAG_STRING)
    }

    pub fn int(value: i32) -> Self {
        Self(value as Word, Self::TAG_INT)
    }

    pub fn block(value: Block<MemValue>) -> Self {
        Self(value.0.0, Self::TAG_BLOCK)
    }

    pub fn path(value: Block<MemValue>) -> Self {
        Self(value.0.0, Self::TAG_PATH)
    }
}

impl Item for MemValue {
    const SIZE: Offset = 8;

    fn load(data: &[u8]) -> Option<Self> {
        let addr = data.get(..4)?;
        let addr = u32::from_le_bytes(addr.try_into().ok()?);
        let tag = data.get(4).copied()?;
        Some(Self(addr, tag))
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        let MemValue(addr, tag) = self;
        let addr_bytes = addr.to_le_bytes();
        if data.len() < 5 {
            None
        } else {
            for i in 0..4 {
                data[i] = addr_bytes[i];
            }
            data[4] = tag;
            Some(())
        }
    }
}

// P A R S E  C O L L E C T O R

struct ParseCollector<'a> {
    memory: &'a mut Memory<'a>,
    heap: Arena,
    parse: Stack<MemValue>,
    base: Stack<Word>,
}

impl<'a> ParseCollector<'a> {
    fn new(
        memory: &'a mut Memory<'a>,
        heap: Arena,
        parse: Stack<MemValue>,
        base: Stack<Word>,
    ) -> Self {
        Self {
            memory,
            heap,
            parse,
            base,
        }
    }

    fn begin(&mut self) -> Option<()> {
        let len = self.parse.len(self.memory)? as Word;
        self.base.push(len, self.memory)
    }

    fn end(&mut self) -> Option<Block<MemValue>> {
        self.parse
            .drain(self.heap.0, self.base.pop(self.memory)?, self.memory)
    }
}

impl Collector for ParseCollector<'_> {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Option<()> {
        let string = self
            .heap
            .alloc_string(self.memory, string)
            .map(MemValue::string)?;
        self.parse.push(string, self.memory)
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Option<()> {
        Some(())
        // self.module.get_or_insert_symbol(word).and_then(|id| {
        //     let value = match kind {
        //         WordKind::Word => VmValue::Word(id),
        //         WordKind::SetWord => VmValue::SetWord(id),
        //         WordKind::GetWord => VmValue::GetWord(id),
        //     };
        //     self.parse.push(value.vm_repr())
        // })
    }

    fn integer(&mut self, value: i32) -> Option<()> {
        self.parse.push(MemValue::int(value), self.memory)
    }

    fn begin_block(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_block(&mut self) -> Option<()> {
        let block = self.end().map(MemValue::block)?;
        self.parse.push(block, self.memory)
    }

    fn begin_path(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_path(&mut self) -> Option<()> {
        let block = self.end().map(MemValue::path)?;
        self.parse.push(block, self.memory)
    }
}

//

pub fn push(memory: &mut Memory, stack: &mut Stack<u8>, item: u8) -> Option<()> {
    stack.push(item, memory)
}

pub fn peek(memory: &Memory, stack: &Stack<u8>) -> Option<u8> {
    stack.peek(memory)
}

pub fn pop(memory: &mut Memory, stack: &mut Stack<u8>) -> Option<u8> {
    stack.pop(memory)
}

pub fn get(memory: &Memory, block: &Block<u8>, index: Offset) -> Option<u8> {
    block.get(index, memory)
}

pub fn parse_block<'a>(
    memory: &'a mut Memory<'a>,
    heap: Arena,
    input: &str,
) -> Option<Stack<MemValue>> {
    // let mut parser_memory = Memory::new(Box::new([0; 1024]).as_mut());

    // let parse = heap.alloc_stack::<MemValue>(100, memory)?;
    // let base = heap.alloc_stack::<Word>(20, memory)?;

    let parse = Stack::<MemValue>(CapAddress(0), PhantomData);
    let base = Stack::<Word>(CapAddress(1024), PhantomData);

    let mut collector = ParseCollector::new(memory, heap, parse, base);
    crate::parse::Parser::parse(input, &mut collector).ok()?;

    // heap.alloc_block::<MemValue>(memory, &[0; 8])?;

    Some(collector.parse)
}
