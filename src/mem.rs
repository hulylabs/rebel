// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::WordKind;
use bytemuck::{
    AnyBitPattern, NoUninit, Pod, PodCastError, Zeroable, cast_slice, cast_slice_mut,
    try_cast_slice, try_cast_slice_mut, try_from_bytes, try_from_bytes_mut,
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
}

pub type Word = u32;
pub type Address = Word;
pub type Offset = Word;
pub type Type = Offset;

const SIZE_OF_WORD: usize = std::mem::size_of::<Word>();

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
    cap: Offset, // Total capacity in 32-bit words, including the header
    len: Offset, // Number of items currently in the block
}

impl Block {
    pub const SIZE_IN_WORDS: Offset = (std::mem::size_of::<Block>() / SIZE_OF_WORD) as Offset;

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

pub type String = Series<u8>;

//

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct Value(pub Type, pub Word);

impl Value {
    pub const SIZE_IN_WORDS: Offset = (std::mem::size_of::<Value>() / SIZE_OF_WORD) as Offset;

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
pub struct MemHeader {
    dead_beef: Word,
    heap_top: Address,
    symbol_table: Address,
}

pub struct Memory {
    memory: Box<[Word]>,
}

fn podcast_error(_err: PodCastError) -> MemoryError {
    MemoryError::AlignmentError
}

impl Memory {
    pub fn new(size: usize) -> Result<Self, MemoryError> {
        let words = vec![0u32; size].into_boxed_slice();
        let mut memory = Self { memory: words };

        let header = memory.get_mut::<MemHeader>(0)?;
        header.dead_beef = 0xDEADBEEF;
        header.heap_top = std::mem::size_of::<MemHeader>() as Address;

        let symbol_table = memory.alloc::<Address>(1024)?.address;
        let header = memory.get_mut::<MemHeader>(0)?;
        header.symbol_table = symbol_table;

        Ok(memory)
    }

