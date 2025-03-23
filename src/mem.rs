// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

//! Memory management system for the Rebel interpreter.
//!
//! This module provides low-level memory management primitives for the Rebel VM,
//! including:
//! - Tagged value representation (MemValue)
//! - Memory abstractions (Slice, Stack, Block, etc.)
//! - Memory allocation and access (Arena, Memory)
//! - Symbol table for efficient word storage

use crate::parse::{Collector, WordKind};
use std::marker::PhantomData;
use thiserror::Error;
use xxhash_rust::xxh32::xxh32;

/// Errors that can occur during memory operations
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Tag provided is not a valid tag
    #[error("invalid tag")]
    InvalidTag,
    /// Operation would access memory out of bounds
    #[error("out of bounds")]
    OutOfBounds,
    /// Error converting slice to array
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),
}

/// 32-bit word, the basic unit of memory allocation
pub type Word = u32;
/// Memory offset, measured in words
pub type Offset = Word;

/// Trait for types that can be stored in and loaded from memory
pub trait Item: Sized {
    /// Size of the item in bytes
    const SIZE: Offset;

    /// Load an item from a byte slice
    fn load(data: &[u8]) -> Option<Self>;

    /// Store an item into a byte slice
    fn store(self, data: &mut [u8]) -> Option<()>;
}

fn u32_slice_to_u8_slice(slice: &[u32]) -> &[u8] {
    let ptr = slice.as_ptr() as *const u8;
    let len = std::mem::size_of_val(slice);
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn u32_slice_to_u8_slice_mut(slice: &mut [u32]) -> &mut [u8] {
    let ptr = slice.as_mut_ptr() as *mut u8;
    let len = std::mem::size_of_val(slice);
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}

/// Address pointing to a length-prefixed block of memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LenAddress(pub Offset);

impl LenAddress {
    /// Get the length of the block in bytes
    pub fn get_len(&self, memory: &Memory) -> Option<Word> {
        memory.get_word(self.address())
    }

    /// Set the length of the block in bytes
    pub fn set_len(&self, len: Word, memory: &mut Memory) -> Option<()> {
        memory.set_word(self.address(), len)
    }

    /// Get the address of the length field
    pub fn address(&self) -> Offset {
        self.0
    }

    /// Get the address of the data following the length field
    pub fn data_address(&self) -> Offset {
        self.0 + 1
    }

    /// Get the data as a byte slice
    pub fn get_data<'a>(&self, memory: &'a Memory) -> Option<&'a [u8]> {
        let len = self.get_len(memory)?; // in bytes
        let words = (len + 3) / 4; // Round up to word boundary
        let data = memory.get(self.data_address(), words)?;
        // Safety: We're reinterpreting Word array as bytes, which is safe as
        // we're only returning a slice up to the actual length in bytes
        let data = u32_slice_to_u8_slice(data);
        data.get(..len as usize)
    }

    /// Get the data as a mutable byte slice
    pub fn get_data_mut<'a>(&self, memory: &'a mut Memory) -> Option<&'a mut [u8]> {
        let len = self.get_len(memory)?; // in bytes
        let words = (len + 3) / 4; // Round up to word boundary
        let data = memory.get_mut(self.data_address(), words)?;
        // Safety: We're reinterpreting Word array as bytes, which is safe as
        // we're only returning a slice up to the actual length in bytes
        let data = u32_slice_to_u8_slice_mut(data);
        data.get_mut(..len as usize)
    }
}

/// Address pointing to a capacity-prefixed region of memory
#[derive(Debug, Clone)]
pub struct CapAddress(pub Offset);

impl CapAddress {
    /// Get the capacity in words, not including header (cap, len)
    pub fn get_cap(&self, memory: &Memory) -> Option<Word> {
        memory.get_word(self.address())
    }

