// Memory system tests

use crate::mem::*;

const SYMBOL_TABLE_SIZE: u32 = 1024;
const PARSE_STACK_SIZE: u32 = 1024;
const PARSE_BASE_SIZE: u32 = 256;
const HEAP_SIZE: u32 = 4096;
const MEMORY_SIZE: usize = 8192;

// Helper function to create a new memory instance
fn new_memory<'a>(memory: &'a mut [u32]) -> Memory<'a> {
    Memory::init(
        memory,
        [
            SYMBOL_TABLE_SIZE,
            PARSE_STACK_SIZE,
            PARSE_BASE_SIZE,
            HEAP_SIZE,
        ],
    )
    .expect("Failed to initialize memory")
}

#[test]
fn test_memory_init() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let memory = new_memory(&mut memory_vec);

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
fn test_stack_operations_1() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_memory(&mut memory_vec);

    let base = memory.get_parse_base().unwrap();
    base.push(42424242, &mut memory).unwrap();
    assert_eq!(base.len(&memory), Some(1));
    assert_eq!(base.peek(&memory), Some(42424242));
    assert_eq!(base.pop(&mut memory), Some(42424242));
    assert_eq!(base.len(&memory), Some(0));
    assert_eq!(base.pop(&mut memory), None);
}

#[test]
fn test_stack_operations() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_memory(&mut memory_vec);

    // Get the parse stack
    let stack = memory.get_parse_stack().unwrap();

    // Check initial state
    assert_eq!(stack.len(&memory), Some(0));

    // Test push
    let value = MemValue::int(42);
    assert!(stack.push(value, &mut memory).is_some());
    assert_eq!(stack.len(&memory), Some(1));

    // Test peek
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
fn test_string_storage() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_memory(&mut memory_vec);

    // Get the heap
    let heap = memory.get_heap().unwrap();

    // Store a string
    let test_str = "Hello, Rebel!";
    let str_handle = heap.alloc_string(&mut memory, test_str).unwrap();

    // Verify length
    assert_eq!(str_handle.len(&memory), Some(test_str.len() as u32));

    // Verify content
    let bytes = str_handle.as_bytes(&memory).unwrap();
    assert_eq!(bytes, test_str.as_bytes());
}

#[test]
fn test_block_operations() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_memory(&mut memory_vec);

    // First, get the parse stack
    let stack = memory.get_parse_stack().unwrap();

    // Push some values to the stack
    let values = [MemValue::int(10), MemValue::int(20), MemValue::int(30)];
    for &val in &values {
        stack.push(val, &mut memory).unwrap();
    }

    // Create a block
    memory.begin().unwrap();
    for &val in &values {
        stack.push(val, &mut memory).unwrap();
    }

    println!("* after pushes stack {:?}", stack.peek(&memory).unwrap());
    println!(
        "* after pushes base {:?}",
        memory.get_parse_base().unwrap().peek(&memory).unwrap()
    );
    println!("* after pushes 3 {:?}", stack.get(3, &memory).unwrap());

    let block = memory.end().unwrap();

    // Verify block length
    assert_eq!(block.len(&memory), Some(3));

    // Verify block contents
    for (i, &val) in values.iter().enumerate() {
        assert_eq!(block.get(i as u32, &memory), Some(val));
    }
}

#[test]
fn test_symbol_table() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_memory(&mut memory_vec);

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
