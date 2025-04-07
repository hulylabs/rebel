// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use bytemuck::{
    AnyBitPattern, NoUninit, Pod, Zeroable, cast_slice, cast_slice_mut, must_cast_mut,
    must_cast_ref, try_from_bytes, try_from_bytes_mut,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Memory access out of bounds")]
    OutOfBounds,
    #[error("Stack overflow")]
    StackOverflow,
}

pub type Word = u32;
pub type Address = Word;
pub type Offset = Word;
pub type Type = Offset;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Block(Offset, Word);

impl Block {
    fn cap(&self) -> usize {
        self.0 as usize
    }

    fn len(&self) -> usize {
        self.1 as usize
    }

    fn set_len(&mut self, len: usize) {
        self.1 = len as Word;
    }
}

pub struct Series<T> {
    address: Address,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Series<T> {
    fn new(address: Address) -> Self {
        Self {
            address,
            _marker: std::marker::PhantomData,
        }
    }

    fn address(&self) -> usize {
        self.address as usize
    }
}

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Value {
    kind: Type,
    data: Word,
}

pub struct Memory {
    memory: Box<[Word]>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        let memory = vec![0u32; size].into_boxed_slice();
        Self { memory }
    }

    pub fn get<I: AnyBitPattern>(&self, address: usize) -> Option<&I> {
        let size_in_words = std::mem::size_of::<I>() / std::mem::size_of::<Word>();
        let words_slice = self.memory.get(address..address + size_in_words)?;
        try_from_bytes(cast_slice(words_slice)).ok()
    }

    pub fn get_mut<I: Pod>(&mut self, address: usize) -> Option<&mut I> {
        let size_in_words = std::mem::size_of::<I>() / std::mem::size_of::<Word>();
        let words_slice = self.memory.get_mut(address..address + size_in_words)?;
        try_from_bytes_mut(cast_slice_mut(words_slice)).ok()
    }

    pub fn push<I: Pod>(&mut self, series: Series<I>, value: I) -> Option<()> {
        let item_size = std::mem::size_of::<I>() / std::mem::size_of::<Word>();
        let block = self.get_mut::<Block>(series.address())?;

        let new_len = block.len() + 1;
        let item_start = new_len * item_size;
        let item_end = item_start + item_size;

        if block.cap() < item_end {
            None
        } else {
            block.set_len(new_len);
            let item = self.get_mut::<I>(series.address() + item_start)?;
            *item = value;
            Some(())
        }
    }
}

pub fn get(memory: &Memory, offset: usize) -> Option<&Block> {
    memory.get(offset)
}

pub fn push(memory: &mut Memory, series: Series<Value>, value: Value) -> Option<()> {
    memory.push(series, value)
}