    /// Allocate a slot of the given size in bytes
    pub fn alloc_slot<'a>(&self, size_bytes: Word, memory: &'a mut Memory) -> Option<&'a mut [u8]> {
        // Get all the information we need up front to avoid double borrow issues
        let cap_words = self.get_cap(memory)?;
        let cap_bytes = cap_words * 4;
        let len_address = self.len_address();
        let old_len = len_address.get_len(memory)?;
        let new_len = old_len + size_bytes;

        // Check if we have enough space
        if new_len > cap_bytes {
            return None;
        }

        // Both operations below mutate memory, but we need to sequence them
        // to avoid the double mutable borrow problem

        // 1. Write the new length to memory
        let len_addr = len_address.address();
        memory.set_word(len_addr, new_len)?;

        // 2. Get the data area and create a slice for the newly allocated region
        let data_addr = self.data_address();
        let data = memory.get_mut(data_addr, cap_words)?;
        let bytes = u32_slice_to_u8_slice_mut(data);

        // Return the newly allocated slice
        bytes.get_mut(old_len as usize..new_len as usize)
    }

    /// Reserve a block of the given size in bytes
    pub fn reserve_block(&self, data_len_bytes: Word, memory: &mut Memory) -> Option<LenAddress> {
        let cap_words = self.get_cap(memory)?;
        let cap_bytes = cap_words * 4;

        let len_address = self.len_address();
        let len = len_address.get_len(memory)?;
        let aligned_len = (len + 3) & !3;
        let new_len = aligned_len + data_len_bytes + 4; // 4 bytes for the length field

        if new_len <= cap_bytes {
            len_address.set_len(new_len, memory)?;
            let reserved = len_address.data_address() + (aligned_len / 4);
            memory.init_block(reserved, data_len_bytes)
        } else {
            None
        }
    }

    /// Allocate a block and fill it with the given data
    pub fn alloc_block(&self, data: &[u8], memory: &mut Memory) -> Option<LenAddress> {
        let data_len = data.len() as Offset;
        let block = self.reserve_block(data_len, memory)?;
        let dst = block.get_data_mut(memory)?;
        if dst.len() == data.len() {
            dst.copy_from_slice(data);
            Some(block)
        } else {
            None
        }
    }

    pub fn alloc_cap(&self, cap_words: Word, memory: &mut Memory) -> Option<CapAddress> {
        let len_address = self.len_address();
        let len = len_address.get_len(memory)?;
        let aligned_len = (len + 3) & !3;
        let cap_bytes = cap_words * 4;
        let new_len = aligned_len + cap_bytes + 8; // 2 words for the cap and len fields

        // Check against the arena's capacity, not the new stack's capacity
        let arena_cap_bytes = self.get_cap(memory)? * 4;
        if new_len > arena_cap_bytes {
            None
        } else {
            let offset = aligned_len / 4;
            len_address.set_len(new_len, memory)?;
            memory.init_cap(len_address.data_address() + offset, cap_words)
        }
    }

    /// Get the length address associated with this capacity address
    pub fn len_address(&self) -> LenAddress {
        LenAddress(self.0 + 1)
    }

    /// Get the address of the capacity field
    pub fn address(&self) -> Offset {
        self.0
    }

    /// Get the address of the data following the capacity and length fields
    pub fn data_address(&self) -> Offset {
        self.0 + 2
    }
}

/// Stack data structure for storing items of type I
pub struct Stack<I>(CapAddress, PhantomData<I>);

