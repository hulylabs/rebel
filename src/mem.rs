// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, WordKind};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("out of memory")]
    OutOfMemory,

    #[error("invalid address")]
    InvalidAddress,

    #[error("operation failed")]
    OperationFailed,
}

pub type Word = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Addr<T>(Word, PhantomData<T>);

impl<T> Addr<T>
where
    T: Default + Copy,
{
    pub fn new(address: Word) -> Self {
        Self(address, PhantomData)
    }

    pub fn address(self, cap: Word) -> Option<usize> {
        if self.0 >= cap {
            None
        } else {
            Some(self.0 as usize)
        }
    }

    pub fn range(self, len: Word, cap: Word) -> Option<Range<usize>> {
        let start = self.0;
        let end = start + len;
        if end > cap {
            None
        } else {
            Some(start as usize..end as usize)
        }
    }

    pub fn prev(self, n: Word) -> Option<Self> {
        self.0.checked_sub(n).map(Self::new)
    }

    pub fn next(self, n: Word) -> Option<Self> {
        self.0.checked_add(n).map(Self::new)
    }

    pub fn capped_next(self, n: Word, cap: Word) -> Option<Self> {
        self.next(n).and_then(|next| next.verify(cap))
    }

    pub fn verify(self, cap: Word) -> Option<Self> {
        if self.0 < cap { Some(self) } else { None }
    }

    // pub fn deref<'a>(self, domain: &'a Domain<T>) -> Option<T> {
    //     domain.get_item(self)
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyValue {
    key: Addr<Block<u8>>,
    value: VmValue,
}

impl KeyValue {
    /// Create a new KeyValue pair
    pub fn new(key: Addr<Block<u8>>, value: VmValue) -> Self {
        Self { key, value }
    }

    /// Get the key address
    pub fn key(&self) -> Addr<Block<u8>> {
        self.key
    }

    /// Get the value
    pub fn value(&self) -> VmValue {
        self.value
    }

    /// Set the value
    pub fn set_value(&mut self, new_value: VmValue) {
        self.value = new_value;
    }
}

impl Default for KeyValue {
    fn default() -> Self {
        Self {
            key: Addr::new(0),
            value: VmValue::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmValue {
    None,
    Int(i32),
    Block(Addr<Block<VmValue>>),
    Context(Addr<Block<KeyValue>>),
    String(Addr<Block<u8>>),
    Word(Addr<Block<u8>>),
    SetWord(Addr<Block<u8>>),
    GetWord(Addr<Block<u8>>),
    Path(Addr<Block<VmValue>>),
}

impl Default for VmValue {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block<T> {
    cap: Word,
    len: Word,
    data: Addr<T>,
}

impl<T> Block<T>
where
    T: Default + Copy,
{
    /// Create a new Block with specified capacity and data address
    pub fn new(cap: Word, len: Word, data: Addr<T>) -> Self {
        Self { cap, len, data }
    }

    /// Returns the current length of the block
    pub fn len(&self) -> Word {
        self.len
    }

    /// Returns the capacity of the block
    pub fn cap(&self) -> Word {
        self.cap
    }

    /// Returns the data address of the block
    pub fn data(&self) -> Addr<T> {
        self.data
    }

    pub fn get_item<'a>(&self, index: Word, domain: &'a Domain<T>) -> Option<&'a T> {
        domain.get_item(self.data.capped_next(index, self.len)?)
    }

    pub fn push(&mut self, item: T, domain: &mut Domain<T>) -> Option<()> {
        domain
            .get_item_mut(self.data.capped_next(self.len, self.cap)?)
            .map(|slot| {
                *slot = item;
            })?;
        self.len += 1;
        Some(())
    }

    pub fn push_all(&mut self, items: &[T], domain: &mut Domain<T>) -> Option<()> {
        let addr = self.data.capped_next(self.len, self.cap)?;
        let len = items.len() as Word;
        domain.get_mut(addr, len).map(|slot| {
            slot.copy_from_slice(items);
        })?;
        self.len += len;
        Some(())
    }

    /// Truncates the block at specified offset and returns removed items.
    ///
    /// This method:
    /// - Keeps elements [0..offset] in the block
    /// - Returns elements [offset..len] that were removed
    /// - Reduces the block's length to `offset`
    ///
    /// For example, a block containing [1,2,3,4,5] with trim_after(2)
    /// would keep [1,2] in the block and return [3,4,5].
    pub fn trim_after<'a>(&mut self, offset: Word, domain: &'a mut Domain<T>) -> Option<&'a [T]> {
        let items = self.len.checked_sub(offset)?;
        let result = domain.get(self.data.capped_next(offset, self.cap)?, items);
        // Update the block length to be equal to the offset
        self.len = offset;
        result
    }

    pub fn move_to(&mut self, dest: &Block<T>, items: Word, domain: &mut Domain<T>) -> Option<()> {
        let from_new_len = self.len.checked_sub(items)?;
        let from = self.data.capped_next(from_new_len, self.cap)?;
        let to = dest.data.capped_next(dest.len, dest.cap)?;

        domain.move_items(from, to, items)?;
        self.len = from_new_len;
        Some(())
    }

    pub fn pop(&mut self, domain: &mut Domain<T>) -> Option<T> {
        self.len = self.len.checked_sub(1)?;
        domain
            .get_item(self.data.capped_next(self.len, self.cap)?)
            .copied()
    }
}

impl<T> Default for Block<T>
where
    T: Default + Copy,
{
    fn default() -> Self {
        Self {
            cap: 0,
            len: 0,
            data: Addr::new(0),
        }
    }
}

pub struct Domain<T> {
    items: Box<[T]>,
    len: Word,
}

impl<T> Domain<T>
where
    T: Default + Copy,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            items: vec![T::default(); capacity].into_boxed_slice(),
            len: 0,
        }
    }

    /// Returns the current length of the domain
    pub fn len(&self) -> Word {
        self.len
    }

    pub fn get_item(&self, addr: Addr<T>) -> Option<&T> {
        self.items.get(addr.address(self.len)?)
    }

    pub fn get(&self, addr: Addr<T>, len: Word) -> Option<&[T]> {
        self.items.get(addr.range(len, self.len)?)
    }

    pub fn get_item_mut(&mut self, addr: Addr<T>) -> Option<&mut T> {
        self.items.get_mut(addr.address(self.len)?)
    }

    pub fn get_mut(&mut self, addr: Addr<T>, len: Word) -> Option<&mut [T]> {
        self.items.get_mut(addr.range(len, self.len)?)
    }

    pub fn push_all(&mut self, items: &[T]) -> Option<Addr<T>> {
        let addr = self.len;
        let begin = addr as usize;
        let end = begin + items.len();
        self.items.get_mut(begin..end).map(|slot| {
            slot.copy_from_slice(items);
        })?;
        self.len = end as Word;
        Some(Addr::new(addr))
    }

    pub fn push(&mut self, item: T) -> Option<Addr<T>> {
        let addr = self.len;
        self.items.get_mut(addr as usize).map(|slot| {
            *slot = item;
        })?;
        self.len += 1;
        Some(Addr::new(addr))
    }

    pub fn alloc(&mut self, items: Word) -> Option<Addr<T>> {
        let addr = self.len;
        let new_addr = addr + items;
        if new_addr > self.items.len() as Word {
            None
        } else {
            self.len = new_addr;
            Some(Addr::new(addr))
        }
    }

    pub fn move_items(&mut self, from: Addr<T>, to: Addr<T>, items: Word) -> Option<()> {
        let from = from.address(self.len)?;
        let to = to.address(self.len)?;
        let items = items as usize;

        if from + items > self.items.len() || to + items > self.items.len() {
            return None;
        }

        for i in 0..items {
            self.items[to + i] = self.items[from + i];
        }

        Some(())
    }
}

