// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Addr, Block, Domain, Memory, MemoryError, VmValue, Word};

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function to create a block with values
    fn create_block_with_values(
        memory: &mut Memory,
        values: &[VmValue],
    ) -> Result<Addr<Block<VmValue>>, MemoryError> {
        let block_addr = memory.alloc_empty_block(values.len() as Word)?;
        block_addr.push_all(values, memory)?;
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
        let _addr0 = domain.alloc(0)?;
        assert_eq!(domain.len(), 0, "Zero allocation should not change length");

        // Test normal allocation
        let _addr1 = domain.alloc(3)?;
        assert_eq!(domain.len(), 3, "Length should match allocated size");

        // Test allocation at capacity
        let addr2 = domain.alloc(2)?;
        assert_eq!(domain.len(), 5, "Length should be updated");
        
        // Verify that the allocated address is at the expected position
        let allocated_pos = addr2.address(domain.len())?;
        assert_eq!(allocated_pos, 3, "Should allocate at correct address");

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
        assert!(!block.is_empty());

        let empty_block = Block::<i32>::default();
        assert_eq!(empty_block.len(), 0);
        assert_eq!(empty_block.cap(), 0);
        assert!(empty_block.is_empty());
    }

    // Memory Tests

    #[test]
    fn test_memory_block_allocation() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test block allocation
        let block_addr = memory.alloc_empty_block(5)?;

        // Verify block is empty
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 0);
        
        // Push an item and check capacity
        block_addr.push(VmValue::Int(1), &mut memory)?;
        block_addr.push(VmValue::Int(2), &mut memory)?;
        
        // Test getting values back
        let values = block_addr.get_all(&memory)?;
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], VmValue::Int(1));
        assert_eq!(values[1], VmValue::Int(2));

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

        // Verify contents
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 2);
        assert_eq!(block_content, &[VmValue::Int(42), VmValue::Int(24)]);

        // Pop values
        let val2 = block_addr.pop(&mut memory)?;
        let val1 = block_addr.pop(&mut memory)?;

        assert_eq!(val2, VmValue::Int(24));
        assert_eq!(val1, VmValue::Int(42));

        // Verify empty
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 0);

        Ok(())
    }
    
    #[test]
    fn test_block_drop() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        let block_addr = memory.alloc_empty_block(5)?;
        
        // Test drop on empty block (should return StackUnderflow)
        let result = block_addr.drop(&mut memory);
        assert!(matches!(result.unwrap_err(), MemoryError::StackUnderflow), "Drop on empty block should return StackUnderflow");
        
        // Push items
        block_addr.push(VmValue::Int(42), &mut memory)?;
        block_addr.push(VmValue::Int(24), &mut memory)?;
        block_addr.push(VmValue::Int(13), &mut memory)?;
        
        // Verify initial content
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 3, "Block should have 3 items");
        assert_eq!(block_content, &[VmValue::Int(42), VmValue::Int(24), VmValue::Int(13)]);
        
        // Drop one item
        block_addr.drop(&mut memory)?;
        
        // Verify content after drop
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 2, "Block should have 2 items after drop");
        assert_eq!(block_content, &[VmValue::Int(42), VmValue::Int(24)], "Last item should be dropped");
        
        // Drop another item
        block_addr.drop(&mut memory)?;
        
        // Verify content after second drop
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 1, "Block should have 1 item after second drop");
        assert_eq!(block_content, &[VmValue::Int(42)], "Second-to-last item should be dropped");
        
        // Drop final item
        block_addr.drop(&mut memory)?;
        
        // Verify block is empty
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 0, "Block should be empty after all drops");
        
        // Try to drop from empty block (should return StackUnderflow)
        let result = block_addr.drop(&mut memory);
        assert!(matches!(result.unwrap_err(), MemoryError::StackUnderflow), "Drop on empty block should return StackUnderflow");
        
        Ok(())
    }
    
    #[test]
    fn test_block_peek() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        let block_addr = memory.alloc_empty_block(5)?;
        
        // Test peek on empty block
        let result = block_addr.peek(&memory)?;
        assert_eq!(result, None, "Peek on empty block should return None");
        
        // Add items and test peek
        block_addr.push(VmValue::Int(42), &mut memory)?;
        let result = block_addr.peek(&memory)?;
        assert_eq!(result, Some(VmValue::Int(42)), "Peek should return the top item");
        
        // Add another item and test peek again
        block_addr.push(VmValue::Int(24), &mut memory)?;
        let result = block_addr.peek(&memory)?;
        assert_eq!(result, Some(VmValue::Int(24)), "Peek should return the new top item");
        
        // Ensure peek doesn't modify the block
        let block_content = block_addr.get_all(&memory)?;
        assert_eq!(block_content.len(), 2, "Peek should not modify block length");
        assert_eq!(block_content, &[VmValue::Int(42), VmValue::Int(24)], "Block content should be unchanged");
        
        // Pop and verify peek updates
        let _ = block_addr.pop(&mut memory)?;
        let result = block_addr.peek(&memory)?;
        assert_eq!(result, Some(VmValue::Int(42)), "Peek should return the new top after pop");
        
        // Pop again and verify peek returns None
        let _ = block_addr.pop(&mut memory)?;
        let result = block_addr.peek(&memory)?;
        assert_eq!(result, None, "Peek should return None after popping all items");
        
        Ok(())
    }

    #[test]
    fn test_memory_parser_support() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Parse a block using the parse_block method which uses begin()/end() internally
        memory.parse_block("1 2 3").expect("Failed to parse block");
        
        // We should be able to continue parsing
        memory.parse_block("[4 5 6]").expect("Failed to parse nested block");

        Ok(())
    }

    #[test]
    fn test_string_and_symbol_handling() -> Result<(), MemoryError> {
        let mut memory = Memory::new(1024);
        memory.init()?;

        // Test string allocation and content
        let str_addr = memory.alloc_string("Hello")?;
        let str_bytes = str_addr.get_all(&memory)?;
        assert_eq!(str_bytes, b"Hello", "String content should match");

        // Test symbol management
        let symbol1 = memory.get_symbol("test")?;
        let symbol2 = memory.get_symbol("test")?;
        assert_eq!(symbol1, symbol2, "Same symbol should return same address");

        // Test symbol content
        let symbol_bytes = symbol1.get_all(&memory)?;
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

        // Create another block that references the first block
        let outer_block_addr = memory.alloc_empty_block(1)?;
        outer_block_addr.push(VmValue::Block(block_addr), &mut memory)?;

        // Verify the reference is preserved
        let outer_content = outer_block_addr.get_all(&memory)?;
        if let VmValue::Block(addr) = outer_content[0] {
            let content = addr.get_all(&memory)?;
            assert_eq!(
                content, &values,
                "Block content should be preserved through reference"
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
        let content = outer_block.get_all(&memory)?;
        assert_eq!(content.len(), 2, "Outer block should have 2 elements");
        assert_eq!(
            content[0],
            VmValue::Int(42),
            "First element should be preserved"
        );

        // Verify inner block content through reference
        if let VmValue::Block(addr) = content[1] {
            let inner_content = addr.get_all(&memory)?;
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
            invalid_addr.get_all(&memory).unwrap_err(),
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
        assert!(matches!(
            block_addr.get(999, &memory).unwrap_err(),
            MemoryError::OutOfBounds
        ));

        Ok(())
    }
}