impl<I> Stack<I>
where
    I: Item,
{
    /// Create a new stack at the given capacity address
    pub fn new(addr: CapAddress) -> Self {
        Self(addr, PhantomData)
    }

    /// Get the number of items in the stack
    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0
            .len_address()
            .get_len(memory)
            .map(|len_bytes| len_bytes / I::SIZE)
    }

    /// Look at the top item without removing it
    pub fn peek(&self, memory: &Memory) -> Option<I> {
        let len_address = self.0.len_address();
        let len = len_address.get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let data = len_address.get_data(memory)?;
        let start = start as usize;
        let end = start + I::SIZE as usize;
        data.get(start..end).and_then(I::load)
    }

    /// Push an item onto the stack
    pub fn push(&self, item: I, memory: &mut Memory) -> Option<()> {
        self.0
            .alloc_slot(I::SIZE, memory)
            .and_then(|slot| item.store(slot))
    }

    /// Pop an item from the stack
    pub fn pop(&self, memory: &mut Memory) -> Option<I> {
        let len_address = self.0.len_address();
        let len = len_address.get_len(memory)?;
        let start = len.checked_sub(I::SIZE)?;
        let begin = start as usize;
        let end = begin + I::SIZE as usize;
        let item = len_address
            .get_data(memory)?
            .get(begin..end)
            .and_then(I::load)?;
        len_address.set_len(start, memory)?;
        Some(item)
    }

    /// Get an item at the given index
    pub fn get(&self, index: Word, memory: &Memory) -> Option<I> {
        let len_address = self.0.len_address();
        let data = len_address.get_data(memory)?;
        let start = (index * I::SIZE) as usize;
        let end = start + I::SIZE as usize;
        data.get(start..end).and_then(I::load)
    }

    /// Cut a block from the stack and move it to the given destination
    pub fn cut_block(&self, to: CapAddress, items: Word, memory: &mut Memory) -> Option<Block<I>> {
        let size_bytes = items * I::SIZE;
        let dst = to.reserve_block(size_bytes, memory)?;
        memory.move_items(dst, self.0.len_address(), size_bytes)?;
        Some(Block::new(dst))
    }
}

/// Block data structure for storing a sequence of items of type I
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block<I>(LenAddress, PhantomData<I>);

impl<I> Block<I>
where
    I: Item,
{
    /// Create a new block at the given length address
    pub fn new(addr: LenAddress) -> Self {
        Self(addr, PhantomData)
    }

    /// Get the number of items in the block
    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0.get_len(memory).map(|x| x / I::SIZE)
    }

    /// Get an item at the given index
    pub fn get(&self, index: Word, memory: &Memory) -> Option<I> {
        let data = self.0.get_data(memory)?;
        let start = (index * I::SIZE) as usize;
        let end = start + I::SIZE as usize;
        data.get(start..end).and_then(I::load)
    }

    pub fn set(&self, index: Word, value: I, memory: &mut Memory) -> Option<()> {
        let data = self.0.get_data_mut(memory)?;
        let start = (index * I::SIZE) as usize;
        let end = start + I::SIZE as usize;
        data.get_mut(start..end).and_then(|slot| value.store(slot))
    }
}

/// String data structure for storing UTF-8 encoded text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Str(LenAddress);

impl Str {
    /// Get the length of the string in bytes
    pub fn len(&self, memory: &Memory) -> Option<Word> {
        self.0.get_len(memory)
    }

    /// Get the string data as a byte slice
    pub fn as_bytes<'a>(&self, memory: &'a Memory) -> Option<&'a [u8]> {
        self.0.get_data(memory)
    }
}

/// Memory arena for allocating objects
#[derive(Debug, Clone)]
pub struct Arena(CapAddress);

impl Arena {
    /// Allocate a string in the arena
    pub fn alloc_string(&self, memory: &mut Memory, string: &str) -> Option<Str> {
        self.0.alloc_block(string.as_bytes(), memory).map(Str)
    }

    /// Allocate a new stack in the arena with capacity for `cap_items` items of type `I`
    ///
    /// The capacity is specified in number of items, not bytes. The actual memory
    /// allocated will be rounded up to the nearest word boundary.
    ///
    /// # Parameters
    /// * `memory` - The memory to allocate the stack in
    /// * `cap_items` - The capacity of the stack in items
    ///
    /// # Returns
    /// * `Some(Stack<I>)` - A new stack if allocation succeeded
    /// * `None` - If allocation failed (e.g., not enough memory)
    pub fn alloc_stack<I: Item>(&self, memory: &mut Memory, cap_items: Word) -> Option<Stack<I>> {
        // Calculate capacity in words, rounding up to ensure we have enough space
        // for all items plus any necessary padding
        let total_bytes = cap_items * I::SIZE;
        let cap_words = (total_bytes + 3) / 4; // Round up to nearest word
        self.0.alloc_cap(cap_words, memory).map(Stack::new)
    }