pub trait DomainProvider<T> {
    /// Gets a reference to the domain for type T
    fn domain(&self) -> &Domain<T>;

    /// Gets a mutable reference to the domain for type T
    fn domain_mut(&mut self) -> &mut Domain<T>;
}

// // Type-specific marker traits to help with type inference
// pub trait ValueDomain {}
// impl ValueDomain for VmValue {}

// pub trait BlockDomain {}
// impl BlockDomain for Block<VmValue> {}

// pub trait StringDomain {}
// impl StringDomain for Block<u8> {}

// pub trait ByteDomain {}
// impl ByteDomain for u8 {}

// pub trait WordDomain {}
// impl WordDomain for Word {}

// pub trait PairDomain {}
// impl PairDomain for KeyValue {}

pub struct Memory {
    values: Domain<VmValue>,
    blocks: Domain<Block<VmValue>>,
    strings: Domain<Block<u8>>,
    bytes: Domain<u8>,
    words: Domain<Word>,
    pairs: Domain<KeyValue>,
    contexts: Domain<Block<KeyValue>>,
    //
    symbols: HashMap<SmolStr, Addr<Block<u8>>>,
    system: HashMap<Addr<Block<u8>>, VmValue>,
    //
    stack: Block<VmValue>,
    op_stack: Block<Word>,
}

