// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::parse::{Collector, Parser, ParserError, WordKind};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("out of bounds access")]
    OutOfBounds,
    #[error("stack underflow")]
    StackUnderflow,
    #[error("stack overflow")]
    StackOverflow,
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

    pub fn address(self, cap: Word) -> Result<usize, MemoryError> {
        if self.0 >= cap {
            Err(MemoryError::OutOfBounds)
        } else {
            Ok(self.0 as usize)
        }
    }

    pub fn range(self, len: Word, cap: Word) -> Result<Range<usize>, MemoryError> {
        let start = self.0;
        let end = start.checked_add(len).ok_or(MemoryError::OutOfBounds)?;
        if end > cap {
            Err(MemoryError::OutOfBounds)
        } else {
            Ok(start as usize..end as usize)
        }
    }

    // pub fn prev(self) -> Option<Self> {
    //     self.0.checked_sub(1).map(Self::new)
    // }

    pub fn next(self, n: Word) -> Result<Self, MemoryError> {
        self.0
            .checked_add(n)
            .map(Self::new)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn verify(self, cap: Word) -> bool {
        self.0 < cap
    }

    pub fn capped_next(self, n: Word, cap: Word) -> Result<Self, MemoryError> {
        self.0
            .checked_add(n)
            .filter(|&next| next < cap)
            .map(Self::new)
            .ok_or(MemoryError::OutOfBounds)
    }
}

/// This is The Block API users intended to use
impl<'a, T> Addr<Block<T>>
where
    T: Default + Copy,
{
    pub fn push<D>(&self, item: T, memory: &mut D) -> Result<(), MemoryError>
    where
        D: BlockStorage<'a, T>,
    {
        memory
            .access_block_mut(Addr::new(self.0))
            .and_then(|(block, domain)| block.push(item, domain))
    }

    pub fn push_all<D>(&self, items: &[T], memory: &mut D) -> Result<(), MemoryError>
    where
        D: BlockStorage<'a, T>,
    {
        memory
            .access_block_mut(Addr::new(self.0))
            .and_then(|(block, domain)| block.push_all(items, domain))
    }

    pub fn pop<D>(&self, memory: &mut D) -> Result<T, MemoryError>
    where
        D: BlockStorage<'a, T>,
    {
        memory
            .access_block_mut(Addr::new(self.0))
            .and_then(|(block, domain)| block.pop(domain))
    }

    pub fn get_all<'d, D>(&self, memory: &'d D) -> Result<&'d [T], MemoryError>
    where
        D: BlockStorage<'a, T>,
    {
        memory
            .access_block(Addr::new(self.0))
            .and_then(|(block, domain)| block.get_all(domain))
    }

    pub fn get<'d, D>(&self, index: Word, memory: &'d D) -> Result<T, MemoryError>
    where
        D: BlockStorage<'a, T>,
    {
        memory
            .access_block(Addr::new(self.0))
            .and_then(|(block, domain)| block.get(index, domain))
    }

    pub fn set<'d, D>(&self, index: Word, value: &T, memory: &'d mut D) -> Result<(), MemoryError>
    where
        D: BlockStorage<'a, T>,
    {
        memory
            .access_block_mut(Addr::new(self.0))
            .and_then(|(block, domain)| block.set(index, value, domain))
    }
}

//

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

//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AnyBlock {
    cap: Word,
    len: Word,
    data: Word,
}

