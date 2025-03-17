// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, WordKind};
use std::{marker::PhantomData, ops::Div};
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

fn write_word(data: &mut [u8], value: Word) -> Option<()> {
    if data.len() < 4 {
        None
    } else {
        data[0] = value as u8;
        data[1] = (value >> 8) as u8;
        data[2] = (value >> 16) as u8;
        data[3] = (value >> 24) as u8;
        Some(())
    }
}

fn read_word(data: &[u8]) -> Option<Word> {
    if data.len() < 4 {
        None
    } else {
        let result = data[0] as u32
            | (data[1] as u32) << 8
            | (data[2] as u32) << 16
            | (data[3] as u32) << 24;
        Some(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Addr(Word);

impl Addr {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug)]
pub struct Symbol(Addr);

type Tag = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemValue(Word, Tag);

#[derive(Debug)]
pub enum VmValue {
    None,
    Int(i32),
    Bool(bool),
    Block(Addr),
    Context(Addr),
    Path(Addr),
    String(Addr),
    Word(Symbol),
    SetWord(Symbol),
    GetWord(Symbol),
}

impl VmValue {
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

    const fn to_mem_value(&self) -> MemValue {
        match self {
            VmValue::None => MemValue(0, VmValue::TAG_NONE),
            VmValue::Int(value) => MemValue(*value as u32, VmValue::TAG_INT),
            VmValue::Bool(value) => MemValue(if *value { 1 } else { 0 }, VmValue::TAG_BOOL),
            VmValue::Block(addr) => MemValue(addr.0, VmValue::TAG_BLOCK),
            VmValue::Context(addr) => MemValue(addr.0, VmValue::TAG_CONTEXT),
            VmValue::Path(addr) => MemValue(addr.0, VmValue::TAG_PATH),
            VmValue::String(addr) => MemValue(addr.0, VmValue::TAG_STRING),
            VmValue::Word(symbol) => MemValue(symbol.0.0, VmValue::TAG_WORD),
            VmValue::SetWord(symbol) => MemValue(symbol.0.0, VmValue::TAG_SET_WORD),
            VmValue::GetWord(symbol) => MemValue(symbol.0.0, VmValue::TAG_GET_WORD),
        }
    }

    const fn try_from(mem: MemValue) -> Option<VmValue> {
        let MemValue(addr, tag) = mem;
        match tag {
            VmValue::TAG_NONE => Some(VmValue::None),
            VmValue::TAG_INT => Some(VmValue::Int(addr as i32)),
            VmValue::TAG_BOOL => Some(VmValue::Bool(addr != 0)),
            VmValue::TAG_BLOCK => Some(VmValue::Block(Addr(addr))),
            VmValue::TAG_CONTEXT => Some(VmValue::Context(Addr(addr))),
            VmValue::TAG_PATH => Some(VmValue::Path(Addr(addr))),
            VmValue::TAG_STRING => Some(VmValue::String(Addr(addr))),
            VmValue::TAG_WORD => Some(VmValue::Word(Symbol(Addr(addr)))),
            VmValue::TAG_SET_WORD => Some(VmValue::SetWord(Symbol(Addr(addr)))),
            VmValue::TAG_GET_WORD => Some(VmValue::GetWord(Symbol(Addr(addr)))),
            _ => None,
        }
    }
}

impl TryFrom<MemValue> for VmValue {
    type Error = MemoryError;

    fn try_from(mem: MemValue) -> Result<Self, Self::Error> {
        Self::try_from(mem).ok_or(MemoryError::InvalidTag)
    }
}

pub trait Item: Sized {
    const SIZE: usize;

    fn load(data: &[u8]) -> Option<Self>;
    fn store(self, data: &mut [u8]) -> Option<()>;
}

impl Item for MemValue {
    const SIZE: usize = 8;

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

pub struct Slice<'a, I: Item>(&'a [u8], PhantomData<I>);

impl<'a, I> Slice<'a, I>
where
    I: Item,
{
    pub fn len(&self) -> usize {
        self.0.len() / I::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<I> {
        let start = I::SIZE * index;
        self.0.get(start..start + I::SIZE).and_then(I::load)
    }
}

pub struct SliceMut<'a, I: Item>(&'a mut [u8], PhantomData<I>);

impl<'a, I> SliceMut<'a, I>
where
    I: Item,
{
    pub fn len(&self) -> usize {
        self.0.len() / I::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn set(&mut self, index: usize, value: I) -> Option<()> {
        let start = I::SIZE * index;
        let slot = self.0.get_mut(start..start + I::SIZE)?;
        value.store(slot)
    }
}

pub trait Series {
    type Item: Item;

    /// Returns number of words needed to store the given number of items.
    fn words_needed(items: usize) -> usize;

    // fn split_mut(&mut self) -> Result<(&mut Word, &mut [Word]), MemoryError>;

    /// Returns the number of items in the series.
    fn len(&self) -> Option<usize>;

    /// Returns `true` if the series is empty.
    fn is_empty(&self) -> Option<bool>;

    // /// Appends an item to the series.
    // fn push(&mut self, item: Self::Item) -> Result<(), MemoryError> {
    //     let (len, data) = self.split_mut()?;
    //     let addr = *len as usize;
    //     *len += Self::Item::SIZE as Word;
    //     let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
    //     Self::Item::store(&item, &mut data[addr..addr + Self::Item::SIZE])
    // }
}

// pub struct Boxed<I>(Addr, PhantomData<I>);

// impl<'a, I> Boxed<I>
// where
//     I: Item<Error = MemoryError>,
// {
//     fn unbox<T: AsRef<[Word]> + ?Sized>(self, memory: &'a T) -> Option<&'a [Word]> {
//         let addr = self.0.as_usize();
//         let memory = memory.as_ref();
//         let cap = memory.get(addr).copied()? as usize;
//         memory.get(addr + 1..addr + cap)
//     }

//     pub fn unbox_sliced<T: AsRef<[Word]> + ?Sized>(self, memory: &'a T) -> Option<Sliced<'a, I>> {
//         let memory = self.unbox(memory)?;
//         let (items, data) = memory.split_first()?;
//         let bytes = unsafe { std::mem::transmute::<&[u32], &[u8]>(data) };
//         let allocated = (*items as usize) * I::SIZE;
//         bytes.get(..allocated).map(|data| Sliced(data, PhantomData))
//     }
// }

impl Item for u8 {
    const SIZE: usize = 1;

    fn load(data: &[u8]) -> Option<Self> {
        data.get(0).copied()
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        data.get_mut(0).map(|slot| *slot = self)
    }
}

pub struct Heap<'a>(&'a mut [Word]);

impl<'a> Heap<'a> {
    fn get_word(&self, addr: Addr) -> Option<Word> {
        self.0.get(addr.as_usize() + 1).copied()
    }

    fn get(&self, addr: Addr, len: Word) -> Option<&[Word]> {
        let addr = addr.as_usize() + 1;
        self.0.get(addr..addr + len as usize)
    }

    fn get_mut(&mut self, addr: Addr, len: Word) -> Option<&mut [Word]> {
        let addr = addr.as_usize() + 1;
        self.0.get_mut(addr..addr + len as usize)
    }

    fn alloc_words(&mut self, words: usize) -> Option<(&mut [Word], Addr)> {
        let (len, data) = self.0.split_first_mut()?;
        let addr = *len as usize;
        *len += words as Word;
        data.get_mut(addr..addr + words)
            .map(|data| (data, Addr(addr as Word)))
    }

    fn alloc_raw(&mut self, raw: &[u8]) -> Option<Addr> {
        let bytes = raw.len();
        let size_words = (bytes + 3) / 4 + 1;
        let (sealed, addr) = self.alloc_words(size_words)?;

        let (len, data) = sealed.split_first_mut()?;
        *len = bytes as Word;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };

        for (src, dst) in raw.iter().zip(data.iter_mut()) {
            *dst = *src;
        }

        Some(addr)
    }

    pub fn alloc_string(&mut self, string: &str) -> Option<Addr> {
        self.alloc_raw(string.as_bytes())
    }

    pub fn alloc_sealed<I: Item>(&mut self, slice: Slice<I>) -> Option<Addr> {
        self.alloc_raw(slice.0)
    }

    pub fn load_sealed<I: Item>(&self, addr: Addr) -> Option<Sealed<I>> {
        let len = self.get_word(addr)?;
        let allocated = (len + 3) / 4 + 1;
        let data = self.get(addr, allocated)?;
        Some(Sealed(data, PhantomData))
    }
}

pub struct Sealed<'a, I>(&'a [Word], PhantomData<I>);