    /// Allocate a block containing the given items
    ///
    /// This method allows direct creation of blocks with any type of items that
    /// implement the `Item` trait, without using the parse API.
    ///
    /// # Parameters
    /// * `items` - The items to store in the block
    /// * `memory` - The memory to allocate the block in
    ///
    /// # Returns
    /// * `Some(Block<I>)` - A block containing the items if allocation succeeded
    /// * `None` - If allocation failed (e.g., not enough memory)
    pub fn alloc_block<I: Item + Copy>(
        &self,
        items: &[I],
        memory: &mut Memory,
    ) -> Option<Block<I>> {
        // Calculate the total size in bytes
        let total_size_bytes = I::SIZE * items.len() as Word;

        // Reserve a block of the appropriate size
        let block_addr = self.0.reserve_block(total_size_bytes, memory)?;

        // Get the data area for writing
        let data = block_addr.get_data_mut(memory)?;

        // Write each item to the block
        for (i, &item) in items.iter().enumerate() {
            let start = (i as Word * I::SIZE) as usize;
            let end = start + I::SIZE as usize;

            if let Some(slot) = data.get_mut(start..end) {
                item.store(slot)?;
            } else {
                return None; // Out of bounds
            }
        }

        // Return a Block wrapper for the allocated memory
        Some(Block::new(block_addr))
    }
}

/// Symbol table for efficient string interning
pub struct SymbolTable(CapAddress);

impl SymbolTable {
    /// Hash seed for the xxHash algorithm
    const HASH_SEED: u32 = 0xC0FFEE;

    /// Get a symbol from the table, or insert it if it doesn't exist
    pub fn get_or_insert_symbol(
        &self,
        symbol: &str,
        heap: Arena,
        memory: &mut Memory,
    ) -> Option<LenAddress> {
        let cap = self.0.get_cap(memory)?;
        if cap == 0 {
            return None;
        }
        let len_address = self.0.len_address();
        let count = len_address.get_len(memory)?;

        let bytes = symbol.as_bytes();
        let hash = xxh32(bytes, Self::HASH_SEED);

        let data_address = self.0.data_address();
        let start = hash % cap;
        let mut index = start;

        // Open addressing with linear probing
        loop {
            let entry = memory.get_word(data_address + index)?;
            if entry == 0 {
                // Empty slot, insert new symbol
                let str = heap.alloc_string(memory, symbol)?;
                memory.set_word(data_address + index, str.0.0)?;
                len_address.set_len(count + 1, memory)?;
                return Some(str.0);
            }

            // Check if this is the symbol we're looking for
            let stored = Str(LenAddress(entry));
            if stored.as_bytes(memory)? == bytes {
                return Some(stored.0);
            }

            // Try next slot
            index = (index + 1) % cap;
            if index == start {
                return None; // Table is full
            }
        }
    }
}

pub struct Memory<'a> {
    memory: &'a mut [Word],
}

const LAYOUT_REGIONS: Offset = 4;

impl<'a> Memory<'a> {
    const LAYOUT_OFFSET: Offset = 1;

    const LAYOUT_SYMBOL_TABLE: Offset = 0;
    const LAYOUT_PARSE_STACK: Offset = 1;
    const LAYOUT_PARSE_BASE: Offset = 2;
    const LAYOUT_HEAP: Offset = 3;

