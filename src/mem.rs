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

    fn serialize(&self) -> (Word, u8) {
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
}

impl TryFrom<(Word, u8)> for VmValue {
    type Error = MemoryError;

    fn try_from((addr, tag): (Word, u8)) -> Result<Self, Self::Error> {
        match tag {
            VmValue::TAG_NONE => Ok(VmValue::None),
            VmValue::TAG_INT => Ok(VmValue::Int(addr as i32)),
            VmValue::TAG_BOOL => Ok(VmValue::Bool(addr != 0)),
            VmValue::TAG_BLOCK => Ok(VmValue::Block(Addr(addr))),
            VmValue::TAG_CONTEXT => Ok(VmValue::Context(Addr(addr))),
            VmValue::TAG_PATH => Ok(VmValue::Path(Addr(addr))),
            VmValue::TAG_STRING => Ok(VmValue::String(Addr(addr))),
            VmValue::TAG_WORD => Ok(VmValue::Word(Symbol(Addr(addr)))),
            VmValue::TAG_SET_WORD => Ok(VmValue::SetWord(Symbol(Addr(addr)))),
            VmValue::TAG_GET_WORD => Ok(VmValue::GetWord(Symbol(Addr(addr)))),
            _ => Err(MemoryError::InvalidTag),
        }
    }
}

pub trait Item: Sized {
    type Error;
    const SIZE: usize;

    fn load(data: &[u8]) -> Result<Self, Self::Error>;
    fn store(&self, data: &mut [u8]) -> Result<(), Self::Error>;
}

impl Item for VmValue {
    type Error = MemoryError;
    const SIZE: usize = 8;

    fn load(data: &[u8]) -> Result<Self, Self::Error> {
        let addr = data.get(..4).ok_or(MemoryError::OutOfBounds)?;
        let tag = data.get(4).copied().ok_or(MemoryError::OutOfBounds)?;
        (u32::from_le_bytes(addr.try_into()?), tag).try_into()
    }

    fn store(&self, data: &mut [u8]) -> Result<(), Self::Error> {
        let (addr, tag) = self.serialize();
        data.get_mut(..4)
            .ok_or(MemoryError::OutOfBounds)?
            .copy_from_slice(&addr.to_le_bytes());
        data.get_mut(4)
            .map(|tag_byte| *tag_byte = tag)
            .ok_or(MemoryError::OutOfBounds)
    }
}

pub struct Boxed<I>(Addr, PhantomData<I>);

impl<'a, I> Boxed<I>
where
    I: Item<Error = MemoryError>,
{
    fn unbox<T: AsRef<[Word]> + ?Sized>(self, memory: &'a T) -> Option<&'a [Word]> {
        let addr = self.0.as_usize();
        let memory = memory.as_ref();
        let cap = memory.get(addr).copied()? as usize;
        memory.get(addr + 1..addr + cap)
    }

    pub fn unbox_sliced<T: AsRef<[Word]> + ?Sized>(self, memory: &'a T) -> Option<Sliced<'a, I>> {
        let memory = self.unbox(memory)?;
        let (items, data) = memory.split_first()?;
        let bytes = unsafe { std::mem::transmute::<&[u32], &[u8]>(data) };
        let allocated = (*items as usize) * I::SIZE;
        bytes.get(..allocated).map(|data| Sliced(data, PhantomData))
    }
}

pub struct HeapMut<'a>(&'a mut [Word]);

impl<'a> HeapMut<'a> {
    pub fn alloc_series<I: Item<Error = MemoryError>>(
        &mut self,
        cap_items: usize,
    ) -> Option<Boxed<I>> {
        let size_words = (I::SIZE * cap_items + 3) / 4 + 1;
        let (len, data) = self.0.split_first_mut()?;
        let addr = *len as usize;
        *len += size_words as Word;

        let allocated = data.get_mut(addr..addr + size_words)?;
        let (cap, object) = allocated.split_first_mut()?;
        *cap = size_words as Word;
        SeriesMut::<I>::init(object)?;
        Some(Boxed(Addr(addr as Word), PhantomData))
    }
}

pub struct Sliced<'a, I: Item>(&'a [u8], PhantomData<I>);

impl<'a, I> Sliced<'a, I>
where
    I: Item<Error = MemoryError>,
{
    pub fn len(&self) -> usize {
        self.0.len() / I::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<I, I::Error> {
        let start = I::SIZE * index;
        self.0
            .get(start..start + I::SIZE)
            .ok_or(MemoryError::OutOfBounds)
            .and_then(I::load)
    }
}

pub struct SeriesMut<'a, I: Item>(&'a mut [Word], PhantomData<I>);

impl<'a, I> SeriesMut<'a, I>
where
    I: Item<Error = MemoryError>,
{
    fn init(data: &'a mut [Word]) -> Option<Self> {
        data.first_mut().map(|len| *len = 0)?;
        Some(SeriesMut(data, PhantomData))
    }

    pub fn len(&self) -> Option<usize> {
        self.0.first().map(|len| *len as usize)
    }

    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    pub fn push(&mut self, item: I) -> Result<(), I::Error> {
        let (len, data) = self.0.split_first_mut().ok_or(MemoryError::OutOfBounds)?;
        let data = unsafe { std::mem::transmute::<&mut [Word], &mut [u8]>(data) };
        let addr = *len as usize;
        *len += I::SIZE as Word;
        I::store(&item, &mut data[addr..addr + I::SIZE])
    }
}

pub fn alloc_series<'a>(heap: &mut HeapMut) -> Result<Boxed<VmValue>, MemoryError> {
    let series = heap
        .alloc_series::<VmValue>(64)
        .ok_or(MemoryError::OutOfBounds)?;
}

pub fn load_slices(
    memory: &[Word],
    obj: Boxed<VmValue>,
    pos: usize,
) -> Result<VmValue, MemoryError> {
    let slices = obj.unbox_sliced(memory).ok_or(MemoryError::OutOfBounds)?;
    slices.get(pos)
}
