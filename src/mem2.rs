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

    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(Self::new)
    }

    pub fn next(self, n: Word) -> Result<Self, MemoryError> {
        self.0
            .checked_add(n)
            .map(Self::new)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn verify(self, cap: Word) -> bool {
        self.0 < cap
    }
}

impl<'a, T> Addr<Block<T>>
where
    T: Default + Copy,
{
    pub fn push<D>(&self, item: T, memory: &mut D) -> Result<(), MemoryError>
    where
        D: GetDomain<'a, T>,
    {
        let (domain, block) = memory.get_domain_mut(Addr::new(self.0))?;
        block.push(item, domain)
    }

    pub fn pop<D>(&self, memory: &mut D) -> Result<T, MemoryError>
    where
        D: GetDomain<'a, T>,
    {
        let (domain, block) = memory.get_domain_mut(Addr::new(self.0))?;
        block.pop(domain)
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

    pub fn push(&mut self, item: T, domain: &mut Domain<T>) -> Result<(), MemoryError> {
        if self.0.len < self.0.cap {
            let index = self.data().next(self.0.len)?;
            domain.get_item_mut(index).map(|slot| *slot = item)?;
            self.0.len += 1;
            Ok(())
        } else {
            Err(MemoryError::StackOverflow)
        }
    }

    pub fn pop(&mut self, domain: &mut Domain<T>) -> Result<T, MemoryError> {
        self.0.len = self
            .0
            .len
            .checked_sub(1)
            .ok_or(MemoryError::StackUnderflow)?;
        domain.get_item(self.data().next(self.0.len)?).copied()
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

pub trait GetDomain<'a, T> {
    fn get_domain(&self, addr: Addr<Block<T>>) -> Result<(&Domain<T>, &Block<T>), MemoryError>;
    fn get_domain_mut(
        &mut self,
        addr: Addr<Block<T>>,
    ) -> Result<(&mut Domain<T>, &mut Block<T>), MemoryError>;
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

    fn get_block<T>(&self, addr: Addr<Block<T>>) -> Result<&Block<T>, MemoryError>
    where
        T: Default + Copy,
    {
        let typeless = self.blocks.get_item(Addr::new(addr.0))?;
        let ptr = typeless as *const AnyBlock;
        let block = unsafe { &*ptr.cast::<Block<T>>() };
        Ok(block)
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

impl<'a> GetDomain<'a, VmValue> for Memory {
    fn get_domain(
        &self,
        addr: Addr<Block<VmValue>>,
    ) -> Result<(&Domain<VmValue>, &Block<VmValue>), MemoryError> {
        Ok((&self.values, self.get_block(addr)?))
    }

    fn get_domain_mut(
        &mut self,
        addr: Addr<Block<VmValue>>,
    ) -> Result<(&mut Domain<VmValue>, &mut Block<VmValue>), MemoryError> {
        let typeless = self.blocks.get_item_mut(Addr::new(addr.0))?;
        let ptr = typeless as *mut AnyBlock;
        let block = unsafe { &mut *ptr.cast::<Block<VmValue>>() };
        Ok((&mut self.values, block))
    }
}

impl<'a> GetDomain<'a, Word> for Memory {
    fn get_domain(
        &self,
        addr: Addr<Block<Word>>,
    ) -> Result<(&Domain<Word>, &Block<Word>), MemoryError> {
        Ok((&self.words, self.get_block(addr)?))
    }

    fn get_domain_mut(
        &mut self,
        addr: Addr<Block<Word>>,
    ) -> Result<(&mut Domain<Word>, &mut Block<Word>), MemoryError> {
        let typeless = self.blocks.get_item_mut(Addr::new(addr.0))?;
        let ptr = typeless as *mut AnyBlock;
        let block = unsafe { &mut *ptr.cast::<Block<Word>>() };
        Ok((&mut self.words, block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a block with values
    fn create_block_with_values(
        memory: &mut Memory,
        values: &[VmValue],
    ) -> Result<Addr<Block<VmValue>>, MemoryError> {
        let block_addr = memory.alloc_empty_block(values.len() as Word)?;
        let (domain, block) = memory.get_domain_mut(block_addr)?;
        for value in values {
            block.push(*value, domain)?;
        }
        Ok(block_addr)
    }

    // Construction & Basic Properties Tests
    #[test]
    fn test_domain_construction() {
        let domain = Domain::<i32>::new(10);
        assert_eq!(domain.len(), 0, "New domain should have length 0");
        assert!(domain.is_empty(), "New domain should be empty");
    }

    #[test]
    fn test_domain_capacity() {
        let mut domain: Domain<i32> = Domain::new(3);
        assert!(domain.push(1).is_ok(), "First push should succeed");
        assert!(domain.push(2).is_ok(), "Second push should succeed");
        assert!(domain.push(3).is_ok(), "Third push should succeed");
        assert!(domain.push(4).is_err(), "Push beyond capacity should fail");
    }

    // Single Item Operations Tests
    #[test]
    fn test_push_and_get() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(5);

        // Test push and get_item
        let addr1 = domain.push(42)?;
        let item = domain.get_item(addr1)?;
        assert_eq!(item, &42, "Should get pushed item");

        // Test get_item with invalid address
        assert!(matches!(
            domain.get_item(Addr::new(5)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        assert!(matches!(
            domain.get_item(Addr::new(u32::MAX)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    #[test]
    fn test_get_item_mut() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(5);
        let addr = domain.push(42)?;

        // Test get_item_mut and modify value
        *domain.get_item_mut(addr)? = 24;
        let item = domain.get_item(addr)?;
        assert_eq!(item, &24, "Value should be modified");

        // Test get_item_mut with invalid address
        assert!(matches!(
            domain.get_item_mut(Addr::new(5)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    // Multiple Items Operations Tests
    #[test]
    fn test_push_all() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(10);

        // Test pushing empty slice
        let _addr_empty = domain.push_all(&[])?;
        assert_eq!(
            domain.len(),
            0,
            "Pushing empty slice shouldn't change length"
        );

        // Test pushing multiple items
        let items = [1, 2, 3, 4];
        let addr = domain.push_all(&items)?;
        let slice = domain.get(addr, 4)?;
        assert_eq!(slice, &items[..], "Should get all pushed items");

        // Test pushing beyond capacity
        assert!(matches!(
            domain.push_all(&[5, 5, 5, 5, 5, 5, 5]).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    #[test]
    fn test_get_range() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(10);
        let items = [1, 2, 3, 4, 5];
        let addr = domain.push_all(&items)?;

        // Test valid ranges
        let slice = domain.get(addr, 3)?;
        assert_eq!(slice, &items[..3], "Should get correct slice");

        let empty_slice: &[i32] = &[];
        let empty = domain.get(addr, 0)?;
        assert_eq!(empty, empty_slice, "Should get empty slice");

        // Test invalid ranges
        assert!(matches!(
            domain.get(addr, 6).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        assert!(matches!(
            domain.get(Addr::new(6), 1).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    // Memory Management Tests
    #[test]
    fn test_alloc() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(5);

        // Test zero allocation
        let addr0 = domain.alloc(0)?;
        assert_eq!(addr0.0, 0, "Zero allocation should return address 0");

        // Test normal allocation
        let _addr1 = domain.alloc(3)?;
        assert_eq!(domain.len(), 3, "Length should match allocated size");

        // Test allocation at capacity
        let addr2 = domain.alloc(2)?;
        assert_eq!(addr2.0, 3, "Should allocate at correct address");

        // Test allocation beyond capacity
        assert!(matches!(
            domain.alloc(1).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    #[test]
    fn test_copy_items() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(10);

        // Setup initial data
        let addr = domain.push_all(&[1, 2, 3, 4, 5])?;
        assert_eq!(domain.len(), 5, "Initial length should be 5");

        // Test basic copy
        domain.copy_items(addr, Addr::new(2), 3)?;
        let copied = domain.get(Addr::new(2), 3)?;
        assert_eq!(copied, &[1, 2, 3][..], "Copied items should match");

        // Test zero-length copy (should be no-op)
        domain.copy_items(addr, Addr::new(2), 0)?;
        let zero_copy = domain.get(Addr::new(0), 5)?;
        assert_eq!(
            zero_copy,
            &[1, 2, 1, 2, 3][..],
            "Zero-length copy should not modify data"
        );

        // Test invalid copy operations
        assert!(matches!(
            domain
                .copy_items(Addr::new(4), Addr::new(0), 2)
                .unwrap_err(),
            MemoryError::OutOfBounds
        ));

        assert!(matches!(
            domain
                .copy_items(Addr::new(0), Addr::new(4), 2)
                .unwrap_err(),
            MemoryError::OutOfBounds
        ));

        // Test integer overflow cases
        assert!(matches!(
            domain
                .copy_items(Addr::new(u32::MAX - 1), Addr::new(0), 3)
                .unwrap_err(),
            MemoryError::OutOfBounds
        ));

        assert!(matches!(
            domain
                .copy_items(Addr::new(0), Addr::new(u32::MAX - 1), 3)
                .unwrap_err(),
            MemoryError::OutOfBounds
        ));

        Ok(())
    }

    // Block Tests
    #[test]
    fn test_block_operations() {
        let block = Block::<i32>::new(10, 5, Addr::new(0));
        assert_eq!(block.len(), 5);
        assert_eq!(block.cap(), 10);
        assert_eq!(block.data(), Addr::new(0));
        assert!(!block.is_empty());

        let empty_block = Block::<i32>::default();
        assert_eq!(empty_block.len(), 0);
        assert_eq!(empty_block.cap(), 0);
        assert_eq!(empty_block.data(), Addr::new(0));
        assert!(empty_block.is_empty());
    }

    // Memory Tests

    #[test]
    fn test_memory_block_allocation() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test block allocation
        let block_addr = memory.alloc_empty_block(5)?;

        // Get domain and block through GetDomain trait
        let (values, block) = memory.get_domain(block_addr)?;

        // Verify block properties
        assert_eq!(block.cap(), 5);
        assert_eq!(block.len(), 0);
        assert!(block.data().verify(values.capacity()));

        // Test block data access through domain
        let data = values.get(block.data(), block.cap())?;
        assert_eq!(data.len(), 5);
        assert!(data.iter().all(|v| matches!(v, VmValue::None)));

        Ok(())
    }

    #[test]
    fn test_memory_block_operations() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        let block_addr = memory.alloc_empty_block(5)?;

        // Push values using Addr<Block<T>> methods
        block_addr.push(VmValue::Int(42), &mut memory)?;
        block_addr.push(VmValue::Int(24), &mut memory)?;

        // Get block info
        let (domain, block) = memory.get_domain(block_addr)?;
        assert_eq!(block.len(), 2);
        let data = domain.get(block.data(), block.len())?;
        assert_eq!(data, &[VmValue::Int(42), VmValue::Int(24)]);

        // Pop values
        let val2 = block_addr.pop(&mut memory)?;
        let val1 = block_addr.pop(&mut memory)?;

        assert_eq!(val2, VmValue::Int(24));
        assert_eq!(val1, VmValue::Int(42));

        // Verify empty
        let (_, block) = memory.get_domain(block_addr)?;
        assert_eq!(block.len(), 0);

        Ok(())
    }

    #[test]
    fn test_memory_parser_support() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Begin a block
        memory.begin()?;

        // Push some values to the stack
        memory.stack.push(VmValue::Int(1), &mut memory.values)?;
        memory.stack.push(VmValue::Int(2), &mut memory.values)?;
        memory.stack.push(VmValue::Int(3), &mut memory.values)?;

        // End the block
        let block_addr = memory.end()?;

        // Verify the block contents
        let (domain, block) = memory.get_domain(block_addr)?;
        assert_eq!(block.len(), 3);

        let data = domain.get(block.data(), block.len())?;
        assert_eq!(data, &[VmValue::Int(1), VmValue::Int(2), VmValue::Int(3)]);

        Ok(())
    }

    #[test]
    fn test_string_and_symbol_handling() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test string allocation and content
        let str_addr = memory.alloc_string("Hello")?;
        let str_block = memory.get_block(str_addr)?;
        let str_bytes = memory.get_bytes(Addr::new(str_block.data().0), str_block.len())?;
        assert_eq!(str_bytes, b"Hello", "String content should match");

        // Test symbol management
        let symbol1 = memory.get_symbol("test")?;
        let symbol2 = memory.get_symbol("test")?;
        assert_eq!(symbol1, symbol2, "Same symbol should return same address");

        // Test symbol content
        let symbol_block = memory.get_block(symbol1)?;
        let symbol_bytes =
            memory.get_bytes(Addr::new(symbol_block.data().0), symbol_block.len())?;
        assert_eq!(symbol_bytes, b"test", "Symbol content should match");

        Ok(())
    }

    #[test]
    fn test_parser_integration() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test basic parsing
        memory
            .parse_block("1 2 \"test\"")
            .expect("Failed to parse basic block");

        // Test nested blocks
        memory
            .parse_block("1 [2 3] 4")
            .expect("Failed to parse nested blocks");

        // Test words and paths
        memory
            .parse_block("word: value word/path")
            .expect("Failed to parse words and paths");

        // Test error handling
        assert!(
            memory.parse_block("99999999999").is_err(),
            "Should detect integer overflow"
        );
        assert!(memory.parse_block(":").is_err(), "Should detect empty word");

        Ok(())
    }

    #[test]
    fn test_block_content_preservation() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Create a block with values
        let values = [VmValue::Int(1), VmValue::Int(2), VmValue::Int(3)];
        let block_addr = create_block_with_values(&mut memory, &values)?;

        // Push block to stack
        memory
            .stack
            .push(VmValue::Block(block_addr), &mut memory.values)?;

        // Pop block from stack
        let popped = memory.stack.pop(&mut memory.values)?;

        // Verify block content is preserved
        if let VmValue::Block(addr) = popped {
            let (domain, block) = memory.get_domain(addr)?;
            let content = domain.get(block.data(), block.len())?;
            assert_eq!(
                content, &values,
                "Block content should be preserved after stack operations"
            );
        } else {
            panic!("Expected Block value");
        }

        Ok(())
    }

    #[test]
    fn test_nested_block_integrity() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Create inner block
        let inner_values = [VmValue::Int(1), VmValue::Int(2)];
        let inner_block = create_block_with_values(&mut memory, &inner_values)?;

        // Create outer block containing the inner block
        let outer_values = [VmValue::Int(42), VmValue::Block(inner_block)];
        let outer_block = create_block_with_values(&mut memory, &outer_values)?;

        // Verify outer block structure
        let (domain, block) = memory.get_domain(outer_block)?;
        let content = domain.get(block.data(), block.len())?;
        assert_eq!(content.len(), 2, "Outer block should have 2 elements");
        assert_eq!(
            content[0],
            VmValue::Int(42),
            "First element should be preserved"
        );

        // Verify inner block content through reference
        if let VmValue::Block(addr) = content[1] {
            let (inner_domain, inner_block) = memory.get_domain(addr)?;
            let inner_content = inner_domain.get(inner_block.data(), inner_block.len())?;
            assert_eq!(
                inner_content, &inner_values,
                "Inner block content should be preserved"
            );
        } else {
            panic!("Expected Block value");
        }

        Ok(())
    }

    #[test]
    fn test_memory_error_conditions() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test invalid block address
        let invalid_addr = Addr::<Block<VmValue>>::new(999);
        assert!(matches!(
            memory.get_domain(invalid_addr).unwrap_err(),
            MemoryError::OutOfBounds
        ));

        // Test stack overflow
        let block_addr = memory.alloc_empty_block(2)?;
        block_addr.push(VmValue::Int(1), &mut memory)?;
        block_addr.push(VmValue::Int(2), &mut memory)?;
        assert!(matches!(
            block_addr.push(VmValue::Int(3), &mut memory).unwrap_err(),
            MemoryError::StackOverflow
        ));

        // Test stack underflow
        let empty_block_addr = memory.alloc_empty_block(1)?;
        assert!(matches!(
            empty_block_addr.pop(&mut memory).unwrap_err(),
            MemoryError::StackUnderflow
        ));

        // Test out of bounds access
        let (domain, block) = memory.get_domain(block_addr)?;
        assert!(matches!(
            domain.get(Addr::new(u32::MAX), 1).unwrap_err(),
            MemoryError::OutOfBounds
        ));

        Ok(())
    }
}