// Public accessor methods for Memory
impl Memory {
    // Block accessor methods
    pub fn get_block(&self, addr: Addr<Block<VmValue>>) -> Option<&Block<VmValue>> {
        self.blocks.get_item(addr)
    }

    pub fn get_block_mut(&mut self, addr: Addr<Block<VmValue>>) -> Option<&mut Block<VmValue>> {
        self.blocks.get_item_mut(addr)
    }

    pub fn get_string(&self, addr: Addr<Block<u8>>) -> Option<&Block<u8>> {
        self.strings.get_item(addr)
    }
}

// Module for test-only access to private fields
#[cfg(test)]
pub mod test_access {
    use super::*;

    // Memory domain accessors (consolidated)
    pub fn domain<'a, T>(memory: &'a Memory, domain_type: &'a str) -> Option<&'a Domain<T>> {
        match domain_type {
            "values" => Some(unsafe { std::mem::transmute(&memory.values) }),
            "blocks" => Some(unsafe { std::mem::transmute(&memory.blocks) }),
            "strings" => Some(unsafe { std::mem::transmute(&memory.strings) }),
            "bytes" => Some(unsafe { std::mem::transmute(&memory.bytes) }),
            "words" => Some(unsafe { std::mem::transmute(&memory.words) }),
            "pairs" => Some(unsafe { std::mem::transmute(&memory.pairs) }),
            "contexts" => Some(unsafe { std::mem::transmute(&memory.contexts) }),
            _ => None,
        }
    }

    // Block data accessor
    pub fn block_data<T: Default + Copy>(block: &Block<T>) -> Addr<T> {
        block.data
    }

    // Symbol comparison functions
    pub fn symbols_equal(addr1: &Addr<Block<u8>>, addr2: &Addr<Block<u8>>) -> bool {
        addr1.0 == addr2.0
    }

    pub fn symbols_not_equal(addr1: &Addr<Block<u8>>, addr2: &Addr<Block<u8>>) -> bool {
        addr1.0 != addr2.0
    }
}

// Essential test helpers for Memory
#[cfg(test)]
impl Memory {
    // Get string block by address
    pub fn get_string_block(&self, addr: Addr<Block<u8>>) -> Option<&Block<u8>> {
        self.strings.get_item(addr)
    }

    // Get string bytes directly
    pub fn get_string_bytes(&self, addr: Addr<Block<u8>>) -> Option<&[u8]> {
        let string_block = self.get_string_block(addr)?;
        self.bytes.get(string_block.data(), string_block.len())
    }

