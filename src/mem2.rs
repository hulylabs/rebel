// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

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
        self.0 >= cap
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
struct Block<T>(AnyBlock, PhantomData<T>);

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
        let index = self.data().next(self.0.len)?;
        if index.verify(self.0.cap) {
            Err(MemoryError::StackOverflow)
        } else {
            domain.get_item_mut(index).map(|slot| *slot = item)?;
            self.0.len += 1;
            Ok(())
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
    // contexts: Domain<Block<KeyValue>>,
    // strings: Domain<Block<u8>>,
    //
    values: Domain<VmValue>,
    pairs: Domain<KeyValue>,
    bytes: Domain<u8>,
    words: Domain<Word>,
    //
    symbols: HashMap<SmolStr, Addr<Block<u8>>>,
    system: HashMap<Addr<Block<u8>>, VmValue>,
    //
    stack: Block<VmValue>,
    op_stack: Block<Word>,
}

// Public accessor methods for Memory
impl Memory {
    pub fn new(capacity: usize) -> Self {
        Self {
            blocks: Domain::new(capacity),
            // contexts: Domain::new(capacity),
            // strings: Domain::new(capacity),
            //
            values: Domain::new(capacity),
            pairs: Domain::new(capacity),
            bytes: Domain::new(capacity),
            words: Domain::new(capacity),
            //
            symbols: HashMap::new(),
            system: HashMap::new(),
            //
            stack: Block::default(),
            op_stack: Block::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), MemoryError> {
        // let stack_space = self.values.alloc(256)?;
        // self.stack = Block::new(256, 0, stack_space);

        // self.stack = self.alloc_block(256)?;

        // let op_stack_space = self.words.alloc(256)?;
        // self.blocks.push(AnyBlock::new(256, 0, op_stack_space.0));

        Ok(())
    }

    // pub fn get_block(&self, addr: Addr<Block<VmValue>>) -> Result<&Block<VmValue>, MemoryError> {
    //     self.blocks.get_item(addr)
    // }

    // pub fn get_block_mut(
    //     &mut self,
    //     addr: Addr<Block<VmValue>>,
    // ) -> Result<&mut Block<VmValue>, MemoryError> {
    //     self.blocks.get_item_mut(addr)
    // }

    // pub fn get_string(&self, addr: Addr<Block<u8>>) -> Result<&Block<u8>, MemoryError> {
    //     self.strings.get_item(addr)
    // }

    // pub fn get_string_mut(&mut self, addr: Addr<Block<u8>>) -> Result<&mut Block<u8>, MemoryError> {
    //     self.strings.get_item_mut(addr)
    // }

    // pub fn get_context(
    //     &self,
    //     addr: Addr<Block<KeyValue>>,
    // ) -> Result<&Block<KeyValue>, MemoryError> {
    //     self.contexts.get_item(addr)
    // }

    // pub fn get_context_mut(
    //     &mut self,
    //     addr: Addr<Block<KeyValue>>,
    // ) -> Result<&mut Block<KeyValue>, MemoryError> {
    //     self.contexts.get_item_mut(addr)
    // }

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

    pub fn alloc_block(&mut self, cap: Word) -> Result<Addr<Block<VmValue>>, MemoryError> {
        let data = self.values.alloc(cap)?;
        self.alloc_block_header(cap, 0, data.0)
    }

    // pub fn alloc_string(&mut self, cap: Word) -> Result<Addr<Block<u8>>, MemoryError> {
    //     let data = self.bytes.alloc(cap)?;
    //     let block = Block::new(cap, 0, data);
    //     self.strings.push(block)
    // }

    // pub fn alloc_context(&mut self, cap: Word) -> Result<Addr<Block<KeyValue>>, MemoryError> {
    //     let data = self.pairs.alloc(cap)?;
    //     let block = Block::new(cap, 0, data);
    //     self.contexts.push(block)
    // }

    pub fn get_values(&self, addr: Addr<VmValue>, len: Word) -> Result<&[VmValue], MemoryError> {
        self.values.get(addr, len)
    }

    pub fn get_values_mut(
        &mut self,
        addr: Addr<VmValue>,
        len: Word,
    ) -> Result<&mut [VmValue], MemoryError> {
        self.values.get_mut(addr, len)
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
}

impl<'a> GetDomain<'a, VmValue> for Memory {
    fn get_domain(
        &self,
        addr: Addr<Block<VmValue>>,
    ) -> Result<(&Domain<VmValue>, &Block<VmValue>), MemoryError> {
        let x = self.blocks.get_item(Addr::new(addr.0))?;
        Ok((&self.values,))
    }

    fn get_domain_mut(
        &mut self,
        addr: Addr<AnyBlock>,
    ) -> Result<(&mut Domain<VmValue>, &mut AnyBlock), MemoryError> {
        Ok((&mut self.values, self.blocks.get_item_mut(addr)?))
    }
}

impl<'a> GetDomain<'a, Word> for Memory {
    fn get_domain(&self, addr: Addr<AnyBlock>) -> Result<(&Domain<Word>, &AnyBlock), MemoryError> {
        Ok((&self.words, self.blocks.get_item(addr)?))
    }

    fn get_domain_mut(
        &mut self,
        addr: Addr<AnyBlock>,
    ) -> Result<(&mut Domain<Word>, &mut AnyBlock), MemoryError> {
        Ok((&mut self.words, self.blocks.get_item_mut(addr)?))
    }
}

//

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(block.capacity(), 10);
        assert_eq!(block.data(), Addr::new(0));
        assert!(!block.is_empty());

        let empty_block = Block::<i32>::default();
        assert_eq!(empty_block.len(), 0);
        assert_eq!(empty_block.capacity(), 0);
        assert_eq!(empty_block.data(), Addr::new(0));
        assert!(empty_block.is_empty());
    }