    #[allow(clippy::assertions_on_constants)]
    pub fn init(memory: &'a mut [u32], sizes: [Offset; LAYOUT_REGIONS as usize]) -> Option<Self> {
        let mut memory = Self { memory };

        memory.set_word(0, 0x0BAD_F00D)?;
        memory.init_block(Self::LAYOUT_OFFSET, LAYOUT_REGIONS * 4)?;

        debug_assert!((8 + LAYOUT_REGIONS * 4) < 64);
        let mut addr: u32 = 64;

        for (i, size) in sizes.iter().enumerate() {
            let cap = size.checked_sub(8)?;
            let cap_address = memory.init_cap(addr, cap)?;
            memory.set_word(Self::LAYOUT_OFFSET + 1 + i as Offset, cap_address.0)?;
            addr += size;
        }

        Some(memory)
    }

    fn get_region(&self, region: Offset) -> Option<CapAddress> {
        self.get_word(Self::LAYOUT_OFFSET + 1 + region)
            .map(CapAddress)
    }

    pub fn get_symbol_table(&self) -> Option<SymbolTable> {
        self.get_region(Self::LAYOUT_SYMBOL_TABLE).map(SymbolTable)
    }

    pub fn get_parse_stack(&self) -> Option<Stack<VmValue>> {
        self.get_region(Self::LAYOUT_PARSE_STACK).map(Stack::new)
    }

    pub fn get_parse_base(&self) -> Option<Stack<Word>> {
        self.get_region(Self::LAYOUT_PARSE_BASE).map(Stack::new)
    }

    pub fn get_heap(&self) -> Option<Arena> {
        self.get_region(Self::LAYOUT_HEAP).map(Arena)
    }

    fn init_block(&mut self, address: Offset, len: Word) -> Option<LenAddress> {
        let len_address = LenAddress(address);
        len_address.set_len(len, self)?;
        Some(len_address)
    }

    fn init_cap(&mut self, address: Offset, cap: Word) -> Option<CapAddress> {
        self.set_word(address, cap)?;
        let cap_address = CapAddress(address);
        cap_address.len_address().set_len(0, self)?;
        Some(cap_address)
    }

    // move bytes from one block to another. we assume target block has slot allocated already -- we replace the data
    pub fn move_items(&mut self, to: LenAddress, from: LenAddress, size_bytes: Word) -> Option<()> {
        let to_data_address_bytes = to.data_address() * 4;
        let to_data_len_bytes = to.get_len(self)?;
        let to_new_data_len_bytes = to_data_len_bytes.checked_sub(size_bytes)?;

        let from_data_address_bytes = from.data_address() * 4;
        let from_data_len_bytes = from.get_len(self)?;
        let from_new_data_len_bytes = from_data_len_bytes.checked_sub(size_bytes)?;

        let memory_bytes = u32_slice_to_u8_slice_mut(self.memory);

        let src = (from_data_address_bytes + from_new_data_len_bytes) as usize;
        let dst = (to_data_address_bytes + to_new_data_len_bytes) as usize;
        let size = size_bytes as usize;

        if dst + size > memory_bytes.len() || src + size > memory_bytes.len() {
            None
        } else {
            for i in 0..size {
                memory_bytes[dst + i] = memory_bytes[src + i];
            }
            from.set_len(from_new_data_len_bytes, self)?;
            Some(())
        }
    }

    pub fn get_word(&self, address: Offset) -> Option<Word> {
        self.memory.get(address as usize).copied()
    }

    pub fn set_word(&mut self, address: Offset, word: Word) -> Option<()> {
        self.memory
            .get_mut(address as usize)
            .map(|slot| *slot = word)
    }

    pub fn get(&self, start: Offset, len: Offset) -> Option<&[Word]> {
        let start = start as usize;
        let end = start + len as usize;
        self.memory.get(start..end)
    }

    pub fn get_mut(&mut self, start: Offset, len: Offset) -> Option<&mut [Word]> {
        let start = start as usize;
        let end = start + len as usize;
        self.memory.get_mut(start..end)
    }

    // P A R S E  H E L P E R S

