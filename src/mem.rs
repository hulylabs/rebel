// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{self, Collector, WordKind};
use std::marker::PhantomData;
use thiserror::Error;
use xxhash_rust::xxh32::xxh32;

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

#[derive(Debug, Clone)]
struct LenAddress(Offset);

impl LenAddress {
    // fn drain(&self, size: Offset, memory: &mut Memory) -> Option<Offset> {
    //     let len = self.get_len(memory)?;
    //     let start = len.checked_sub(size)?;
    //     self.set_len(start, memory)?;
    //     Some(start)
    // }

    /// in bytes
    fn get_len(&self, memory: &Memory) -> Option<Word> {
        memory.get_word(self.address())
    }

    /// in bytes
    fn set_len(&self, len: Word, memory: &mut Memory) -> Option<()> {
        memory.set_word(self.address(), len)
    }

    // fn get_len_mut<'a>(&self, memory: &'a mut Memory) -> Option<&'a mut Word> {
    //     memory.get_word_mut(self.address())
    // }

    fn address(&self) -> Offset {
        self.0
    }

    fn data_address(&self) -> Offset {
        self.0 + 1
    }

    fn get_data(&self, memory: &Memory) -> Option<&[u8]> {
        let len = self.get_len(memory)?; // in bytes
        let words = (len + 3) / 4;
        let data = memory.get(self.data_address(), words)?;
        let data = unsafe { std::mem::transmute::<&[Word], &[u8]>(data) };
        data.get(..len as usize)
    }

    fn get_data_mut(&self, memory: &mut Memory) -> Option<&mut [u8]> {
        let len = self.get_len(memory)?; // in bytes
        let words = (len + 3) / 4;
        let data = memory.get_mut(self.data_address(), words)?;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
        data.get_mut(..len as usize)
    }
}

#[derive(Debug, Clone)]
struct CapAddress(Offset);

impl CapAddress {
    // fn init(&self, cap: Offset, memory: &mut Memory) -> Option<()> {
    //     let data_cap = cap.checked_sub(8)?;
    //     self.set_cap(data_cap, memory)?;
    //     self.len_address().set_len(0, memory)?;
    //     Some(())
    // }

    // in words, not including header (cap, len)
    fn get_cap(&self, memory: &Memory) -> Option<Word> {
        memory.get_word(self.address())
    }

    fn get_data<'a>(&self, memory: &'a Memory) -> Option<&'a [Word]> {
        let cap = self.get_cap(memory)?;
        memory.get(self.data_address(), cap)
    }

    // fn set_cap(&self, cap: Offset, memory: &mut Memory) -> Option<()> {
    //     memory
    //         .get_mut(self.address(), 4)
    //         .map(|slot| slot.copy_from_slice(&u32::to_le_bytes(cap)))
    // }

    fn alloc_slot(&self, size_bytes: Word, memory: &mut Memory) -> Option<&mut [u8]> {
        let cap_words = self.get_cap(memory)?;
        let len_address = self.len_address();
        let len = len_address.get_len(memory)?;
        let new_len = len + size_bytes;

        if new_len <= cap_words * 4 {
            let object = memory.get_mut(self.data_address(), cap_words)?;
            let bytes = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(object) };
            let slot = bytes.get_mut(len as usize..new_len as usize)?;
            len_address.set_len(new_len, memory)?;
            Some(slot)
        } else {
            None
        }
    }

    fn reserve_block(&self, size_bytes: Word, memory: &mut Memory) -> Option<LenAddress> {
        let aligned_len = (size_bytes + 3) & !3;
        let slot = self.alloc_slot(4 + aligned_len, memory)?;
        slot.get_mut(..4)?
            .copy_from_slice(&u32::to_le_bytes(size_bytes));
        Some(LenAddress(self.0 + 1))
    }

    fn alloc_block(&self, data: &[u8], memory: &mut Memory) -> Option<LenAddress> {
        let data_len = data.len() as Offset;
        let block = self.reserve_block(data_len, memory)?;
        let dst = block.get_data_mut(memory)?;
        if dst.len() == data.len() {
            dst.copy_from_slice(data);
            Some(block)
        } else {
            None
        }
    }

    // fn alloc_cap(&self, size_bytes: Offset, memory: &mut Memory) -> Option<CapAddress> {
    //     let to_allocate = size_words + 8;
    //     let cap = self.reserve_slot(to_allocate, memory).map(CapAddress)?;
    //     cap.init(to_allocate, memory)?;
    //     Some(cap)
    // }

    fn len_address(&self) -> LenAddress {
        LenAddress(self.0 + 1)
    }

    fn address(&self) -> Offset {
        self.0
    }

    fn data_address(&self) -> Offset {
        self.0 + 2
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

    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0.len_address().get_len(memory).map(|x| x / I::SIZE)
    }

    pub fn peek(&self, memory: &Memory) -> Option<I> {
        let len_address = self.0.len_address();
        let len = len_address.get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let data = len_address.get_data(memory)?;
        let start = start as usize;
        let end = start + I::SIZE as usize;
        data.get(start..end).and_then(I::load)
    }

    pub fn push(&self, item: I, memory: &mut Memory) -> Option<()> {
        self.0
            .alloc_slot(I::SIZE, memory)
            .and_then(|slot| item.store(slot))
    }

    pub fn pop(&self, memory: &mut Memory) -> Option<I> {
        let len_address = self.0.len_address();
        let len = len_address.get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let data = len_address.get_data(memory)?;
        let begin = start as usize;
        let end = begin + I::SIZE as usize;
        len_address.set_len(start, memory)?;
        data.get(begin..end).and_then(I::load)
    }

    fn cut_block(&self, to: CapAddress, items: Word, memory: &mut Memory) -> Option<Block<I>> {
        let size_bytes = items * I::SIZE;
        let dst = to.reserve_block(size_bytes, memory)?;
        memory.move_items(dst.clone(), self.0.len_address(), size_bytes)?;
        Some(Block::new(dst))
    }

    // fn drain(&self, to: CapAddress, items: Word, memory: &mut Memory) -> Option<Block<I>> {
    //     memory
    //         .cut_and_paste(to, self.0.len_address(), items * I::SIZE)
    //         .map(Block::new)
    // }

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
        let data = self.0.get_data(memory)?;
        let start = (index * I::SIZE) as usize;
        let sned = start + I::SIZE as usize;
        data.get(start..sned).and_then(I::load)
    }
}