impl<'a, I> Sealed<'a, I>
where
    I: Item,
{
    // pub fn alloc(heap: &'a mut Heap<'a>, slice: Slice<'a, I>) -> Option<Addr> {
    // let bytes = slice.0.len();
    // let size_words = (bytes + 3) / 4 + 1;
    // let (sealed, addr) = heap.alloc_words(size_words)?;

    // let (len, data) = sealed.split_first_mut()?;
    // *len = bytes as Word;
    // let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };

    // for (src, dst) in slice.0.iter().zip(data.iter_mut()) {
    //     *dst = *src;
    // }

    // Some(addr)
    //     heap.alloc_raw(slice.0)
    // }

    // pub fn load(heap: &'a mut Heap<'a>, addr: Addr) -> Option<Self> {
    //     let len = heap.get_word(addr)?;
    //     let allocated = (len + 3) / 4 + 1;
    //     let data = heap.get_mut(addr, allocated)?;
    //     Some(Sealed(data, PhantomData))
    // }

    pub fn len(&self) -> Option<usize> {
        self.0.first().map(|len| *len as usize)
    }

    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    pub fn items(self) -> Option<Slice<'a, I>> {
        let (len, data) = self.0.split_first()?;
        let len = *len as usize;
        let data = unsafe { std::mem::transmute::<&[Word], &[u8]>(data) };
        let data = data.get(..len)?;
        Some(Slice(data, PhantomData))
    }
}