    fn alloc_words(&mut self, words: Offset) -> Result<Address, MemoryError> {
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

    /// Allocates a new Series with capacity for at least `cap` items of type I.
    ///
    /// Notes about capacity:
    /// - The capacity is in items, not bytes or words
    /// - The actual storage will be rounded up to whole words
    /// - The minimum allocation is Block::SIZE_IN_WORDS (header) + storage for items
    ///
    /// For types smaller than a word (like u8), multiple items can be stored per word.
    /// For types larger than a word (like Value), multiple words are needed per item.
    pub fn alloc<I>(&mut self, cap: Offset) -> Result<Series<I>, MemoryError> {
        // Calculate bytes needed for all items
        let item_size_bytes = std::mem::size_of::<I>();
        let total_bytes_needed = item_size_bytes * cap as usize;

        // Round up to whole words (4-byte units)
        let bytes_per_word = SIZE_OF_WORD;
        let words_needed = total_bytes_needed.div_ceil(bytes_per_word);

        // Allocate the block with header + data
        let address = self.alloc_words(words_needed as Offset)?;
        Ok(Series::new(address))
    }

    #[allow(dead_code)]
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

        let size_in_words = size.div_ceil(SIZE_OF_WORD);
        let size_in_words = size_in_words as Offset;
        let address = self.alloc_words(size_in_words)?;
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

    pub fn get_string(&self, address: Address) -> Result<&str, MemoryError> {
        let block = self.get::<Block>(address)?;
        let len = block.len as usize;
        let size_in_words = len.div_ceil(SIZE_OF_WORD);
        let words_slice = self.get_words_slice(address, 0..size_in_words as Offset)?;
        let bytes_slice = try_cast_slice(words_slice).map_err(podcast_error)?;
        let bytes = &bytes_slice[..len];
        let string = unsafe { std::str::from_utf8_unchecked(bytes) };
        Ok(string)
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
        // To pop from the end of the stack, we need to use new_len (len-1)
        let item_start = new_len * item_size;

        block.len = new_len;
        self.get::<I>(series.address + Block::SIZE_IN_WORDS + item_start)
            .copied()
    }

    pub fn peek<I: AnyBitPattern>(&self, series: Series<I>) -> Result<Option<&I>, MemoryError> {
        let item_size = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let item_size = item_size as Offset;
        let block = self.get::<Block>(series.address)?;

        let len = block.len;
        if len == 0 {
            Ok(None)
        } else {
            let item_offset = len - 1;
            let item_start = item_offset * item_size;

            self.get::<I>(series.address + Block::SIZE_IN_WORDS + item_start)
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
        let item_size = std::mem::size_of::<I>() / SIZE_OF_WORD;
        let item_size = item_size as Offset;

        let from_block = self.get_mut::<Block>(from.address)?;
        let from_len = from_block.len;
        let copy_len = from_len - pos;
        from_block.len = pos;

        let copy_words = copy_len * item_size;
        let to_address = self.alloc_words(copy_words)?;
        let to_block = self.get_mut::<Block>(to_address)?;
        to_block.len = copy_len;

        let items_start = (from.address + Block::SIZE_IN_WORDS) as usize;
        let start = items_start + (pos * item_size) as usize;
        let copy_words = copy_words as usize;

        let end = start + copy_words;
        if end > self.memory.len() {
            return Err(MemoryError::OutOfBounds);
        }

        let dst = (to_address + Block::SIZE_IN_WORDS) as usize;
        if dst > self.memory.len() - copy_words {
            return Err(MemoryError::OutOfBounds);
        }

        self.memory.copy_within(start..end, dst);
        Ok(Series::new(to_address))
    }

    pub fn get_or_add_symbol(&mut self, symbol: &str) -> Result<Series<u8>, MemoryError> {
        let header = self.get_mut::<MemHeader>(0)?;
        let symbol_table = header.symbol_table;

        let block = self.get::<Block>(symbol_table)?;
        let cap = block.cap - Block::SIZE_IN_WORDS;
        let len = block.len;

        let hash_code = {
            let mut hasher = DefaultHasher::new();
            symbol.hash(&mut hasher);
            hasher.finish() as u32
        };

        let start = hash_code % cap;
        let mut idx = start;
        loop {
            let item = self.get::<Address>(symbol_table + Block::SIZE_IN_WORDS + idx)?;
            if *item == 0 {
                let string = self.alloc_string(symbol)?;
                let item = self.get_mut::<Address>(symbol_table + Block::SIZE_IN_WORDS + idx)?;
                *item = string.address();

                let block = self.get_mut::<Block>(symbol_table)?;
                block.len = len + 1;

                return Ok(string);
            } else {
                let string = self.get_string(*item)?;
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
}

// These functions are only used in tests but need to be public

/// Returns the capacity of a block in items (not including header size)
///
/// This calculates how many items of type I can fit in the block's data area.
/// Note that this is different from the block's `cap` field, which is in words.
///
/// Only for testing - not part of the stable API.
#[doc(hidden)]
pub fn capacity_in_items<I>(memory: &Memory, series: Series<I>) -> Result<Offset, MemoryError> {
    let block = memory.get::<Block>(series.address)?;

    // Calculate total available space in bytes (excluding header)
    let data_words = block.cap - Block::SIZE_IN_WORDS;
    let data_bytes = data_words as usize * SIZE_OF_WORD;

    // Calculate how many items of type I can fit in that many bytes
    let item_size_bytes = std::mem::size_of::<I>();
    let capacity_in_items = data_bytes / item_size_bytes;

    Ok(capacity_in_items as Offset)
}

/// Returns the total size of a block in words (including header)
///
/// Only for testing - not part of the stable API.
#[doc(hidden)]
pub fn block_size_in_words<I>(memory: &Memory, series: Series<I>) -> Result<Offset, MemoryError> {
    let block = memory.get::<Block>(series.address)?;
    Ok(block.cap)
}
