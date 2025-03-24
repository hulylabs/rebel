// String management tests for the memory system
use crate::mem::*;
use crate::tests::test_utils::*;

#[test]
fn test_string_allocation_and_retrieval() {
    let mut memory = new_test_memory();

    // Store a string
    let test_str = "Hello, Rebel!";
    let string_addr = memory.alloc_string(test_str).unwrap();

    // Get the string block using our helper method
    let string_block = memory.get_string_block(string_addr).unwrap();

    // Verify string length
    assert_eq!(string_block.len(), test_str.len() as Word);

    // Verify string content by reading the bytes
    let bytes_slice = memory.get_string_bytes(string_addr).unwrap();
    assert_eq!(bytes_slice, test_str.as_bytes());

    // Convert bytes back to string for verification
    let retrieved_str = std::str::from_utf8(bytes_slice).unwrap();
    assert_eq!(retrieved_str, test_str);
}

#[test]
fn test_string_in_vm_value() {
    let mut memory = new_test_memory();

    // Store a string and create a VmValue that references it
    let test_str = "String value";
    let string_addr = memory.alloc_string(test_str).unwrap();
    let string_value = VmValue::String(string_addr);

    // Push to stack and pop back
    memory.stack_push(string_value).unwrap();
    let popped_value = memory.stack_pop().unwrap();

    // Verify the popped value is a string with the right address
    if let VmValue::String(addr) = popped_value {
        assert_eq!(addr, string_addr);

        // Get the string block and verify its content using our helper methods
        let bytes_slice = memory.get_string_bytes(addr).unwrap();
        let retrieved_str = std::str::from_utf8(bytes_slice).unwrap();
        assert_eq!(retrieved_str, test_str);
    } else {
        panic!("Expected a string value");
    }
}

#[test]
fn test_multiple_string_allocations() {
    let mut memory = new_test_memory();

    // Allocate multiple strings
    let strings = ["First string", "Second string", "Third string"];
    let mut string_addrs = Vec::new();

    for s in &strings {
        let addr = memory.alloc_string(s).unwrap();
        string_addrs.push(addr);
    }

    // Verify each string can be retrieved correctly
    for (i, &addr) in string_addrs.iter().enumerate() {
        let bytes = memory.get_string_bytes(addr).unwrap();
        let retrieved_str = std::str::from_utf8(bytes).unwrap();
        assert_eq!(retrieved_str, strings[i]);
    }
}

#[test]
fn test_string_in_block() {
    let mut memory = new_test_memory();

    // Create a string
    let test_str = "String in block";
    let string_addr = memory.alloc_string(test_str).unwrap();
    let string_value = VmValue::String(string_addr);

    // Create a block containing an integer and the string
    let block_addr = memory.alloc_empty_block(2).unwrap();

    // Push values to the block using our helper method
    memory.push_to_block(block_addr, VmValue::Int(42)).unwrap();
    memory.push_to_block(block_addr, string_value).unwrap();

    // Get the block and verify its contents
    let block = memory.get_block(block_addr).unwrap();
    assert_eq!(block.len(), 2);

    // Verify the integer value
    let int_value = memory.get_block_item(block_addr, 0).unwrap();
    assert_eq!(int_value, &VmValue::Int(42));

    // Verify the string value
    if let Some(&VmValue::String(addr)) = memory.get_block_item(block_addr, 1) {
        // Get the string bytes and verify
        let bytes = memory.get_string_bytes(addr).unwrap();
        let retrieved_str = std::str::from_utf8(bytes).unwrap();
        assert_eq!(retrieved_str, test_str);
    } else {
        panic!("Expected a string value at index 1");
    }
}

#[test]
fn test_symbol_table() {
    let mut memory = new_test_memory();

    // Create a symbol
    let symbol_name = "test-symbol";
    let symbol_addr = memory.get_symbol(symbol_name).unwrap();

    // Lookup the same symbol again and verify it's the same address
    let symbol_addr2 = memory.get_symbol(symbol_name).unwrap();

    // Use our test helper to compare addresses
    assert!(crate::mem::test_access::symbols_equal(
        &symbol_addr,
        &symbol_addr2
    ));

    // Verify the symbol is stored as a string
    let bytes = memory.get_string_bytes(symbol_addr).unwrap();
    let retrieved_str = std::str::from_utf8(bytes).unwrap();
    assert_eq!(retrieved_str, symbol_name);

    // Verify that different symbols get different addresses
    let different_symbol = "different-symbol";
    let different_addr = memory.get_symbol(different_symbol).unwrap();

    // Use our test helper to compare addresses
    assert!(crate::mem::test_access::symbols_not_equal(
        &symbol_addr,
        &different_addr
    ));
}

#[test]
fn test_string_with_special_chars() {
    let mut memory = new_test_memory();

    // Test with a string containing special characters
    let special_str = "Unicode: 你好, Emoji: 🚀, Symbols: ©®™";
    let string_addr = memory.alloc_string(special_str).unwrap();

    // Verify the string contents using our helper methods
    let bytes = memory.get_string_bytes(string_addr).unwrap();
    let retrieved_str = std::str::from_utf8(bytes).unwrap();
    assert_eq!(retrieved_str, special_str);

    // Verify the length in bytes is correct
    let string_block = memory.get_string_block(string_addr).unwrap();
    assert_eq!(string_block.len(), special_str.len() as Word);
}
