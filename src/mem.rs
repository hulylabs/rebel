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
}

pub struct Sealed<'a, I: Item>(&'a mut [Word], PhantomData<I>);

impl<'a, I> Sealed<'a, I>
where
    I: Item,
{
    pub fn alloc(heap: &'a mut HeapMut<'a>, slice: Slice<'a, I>) -> Option<Addr> {
        let bytes = slice.0.len();
        let size_words = (bytes + 3) / 4 + 1;
        let (sealed, addr) = heap.alloc_words(size_words)?;

        let (len, data) = sealed.split_first_mut()?;
        *len = bytes as Word;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };

        for (src, dst) in slice.0.iter().zip(data.iter_mut()) {
            *dst = *src;
        }

        Some(addr)
    }

    pub fn load(heap: &'a mut HeapMut<'a>, addr: Addr) -> Option<Self> {
        let len = heap.get_word(addr)?;
        let allocated = (len + 3) / 4 + 1;
        let data = heap.get_mut(addr, allocated)?;
        Some(Sealed(data, PhantomData))
    }

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

    fn push(&mut self, item: I) -> Option<()> {
        let (len, data) = self.0.split_first_mut()?;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
        let addr = *len as usize;
        *len += I::SIZE as Word;
        let slot = data.get_mut(addr..addr + I::SIZE)?;
        I::store(item, slot)
    }
}

//

pub struct Stack<'a, I: Item>(&'a mut [Word], PhantomData<I>);

impl<'a, I> Stack<'a, I>
where
    I: Item,
{
    pub fn alloc(heap: &'a mut HeapMut<'a>, items: usize) -> Option<Addr> {
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

    pub fn load(heap: &'a mut HeapMut<'a>, addr: Addr) -> Option<Self> {
        let cap = heap.get_word(addr)?;
        let data = heap.get_mut(addr, cap)?;
        Some(Stack(data, PhantomData))
    }

    pub fn push(&mut self, item: I) -> Option<()> {
        let (_, sealed) = self.0.split_first_mut()?;
        let mut sealed = Sealed::<I>(sealed, PhantomData);
        sealed.push(item)
    }
}

//

pub fn alloc_sealed<'a>(heap: &'a mut HeapMut<'a>, slice: Slice<'a, MemValue>) -> Option<Addr> {
    Sealed::alloc(heap, slice)
}

pub fn get_sealed_item<'a>(heap: &'a mut HeapMut<'a>, addr: Addr, pos: usize) -> Option<MemValue> {
    Sealed::load(heap, addr).and_then(Sealed::items)?.get(pos)
}

pub fn push_item(stack: &mut Stack<MemValue>, item: MemValue) -> Option<()> {
    stack.push(item)
}
