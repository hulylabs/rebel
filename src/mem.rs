// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::WordKind;
use bytemuck::{
    AnyBitPattern, NoUninit, Pod, PodCastError, Zeroable, cast_slice, cast_slice_mut,
    try_cast_slice, try_cast_slice_mut, try_from_bytes, try_from_bytes_mut,
};
use std::ops::Range;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Alignment error")]
    AlignmentError,
    #[error("Memory access out of bounds")]
    OutOfBounds,
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Type mismatch")]
    TypeMismatch,
    #[error("Out of memory")]
    OutOfMemory,
}

pub type Word = u32;
pub type Address = Word;
pub type Offset = Word;
pub type Type = Offset;

const SIZE_OF_WORD: usize = std::mem::size_of::<Word>();

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Block {
    cap: Offset,
    len: Offset,
}

impl Block {
    const SIZE_IN_WORDS: Offset = (std::mem::size_of::<Block>() / SIZE_OF_WORD) as Offset;
}

#[derive(Debug, Clone, Copy)]
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
}

pub type String = Series<u8>;

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Value(Type, Word);

impl Value {
    pub const NONE: Type = 0;
    pub const INT: Type = 1;
    pub const BOOL: Type = 2;
    pub const STRING: Type = 3;
    pub const BLOCK: Type = 4;
    pub const WORD: Type = 5;
    pub const SET_WORD: Type = 6;
    pub const GET_WORD: Type = 7;
    pub const PATH: Type = 8;

    pub fn new(kind: Type, data: Word) -> Self {
        Self(kind, data)
    }

    pub fn kind(&self) -> Type {
        self.0
    }

    pub fn data(&self) -> Word {
        self.1
    }

    pub fn none() -> Self {
        Value(Self::NONE, 0)
    }

    pub fn int(value: i32) -> Self {
        Value(Self::INT, value as Word)
    }

    pub fn bool(value: bool) -> Self {
        Value(Self::BOOL, value as Word)
    }

    pub fn string(value: Series<u8>) -> Self {
        Value(Self::STRING, value.address)
    }

    pub fn block(value: Series<Value>) -> Self {
        Value(Self::BLOCK, value.address)
    }

    pub fn path(value: Series<Value>) -> Self {
        Value(Self::PATH, value.address)
    }

    /// Returns true if the value is of the given type
    pub fn is_type(&self, kind: Type) -> bool {
        self.kind() == kind
    }

    /// Returns true if the value is a block
    pub fn is_block(&self) -> bool {
        self.is_type(Self::BLOCK)
    }

    /// Returns true if the value is an integer
    pub fn is_int(&self) -> bool {
        self.is_type(Self::INT)
    }

    /// Returns true if the value is a string
    pub fn is_string(&self) -> bool {
        self.is_type(Self::STRING)
    }

    /// Returns true if the value is a word
    pub fn is_word(&self) -> bool {
        self.is_type(Self::WORD)
    }

    /// Returns true if the value is a path
    pub fn is_path(&self) -> bool {
        self.is_type(Self::PATH)
    }

    pub fn any_word(kind: WordKind, symbol: Series<u8>) -> Self {
        let typ = match kind {
            WordKind::Word => Self::WORD,
            WordKind::SetWord => Self::SET_WORD,
            WordKind::GetWord => Self::GET_WORD,
        };
        Value(typ, symbol.address)
    }

    pub fn as_block(&self) -> Result<Series<Value>, MemoryError> {
        if self.is_block() {
            Ok(Series::new(self.1))
        } else {
            Err(MemoryError::TypeMismatch)
        }
    }

    pub fn as_string(&self) -> Result<Series<u8>, MemoryError> {
        if self.is_string() {
            Ok(Series::new(self.1))
        } else {
            Err(MemoryError::TypeMismatch)
        }
    }

    pub fn as_path(&self) -> Result<Series<Value>, MemoryError> {
        if self.is_path() {
            Ok(Series::new(self.1))
        } else {
            Err(MemoryError::TypeMismatch)
        }
    }

    pub fn as_int(&self) -> Result<i32, MemoryError> {
        if self.is_int() {
            Ok(self.1 as i32)
        } else {
            Err(MemoryError::TypeMismatch)
        }
    }
}

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MemHeader {
    dead_beef: Word,
    heap_top: Address,
}

pub struct Memory {
    memory: Box<[Word]>,
}

