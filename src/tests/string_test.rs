// String management tests for the memory system
use crate::mem::*;
use crate::tests::test_utils::*;

#[test]
fn test_string_allocation_and_retrieval() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Get the heap
    let heap = memory.get_heap().unwrap();

    // Store a string
    let test_str = "Hello, Rebel!";
    let string_handle = heap.alloc_string(&mut memory, test_str).unwrap();

    // Verify string length
    assert_eq!(string_handle.len(&memory), Some(test_str.len() as u32));

    // Verify string content
    let bytes = string_handle.as_bytes(&memory).unwrap();
    assert_eq!(bytes, test_str.as_bytes());

    // Print string information
    println!("String length: {:?}", string_handle.len(&memory));
    println!("String content: {:?}", std::str::from_utf8(bytes).unwrap());
}

#[test]
fn test_parse_and_retrieve_block() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create a block directly
    let stack = memory.get_parse_stack().unwrap();
    memory.begin().unwrap();

    // Push some values to the stack
    stack.push(VmValue::Int(10), &mut memory).unwrap();
    stack.push(VmValue::Int(20), &mut memory).unwrap();
    stack.push(VmValue::Int(30), &mut memory).unwrap();

    // Verify stack length
    assert_eq!(stack.len(&memory), Some(3));

    // End the block and get its handle
    let block = memory.end().unwrap();

    // Verify block length
    assert_eq!(block.len(&memory), Some(3));

    // Verify block content by accessing individual elements
    assert_eq!(block.get(0, &memory), Some(VmValue::Int(10)));
    assert_eq!(block.get(1, &memory), Some(VmValue::Int(20)));
    assert_eq!(block.get(2, &memory), Some(VmValue::Int(30)));

    // Verify out-of-bounds access returns None
    assert_eq!(block.get(3, &memory), None);
}

#[test]
fn test_multiple_string_allocations() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);
    let heap = memory.get_heap().unwrap();

    // Allocate multiple strings
    let strings = ["First string", "Second string", "Third string"];
    let mut handles = Vec::new();

    for s in &strings {
        let handle = heap.alloc_string(&mut memory, s).unwrap();
        handles.push(handle);
    }

    // Verify each string can be retrieved correctly
    for (i, handle) in handles.iter().enumerate() {
        let bytes = handle.as_bytes(&memory).unwrap();
        assert_eq!(bytes, strings[i].as_bytes());
    }
}

#[test]
fn test_memory_system_integration() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = new_equal_region_memory(&mut memory_vec);

    // Create a string
    let heap = memory.get_heap().unwrap();
    let test_str = "String in block";
    let str_handle = heap.alloc_string(&mut memory, test_str).unwrap();

    // Create a block with integer values
    memory.begin().unwrap();
    let stack = memory.get_parse_stack().unwrap();
    stack.push(VmValue::Int(42), &mut memory).unwrap();
    stack
        .push(VmValue::String(str_handle), &mut memory)
        .unwrap();

    let block = memory.end().unwrap();

    // Verify we can access the integer
    assert_eq!(block.get(0, &memory), Some(VmValue::Int(42)));

    // Verify we can access the second element (which should be the string)
    if block.get(1, &memory).is_none() {
        panic!("Expected a value at index 1");
    }
}