pub struct SealedMut<'a, I>(&'a mut [Word], PhantomData<I>);

impl<'a, I> SealedMut<'a, I>
where
    I: Item,
{
    fn push(&mut self, item: I) -> Option<()> {
        let (len, data) = self.0.split_first_mut()?;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
        let addr = *len as usize;
        *len += I::SIZE as Word;
        let slot = data.get_mut(addr..addr + I::SIZE)?;
        item.store(slot)
    }

    fn pop(&mut self) -> Option<I> {
        let (len, data) = self.0.split_first_mut()?;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
        let addr = len.checked_sub(I::SIZE as Word)?;
        *len = addr;
        let addr = addr as usize;
        data.get(addr..addr + I::SIZE).and_then(I::load)
    }
}

//

pub struct Stack<'a, I: Item>(&'a mut [Word], PhantomData<I>);

impl<'a, I> Stack<'a, I>
where
    I: Item,
{
    pub fn alloc(heap: &'a mut Heap<'a>, items: usize) -> Option<Addr> {
        let bytes = items * I::SIZE;
        let size_words = (bytes + 3) / 4 + 2;
        let (stack, addr) = heap.alloc_words(size_words)?;
        if stack.len() < 2 {
            None
        } else {
            stack[0] = size_words as Word;
            stack[1] = 0;
            Some(addr)
        }
    }

    pub fn load(heap: &'a mut Heap<'a>, addr: Addr) -> Option<Self> {
        let cap = heap.get_word(addr)?;
        let data = heap.get_mut(addr, cap)?;
        Some(Stack(data, PhantomData))
    }

    pub fn len(&self) -> Option<Word> {
        self.0.get(1).map(|len| len.div(I::SIZE as Word))
    }

    fn get_sealed_mut(&mut self) -> Option<SealedMut<I>> {
        let (_, sealed) = self.0.split_first_mut()?;
        Some(SealedMut(sealed, PhantomData))
    }

    pub fn push(&mut self, item: I) -> Option<()> {
        self.get_sealed_mut()?.push(item)
    }

    pub fn pop(&mut self) -> Option<I> {
        self.get_sealed_mut()?.pop()
    }
}

