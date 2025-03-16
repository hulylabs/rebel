// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use std::marker::PhantomData;
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

#[derive(Debug)]
pub struct Addr(Word);

impl Addr {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug)]
pub struct Symbol(Addr);

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

    const fn serialize(&self) -> (Word, u8) {
        match self {
            VmValue::None => (0, VmValue::TAG_NONE),
            VmValue::Int(value) => (*value as u32, VmValue::TAG_INT),
            VmValue::Bool(value) => (if *value { 1 } else { 0 }, VmValue::TAG_BOOL),
            VmValue::Block(addr) => (addr.0, VmValue::TAG_BLOCK),
            VmValue::Context(addr) => (addr.0, VmValue::TAG_CONTEXT),
            VmValue::Path(addr) => (addr.0, VmValue::TAG_PATH),
            VmValue::String(addr) => (addr.0, VmValue::TAG_STRING),
            VmValue::Word(symbol) => (symbol.0.0, VmValue::TAG_WORD),
            VmValue::SetWord(symbol) => (symbol.0.0, VmValue::TAG_SET_WORD),
            VmValue::GetWord(symbol) => (symbol.0.0, VmValue::TAG_GET_WORD),
        }
    }

    const fn deserialize(addr: Word, tag: u8) -> Option<VmValue> {
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

impl TryFrom<(Word, u8)> for VmValue {
    type Error = MemoryError;

    fn try_from((addr, tag): (Word, u8)) -> Result<Self, Self::Error> {
        Self::deserialize(addr, tag).ok_or(MemoryError::InvalidTag)
    }
}

pub trait Item: Sized {
    const SIZE: usize;

    fn load(data: &[u8]) -> Result<Self, MemoryError>;
    fn store(&self, data: &mut [u8]) -> Result<(), MemoryError>;
}

impl Item for VmValue {
    const SIZE: usize = 8;

    fn load(data: &[u8]) -> Result<Self, MemoryError> {
        let addr = data.get(..4).ok_or(MemoryError::OutOfBounds)?;
        let tag = data.get(4).copied().ok_or(MemoryError::OutOfBounds)?;
        (u32::from_le_bytes(addr.try_into()?), tag).try_into()
    }

    fn store(&self, data: &mut [u8]) -> Result<(), MemoryError> {
        let (addr, tag) = self.serialize();
        data.get_mut(..4)
            .ok_or(MemoryError::OutOfBounds)?
            .copy_from_slice(&addr.to_le_bytes());
        data.get_mut(4)
            .map(|tag_byte| *tag_byte = tag)
            .ok_or(MemoryError::OutOfBounds)
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

    pub fn get(&self, index: usize) -> Result<I, MemoryError> {
        let start = I::SIZE * index;
        self.0
            .get(start..start + I::SIZE)
            .ok_or(MemoryError::OutOfBounds)
            .and_then(I::load)
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

    pub fn set(&mut self, index: usize, value: &I) -> Result<(), MemoryError> {
        let start = I::SIZE * index;
        let slot = self
            .0
            .get_mut(start..start + I::SIZE)
            .ok_or(MemoryError::OutOfBounds)?;
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

pub struct HeapMut<'a>(&'a mut [Word]);

impl<'a> HeapMut<'a> {
    fn get_word(&self, addr: Addr) -> Option<Word> {
        self.0.get(addr.as_usize() + 1).copied()
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

    pub fn alloc_sealed<I: Item>(&mut self, slice: Slice<'a, I>) -> Option<Addr> {
        let bytes = slice.0.len();
        let size_words = (bytes + 3) / 4 + 1;
        let (sealed, addr) = self.alloc_words(size_words)?;

        let (len, data) = sealed.split_first_mut()?;
        *len = bytes as Word;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };

        for (src, dst) in slice.0.iter().zip(data.iter_mut()) {
            *dst = *src;
        }

        Some(addr)
    }
}

pub struct Sealed<'a, I: Item>(&'a mut [Word], PhantomData<I>);

impl<'a, I> Sealed<'a, I>
where
    I: Item,
{
    // fn init(data: &'a mut [Word], src: &[u8]) -> Option<Self> {
    //     let (cap, dst) = data.split_first_mut()?;
    //     *cap = src.len() as Word;
    //     let dst = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(dst) };
    //     dst.copy_from_slice(src);
    //     Some(Sealed(data, PhantomData))
    // }

    pub fn load(heap: &HeapMut<'a>, addr: Addr) -> Option<Self> {
        let len = heap.get_word(addr)?;
        let allocated = (len + 3) / 4 + 1;
        let data = heap.get(addr, allocated)?;
        Some(Sealed(data, PhantomData))
    }

    pub fn len(&self) -> Option<usize> {
        self.0.first().map(|len| *len as usize)
    }

    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    pub fn items(&self) -> Option<Slice<'a, I>> {
        let (len, data) = self.0.split_first()?;
        let len = *len as usize;
        let data = unsafe { std::mem::transmute::<&[Word], &[u8]>(data) };
        let data = data.get(..len)?;
        Some(Slice(data, PhantomData))
    }

    // pub fn push(&mut self, item: I) -> Result<(), MemoryError> {
    //     let (len, data) = self.0.split_first_mut().ok_or(MemoryError::OutOfBounds)?;
    //     let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
    //     let addr = *len as usize;
    //     *len += I::SIZE as Word;
    //     I::store(&item, &mut data[addr..addr + I::SIZE])
    // }
}

pub fn alloc_sealed<'a>(heap: &'a mut HeapMut<'a>, slice: Slice<'a, VmValue>) -> Option<Addr> {
    heap.alloc_sealed(slice)
}

pub fn get_item<'a>(heap: &HeapMut<'a>, addr: Addr, pos: usize) -> Result<VmValue, MemoryError> {}
