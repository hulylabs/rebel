// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::WordKind;
use bytemuck::{
    AnyBitPattern, NoUninit, Pod, PodCastError, Zeroable, try_cast_slice, try_cast_slice_mut,
    try_from_bytes, try_from_bytes_mut,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
    #[error("Word not found")]
    WordNotFound,
}

pub type Word = u32;
pub type Address = Word;
pub type Offset = Word;
pub type Type = Offset;

// const SIZE_OF_WORD: usize = std::mem::size_of::<Word>();

//

/// Block is the header structure for all allocated Series in the Rebel memory system.
///
/// Every Series has a Block header at its start, containing:
/// - `cap`: Total capacity of the block in Words (u32), including the header itself
/// - `len`: Current length of the block in items (type-dependent)
///
/// Memory layout:
/// ```text
/// +------------------+
/// | Block (8 bytes)  |
/// | - cap: u32       | <- Series.address points here
/// | - len: u32       |
/// +------------------+
/// | Data area...     | <- Items are stored here
/// | (cap - 2) words  |
/// +------------------+
/// ```
///
/// The Series<T> type is just a reference to a Block, it holds an address
/// to the start of the Block and a marker for the contained type.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Block {
    cap: Offset, // Total capacity in bytes, including the header (word-aligned)
    len: Offset, // Number of items currently in the block
}

impl Block {
    pub const SIZE: Offset = std::mem::size_of::<Block>() as Offset;

    /// Returns the current number of items in the block
    pub fn len(&self) -> Offset {
        self.len
    }

    /// Returns true if the block contains no items
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Series represents a reference to a block of homogeneous items in memory.
///
/// Series is a low-level handle to a memory block containing items of a specific type.
/// The Series itself doesn't own the memory, it's just a reference to a block
/// allocated in the Memory system.
///
/// Series are the building blocks for strings, arrays, blocks, and other
/// composite data structures in the Rebel system.
///
/// ## Memory Layout
///
/// A Series is just a reference (address) to a Block in memory.
/// The Block contains a header with capacity and length information,
/// followed by the actual data items.
///
/// ```text
/// +------------------+
/// | Block header     |
/// +------------------+ <- Series.address points here
/// | Item 0           |
/// | Item 1           |
/// | ...              |
/// | Item N-1         |
/// +------------------+
/// ```
///
/// Use the Memory methods (push, pop, len, etc.) to interact with the Series data.
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

    pub fn address(&self) -> Address {
        self.address
    }
}

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct Value(pub Type, pub Word);

impl Value {
    pub const SIZE: Offset = std::mem::size_of::<Value>() as Offset;

    pub const NONE: Type = 0;
    pub const INT: Type = 1;
    pub const BOOL: Type = 2;
    pub const STRING: Type = 3;
    pub const BLOCK: Type = 4;
    pub const WORD: Type = 5;
    pub const SET_WORD: Type = 6;
    pub const GET_WORD: Type = 7;
    pub const PATH: Type = 8;
    pub const FLOAT: Type = 9;
    pub const INTRINSIC: Type = 10;

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