    // Get a byte from the bytes domain
    pub fn get_byte(&self, addr: Addr<u8>) -> Option<&u8> {
        self.bytes.get_item(addr)
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            bytes: Domain::new(0x10000),
            words: Domain::new(0x10000),
            pairs: Domain::new(0x10000),
            values: Domain::new(0x10000),
            blocks: Domain::new(0x10000),
            strings: Domain::new(0x10000),
            contexts: Domain::new(0x10000),
            //
            symbols: HashMap::new(),
            system: HashMap::new(),
            //
            stack: Block::default(),
            op_stack: Block::default(),
        }
    }

    pub fn init(&mut self) -> Option<()> {
        // Initialize the stack for values
        let stack_space = self.values.alloc(256)?;
        self.stack = Block::new(256, 0, stack_space);

        // Initialize the op_stack
        let op_stack_space = self.words.alloc(128)?;
        self.op_stack = Block::new(128, 0, op_stack_space);

        Some(())
    }

    // Stack manipulation helpers
    pub fn stack_push(&mut self, value: VmValue) -> Option<()> {
        self.stack.push(value, &mut self.values)
    }

    pub fn stack_pop(&mut self) -> Option<VmValue> {
        self.stack.pop(&mut self.values)
    }

    pub fn stack_len(&self) -> Word {
        self.stack.len()
    }

    pub fn alloc_empty_block(&mut self, cap: Word) -> Option<Addr<Block<VmValue>>> {
        self.blocks.push(Block::new(cap, 0, Addr::new(0)))
    }

    pub fn alloc_block(&mut self, items: &[VmValue]) -> Option<Addr<Block<VmValue>>> {
        let data = self.values.push_all(items)?;
        let len = items.len() as Word;
        self.blocks.push(Block::new(len, len, data))
    }

    pub fn alloc_string(&mut self, s: &str) -> Option<Addr<Block<u8>>> {
        let bytes = s.as_bytes();
        let len = bytes.len() as Word;
        let data = self.bytes.push_all(bytes)?;
        self.strings.push(Block::new(len, len, data))
    }

    pub fn get_symbol(&mut self, string: &str) -> Option<Addr<Block<u8>>> {
        let symbol = self.symbols.get(string).copied();
        if symbol.is_none() {
            let new_symbol = self.alloc_string(string)?;
            self.symbols.insert(string.into(), new_symbol);
            Some(new_symbol)
        } else {
            symbol
        }
    }

    // P A R S E  H E L P E R S

    pub fn begin(&mut self) -> Option<()> {
        self.op_stack.push(self.stack.len(), &mut self.words)
    }

    pub fn end(&mut self) -> Option<Addr<Block<VmValue>>> {
        let offset = { self.op_stack.pop(&mut self.words)? };
        let items = self.stack.len().checked_sub(offset)?;
        let block = self.alloc_empty_block(items)?;
        let to = self.blocks.get_item(block)?;
        self.stack.move_to(to, items, &mut self.values)?;
        Some(block)
    }
}

// Implement DomainProvider for each domain type
impl DomainProvider<VmValue> for Memory {
    fn domain(&self) -> &Domain<VmValue> {
        &self.values
    }

    fn domain_mut(&mut self) -> &mut Domain<VmValue> {
        &mut self.values
    }
}

impl DomainProvider<Block<VmValue>> for Memory {
    fn domain(&self) -> &Domain<Block<VmValue>> {
        &self.blocks
    }

    fn domain_mut(&mut self) -> &mut Domain<Block<VmValue>> {
        &mut self.blocks
    }
}

impl DomainProvider<Block<u8>> for Memory {
    fn domain(&self) -> &Domain<Block<u8>> {
        &self.strings
    }

    fn domain_mut(&mut self) -> &mut Domain<Block<u8>> {
        &mut self.strings
    }
}

impl DomainProvider<u8> for Memory {
    fn domain(&self) -> &Domain<u8> {
        &self.bytes
    }

    fn domain_mut(&mut self) -> &mut Domain<u8> {
        &mut self.bytes
    }
}

impl DomainProvider<Word> for Memory {
    fn domain(&self) -> &Domain<Word> {
        &self.words
    }

    fn domain_mut(&mut self) -> &mut Domain<Word> {
        &mut self.words
    }
}