    pub fn begin(&mut self) -> Option<()> {
        let len = self.get_parse_stack()?.len(self)?;
        self.get_parse_base()?.push(len, self)
    }

    pub fn end(&mut self) -> Option<Block<VmValue>> {
        let offset = self.get_parse_base()?.pop(self)?;
        let stack = self.get_parse_stack()?;
        let items = stack.len(self).and_then(|len| len.checked_sub(offset))?;
        stack.cut_block(self.get_heap()?.0, items, self)
    }
}

//

impl Item for u8 {
    const SIZE: Offset = 1;

    fn load(data: &[u8]) -> Option<Self> {
        data.first().copied()
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        data.get_mut(0).map(|slot| *slot = self)
    }
}

impl Item for Word {
    const SIZE: Offset = 4;

    fn load(data: &[u8]) -> Option<Self> {
        let bytes = data.try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        if data.len() == 4 {
            let bytes = self.to_le_bytes();
            data.copy_from_slice(&bytes);
            Some(())
        } else {
            None
        }
    }
}

//

type Tag = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemValueAligned(Word, Word);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemValue(Word, Tag);

impl MemValue {
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

    pub fn none() -> Self {
        MemValue(0, Self::TAG_NONE)
    }

    pub fn string(value: Str) -> Self {
        MemValue(value.0.0, Self::TAG_STRING)
    }

    pub fn bool(value: bool) -> Self {
        MemValue(value as Word, Self::TAG_BOOL)
    }

    pub fn int(value: i32) -> Self {
        MemValue(value as Word, Self::TAG_INT)
    }

    pub fn block(value: Block<VmValue>) -> Self {
        MemValue(value.0.0, Self::TAG_BLOCK)
    }

    pub fn context(value: Block<VmValue>) -> Self {
        MemValue(value.0.0, Self::TAG_CONTEXT)
    }

    pub fn path(value: Block<VmValue>) -> Self {
        MemValue(value.0.0, Self::TAG_PATH)
    }

    pub fn word(value: LenAddress) -> Self {
        MemValue(value.0, Self::TAG_WORD)
    }

    pub fn set_word(value: LenAddress) -> Self {
        MemValue(value.0, Self::TAG_SET_WORD)
    }