    // Memory Tests
    #[test]
    fn test_memory_initialization() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        assert_eq!(memory.stack.capacity(), 256);
        assert_eq!(memory.stack.len(), 0);
        assert_eq!(memory.op_stack.capacity(), 256);
        assert_eq!(memory.op_stack.len(), 0);

        // Test stack access
        let stack = memory.get_stack();
        assert!(stack.is_empty());
        assert_eq!(stack.capacity(), 256);

        let op_stack = memory.get_op_stack();
        assert!(op_stack.is_empty());
        assert_eq!(op_stack.capacity(), 256);

        Ok(())
    }

    #[test]
    fn test_memory_block_operations() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test block allocation
        let block_addr = memory.alloc_block(5)?;
        {
            let block = memory.get_block(block_addr)?;
            assert_eq!(block.capacity(), 5);
            assert_eq!(block.len(), 0);

            // Test block data access
            let values = memory.get_values(block.data(), block.capacity())?;
            assert_eq!(values.len(), 5);
            assert!(values.iter().all(|v| matches!(v, VmValue::None)));
        }

        // Test block mutation
        {
            let block_mut = memory.get_block_mut(block_addr)?;
            assert_eq!(block_mut.capacity(), 5);
        }

        // Get block data address for mutation
        let data_addr = memory.get_block(block_addr)?.data();

        // Test block data mutation
        {
            let values_mut = memory.get_values_mut(data_addr, 5)?;
            values_mut[0] = VmValue::Int(42);
        }

        // Verify mutation
        {
            let values = memory.get_values(data_addr, 5)?;
            assert_eq!(values[0], VmValue::Int(42));
        }

        // Test invalid block access
        assert!(matches!(
            memory.get_block(Addr::new(999)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    #[test]
    fn test_memory_string_operations() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test string allocation and get data address
        let string_addr = memory.alloc_string(10)?;
        let data_addr = {
            let string = memory.get_string(string_addr)?;
            assert_eq!(string.capacity(), 10);
            assert_eq!(string.len(), 0);
            string.data()
        };

        // Test initial string data
        {
            let bytes = memory.get_bytes(data_addr, 10)?;
            assert_eq!(bytes.len(), 10);
            assert!(bytes.iter().all(|&b| b == 0));
        }

        // Test string data mutation
        {
            let bytes_mut = memory.get_bytes_mut(data_addr, 10)?;
            bytes_mut[0] = b'A';
        }

        // Verify mutation
        {
            let bytes = memory.get_bytes(data_addr, 10)?;
            assert_eq!(bytes[0], b'A');
        }

        // Test string mutation
        {
            let string_mut = memory.get_string_mut(string_addr)?;
            assert_eq!(string_mut.capacity(), 10);
        }

        // Test invalid string access
        assert!(matches!(
            memory.get_string(Addr::new(999)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    #[test]
    fn test_memory_context_operations() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test context allocation and get data address
        let context_addr = memory.alloc_context(8)?;
        let data_addr = {
            let context = memory.get_context(context_addr)?;
            assert_eq!(context.capacity(), 8);
            assert_eq!(context.len(), 0);
            context.data()
        };

        // Test initial context data
        {
            let pairs = memory.get_pairs(data_addr, 8)?;
            assert_eq!(pairs.len(), 8);
            assert!(pairs.iter().all(|p| matches!(p.value, VmValue::None)));
        }

        // Test context data mutation
        {
            let pairs_mut = memory.get_pairs_mut(data_addr, 8)?;
            pairs_mut[0].value = VmValue::Int(42);
        }

        // Verify mutation
        {
            let pairs = memory.get_pairs(data_addr, 8)?;
            assert_eq!(pairs[0].value, VmValue::Int(42));
        }

        // Test context mutation
        {
            let context_mut = memory.get_context_mut(context_addr)?;
            assert_eq!(context_mut.capacity(), 8);
        }

        // Test invalid context access
        assert!(matches!(
            memory.get_context(Addr::new(999)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    // Integration test combining multiple operations
    #[test]
    fn test_domain_integration() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(10);

        // Push single items
        let addr1 = domain.push(42)?;
        assert_eq!(domain.len(), 1);
        assert!(!domain.is_empty());
        let item1 = domain.get_item(addr1)?;
        assert_eq!(item1, &42);

        // Push multiple items
        let addr2 = domain.push_all(&[1, 2, 3])?;
        assert_eq!(domain.len(), 4);
        let slice2 = domain.get(addr2, 3)?;
        assert_eq!(slice2, [1, 2, 3].as_slice());

        // Allocate space
        let addr3 = domain.alloc(3)?;
        assert_eq!(domain.len(), 7);

        // Copy items
        domain.copy_items(Addr::new(1), Addr::new(4), 3)?;
        let copied = domain.get(Addr::new(4), 3)?;
        assert_eq!(copied, [1, 2, 3].as_slice());

        // Verify final state
        assert_eq!(domain.len(), 7);
        let final1 = domain.get_item(addr1)?;
        assert_eq!(final1, &42);
        let final2 = domain.get(addr2, 3)?;
        assert_eq!(final2, [1, 2, 3].as_slice());
        let final3 = domain.get(addr3, 3)?;
        assert_eq!(final3, [1, 2, 3].as_slice());
        Ok(())
    }

    #[test]
    fn test_block_push() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        let block_addr = memory.alloc_block(10)?;
        let block = memory.get_block_mut(block_addr)?;
        let values = &mut memory.values;

        // Test pushing items
        block.push(VmValue::Int(42), values)?;
        assert_eq!(block.len(), 1);
        block.push(VmValue::Int(24), values)?;
        assert_eq!(block.len(), 2);

        // Test popping items
        let popped1 = block.pop(values)?;
        assert_eq!(popped1, VmValue::Int(24));
        assert_eq!(block.len(), 1);
        let popped2 = block.pop(values)?;
        assert_eq!(popped2, VmValue::Int(42));
        assert_eq!(block.len(), 0);

        // Test underflow
        assert!(matches!(
            block.pop(values).unwrap_err(),
            MemoryError::StackUnderflow
        ));
        Ok(())
    }
}

// DO NOT USE FOLLOWING CODE:

pub fn copy_items(
    domain: &mut Domain<usize>,
    from: Addr<usize>,
    to: Addr<usize>,
    items: Word,
) -> Result<(), MemoryError> {
    domain.copy_items(from, to, items)
}