impl Default for AnyBlock {
    fn default() -> Self {
        Self {
            cap: 0,
            len: 0,
            data: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block<T>(AnyBlock, PhantomData<T>);

impl<T> Block<T>
where
    T: Default + Copy,
{
    /// Create a new Block with specified capacity and data address
    pub fn new(cap: Word, len: Word, data: Addr<T>) -> Self {
        let data = data.0;
        Self(AnyBlock { cap, len, data }, PhantomData)
    }

    /// Returns the current length of the block
    pub fn len(&self) -> Word {
        self.0.len
    }

    /// Returns true if the block is empty
    pub fn is_empty(&self) -> bool {
        self.0.len == 0
    }

    /// Returns the capacity of the block
    pub fn cap(&self) -> Word {
        self.0.cap
    }

    fn data(&self) -> Addr<T> {
        Addr::new(self.0.data)
    }

    fn push(&mut self, item: T, domain: &mut Domain<T>) -> Result<(), MemoryError> {
        if self.0.len < self.0.cap {
            let index = self.data().next(self.0.len)?;
            domain.get_item_mut(index).map(|slot| *slot = item)?;
            self.0.len += 1;
            Ok(())
        } else {
            Err(MemoryError::StackOverflow)
        }
    }

    fn push_all(&mut self, items: &[T], domain: &mut Domain<T>) -> Result<(), MemoryError> {
        let items_len = items.len() as Word;
        if self.0.len + items_len > self.0.cap {
            return Err(MemoryError::StackOverflow);
        }

        if items_len > 0 {
            let dest_addr = self.data().next(self.0.len)?;
            domain.get_mut(dest_addr, items_len)?.copy_from_slice(items);
            self.0.len += items_len;
        }

        Ok(())
    }

    fn pop(&mut self, domain: &mut Domain<T>) -> Result<T, MemoryError> {
        self.0.len = self
            .0
            .len
            .checked_sub(1)
            .ok_or(MemoryError::StackUnderflow)?;
        domain.get_item(self.data().next(self.0.len)?).copied()
    }

    fn get_all<'a>(&self, domain: &'a Domain<T>) -> Result<&'a [T], MemoryError> {
        domain.get(self.data(), self.0.len)
    }

    fn get(&self, index: Word, domain: &Domain<T>) -> Result<T, MemoryError> {
        domain
            .get_item(self.data().capped_next(index, self.0.len)?)
            .copied()
    }

    fn set(&self, index: Word, value: &T, domain: &mut Domain<T>) -> Result<(), MemoryError> {
        domain
            .get_item_mut(self.data().capped_next(index, self.0.len)?)
            .map(|slot| *slot = *value)
    }
}

impl<T> Default for Block<T>
where
    T: Default + Copy,
{
    fn default() -> Self {
        Self(AnyBlock::default(), PhantomData)
    }
}

//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyValue {
    key: Addr<Block<u8>>,
    value: VmValue,
}

impl Default for KeyValue {
    fn default() -> Self {
        Self {
            key: Addr::new(0),
            value: VmValue::None,
        }
    }
}

//