    pub fn get_word(value: LenAddress) -> Self {
        MemValue(value.0, Self::TAG_GET_WORD)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmValue {
    None,
    Int(i32),
    Bool(bool),
    Block(Block<VmValue>),
    Context(Block<VmValue>),
    Path(Block<VmValue>),
    String(Str),
    Word(LenAddress),
    SetWord(LenAddress),
    GetWord(LenAddress),
}

impl From<VmValue> for MemValue {
    fn from(value: VmValue) -> Self {
        match value {
            VmValue::None => MemValue::none(),
            VmValue::Int(value) => MemValue::int(value),
            VmValue::Bool(value) => MemValue::bool(value),
            VmValue::Block(value) => MemValue::block(value),
            VmValue::Context(value) => MemValue::context(value),
            VmValue::Path(value) => MemValue::path(value),
            VmValue::String(value) => MemValue::string(value),
            VmValue::Word(value) => MemValue::word(value),
            VmValue::SetWord(value) => MemValue::set_word(value),
            VmValue::GetWord(value) => MemValue::get_word(value),
        }
    }
}

impl TryFrom<MemValue> for VmValue {
    type Error = MemoryError;

    fn try_from(value: MemValue) -> Result<Self, Self::Error> {
        let tag = value.1 as Tag;
        match tag {
            MemValue::TAG_NONE => Ok(VmValue::None),
            MemValue::TAG_INT => Ok(VmValue::Int(value.0 as i32)),
            MemValue::TAG_BOOL => Ok(VmValue::Bool(value.0 != 0)),
            MemValue::TAG_BLOCK => Ok(VmValue::Block(Block::new(LenAddress(value.0)))),
            MemValue::TAG_CONTEXT => Ok(VmValue::Context(Block::new(LenAddress(value.0)))),
            MemValue::TAG_PATH => Ok(VmValue::Path(Block::new(LenAddress(value.0)))),
            MemValue::TAG_STRING => Ok(VmValue::String(Str(LenAddress(value.0)))),
            MemValue::TAG_WORD => Ok(VmValue::Word(LenAddress(value.0))),
            MemValue::TAG_SET_WORD => Ok(VmValue::SetWord(LenAddress(value.0))),
            MemValue::TAG_GET_WORD => Ok(VmValue::GetWord(LenAddress(value.0))),
            _ => Err(MemoryError::InvalidTag),
        }
    }
}

impl From<MemValue> for MemValueAligned {
    fn from(value: MemValue) -> Self {
        MemValueAligned(value.0, value.1 as Word)
    }
}

impl TryFrom<MemValueAligned> for MemValue {
    type Error = MemoryError;

    fn try_from(value: MemValueAligned) -> Result<Self, Self::Error> {
        Ok(MemValue(value.0, value.1 as Tag))
    }
}

impl Item for MemValue {
    const SIZE: Offset = 5;

    fn load(data: &[u8]) -> Option<Self> {
        let word = data
            .get(0..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)?;
        let tag = data.get(4).copied()?;
        Some(MemValue(word, tag))
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        let word = data.get_mut(0..4)?;
        word.copy_from_slice(&self.0.to_le_bytes());
        data.get_mut(4).map(|tag| *tag = self.1)
    }
}

impl Item for VmValue {
    const SIZE: Offset = 5;

    fn load(data: &[u8]) -> Option<Self> {
        MemValue::load(data)?.try_into().ok()
    }

    fn store(self, data: &mut [u8]) -> Option<()> {
        MemValue::from(self).store(data)
    }
}

// P A R S E  C O L L E C T O R

impl Collector for Memory<'_> {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Option<()> {
        let string = self
            .get_heap()?
            .alloc_string(self, string)
            .map(VmValue::String)?;
        self.get_parse_stack()?.push(string, self)
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Option<()> {
        let symbol = self
            .get_symbol_table()?
            .get_or_insert_symbol(word, self.get_heap()?, self)?;
        let value = match kind {
            WordKind::Word => VmValue::Word(symbol),
            WordKind::SetWord => VmValue::SetWord(symbol),
            WordKind::GetWord => VmValue::GetWord(symbol),
        };
        self.get_parse_stack()?.push(value, self)
    }

    fn integer(&mut self, value: i32) -> Option<()> {
        self.get_parse_stack()?.push(VmValue::Int(value), self)
    }

    fn begin_block(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_block(&mut self) -> Option<()> {
        let block = self.end().map(VmValue::Block)?;
        self.get_parse_stack()?.push(block, self)
    }

    fn begin_path(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_path(&mut self) -> Option<()> {
        let block = self.end().map(VmValue::Path)?;
        self.get_parse_stack()?.push(block, self)
    }
}

//

pub fn push(memory: &mut Memory, stack: &mut Stack<u8>, item: u8) -> Option<()> {
    stack.push(item, memory)
}

pub fn peek(memory: &Memory, stack: &Stack<u8>) -> Option<u8> {
    stack.peek(memory)
}

pub fn pop(memory: &mut Memory, stack: &mut Stack<u8>) -> Option<u8> {
    stack.pop(memory)
}

pub fn get(memory: &Memory, block: &Block<u8>, index: Offset) -> Option<u8> {
    block.get(index, memory)
}

pub fn parse_block<'a>(memory: &'a mut Memory<'a>, input: &str) -> Option<Block<VmValue>> {
    crate::parse::Parser::parse(input, memory).ok()?;
    let parse = memory.get_parse_stack()?;
    let heap = memory.get_heap()?;
    parse.cut_block(heap.0, parse.len(memory)?, memory)
}

// Tests for the memory system are in src/tests/mem_test.rs