//

pub struct Memory<'a, T> {
    memory: &'a mut T,
}

impl<'a, T> Memory<'a, T>
where
    T: AsMut<[Word]>,
{
    pub fn new(memory: &'a mut T) -> Self {
        // let (parser, rest) = memory.as_mut().split_at_mut_checked(1000)?;

        // Some(Self {
        //     heap: Heap(rest),
        //     parser: Stack(parser, PhantomData),
        // })
        Self { memory }
    }

    // fn parse(&mut self, input: &str) -> Option<()> {}
}

//

impl Item for Word {
    const SIZE: usize = 4;

    fn load(data: &[u8]) -> Option<Self> {
        read_word(data)
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        write_word(data, self)
    }
}

// P A R S E  C O L L E C T O R

struct ParseCollector<'a> {
    heap: Heap<'a>,
    parse: Stack<'a, MemValue>,
    base: Stack<'a, Word>,
}

impl<'a> ParseCollector<'a> {
    fn new(heap: Heap<'a>, parse: Stack<'a, MemValue>, base: Stack<'a, Word>) -> Self {
        Self { heap, parse, base }
    }
}

impl<'a> Collector for ParseCollector<'a> {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Option<()> {
        let addr = self.heap.alloc_string(string)?;
        self.parse.push(MemValue(addr.0, VmValue::TAG_STRING))
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
        let mem = MemValue(value as Word, VmValue::TAG_INT);
        self.parse.push(mem)
    }

    fn begin_block(&mut self) -> Option<()> {
        self.parse.len().and_then(|len| self.base.push(len))
    }

    fn end_block(&mut self) -> Option<()> {
        // let [bp] = self.ops.pop()?;
        // let block_data = self.parse.pop_all(bp).ok_or(MemoryError::UnexpectedError)?;
        // let offset = self.module.heap.alloc_block(block_data)?;
        // self.parse.push([VmValue::TAG_BLOCK, offset])
        Some(())
    }

    fn begin_path(&mut self) -> Option<()> {
        self.parse.len().and_then(|len| self.base.push(len))
    }

    fn end_path(&mut self) -> Option<()> {
        // let [bp] = self.ops.pop()?;
        // let block_data = self.parse.pop_all(bp).ok_or(MemoryError::UnexpectedError)?;
        // let offset = self.module.heap.alloc_block(block_data)?;
        // self.parse.push([VmValue::TAG_PATH, offset])
        Some(())
    }
}

//

pub fn alloc_sealed<'a>(heap: &'a mut Heap<'a>, slice: Slice<'a, MemValue>) -> Option<Addr> {
    // Sealed::alloc(heap, slice)
    heap.alloc_sealed(slice)
}

pub fn get_sealed_item<'a>(heap: &'a Heap<'a>, addr: Addr, pos: usize) -> Option<MemValue> {
    heap.load_sealed(addr).and_then(Sealed::items)?.get(pos)
}

pub fn push_item<'a>(heap: &'a mut Heap<'a>, addr: Addr, item: MemValue) -> Option<()> {
    Stack::load(heap, addr)?.push(item)
}

pub fn pop_item<'a>(heap: &'a mut Heap<'a>, addr: Addr) -> Option<MemValue> {
    Stack::load(heap, addr)?.pop()
}

pub fn sealed_load<'a>(heap: &'a mut Heap<'a>, addr: Addr) -> Option<Sealed<'a, MemValue>> {
    heap.load_sealed(addr)
}

pub fn stack_load<'a>(heap: &'a mut Heap<'a>, addr: Addr) -> Option<Stack<'a, MemValue>> {
    Stack::load(heap, addr)
}