pub struct Str(LenAddress);

impl Str {
    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0.get_len(memory)
    }

    pub fn as_bytes(&self, memory: &Memory) -> Option<&[u8]> {
        self.0.get_data(memory)
    }
}

#[derive(Debug, Clone)]
pub struct Arena(CapAddress);

impl Arena {
    // fn new(addr: CapAddress) -> Self {
    //     Self(addr)
    // }

    // pub fn alloc_stack<I: Item>(&self, items: Word, memory: &mut Memory) -> Option<Stack<I>> {
    //     self.0.alloc_cap(items * I::SIZE, memory).map(Stack::new)
    // }

    // pub fn alloc_block<I: Item>(&self, memory: &mut Memory, data: &[u8]) -> Option<Block<I>> {
    //     self.0.alloc_len(data, memory).map(Block::new)
    // }

    pub fn alloc_string(&self, memory: &mut Memory, string: &str) -> Option<Str> {
        self.0.alloc_block(string.as_bytes(), memory).map(Str)
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
        let len_address = self.0.len_address();
        let count = len_address.get_len(memory)?;

        let bytes = symbol.as_bytes();
        let hash = xxh32(bytes, Self::HASH_SEED);

        let cap = self.0.get_cap(memory)?;
        let data_address = self.0.data_address();
        let start = hash % cap;
        let mut index = start;

        loop {
            let entry = memory.get_word(data_address + index)?;
            if entry == 0 {
                let str = heap.alloc_string(memory, symbol)?;
                memory.set_word(data_address + index, str.0.0)?;
                len_address.set_len(count + 1, memory)?;
                return Some(str.0);
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
    memory: &'a mut [Word],
}

const LAYOUT_REGIONS: Offset = 4;

impl<'a> Memory<'a> {
    const LAYOUT_OFFSET: Offset = 1;

    const LAYOUT_SYMBOL_TABLE: Offset = 0;
    const LAYOUT_PARSE_STACK: Offset = 1;
    const LAYOUT_PARSE_BASE: Offset = 2;
    const LAYOUT_HEAP: Offset = 3;

    pub fn init(memory: &'a mut [u32], sizes: [Offset; LAYOUT_REGIONS as usize]) -> Option<Self> {
        let mut memory = Self { memory };

        memory.set_word(0, 0x0BAD_F00D)?;
        memory.make_block(Self::LAYOUT_OFFSET, LAYOUT_REGIONS * 4)?;

        debug_assert!((8 + LAYOUT_REGIONS * 4) < 64);
        let mut addr: u32 = 64;

        for (i, size) in sizes.iter().enumerate() {
            let cap = size.checked_sub(8)?;
            let cap_address = memory.make_cap(addr, cap)?;
            memory.set_word(LAYOUT_REGIONS + 1 + i as Offset, cap_address.0)?;
            addr += size;
        }

        Some(memory)
    }

    fn get_region(&self, region: Offset) -> Option<CapAddress> {
        self.get_word(Self::LAYOUT_OFFSET + 1 + region)
            .map(CapAddress)
    }

    pub fn get_symbol_table(&self) -> Option<SymbolTable> {
        self.get_region(Self::LAYOUT_SYMBOL_TABLE).map(SymbolTable)
    }

    pub fn get_parse_stack(&self) -> Option<Stack<MemValue>> {
        self.get_region(Self::LAYOUT_PARSE_STACK).map(Stack::new)
    }

    pub fn get_parse_base(&self) -> Option<Stack<Word>> {
        self.get_region(Self::LAYOUT_PARSE_BASE).map(Stack::new)
    }

    pub fn get_heap(&self) -> Option<Arena> {
        self.get_region(Self::LAYOUT_HEAP).map(Arena)
    }

    fn make_block(&mut self, address: Offset, len: Word) -> Option<LenAddress> {
        let len_address = LenAddress(address);
        len_address.set_len(len, self)?;
        Some(len_address)
    }

    fn make_cap(&mut self, address: Offset, cap: Word) -> Option<CapAddress> {
        self.set_word(address, cap)?;
        let cap_address = CapAddress(address);
        cap_address.len_address().set_len(0, self)?;
        Some(cap_address)
    }

    // move bytes from one block to another. we assume target block has slot allocated already -- we replace the data
    fn move_items(&mut self, to: LenAddress, from: LenAddress, size_bytes: Word) -> Option<()> {
        let to_data_address_bytes = to.data_address() * 4;
        let to_data_len_bytes = to.get_len(self)?;
        let to_new_data_len_bytes = to_data_len_bytes.checked_sub(size_bytes)?;

        let from_data_address_bytes = from.data_address() * 4;
        let from_data_len_bytes = from.get_len(self)?;
        let from_new_data_len_bytes = from_data_len_bytes.checked_sub(size_bytes)?;

        let memory_bytes = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(self.memory) };

        let src = (from_data_address_bytes + from_new_data_len_bytes) as usize;
        let dst = (to_data_address_bytes + to_new_data_len_bytes) as usize;
        let size = size_bytes as usize;

        if dst + size > memory_bytes.len() || src + size > memory_bytes.len() {
            None
        } else {
            for i in 0..size {
                memory_bytes[dst + i] = memory_bytes[src + i];
            }
            from.set_len(from_new_data_len_bytes, self)?;
            Some(())
        }
    }

    fn get_word(&self, address: Offset) -> Option<Word> {
        self.memory.get(address as usize).copied()
    }

    fn set_word(&mut self, address: Offset, word: Word) -> Option<()> {
        self.memory
            .get_mut(address as usize)
            .map(|slot| *slot = word)
    }

    fn get(&self, start: Offset, len: Offset) -> Option<&[Word]> {
        let start = start as usize;
        let end = start + len as usize;
        self.memory.get(start..end)
    }

    fn get_mut(&mut self, start: Offset, len: Offset) -> Option<&mut [Word]> {
        let start = start as usize;
        let end = start + len as usize;
        self.memory.get_mut(start..end)
    }

    // P A R S E  H E L P E R S

    fn begin(&mut self) -> Option<()> {
        let len = self.get_parse_stack()?.len(self)?;
        self.get_parse_base()?.push(len, self)
    }

    fn end(&mut self) -> Option<Block<MemValue>> {
        let offset = self.get_parse_base()?.pop(self)?;
        self.get_parse_stack()?
            .cut_block(self.get_heap()?.0.clone(), offset, self)
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
        if data.len() == 4 {
            let bytes = self.to_le_bytes();
            data.copy_from_slice(&bytes);
            Some(())
        } else {
            None
        }
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

// struct ParseCollector<'a> {
//     memory: &'a mut Memory<'a>,
// }

// impl<'a> ParseCollector<'a> {
//     fn new(memory: &'a mut Memory<'a>) -> Self {
//         Self { memory }
//     }
// }

impl Collector for Memory<'_> {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Option<()> {
        let string = self
            .get_heap()?
            .alloc_string(self, string)
            .map(MemValue::string)?;
        self.get_parse_stack()?.push(string, self)
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Option<()> {
        let tag = match kind {
            WordKind::Word => MemValue::TAG_WORD,
            WordKind::SetWord => MemValue::TAG_SET_WORD,
            WordKind::GetWord => MemValue::TAG_GET_WORD,
        };
        let symbol =
            self.get_symbol_table()?
                .get_or_insert_symbol(word, self.get_heap()?.clone(), self)?;
        let value = MemValue(symbol.0, tag);
        self.get_parse_stack()?.push(value, self)
    }

    fn integer(&mut self, value: i32) -> Option<()> {
        self.get_parse_stack()?.push(MemValue::int(value), self)
    }

    fn begin_block(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_block(&mut self) -> Option<()> {
        let block = self.end().map(MemValue::block)?;
        self.get_parse_stack()?.push(block, self)
    }

    fn begin_path(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_path(&mut self) -> Option<()> {
        let block = self.end().map(MemValue::path)?;
        self.get_parse_stack()?.push(block, self)
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

pub fn parse_block<'a>(memory: &'a mut Memory<'a>, input: &str) -> Option<Block<MemValue>> {
    crate::parse::Parser::parse(input, memory).ok()?;
    let parse = memory.get_parse_stack()?;
    let heap = memory.get_heap()?;
    parse.cut_block(heap.0, parse.len(memory)?, memory)
}
