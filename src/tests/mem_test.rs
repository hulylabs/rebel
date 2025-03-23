// Memory system core tests - focuses on memory initialization, item serialization, and core operations

use crate::mem::*;
use crate::tests::test_utils::*;

#[test]
fn test_memory_init() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let memory = new_test_memory(&mut memory_vec);

    // Test that we can access the different regions
    assert!(memory.get_symbol_table().is_some());
    assert!(memory.get_parse_stack().is_some());
    assert!(memory.get_parse_base().is_some());
    assert!(memory.get_heap().is_some());

    assert_eq!(memory.get_parse_stack().unwrap().len(&memory), Some(0));
}

#[test]
fn test_item_implementations() {
    // Test u8 implementation
    let mut data = [0u8; 1];
    let value: u8 = 42;
    assert!(value.store(&mut data).is_some());
    assert_eq!(u8::load(&data), Some(42));

    // Test Word implementation
    let mut data = [0u8; 4];
    let value: u32 = 0x12345678;
    assert!(value.store(&mut data).is_some());
    assert_eq!(u32::load(&data), Some(0x12345678));

    // Test MemValue implementation
    let mut data = [0u8; 8];
    let value = MemValue::int(42);
    assert!(value.store(&mut data).is_some());
    let loaded = MemValue::load(&data).unwrap();
    assert_eq!(loaded, value);
}

#[test]
fn test_stack_operations() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_test_memory(&mut memory_vec);

    // Test parse_base stack (basic u32 operations)
    let base = memory.get_parse_base().unwrap();
    base.push(42424242, &mut memory).unwrap();
    assert_eq!(base.len(&memory), Some(1));
    assert_eq!(base.peek(&memory), Some(42424242));
    assert_eq!(base.pop(&mut memory), Some(42424242));
    assert_eq!(base.len(&memory), Some(0));
    assert_eq!(base.pop(&mut memory), None); // Empty stack pop returns None

    // Test parse_stack (MemValue operations)
    let stack = memory.get_parse_stack().unwrap();

    // Check initial state
    assert_eq!(stack.len(&memory), Some(0));

    // Test push and peek
    let value = MemValue::int(42);
    assert!(stack.push(value, &mut memory).is_some());
    assert_eq!(stack.len(&memory), Some(1));
    assert_eq!(stack.peek(&memory), Some(value));

    // Test pop
    assert_eq!(stack.pop(&mut memory), Some(value));
    assert_eq!(stack.len(&memory), Some(0));

    // Test pushing multiple values
    let values = [MemValue::int(1), MemValue::int(2), MemValue::int(3)];
    for &val in &values {
        assert!(stack.push(val, &mut memory).is_some());
    }

    assert_eq!(stack.len(&memory), Some(3));

    // Pop values in reverse order
    for &val in values.iter().rev() {
        assert_eq!(stack.pop(&mut memory), Some(val));
    }

    assert_eq!(stack.len(&memory), Some(0));
}

#[test]
fn test_symbol_table() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_test_memory(&mut memory_vec);

    let symbol_table = memory.get_symbol_table().unwrap();
    let heap = memory.get_heap().unwrap();

    // Insert a symbol
    let symbol1 = "test-symbol";
    let addr1 = symbol_table
        .get_or_insert_symbol(symbol1, heap.clone(), &mut memory)
        .unwrap();

    // Look up the same symbol
    let addr2 = symbol_table
        .get_or_insert_symbol(symbol1, heap.clone(), &mut memory)
        .unwrap();

    // Same symbol should have the same address
    assert_eq!(addr1.0, addr2.0);

    // Insert a different symbol
    let symbol2 = "another-symbol";
    let addr3 = symbol_table
        .get_or_insert_symbol(symbol2, heap.clone(), &mut memory)
        .unwrap();

    // Different symbols should have different addresses
    assert_ne!(addr1.0, addr3.0);
}
