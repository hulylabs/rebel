// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{self, Collector, WordKind};
use std::{marker::PhantomData, mem, ops::Div};
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
    const SIZE: usize;

    fn load(data: &[u8]) -> Option<Self>;
    fn store(self, data: &mut [u8]) -> Option<()>;
}

struct LenAddress(Offset);

impl LenAddress {
    fn get_len(&self, memory: &Memory) -> Option<usize> {
        let address = self.address();
        memory
            .memory
            .get(address..address + 4)?
            .try_into()
            .ok()
            .map(u32::from_le_bytes)
            .map(|x| x as usize)
    }

    fn set_len(&self, memory: &mut Memory, len: usize) -> Option<()> {
        let address = self.0 as usize;
        let len = len as u32;
        let len = len.to_le_bytes();
        memory
            .memory
            .get_mut(address..address + 4)
            .map(|slot| slot.copy_from_slice(&len))
    }

    fn address(&self) -> usize {
        self.0 as usize
    }

    fn data_address(&self) -> usize {
        (self.0 + 4) as usize
    }
}

#[derive(Debug, Clone, Copy)]
struct CapAddress(Offset);

impl CapAddress {
    fn get_cap(&self, memory: &Memory) -> Option<usize> {
        let address = self.address();
        memory
            .memory
            .get(address..address + 4)?
            .try_into()
            .ok()
            .map(u32::from_le_bytes)
            .map(|x| x as usize)
    }

    fn set_cap(&self, memory: &mut Memory, len: usize) -> Option<()> {
        let address = self.0 as usize;
        let len = len as u32;
        let len = len.to_le_bytes();
        memory
            .memory
            .get_mut(address..address + 4)
            .map(|slot| slot.copy_from_slice(&len))
    }

    fn len_address(&self) -> LenAddress {
        LenAddress(self.0 + 4)
    }

    fn address(&self) -> usize {
        self.0 as usize
    }

    fn data_address(&self) -> usize {
        self.0 as usize + 8
    }
}

pub struct Stack<I>(CapAddress, PhantomData<I>);

impl<I> Stack<I>
where
    I: Item,
{
    pub fn new(addr: CapAddress) -> Self {
        Self(addr, PhantomData)
    }

    pub fn peek(&self, memory: &Memory) -> Option<I> {
        let len = self.0.len_address().get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let data = self.0.data_address();
        memory
            .memory
            .get(data + start..data + len)
            .and_then(I::load)
    }

    pub fn push(&self, item: I, memory: &mut Memory) -> Option<()> {
        memory
            .alloc(self.0, I::SIZE)
            .and_then(|slot| item.store(slot))
    }

    pub fn pop(&self, memory: &mut Memory) -> Option<I> {
        let len_address = self.0.len_address();
        let len = len_address.get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let data = self.0.data_address();
        let item = memory
            .memory
            .get(data + start..data + len)
            .and_then(I::load)?;
        len_address.set_len(memory, start)?;
        Some(item)
    }
}

pub struct Memory<'a> {
    memory: &'a mut [u8],
}

impl<'a> Memory<'a> {
    pub fn new(memory: &'a mut [u8]) -> Self {
        Self { memory }
    }

    fn alloc(&mut self, object: CapAddress, size: usize) -> Option<&mut [u8]> {
        let address = object.address();
        let data = object.data_address();
        let header = self.memory.get_mut(address..data)?;

        let cap = u32::from_le_bytes(header[0..4].try_into().ok()?);
        let len = u32::from_le_bytes(header[4..8].try_into().ok()?);

        let new_len = len + size as Offset;
        if new_len <= cap {
            header[4..8].copy_from_slice(&new_len.to_le_bytes());
            let start = data + len as usize;
            self.memory.get_mut(start..start + size)
        } else {
            None
        }
    }
}

//

impl Item for u8 {
    const SIZE: usize = 1;

    fn load(data: &[u8]) -> Option<Self> {
        data.get(0).copied()
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        data.get_mut(0).map(|slot| *slot = self)
    }
}

//

pub fn push(memory: &mut Memory, stack: &mut Stack<u8>, item: u8) -> Option<()> {
    stack.push(item, memory)
}

pub fn peek(memory: &mut Memory, stack: &mut Stack<u8>) -> Option<u8> {
    stack.peek(memory)
}

pub fn pop(memory: &mut Memory, stack: &mut Stack<u8>) -> Option<u8> {
    stack.pop(memory)
}
