// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, WordKind};
use std::{marker::PhantomData, mem};
use thiserror::Error;

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
    // fn get_cap(&self, memory: &Memory) -> Option<usize> {
    //     let address = self.address();
    //     memory
    //         .get(address, address + 4)?
    //         .try_into()
    //         .ok()
    //         .map(u32::from_le_bytes)
    //         .map(|x| x as usize)
    // }

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
        let cap = self.reserve_slot(size + 8, memory).map(CapAddress)?;
        cap.set_cap(size, memory)?;
        cap.len_address().set_len(0, memory)?;
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

// struct SliceIterator<'a, I> {
//     memory: &'a Memory<'a>,
//     pos: Offset,
//     end: Offset,
//     _phantom: PhantomData<I>,
// }

// impl<'a, I> SliceIterator<'a, I>
// where
//     I: Item,
// {
//     fn new(memory: &'a Memory<'a>, start: Offset, len: Offset) -> Self {
//         Self {
//             memory,
//             pos: start,
//             end: start + len,
//             _phantom: PhantomData,
//         }
//     }
// }

// impl<'a, I> Iterator for SliceIterator<'a, I>
// where
//     I: Item,
// {
//     type Item = I;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.pos >= self.end {
//             None
//         } else {
//             let item = self.memory.get(self.pos, I::SIZE)?;
//             self.pos += I::SIZE;
//             I::load(item)
//         }
//     }
// }

// struct SliceIteratorMut<'a, I> {
//     memory: &'a mut Memory<'a>,
//     start: Offset,
//     len: Offset,
//     _phantom: PhantomData<I>,
// }

// impl<'a, I> SliceIteratorMut<'a, I>
// where
//     I: Item,
// {
//     fn new(memory: &'a mut Memory<'a>, start: Offset, len: Offset) -> Self {
//         Self {
//             memory,
//             start,
//             len,
//             _phantom: PhantomData,
//         }
//     }

//     fn next(&mut self) -> Option<&mut [u8]> {
//         let item = self.memory.get_mut(self.start, I::SIZE)?;
//         self.start += I::SIZE;
//         Some(item)
//     }
// }

#[derive(Debug, Clone, Copy)]
pub struct Stack<I>(CapAddress, PhantomData<I>);

impl<I> Stack<I>
where
    I: Item,
{
    pub fn new(addr: CapAddress) -> Self {
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

    pub fn drain_all(&self, to: CapAddress, memory: &mut Memory) -> Option<Block<I>> {
        self.drain(to, self.len(memory)?, memory)
    }
}

pub struct Block<I>(LenAddress, PhantomData<I>);

impl<I> Block<I>
where
    I: Item,
{
    pub fn new(addr: LenAddress) -> Self {
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

#[derive(Debug, Clone, Copy)]
pub struct Arena(CapAddress);

impl Arena {
    pub fn new(addr: CapAddress) -> Self {
        Self(addr)
    }

    pub fn alloc_stack<I: Item>(&self, items: Word, memory: &mut Memory) -> Option<Stack<I>> {
        Some(Stack::new(self.0))
    }

    pub fn alloc_block<I: Item>(&self, memory: &mut Memory, data: &[u8]) -> Option<Block<I>> {
        self.0.alloc_len(data, memory).map(Block::new)
    }

    pub fn alloc_string(&self, memory: &mut Memory, string: &str) -> Option<Block<u8>> {
        self.alloc_block(memory, string.as_bytes())
    }
}

pub struct Memory<'a> {
    memory: &'a mut [u8],
}

impl<'a> Memory<'a> {
    pub fn new(memory: &'a mut [u8]) -> Self {
        Self { memory }
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

    pub fn string(value: Block<u8>) -> Self {
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

pub fn parse<'a>(memory: &'a mut Memory<'a>, heap: Arena, input: &str) -> Option<Block<MemValue>> {
    let parse = heap.alloc_stack::<MemValue>(100, memory)?;
    let base = heap.alloc_stack::<Word>(20, memory)?;

    let mut collector = ParseCollector::new(memory, heap, parse, base);
    crate::parse::Parser::parse(input, &mut collector).ok()?;

    parse.drain_all(heap.0, memory)
}
