// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use std::marker::PhantomData;
use std::ops::Range;

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

    pub fn verify(self, cap: Word) -> Option<Self> {
        if self.0 < cap { Some(self) } else { None }
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

    /// Returns the current length of the domain
    pub fn len(&self) -> Word {
        self.len
    }

    /// Returns true if the domain is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
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
        // Convert addresses using total capacity
        let total_capacity = self.items.len() as Word;
        let from = from.address(total_capacity)?;
        let to = to.address(total_capacity)?;
        let items = items as usize;

        // Debug output
        println!("Move operation:");
        println!("  Domain length: {}", self.len);
        println!("  From address: {}", from);
        println!("  To address: {}", to);
        println!("  Items to move: {}", items);
        println!("  Total capacity: {}", self.items.len());

        // Validate source range is within current length
        if from + items > self.len as usize {
            println!(
                "  Failed source range check: {} + {} > {}",
                from, items, self.len
            );
            return None;
        }

        // Validate destination range is within total capacity
        if to + items > self.items.len() {
            println!(
                "  Failed destination capacity check: {} + {} > {}",
                to,
                items,
                self.items.len()
            );
            return None;
        }

        // Update length if needed
        let new_end = to + items;
        if new_end as Word > self.len {
            self.len = new_end as Word;
        }

        // Copy items
        for i in 0..items {
            self.items[to + i] = self.items[from + i];
        }

        Some(())
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
        assert!(domain.push(1).is_some(), "First push should succeed");
        assert!(domain.push(2).is_some(), "Second push should succeed");
        assert!(domain.push(3).is_some(), "Third push should succeed");
        assert!(domain.push(4).is_none(), "Push beyond capacity should fail");
    }

    // Single Item Operations Tests
    #[test]
    fn test_push_and_get() {
        let mut domain: Domain<i32> = Domain::new(5);

        // Test push and get_item
        let addr1 = domain.push(42).unwrap();
        assert_eq!(domain.get_item(addr1), Some(&42), "Should get pushed item");

        // Test get_item with invalid address
        assert_eq!(
            domain.get_item(Addr::new(5)),
            None,
            "Should return None for invalid address"
        );
        assert_eq!(
            domain.get_item(Addr::new(u32::MAX)),
            None,
            "Should return None for max address"
        );
    }

    #[test]
    fn test_get_item_mut() {
        let mut domain: Domain<i32> = Domain::new(5);
        let addr = domain.push(42).unwrap();

        // Test get_item_mut and modify value
        if let Some(value) = domain.get_item_mut(addr) {
            *value = 24;
        }
        assert_eq!(domain.get_item(addr), Some(&24), "Value should be modified");

        // Test get_item_mut with invalid address
        assert!(
            domain.get_item_mut(Addr::new(5)).is_none(),
            "Should return None for invalid address"
        );
    }

    // Multiple Items Operations Tests
    #[test]
    fn test_push_all() {
        let mut domain: Domain<i32> = Domain::new(10);

        // Test pushing empty slice
        let _addr_empty = domain.push_all(&[]).unwrap();
        assert_eq!(
            domain.len(),
            0,
            "Pushing empty slice shouldn't change length"
        );

        // Test pushing multiple items
        let items = [1, 2, 3, 4];
        let addr = domain.push_all(&items).unwrap();
        assert_eq!(
            domain.get(addr, 4),
            Some(&items[..]),
            "Should get all pushed items"
        );

        // Test pushing beyond capacity
        assert!(
            domain.push_all(&[5, 5, 5, 5, 5, 5, 5]).is_none(),
            "Should fail when exceeding capacity"
        );
    }

    #[test]
    fn test_get_range() {
        let mut domain: Domain<i32> = Domain::new(10);
        let items = [1, 2, 3, 4, 5];
        let addr = domain.push_all(&items).unwrap();

        // Test valid ranges
        assert_eq!(
            domain.get(addr, 3),
            Some(&items[..3]),
            "Should get correct slice"
        );
        let empty_slice: &[i32] = &[];
        assert_eq!(
            domain.get(addr, 0),
            Some(empty_slice),
            "Should get empty slice"
        );

        // Test invalid ranges
        assert!(
            domain.get(addr, 6).is_none(),
            "Should return None for invalid length"
        );
        assert!(
            domain.get(Addr::new(6), 1).is_none(),
            "Should return None for invalid address"
        );
    }

    // Memory Management Tests
    #[test]
    fn test_alloc() {
        let mut domain: Domain<i32> = Domain::new(5);

        // Test zero allocation
        let addr0 = domain.alloc(0).unwrap();
        assert_eq!(addr0.0, 0, "Zero allocation should return address 0");

        // Test normal allocation
        let _addr1 = domain.alloc(3).unwrap();
        assert_eq!(domain.len(), 3, "Length should match allocated size");

        // Test allocation at capacity
        let addr2 = domain.alloc(2).unwrap();
        assert_eq!(addr2.0, 3, "Should allocate at correct address");

        // Test allocation beyond capacity
        assert!(
            domain.alloc(1).is_none(),
            "Should fail when exceeding capacity"
        );
    }

    #[test]
    fn test_move_items() {
        let mut domain: Domain<i32> = Domain::new(10);

        // Setup initial data
        let addr = domain.push_all(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(domain.len(), 5, "Initial length should be 5");
        println!("\nInitial state:");
        println!("  Domain length: {}", domain.len());
        println!("  Initial data address: {}", addr.0);

        // Test non-overlapping move
        println!("\nAttempting non-overlapping move:");
        let move_result = domain.move_items(addr, Addr::new(5), 3);
        println!("  Move result: {:?}", move_result.is_some());

        let moved_data = domain.get(Addr::new(5), 3);
        println!("  Moved data: {:?}", moved_data);
        assert_eq!(moved_data, Some(&[1, 2, 3][..]), "Moved items should match");

        // Test overlapping move
        println!("\nAttempting overlapping move:");
        let overlap_result = domain.move_items(Addr::new(1), Addr::new(0), 3);
        println!("  Overlap result: {:?}", overlap_result.is_some());

        let overlapped_data = domain.get(Addr::new(0), 3);
        println!("  Overlapped data: {:?}", overlapped_data);
        assert_eq!(
            overlapped_data,
            Some(&[2, 3, 4][..]),
            "Overlapped move should work"
        );

        // Test invalid move operations
        println!("\nTesting invalid moves:");
        assert!(
            domain.move_items(Addr::new(8), Addr::new(0), 3).is_none(),
            "Move from invalid source should fail"
        );
        assert!(
            domain.move_items(Addr::new(0), Addr::new(8), 3).is_none(),
            "Move to invalid destination should fail"
        );
    }

    // Boundary Tests
    #[test]
    fn test_address_boundaries() {
        let mut domain: Domain<i32> = Domain::new(5);
        domain.push_all(&[1, 2, 3, 4, 5]).unwrap();

        // Test address 0
        assert!(
            domain.get_item(Addr::new(0)).is_some(),
            "Should access address 0"
        );

        // Test last valid address
        assert!(
            domain.get_item(Addr::new(4)).is_some(),
            "Should access last valid address"
        );

        // Test first invalid address
        assert!(
            domain.get_item(Addr::new(5)).is_none(),
            "Should fail for first invalid address"
        );

        // Test max address
        assert!(
            domain.get_item(Addr::new(u32::MAX)).is_none(),
            "Should fail for max address"
        );
    }

    // Integration test combining multiple operations
    #[test]
    fn test_domain_integration() {
        let mut domain: Domain<i32> = Domain::new(10);

        // Push single items
        let addr1 = domain.push(42).unwrap();
        assert_eq!(domain.len(), 1);
        assert!(!domain.is_empty());
        assert_eq!(domain.get_item(addr1), Some(&42));

        // Push multiple items
        let addr2 = domain.push_all(&[1, 2, 3]).unwrap();
        assert_eq!(domain.len(), 4);
        assert_eq!(domain.get(addr2, 3), Some([1, 2, 3].as_slice()));

        // Allocate space
        let addr3 = domain.alloc(3).unwrap();
        assert_eq!(domain.len(), 7);

        // Copy items
        domain.move_items(Addr::new(1), Addr::new(4), 3).unwrap();
        assert_eq!(domain.get(Addr::new(4), 3), Some([1, 2, 3].as_slice()));

        // Verify final state
        assert_eq!(domain.len(), 7);
        assert_eq!(domain.get_item(addr1), Some(&42));
        assert_eq!(domain.get(addr2, 3), Some([1, 2, 3].as_slice()));
        assert_eq!(domain.get(addr3, 3), Some([1, 2, 3].as_slice()));
    }
}