impl DomainProvider<KeyValue> for Memory {
    fn domain(&self) -> &Domain<KeyValue> {
        &self.pairs
    }

    fn domain_mut(&mut self) -> &mut Domain<KeyValue> {
        &mut self.pairs
    }
}

impl DomainProvider<Block<KeyValue>> for Memory {
    fn domain(&self) -> &Domain<Block<KeyValue>> {
        &self.contexts
    }

    fn domain_mut(&mut self) -> &mut Domain<Block<KeyValue>> {
        &mut self.contexts
    }
}

// Block operation extensions
impl Memory {
    // Block operations that use domain access

    /// Get an item from a block at the specified index
    pub fn get_block_item(
        &self,
        block_addr: Addr<Block<VmValue>>,
        index: Word,
    ) -> Option<&VmValue> {
        let block = self.get_block(block_addr)?;
        block.get_item(index, self.domain())
    }

    /// Set an item in a block at the specified index
    pub fn set_block_item(
        &mut self,
        block_addr: Addr<Block<VmValue>>,
        index: Word,
        value: VmValue,
    ) -> Option<()> {
        // Get the block
        let block = self.get_block(block_addr)?;

        // Make sure index is within range
        if index >= block.len() {
            return None;
        }

        // Get the data address of the element
        let value_addr = block.data().capped_next(index, block.len())?;

        // Set the value
        *self.domain_mut().get_item_mut(value_addr)? = value;

        Some(())
    }

    /// Get a block item that's a reference to another block
    pub fn get_block_ref(
        &self,
        block_addr: Addr<Block<VmValue>>,
        index: Word,
    ) -> Option<Addr<Block<VmValue>>> {
        match self.get_block_item(block_addr, index)? {
            VmValue::Block(addr) => Some(*addr),
            _ => None,
        }
    }

    /// Push a value to a block
    pub fn push_to_block(
        &mut self,
        block_addr: Addr<Block<VmValue>>,
        value: VmValue,
    ) -> Option<()> {
        let mut block = *self.get_block(block_addr)?;
        let result = block.push(value, self.domain_mut());
        // Update the block in memory
        *self.get_block_mut(block_addr)? = block;
        result
    }

    /// Push multiple values to a block
    pub fn push_all_to_block(
        &mut self,
        block_addr: Addr<Block<VmValue>>,
        values: &[VmValue],
    ) -> Option<()> {
        let mut block = *self.get_block(block_addr)?;
        let result = block.push_all(values, self.domain_mut());
        // Update the block in memory
        *self.get_block_mut(block_addr)? = block;
        result
    }

    /// Pop a value from a block
    pub fn pop_from_block(&mut self, block_addr: Addr<Block<VmValue>>) -> Option<VmValue> {
        let mut block = *self.get_block(block_addr)?;
        let value = block.pop(self.domain_mut());
        // Update the block in memory
        *self.get_block_mut(block_addr)? = block;
        value
    }
}

// P A R S E  C O L L E C T O R

impl Collector for Memory {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Option<()> {
        let string = VmValue::String(self.alloc_string(string)?);
        self.stack.push(string, &mut self.values)
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Option<()> {
        let symbol = self.get_symbol(word)?;
        let value = match kind {
            WordKind::Word => VmValue::Word(symbol),
            WordKind::SetWord => VmValue::SetWord(symbol),
            WordKind::GetWord => VmValue::GetWord(symbol),
        };
        self.stack.push(value, &mut self.values)
    }

    fn integer(&mut self, value: i32) -> Option<()> {
        self.stack.push(VmValue::Int(value), &mut self.values)
    }

    fn begin_block(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_block(&mut self) -> Option<()> {
        let block = self.end().map(VmValue::Block)?;
        self.stack.push(block, &mut self.values)
    }

    fn begin_path(&mut self) -> Option<()> {
        self.begin()
    }

    fn end_path(&mut self) -> Option<()> {
        let block = self.end().map(VmValue::Path)?;
        self.stack.push(block, &mut self.values)
    }
}

// End of Memory implementation
