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

/// Represents a Rebel value in VM memory
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
}

impl Item for VmValue {
    type Error = MemoryError;
    const SIZE: usize = 2;

    fn load(data: &[u8]) -> Result<Self, Self::Error> {
        let addr = data.get(..4).ok_or(MemoryError::OutOfBounds)?;
        let tag = data.get(4).copied().ok_or(MemoryError::OutOfBounds)?;
        (u32::from_le_bytes(addr.try_into()?), tag).try_into()
    }
}

pub struct Box(Addr);

impl<'a> Box {
    pub fn open<T: AsRef<[Word]> + ?Sized>(self, memory: &'a T) -> Option<&'a [Word]> {
        let addr = self.0.as_usize();
        let memory = memory.as_ref();
        let cap = memory.get(addr).copied()? as usize;
        memory.get(addr + 1..addr + cap)
    }
}

pub struct Series<'a, I: Item>(&'a [u8], PhantomData<I>);

impl<'a, I> Series<'a, I>
where
    I: Item<Error = MemoryError>,
{
    pub fn len(&self) -> usize {
        self.0.len() / I::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn load<T: AsRef<[u32]> + ?Sized>(memory: &'a T, bx: Box) -> Option<Series<'a, I>> {
        let memory = bx.open(memory)?;
        let (items, data) = memory.split_first()?;
        let bytes = unsafe { std::mem::transmute::<&[u32], &[u8]>(data) };
        let allocated = (*items as usize) * I::SIZE;
        bytes.get(..allocated).map(|data| Series(data, PhantomData))
    }

    pub fn get(&self, index: usize) -> Result<I, I::Error> {
        let start = I::SIZE * index;
        self.0
            .get(start..start + I::SIZE)
            .ok_or(MemoryError::OutOfBounds)
            .and_then(I::load)
    }
}

pub fn load_series(memory: &[Word], bx: Box, pos: usize) -> Result<VmValue, MemoryError> {
    Series::<VmValue>::load(memory, bx)
        .ok_or(MemoryError::OutOfBounds)
        .and_then(|series| series.get(pos))
}
