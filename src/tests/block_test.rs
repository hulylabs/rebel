// Block-specific tests for the memory system

use crate::mem::*;
use crate::tests::test_utils::*;

#[test]
fn test_block_creation_and_read() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create a block with 3 integers
    memory.begin().unwrap();
    let stack = memory.get_parse_stack().unwrap();

    let test_values = [10, 20, 30];
    for &val in &test_values {
        stack.push(MemValue::int(val), &mut memory).unwrap();
    }

    let block = memory.end().unwrap();

    // Test that we can read back the values
    for (i, &val) in test_values.iter().enumerate() {
        assert_eq!(
            block.get(i as u32, &memory),
            Some(MemValue::int(val)),
            "Failed to read value at index {}",
            i
        );
    }

    // Test out-of-bounds read
    assert_eq!(block.get(test_values.len() as u32, &memory), None);
}

#[test]
fn test_block_modification() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create a block with 2 integers
    memory.begin().unwrap();
    let stack = memory.get_parse_stack().unwrap();
    stack.push(MemValue::int(1), &mut memory).unwrap();
    stack.push(MemValue::int(2), &mut memory).unwrap();
    let block = memory.end().unwrap();

    // Verify initial values
    assert_eq!(block.get(0, &memory), Some(MemValue::int(1)));
    assert_eq!(block.get(1, &memory), Some(MemValue::int(2)));

    // Modify a value
    let new_value = MemValue::int(99);
    assert!(block.set(0, new_value, &mut memory).is_some());

    // Verify the modification
    assert_eq!(
        block.get(0, &memory),
        Some(new_value),
        "Modified value not read correctly"
    );
    assert_eq!(
        block.get(1, &memory),
        Some(MemValue::int(2)),
        "Unmodified value changed"
    );
}

#[test]
fn test_multiple_blocks() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create first block with [1, 2]
    memory.begin().unwrap();
    let stack = memory.get_parse_stack().unwrap();
    stack.push(MemValue::int(1), &mut memory).unwrap();
    stack.push(MemValue::int(2), &mut memory).unwrap();
    let block1 = memory.end().unwrap();

    // Create second block with [3, 4]
    memory.begin().unwrap();
    stack.push(MemValue::int(3), &mut memory).unwrap();
    stack.push(MemValue::int(4), &mut memory).unwrap();
    let block2 = memory.end().unwrap();

    // Verify both blocks contain the expected values
    assert_eq!(block1.get(0, &memory), Some(MemValue::int(1)));
    assert_eq!(block1.get(1, &memory), Some(MemValue::int(2)));

    assert_eq!(block2.get(0, &memory), Some(MemValue::int(3)));
    assert_eq!(block2.get(1, &memory), Some(MemValue::int(4)));
}

#[test]
fn test_empty_block() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create an empty block
    memory.begin().unwrap();
    let empty_block = memory.end().unwrap();

    // Verify empty block behavior
    assert_eq!(empty_block.len(&memory), Some(0));
    assert_eq!(empty_block.get(0, &memory), None);
}

#[test]
fn test_large_block() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create a larger block with 100 values
    memory.begin().unwrap();
    let stack = memory.get_parse_stack().unwrap();

    let count = 100;
    for i in 0..count {
        if stack.push(MemValue::int(i as i32), &mut memory).is_none() {
            panic!("Failed to push item {} to stack", i);
        }
    }

    let block = memory.end().unwrap();

    // Verify all values can be retrieved correctly
    for i in 0..count {
        assert_eq!(
            block.get(i as u32, &memory),
            Some(MemValue::int(i as i32)),
            "Failed to read value at index {}",
            i
        );
    }
}