    pub fn float(value: f32) -> Self {
        let bits = value.to_bits();
        Value(Self::FLOAT, bits)
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

    pub fn intrinsic(id: Word) -> Self {
        Value(Self::INTRINSIC, id)
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

    /// Returns true if the value is a float
    pub fn is_float(&self) -> bool {
        self.is_type(Self::FLOAT)
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

    pub fn as_float(&self) -> Result<f32, MemoryError> {
        if self.is_float() {
            Ok(f32::from_bits(self.1))
        } else {
            Err(MemoryError::TypeMismatch)
        }
    }
}

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct KeyValue {
    key: Address,
    value: Value,
}

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MemHeader {
    dead_beef: Word,
    heap_top: Address,
    symbol_table: Address,
    system_words: Address,
}

pub struct Memory {
    memory: Box<[u8]>,
}

fn podcast_error(_err: PodCastError) -> MemoryError {
    MemoryError::AlignmentError
}

impl Memory {
    pub fn new(size: usize) -> Result<Self, MemoryError> {
        let bytes = vec![0u8; size].into_boxed_slice();
        let mut memory = Self { memory: bytes };

        let header = memory.get_mut::<MemHeader>(0)?;
        header.dead_beef = 0xDEADBEEF;
        header.heap_top = std::mem::size_of::<MemHeader>() as Address;

        let symbol_table = memory.alloc::<Address>(1024)?.address;
        let header = memory.get_mut::<MemHeader>(0)?;
        header.symbol_table = symbol_table;

        let system_words = memory.alloc::<KeyValue>(1024)?.address;
        let header = memory.get_mut::<MemHeader>(0)?;
        header.system_words = system_words;

        Ok(memory)
    }

    fn heap_alloc(
        &mut self,
        size_in_bytes: Offset,
        init_len: Offset,
    ) -> Result<Address, MemoryError> {
        let len = self.memory.len() as Offset;
        let cap = Block::SIZE + size_in_bytes;
        let cap = (cap + 3) & !3;
        let header = self.get_mut::<MemHeader>(0)?;
        let heap_top = header.heap_top;
        let new_heap_top = heap_top + cap;
        if new_heap_top > len {
            Err(MemoryError::OutOfMemory)
        } else {
            header.heap_top = new_heap_top;
            let block = self.get_mut::<Block>(heap_top)?;
            block.cap = cap;
            block.len = init_len;
            Ok(heap_top)
        }
    }

    /// Allocates a new Series with capacity for at least `cap` items of type I.
    ///
    /// Notes about capacity:
    /// - The capacity is in items, not bytes or words
    /// - The actual storage will be rounded up to whole words
    /// - The minimum allocation is Block::SIZE_IN_WORDS (header) + storage for items
    ///
    /// For types smaller than a word (like u8), multiple items can be stored per word.
    /// For types larger than a word (like Value), multiple words are needed per item.
    pub fn alloc<I>(&mut self, items: Offset) -> Result<Series<I>, MemoryError> {
        let item_size = std::mem::size_of::<I>() as Offset;
        let address = self.heap_alloc(item_size * items, 0)?;
        Ok(Series::new(address))
    }

    pub fn alloc_items<I: NoUninit + AnyBitPattern>(
        &mut self,
        items: &[I],
    ) -> Result<Series<I>, MemoryError> {
        let len = items.len() as Offset;
        let size_in_bytes = len * std::mem::size_of::<I>() as Offset;
        let address = self.heap_alloc(size_in_bytes, len)?;

        let bytes = self.get_byte_slice_mut(address, 0..size_in_bytes)?;
        let target = try_cast_slice_mut(bytes).map_err(podcast_error)?;

        let iter = target.iter_mut().zip(items.iter());
        for (dst, src) in iter {
            *dst = *src
        }

        Ok(Series::new(address))
    }

    #[allow(dead_code)]
    fn get_byte_slice(&self, address: Address, range: Range<Offset>) -> Result<&[u8], MemoryError> {
        let start = address + Block::SIZE + range.start;
        let end = address + Block::SIZE + range.end;
        self.memory
            .get(start as usize..end as usize)
            .ok_or(MemoryError::OutOfBounds)
    }

    fn get_byte_slice_mut(
        &mut self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&mut [u8], MemoryError> {
        let start = address + Block::SIZE + range.start;
        let end = address + Block::SIZE + range.end;
        self.memory
            .get_mut(start as usize..end as usize)
            .ok_or(MemoryError::OutOfBounds)
    }

    fn get_items_slice<I: AnyBitPattern>(
        &self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&[I], MemoryError> {
        let item_size = std::mem::size_of::<I>() as Offset;
        let range = range.start * item_size..range.end * item_size;
        let bytes = self.get_byte_slice(address, range)?;
        try_cast_slice(bytes).map_err(podcast_error)
    }

    fn get_items_slice_mut<I: AnyBitPattern + NoUninit>(
        &mut self,
        address: Address,
        range: Range<Offset>,
    ) -> Result<&mut [I], MemoryError> {
        let item_size = std::mem::size_of::<I>() as Offset;
        let range = range.start * item_size..range.end * item_size;
        let bytes = self.get_byte_slice_mut(address, range)?;
        try_cast_slice_mut(bytes).map_err(podcast_error)
    }

    pub fn alloc_string(&mut self, string: &str) -> Result<Series<u8>, MemoryError> {
        self.alloc_items(string.as_bytes())
    }

    pub fn get_string(&self, string: Series<u8>) -> Result<&str, MemoryError> {
        let bytes = self.get_items(string)?;
        let string = unsafe { std::str::from_utf8_unchecked(bytes) };
        Ok(string)
    }

    pub fn get_items<I: AnyBitPattern>(&self, series: Series<I>) -> Result<&[I], MemoryError> {
        let address = series.address;
        let block = self.get::<Block>(address)?;
        let len = block.len;
        self.get_items_slice(address, 0..len)
    }

    pub fn get<I: AnyBitPattern>(&self, address: Address) -> Result<&I, MemoryError> {
        let address = address as usize;
        let bytes = self
            .memory
            .get(address..address + std::mem::size_of::<I>())
            .ok_or(MemoryError::OutOfBounds)?;
        try_from_bytes(bytes).map_err(podcast_error)
    }

    pub fn get_mut<I: AnyBitPattern + NoUninit>(
        &mut self,
        address: Address,
    ) -> Result<&mut I, MemoryError> {
        let address = address as usize;
        let bytes = self
            .memory
            .get_mut(address..address + std::mem::size_of::<I>())
            .ok_or(MemoryError::OutOfBounds)?;
        try_from_bytes_mut(bytes).map_err(podcast_error)
    }

    pub fn get_u8(&self, address: usize) -> Option<u8> {
        self.memory.get(address).copied()
    }

    pub fn get_u32_ne(&self, address: usize) -> Option<Word> {
        let bytes = self.memory.get(address..address + 4)?;
        let word = u32::from_ne_bytes(bytes.try_into().ok()?);
        Some(word)
    }

    pub fn len<I>(&self, series: Series<I>) -> Result<Offset, MemoryError> {
        self.get::<Offset>(series.address + 4).copied()
    }

    pub fn push<I: AnyBitPattern + NoUninit>(
        &mut self,
        series: Series<I>,
        value: I,
    ) -> Result<(), MemoryError> {
        let block = self.get_mut::<Block>(series.address)?;
        let len = block.len;
        let item_size = std::mem::size_of::<I>() as Offset;
        let item_start = len * item_size;
        let item_end = item_start + item_size;
        if block.cap < item_end {
            Err(MemoryError::StackOverflow)
        } else {
            block.len = len + 1;
            let item = self.get_mut::<I>(series.address + Block::SIZE + item_start)?;
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

        let item_size = std::mem::size_of::<I>() as Offset;
        let cap_items = (cap - Block::SIZE) / item_size;
        let items_len = values.len() as Offset;
        let new_len = len + items_len;

        if new_len > cap_items {
            Err(MemoryError::StackOverflow)
        } else {
            block.len = new_len;
            let items = self.get_items_slice_mut(series.address, len..new_len)?;
            let iter = items.iter_mut().zip(values.iter());
            for (dst, src) in iter {
                *dst = *src
            }
            Ok(())
        }
    }

    pub fn push_n<const N: usize, I: AnyBitPattern + NoUninit>(
        &mut self,
        series: Series<I>,
        values: &[I; N],
    ) -> Result<(), MemoryError> {
        let block = self.get_mut::<Block>(series.address)?;
        let cap = block.cap;
        let len = block.len;

        let item_size = std::mem::size_of::<I>() as Offset;
        let cap_items = (cap - Block::SIZE) / item_size;
        let items_len = N as Offset;
        let new_len = len + items_len;

        if new_len > cap_items {
            Err(MemoryError::StackOverflow)
        } else {
            block.len = new_len;
            let items = self.get_items_slice_mut(series.address, len..new_len)?;
            if items.len() != N {
                return Err(MemoryError::OutOfBounds);
            }
            for i in 0..N {
                items[i] = values[i];
            }
            Ok(())
        }
    }

    pub fn pop<I: AnyBitPattern>(&mut self, series: Series<I>) -> Result<I, MemoryError> {
        let item_size = std::mem::size_of::<I>() as Offset;
        let block = self.get_mut::<Block>(series.address)?;
        let len = block.len;
        let new_len = len.checked_sub(1).ok_or(MemoryError::StackUnderflow)?;

        let item_start = new_len * item_size;

        block.len = new_len;
        self.get::<I>(series.address + Block::SIZE + item_start)
            .copied()
    }

    pub fn peek<I: AnyBitPattern>(&self, series: Series<I>) -> Result<Option<&I>, MemoryError> {
        let item_size = std::mem::size_of::<I>() as Offset;
        let block = self.get::<Block>(series.address)?;
        let len = block.len;

        if len == 0 {
            Ok(None)
        } else {
            let item_offset = len - 1;
            let item_start = item_offset * item_size;
            self.get::<I>(series.address + Block::SIZE + item_start)
                .map(Some)
        }
    }

    pub fn peek_at<I: AnyBitPattern>(
        &self,
        series: Series<I>,
        pos: Offset,
    ) -> Result<&[I], MemoryError> {
        let block = self.get::<Block>(series.address)?;
        let len = block.len;
        if pos >= len {
            Err(MemoryError::OutOfBounds)
        } else {
            self.get_items_slice(series.address, pos..len)
        }
    }

    pub fn drain<I: AnyBitPattern + NoUninit>(
        &mut self,
        from: Series<I>,
        pos: Offset,
    ) -> Result<Series<I>, MemoryError> {
        let item_size = std::mem::size_of::<I>() as Offset;

        let from_block = self.get_mut::<Block>(from.address)?;
        let from_len = from_block.len;
        let copy_len = from_len - pos;
        from_block.len = pos;

        let copy_bytes = copy_len * item_size;
        let to_address = self.heap_alloc(copy_bytes, copy_len)?;

        let start = from.address + Block::SIZE + (pos * item_size);
        let end = start + copy_bytes;
        let start = start as usize;
        let end = end as usize;

        if end > self.memory.len() {
            return Err(MemoryError::OutOfBounds);
        }

        let dst = to_address + Block::SIZE;
        let dst = dst as usize;
        let copy_bytes = copy_bytes as usize;

        if dst > self.memory.len() - copy_bytes {
            return Err(MemoryError::OutOfBounds);
        }

        self.memory.copy_within(start..end, dst);
        Ok(Series::new(to_address))
    }

    pub fn drop<I>(&mut self, series: Series<I>, items: Offset) -> Result<(), MemoryError> {
        let block = self.get_mut::<Block>(series.address)?;
        let new_len = block
            .len
            .checked_sub(items)
            .ok_or(MemoryError::StackUnderflow)?;
        block.len = new_len;
        Ok(())
    }

    pub fn get_or_add_symbol(&mut self, symbol: &str) -> Result<Series<u8>, MemoryError> {
        let header = self.get_mut::<MemHeader>(0)?;
        let symbol_table = header.symbol_table;

        let size_of_symbol = std::mem::size_of::<Address>() as Offset;

        let block = self.get::<Block>(symbol_table)?;
        let cap = (block.cap - Block::SIZE) / size_of_symbol;
        if cap == 0 {
            return Err(MemoryError::OutOfMemory);
        }

        let hash_code = {
            let mut hasher = DefaultHasher::new();
            symbol.hash(&mut hasher);
            hasher.finish() as u32
        };

        let start = hash_code % cap;
        let mut idx = start;
        loop {
            let item = self.get::<Address>(symbol_table + Block::SIZE + idx * size_of_symbol)?;
            if *item == 0 {
                let string = self.alloc_string(symbol)?;
                let item =
                    self.get_mut::<Address>(symbol_table + Block::SIZE + idx * size_of_symbol)?;
                *item = string.address();

                let block = self.get_mut::<Block>(symbol_table)?;
                block.len += 1;

                return Ok(string);
            } else {
                let string = self.get_string(Series::new(*item))?;
                if string == symbol {
                    return Ok(Series::new(*item));
                }
                idx += 1;
                if idx >= cap {
                    idx = 0;
                }
                if idx == start {
                    return Err(MemoryError::OutOfMemory);
                }
            }
        }
    }

    const PHI: u32 = 0x9e3779b9;

    pub fn get_word(&self, symbol: Address) -> Result<Value, MemoryError> {
        const KV_SIZE: Offset = std::mem::size_of::<KeyValue>() as Offset;

        let header = self.get::<MemHeader>(0)?;
        let system_words = header.system_words;

        let block = self.get::<Block>(system_words)?;
        let cap = (block.cap - Block::SIZE) / KV_SIZE;
        if cap == 0 {
            return Err(MemoryError::WordNotFound);
        }

        let hash_code = symbol.wrapping_mul(Self::PHI);
        let start = hash_code % cap;
        let mut idx = start;
        loop {
            let offset = system_words + Block::SIZE + idx * KV_SIZE;
            let item = self.get::<KeyValue>(offset)?;
            if item.key == symbol {
                return Ok(item.value);
            } else if item.key == 0 {
                return Err(MemoryError::WordNotFound);
            } else {
                idx += 1;
                if idx >= cap {
                    idx = 0;
                }
                if idx == start {
                    return Err(MemoryError::WordNotFound);
                }
            }
        }
    }

    pub fn set_word_str(&mut self, symbol: &str, value: Value) -> Result<(), MemoryError> {
        let symbol = self.get_or_add_symbol(symbol)?;
        self.set_word(symbol.address, value)
    }

    pub fn set_word(&mut self, symbol: Address, value: Value) -> Result<(), MemoryError> {
        const KV_SIZE: Offset = std::mem::size_of::<KeyValue>() as Offset;

        let header = self.get_mut::<MemHeader>(0)?;
        let system_words = header.system_words;

        let block = self.get::<Block>(system_words)?;
        let cap = (block.cap - Block::SIZE) / KV_SIZE;
        if cap == 0 {
            return Err(MemoryError::OutOfMemory);
        }

        let hash_code = symbol.wrapping_mul(Self::PHI);
        let start = hash_code % cap;
        let mut idx = start;
        loop {
            let offset = system_words + Block::SIZE + idx * KV_SIZE;
            let item = self.get_mut::<KeyValue>(offset)?;
            if item.key == symbol {
                item.value = value;
                return Ok(());
            } else if item.key == 0 {
                item.key = symbol;
                item.value = value;
                let block = self.get_mut::<Block>(system_words)?;
                block.len += 1;
                return Ok(());
            } else {
                idx += 1;
                if idx >= cap {
                    idx = 0;
                }
                if idx == start {
                    return Err(MemoryError::OutOfMemory);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_allocation() -> Result<(), MemoryError> {
        let mut memory = Memory::new(65536)?;
        let data = b"Hello, world!";
        let series = memory.alloc_items(data)?;
        let bytes = memory.get_items(series)?;
        assert_eq!(bytes, data);
        Ok(())
    }

    // #[test]
    // fn test_memory_push_pop() {
    //     let mut memory = Memory::new(1024).unwrap();
    //     let series: Series<i32> = memory.alloc(10).unwrap();

    //     memory.push(series, 42).unwrap();
    //     assert_eq!(memory.len(series).unwrap(), 1);

    //     let value = memory.pop::<i32>(series).unwrap();
    //     assert_eq!(value, 42);
    //     assert_eq!(memory.len(series).unwrap(), 0);
    // }
}