fn podcast_error(_err: PodCastError) -> MemoryError {
    MemoryError::AlignmentError
}

impl Memory {
    pub fn new(size: usize) -> Self {
        let words = vec![0u32; size].into_boxed_slice();
        let mut memory = Self { memory: words };

        let header = memory.get_mut::<MemHeader>(0).unwrap();
        header.dead_beef = 0xDEADBEEF;
        header.heap_top = std::mem::size_of::<MemHeader>() as Address;

        memory
    }

    fn alloc(&mut self, words: Offset) -> Result<Address, MemoryError> {
        let len = self.memory.len() as Offset;
        let cap = Block::SIZE_IN_WORDS + words;
        let header = self.get_mut::<MemHeader>(0)?;
        let heap_top = header.heap_top;
        let new_heap_top = heap_top + cap;
        if new_heap_top > len {
            Err(MemoryError::OutOfMemory)
        } else {
            header.heap_top = new_heap_top;
            let block = self.get_mut::<Block>(heap_top)?;
            block.cap = cap;
            block.len = 0;
            Ok(heap_top)
        }
    }

    fn get_words_slice(
        &self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&[Word], MemoryError> {
        let start = address + Block::SIZE_IN_WORDS + range.start;
        let end = address + Block::SIZE_IN_WORDS + range.end;
        self.memory
            .get(start as usize..end as usize)
            .ok_or(MemoryError::OutOfBounds)
    }

    fn get_words_slice_mut(
        &mut self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&mut [Word], MemoryError> {
        let start = address + Block::SIZE_IN_WORDS + range.start;
        let end = address + Block::SIZE_IN_WORDS + range.end;
        self.memory
            .get_mut(start as usize..end as usize)
            .ok_or(MemoryError::OutOfBounds)
    }

    fn get_items_slice<I: AnyBitPattern>(
        &self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&[I], MemoryError> {
        let size_in_words = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let size_in_words = size_in_words as Offset;
        let range = range.start * size_in_words..range.end * size_in_words;
        let words_slice = self.get_words_slice(address, range)?;
        try_cast_slice(words_slice).map_err(podcast_error)
    }

    fn get_items_slice_mut<I: AnyBitPattern + NoUninit>(
        &mut self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&mut [I], MemoryError> {
        let size_in_words = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let size_in_words = size_in_words as Offset;
        let range = range.start * size_in_words..range.end * size_in_words;
        let words_slice = self.get_words_slice_mut(address, range)?;
        try_cast_slice_mut(words_slice).map_err(podcast_error)
    }

    pub fn alloc_string(&mut self, string: &str) -> Result<String, MemoryError> {
        let bytes = string.as_bytes();
        let size = bytes.len();

        let size_in_words = (size + SIZE_OF_WORD - 1) / SIZE_OF_WORD;
        let size_in_words = size_in_words as Offset;
        let address = self.alloc(size_in_words)?;
        let block = self.get_mut::<Block>(address)?;
        block.len = size as Offset;

        let words_slice = self.get_words_slice_mut(address, 0..size_in_words)?;
        let bytes_slice = cast_slice_mut(words_slice);
        let iter = bytes_slice.iter_mut().zip(bytes.iter());
        for (dst, src) in iter {
            *dst = *src
        }

        Ok(String::new(address))
    }

    pub fn get<I: AnyBitPattern>(&self, address: Address) -> Result<&I, MemoryError> {
        let address = address as usize;
        let size_in_words = std::mem::size_of::<I>() / std::mem::size_of::<Word>();
        let words_slice = self
            .memory
            .get(address..address + size_in_words)
            .ok_or(MemoryError::OutOfBounds)?;
        try_from_bytes(cast_slice(words_slice)).map_err(podcast_error)
    }

    pub fn get_mut<I: AnyBitPattern + NoUninit>(
        &mut self,
        address: Address,
    ) -> Result<&mut I, MemoryError> {
        let address = address as usize;
        let size_in_words = std::mem::size_of::<I>() / std::mem::size_of::<Word>();
        let words_slice = self
            .memory
            .get_mut(address..address + size_in_words)
            .ok_or(MemoryError::OutOfBounds)?;
        try_from_bytes_mut(cast_slice_mut(words_slice)).map_err(podcast_error)
    }

    pub fn len<I>(&self, series: Series<I>) -> Result<Offset, MemoryError> {
        let block = self.get::<Block>(series.address)?;
        Ok(block.len)
    }

    pub fn push<I: AnyBitPattern + NoUninit>(
        &mut self,
        series: Series<I>,
        value: I,
    ) -> Result<(), MemoryError> {
        let item_size = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let item_size = item_size as Offset;
        let block = self.get_mut::<Block>(series.address)?;

        let len = block.len;
        let item_start = len * item_size;
        let item_end = item_start + item_size;

        if block.cap < item_end {
            Err(MemoryError::StackOverflow)
        } else {
            block.len = len + 1;
            let item = self.get_mut::<I>(series.address + Block::SIZE_IN_WORDS + item_start)?;
            *item = value;
            Ok(())
        }
    }

    pub fn push_all<I: AnyBitPattern + NoUninit>(
        &mut self,
        series: Series<I>,
        values: &[I],
    ) -> Result<(), MemoryError> {
        let block = self.get_mut::<Block>(series.address)?;
        let cap = block.cap;
        let len = block.len;

        let item_size = (std::mem::size_of::<I>() / SIZE_OF_WORD) as Offset;
        let cap_items = (cap - Block::SIZE_IN_WORDS) / item_size;
        let items_len = values.len() as Offset;

        if len + items_len > cap_items {
            Err(MemoryError::StackOverflow)
        } else {
            let new_len = len + items_len;
            block.len = new_len;
            let items = self.get_items_slice_mut(series.address, len..new_len)?;
            let iter = items.iter_mut().zip(values.iter());
            for (dst, src) in iter {
                *dst = *src
            }
            Ok(())
        }
    }

    pub fn pop<I: AnyBitPattern>(&mut self, series: Series<I>) -> Result<I, MemoryError> {
        let item_size = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let item_size = item_size as Offset;
        let block = self.get_mut::<Block>(series.address)?;

        let len = block.len;
        let new_len = len.checked_sub(1).ok_or(MemoryError::StackUnderflow)?;
        let item_start = len * item_size;

        block.len = new_len;
        self.get::<I>(series.address + Block::SIZE_IN_WORDS + item_start)
            .copied()
    }

    pub fn drain_to<I: AnyBitPattern + NoUninit>(
        &mut self,
        from: Series<I>,
        pos: Offset,
        to: Series<I>,
    ) -> Result<(), MemoryError> {
        let item_size = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let item_size = item_size as Offset;

        let from_block = self.get_mut::<Block>(from.address)?;
        let from_len = from_block.len;
        let copy_len = from_len - pos;
        from_block.len = pos;

        let to_block = self.get_mut::<Block>(to.address)?;
        let to_cap = to_block.cap;
        let to_len = to_block.len;

        let new_to_len = to_len + copy_len;
        let to_cap_items = (to_cap - Block::SIZE_IN_WORDS) / item_size;
        if new_to_len > to_cap_items {
            return Err(MemoryError::StackOverflow);
        }
        to_block.len = new_to_len;

        let items_start = (from.address + Block::SIZE_IN_WORDS) as usize;
        let start = items_start + (pos * item_size) as usize;
        let words_to_copy = (copy_len * item_size) as usize;

        let end = start + words_to_copy;
        if end > self.memory.len() {
            return Err(MemoryError::OutOfBounds);
        }

        let dst = items_start + (to_len * item_size) as usize;
        if dst > self.memory.len() - words_to_copy {
            return Err(MemoryError::OutOfBounds);
        }

        self.memory.copy_within(start..end, dst as usize);

        Ok(())
    }
}

pub fn get(memory: &Memory, offset: Address) -> Result<&Block, MemoryError> {
    memory.get(offset)
}

pub fn push(memory: &mut Memory, series: Series<Value>, value: Value) -> Result<(), MemoryError> {
    memory.push(series, value)
}

pub fn push_all(
    memory: &mut Memory,
    series: Series<Value>,
    values: &[Value],
) -> Result<(), MemoryError> {
    memory.push_all(series, values)
}

pub fn pop(memory: &mut Memory, series: Series<Value>) -> Result<Value, MemoryError> {
    memory.pop(series)
}

pub fn drain_to(
    memory: &mut Memory,
    from: Series<Value>,
    pos: Offset,
    to: Series<Value>,
) -> Result<(), MemoryError> {
    memory.drain_to(from, pos, to)
}