#[derive(Debug)]
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

    pub fn capacity(&self) -> Word {
        self.items.len() as Word
    }

    /// Returns the current length of the domain
    pub fn len(&self) -> Word {
        self.len
    }

    /// Returns true if the domain is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get_item(&self, addr: Addr<T>) -> Result<&T, MemoryError> {
        self.items
            .get(addr.address(self.len)?)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn get(&self, addr: Addr<T>, len: Word) -> Result<&[T], MemoryError> {
        self.items
            .get(addr.range(len, self.len)?)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn get_item_mut(&mut self, addr: Addr<T>) -> Result<&mut T, MemoryError> {
        self.items
            .get_mut(addr.address(self.len)?)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn get_mut(&mut self, addr: Addr<T>, len: Word) -> Result<&mut [T], MemoryError> {
        self.items
            .get_mut(addr.range(len, self.len)?)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn push_all(&mut self, items: &[T]) -> Result<Addr<T>, MemoryError> {
        let addr = self.len;
        let begin = addr as usize;
        let end = begin + items.len();
        self.items
            .get_mut(begin..end)
            .map(|slot| {
                slot.copy_from_slice(items);
            })
            .ok_or(MemoryError::OutOfBounds)?;
        self.len = end as Word;
        Ok(Addr::new(addr))
    }

    pub fn push(&mut self, item: T) -> Result<Addr<T>, MemoryError> {
        let addr = self.len;
        self.items
            .get_mut(addr as usize)
            .map(|slot| {
                *slot = item;
                self.len += 1;
                Addr::new(addr)
            })
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn alloc(&mut self, items: Word) -> Result<Addr<T>, MemoryError> {
        let addr = self.len;
        let new_addr = addr + items;
        if new_addr > self.items.len() as Word {
            Err(MemoryError::OutOfBounds)
        } else {
            self.len = new_addr;
            Ok(Addr::new(addr))
        }
    }

    /// Copies a range of items within the domain using direct memory operations.
    ///
    /// This method performs a safe item-by-item copy between memory regions, handling
    /// overlapping ranges by automatically choosing the appropriate copy direction
    /// (forward or backward). All operations are bounds-checked to ensure memory safety.
    ///
    /// # Arguments
    /// * `from` - Starting address to copy from
    /// * `to` - Destination address to copy to
    /// * `items` - Number of items to copy
    ///
    /// # Returns
    /// * `Ok(())` if the copy was successful
    /// * `Err(MemoryError::OutOfBounds)` if:
    ///   - Integer overflow occurs in address calculations
    ///   - Source or destination range exceeds domain length
    ///   - Invalid address access is attempted
    pub fn copy_items(
        &mut self,
        from: Addr<T>,
        to: Addr<T>,
        items: Word,
    ) -> Result<(), MemoryError> {
        // Verify ranges are within bounds
        let from = from.range(items, self.len)?.start;
        let to = to.range(items, self.len)?.start;
        let items = items as usize;

        unsafe {
            let ptr = self.items.as_mut_ptr();
            if to > from {
                // Copy backwards to handle overlapping ranges
                let mut i = items;
                while i > 0 {
                    i -= 1;
                    *ptr.add(to + i) = *ptr.add(from + i);
                }
            } else {
                // Copy forwards
                for i in 0..items {
                    *ptr.add(to + i) = *ptr.add(from + i);
                }
            }
        }

        Ok(())
    }
}

pub trait BlockStorage<'a, T> {
    fn access_block(&self, addr: Addr<Block<T>>) -> Result<(&Block<T>, &Domain<T>), MemoryError>;
    fn access_block_mut(
        &mut self,
        addr: Addr<Block<T>>,
    ) -> Result<(&mut Block<T>, &mut Domain<T>), MemoryError>;
}

//

pub struct Memory {
    blocks: Domain<AnyBlock>,
    values: Domain<VmValue>,
    pairs: Domain<KeyValue>,
    bytes: Domain<u8>,
    words: Domain<Word>,
    symbols: HashMap<SmolStr, Addr<Block<u8>>>,
    system: HashMap<Addr<Block<u8>>, VmValue>,
    stack: Block<VmValue>,
    op_stack: Block<Word>,
}

impl Memory {
    pub fn new(capacity: usize) -> Self {
        Self {
            blocks: Domain::new(capacity),
            values: Domain::new(capacity),
            pairs: Domain::new(capacity),
            bytes: Domain::new(capacity),
            words: Domain::new(capacity),
            symbols: HashMap::new(),
            system: HashMap::new(),
            stack: Block::default(),
            op_stack: Block::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), MemoryError> {
        // Initialize stack and op_stack with reasonable capacity
        let stack_data = self.values.alloc(64)?;
        let op_stack_data = self.words.alloc(64)?;

        self.stack = Block::new(64, 0, Addr::new(stack_data.0));
        self.op_stack = Block::new(64, 0, Addr::new(op_stack_data.0));

        Ok(())
    }

    fn alloc_block_header<T>(
        &mut self,
        cap: Word,
        len: Word,
        data: Word,
    ) -> Result<Addr<Block<T>>, MemoryError>
    where
        T: Default + Copy,
    {
        Ok(Addr::new(self.blocks.push(AnyBlock { cap, len, data })?.0))
    }

    pub fn alloc_empty_block(&mut self, cap: Word) -> Result<Addr<Block<VmValue>>, MemoryError> {
        let data = self.values.alloc(cap)?;
        self.alloc_block_header(cap, 0, data.0)
    }

    pub fn alloc_string(&mut self, string: &str) -> Result<Addr<Block<u8>>, MemoryError> {
        let bytes = string.as_bytes();
        let len = bytes.len() as Word;
        let data = self.bytes.push_all(bytes)?;
        self.alloc_block_header(len, len, data.0)
    }

    pub fn get_bytes(&self, addr: Addr<u8>, len: Word) -> Result<&[u8], MemoryError> {
        self.bytes.get(addr, len)
    }

    pub fn get_bytes_mut(&mut self, addr: Addr<u8>, len: Word) -> Result<&mut [u8], MemoryError> {
        self.bytes.get_mut(addr, len)
    }

    pub fn get_pairs(&self, addr: Addr<KeyValue>, len: Word) -> Result<&[KeyValue], MemoryError> {
        self.pairs.get(addr, len)
    }

    pub fn get_pairs_mut(
        &mut self,
        addr: Addr<KeyValue>,
        len: Word,
    ) -> Result<&mut [KeyValue], MemoryError> {
        self.pairs.get_mut(addr, len)
    }

    pub fn get_symbol(&mut self, string: &str) -> Result<Addr<Block<u8>>, MemoryError> {
        let symbol = self.symbols.get(string).copied();
        if let Some(symbol) = symbol {
            Ok(symbol)
        } else {
            let new_symbol = self.alloc_string(string)?;
            self.symbols.insert(string.into(), new_symbol);
            Ok(new_symbol)
        }
    }

    // P A R S E R  S U P P O R T

    pub fn begin(&mut self) -> Result<(), MemoryError> {
        self.op_stack.push(self.stack.len(), &mut self.words)
    }

    pub fn end(&mut self) -> Result<Addr<Block<VmValue>>, MemoryError> {
        let offset = self.op_stack.pop(&mut self.words)?;
        let from = self.stack.data().next(offset)?;
        let items = self
            .stack
            .len()
            .checked_sub(offset)
            .ok_or(MemoryError::OutOfBounds)?;
        let to = self.values.alloc(items)?;
        self.values.copy_items(from, to, items)?;
        self.alloc_block_header(items, items, to.0)
    }

    pub fn parse_block(&mut self, input: &str) -> Result<(), ParserError<MemoryError>> {
        Parser::parse_block(input, self)
    }
}

// P A R S E  C O L L E C T O R

impl Collector for Memory {
    type Error = MemoryError;

    fn string(&mut self, string: &str) -> Result<(), Self::Error> {
        let string = VmValue::String(self.alloc_string(string)?);
        self.stack.push(string, &mut self.values)
    }

    fn word(&mut self, kind: WordKind, word: &str) -> Result<(), Self::Error> {
        let symbol = self.get_symbol(word)?;
        let value = match kind {
            WordKind::Word => VmValue::Word(symbol),
            WordKind::SetWord => VmValue::SetWord(symbol),
            WordKind::GetWord => VmValue::GetWord(symbol),
        };
        self.stack.push(value, &mut self.values)
    }

    fn integer(&mut self, value: i32) -> Result<(), Self::Error> {
        self.stack.push(VmValue::Int(value), &mut self.values)
    }

    fn begin_block(&mut self) -> Result<(), Self::Error> {
        self.begin()
    }

    fn end_block(&mut self) -> Result<(), Self::Error> {
        let block = self.end().map(VmValue::Block)?;
        self.stack.push(block, &mut self.values)
    }

    fn begin_path(&mut self) -> Result<(), Self::Error> {
        self.begin()
    }

    fn end_path(&mut self) -> Result<(), Self::Error> {
        let block = self.end().map(VmValue::Path)?;
        self.stack.push(block, &mut self.values)
    }
}

// D O M A I N  S U P P O R T

impl<'a> BlockStorage<'a, VmValue> for Memory {
    fn access_block(
        &self,
        addr: Addr<Block<VmValue>>,
    ) -> Result<(&Block<VmValue>, &Domain<VmValue>), MemoryError> {
        let typeless = self.blocks.get_item(Addr::new(addr.0))?;
        let ptr = typeless as *const AnyBlock;
        let block = unsafe { &*ptr.cast::<Block<VmValue>>() };
        Ok((block, &self.values))
    }

    fn access_block_mut(
        &mut self,
        addr: Addr<Block<VmValue>>,
    ) -> Result<(&mut Block<VmValue>, &mut Domain<VmValue>), MemoryError> {
        let typeless = self.blocks.get_item_mut(Addr::new(addr.0))?;
        let ptr = typeless as *mut AnyBlock;
        let block = unsafe { &mut *ptr.cast::<Block<VmValue>>() };
        Ok((block, &mut self.values))
    }
}

impl<'a> BlockStorage<'a, Word> for Memory {
    fn access_block(
        &self,
        addr: Addr<Block<Word>>,
    ) -> Result<(&Block<Word>, &Domain<Word>), MemoryError> {
        let typeless = self.blocks.get_item(Addr::new(addr.0))?;
        let ptr = typeless as *const AnyBlock;
        let block = unsafe { &*ptr.cast::<Block<Word>>() };
        Ok((block, &self.words))
    }

    fn access_block_mut(
        &mut self,
        addr: Addr<Block<Word>>,
    ) -> Result<(&mut Block<Word>, &mut Domain<Word>), MemoryError> {
        let typeless = self.blocks.get_item_mut(Addr::new(addr.0))?;
        let ptr = typeless as *mut AnyBlock;
        let block = unsafe { &mut *ptr.cast::<Block<Word>>() };
        Ok((block, &mut self.words))
    }
}

impl<'a> BlockStorage<'a, u8> for Memory {
    fn access_block(
        &self,
        addr: Addr<Block<u8>>,
    ) -> Result<(&Block<u8>, &Domain<u8>), MemoryError> {
        let typeless = self.blocks.get_item(Addr::new(addr.0))?;
        let ptr = typeless as *const AnyBlock;
        let block = unsafe { &*ptr.cast::<Block<u8>>() };
        Ok((block, &self.bytes))
    }

    fn access_block_mut(
        &mut self,
        addr: Addr<Block<u8>>,
    ) -> Result<(&mut Block<u8>, &mut Domain<u8>), MemoryError> {
        let typeless = self.blocks.get_item_mut(Addr::new(addr.0))?;
        let ptr = typeless as *mut AnyBlock;
        let block = unsafe { &mut *ptr.cast::<Block<u8>>() };
        Ok((block, &mut self.bytes))
    }
}
