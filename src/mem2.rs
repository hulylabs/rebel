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

    pub fn prev(self, n: Word) -> Result<Self, MemoryError> {
        self.0
            .checked_sub(n)
            .map(Self::new)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn next(self, n: Word) -> Result<Self, MemoryError> {
        self.0
            .checked_add(n)
            .map(Self::new)
            .ok_or(MemoryError::OutOfBounds)
    }

    pub fn verify(self, cap: Word) -> Result<Self, MemoryError> {
        if self.0 < cap {
            Ok(self)
        } else {
            Err(MemoryError::OutOfBounds)
        }
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

//

pub struct Memory {
    blocks: Domain<Block<VmValue>>,
    contexts: Domain<Block<KeyValue>>,
    strings: Domain<Block<u8>>,
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
            contexts: Domain::new(capacity),
            strings: Domain::new(capacity),
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
        let stack_space = self.values.alloc(256)?;
        self.stack = Block::new(256, 0, stack_space);
        let op_stack_space = self.words.alloc(256)?;
        self.op_stack = Block::new(256, 0, op_stack_space);
        Ok(())
    }

    // Block accessor methods
    pub fn get_block(&self, addr: Addr<Block<VmValue>>) -> Result<&Block<VmValue>, MemoryError> {
        self.blocks.get_item(addr)
    }

    pub fn get_block_mut(
        &mut self,
        addr: Addr<Block<VmValue>>,
    ) -> Result<&mut Block<VmValue>, MemoryError> {
        self.blocks.get_item_mut(addr)
    }

    pub fn get_string(&self, addr: Addr<Block<u8>>) -> Result<&Block<u8>, MemoryError> {
        self.strings.get_item(addr)
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
    fn test_domain_capacity() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(3);
        assert!(domain.push(1).is_some(), "First push should succeed");
        assert!(domain.push(2).is_some(), "Second push should succeed");
        assert!(domain.push(3).is_some(), "Third push should succeed");
        assert!(domain.push(4).is_none(), "Push beyond capacity should fail");
        Ok(())
    }

    // Single Item Operations Tests
    #[test]
    fn test_push_and_get() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(5);

        // Test push and get_item
        let addr1 = domain.push(42).ok_or(MemoryError::OutOfBounds)?;
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
        let addr = domain.push(42).ok_or(MemoryError::OutOfBounds)?;

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
        let addr0 = domain.alloc(0).ok_or(MemoryError::OutOfBounds)?;
        assert_eq!(addr0.0, 0, "Zero allocation should return address 0");

        // Test normal allocation
        let _addr1 = domain.alloc(3).ok_or(MemoryError::OutOfBounds)?;
        assert_eq!(domain.len(), 3, "Length should match allocated size");

        // Test allocation at capacity
        let addr2 = domain.alloc(2).ok_or(MemoryError::OutOfBounds)?;
        assert_eq!(addr2.0, 3, "Should allocate at correct address");

        // Test allocation beyond capacity
        assert!(
            domain.alloc(1).is_none(),
            "Should fail when exceeding capacity"
        );
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
        assert!(
            matches!(
                domain
                    .copy_items(Addr::new(4), Addr::new(0), 2)
                    .unwrap_err(),
                MemoryError::OutOfBounds
            ),
            "Should fail when source range exceeds length"
        );

        assert!(
            matches!(
                domain
                    .copy_items(Addr::new(0), Addr::new(4), 2)
                    .unwrap_err(),
                MemoryError::OutOfBounds
            ),
            "Should fail when destination range exceeds length"
        );

        // Test integer overflow cases
        assert!(
            matches!(
                domain
                    .copy_items(Addr::new(u32::MAX - 1), Addr::new(0), 3)
                    .unwrap_err(),
                MemoryError::OutOfBounds
            ),
            "Should fail when source range would overflow"
        );

        assert!(
            matches!(
                domain
                    .copy_items(Addr::new(0), Addr::new(u32::MAX - 1), 3)
                    .unwrap_err(),
                MemoryError::OutOfBounds
            ),
            "Should fail when destination range would overflow"
        );

        Ok(())
    }

    // Boundary Tests
    #[test]
    fn test_address_boundaries() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(5);
        domain.push_all(&[1, 2, 3, 4, 5])?;

        // Test address 0
        assert!(
            domain.get_item(Addr::new(0)).is_ok(),
            "Should access address 0"
        );

        // Test last valid address
        assert!(
            domain.get_item(Addr::new(4)).is_ok(),
            "Should access last valid address"
        );

        // Test first invalid address
        assert!(matches!(
            domain.get_item(Addr::new(5)).unwrap_err(),
            MemoryError::OutOfBounds
        ));

        // Test max address
        assert!(matches!(
            domain.get_item(Addr::new(u32::MAX)).unwrap_err(),
            MemoryError::OutOfBounds
        ));
        Ok(())
    }

    // Integration test combining multiple operations
    #[test]
    fn test_domain_integration() -> Result<(), MemoryError> {
        let mut domain: Domain<i32> = Domain::new(10);

        // Push single items
        let addr1 = domain.push(42).ok_or(MemoryError::OutOfBounds)?;
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
        let addr3 = domain.alloc(3).ok_or(MemoryError::OutOfBounds)?;
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
