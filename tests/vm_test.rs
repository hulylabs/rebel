// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use rebel::mem::{Memory, Value};
use rebel::vm::{Process, VmError};

// Helper function to create a test memory
fn create_test_memory() -> Memory {
    Memory::new(4096)
}

// Test basic block parsing with Process
#[test]
fn test_parse_empty_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[]")?;
    println!("Result: kind={}, data={}", result.kind(), result.data());
    
    // The result should be of kind BLOCK (4)
    assert_eq!(result.kind(), Value::BLOCK, "Result should have BLOCK kind");
    
    // The data field should point to a valid block address
    assert!(result.data() > 0, "Result should have valid data address");
    
    // Check the block's content - parse_block("[]") returns a block containing an empty block [[]]
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    
    // The outer block should have length 1 (containing the empty block)
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1 (containing the empty block)");
    
    // Get the inner block value (first item in the outer block)
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?; // Block::SIZE_IN_WORDS is 2
    
    // This inner value should be of kind BLOCK
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    // Now verify the inner block is empty (length 0)
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 0, "Inner block should be empty (length 0)");
    
    Ok(())
}

#[test]
fn test_parse_integer_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[1 2 3]")?;
    
    // Verify the result is a block
    assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
    assert!(result.data() > 0, "Expected a valid block address");
    
    // Verify the outer block - should contain one inner block
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1");
    
    // Get the inner block and verify it contains integers 1, 2, 3
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 3, "Inner block should contain 3 integers");
    
    // Verify the values: [1, 2, 3]
    let value_1 = *memory.get::<Value>(inner_block_addr + 2)?; // First integer
    let value_2 = *memory.get::<Value>(inner_block_addr + 4)?; // Second integer
    let value_3 = *memory.get::<Value>(inner_block_addr + 6)?; // Third integer
    
    assert_eq!(value_1.kind(), Value::INT, "First value should be INT");
    assert_eq!(value_1.data(), 1, "First value should be 1");
    
    assert_eq!(value_2.kind(), Value::INT, "Second value should be INT");
    assert_eq!(value_2.data(), 2, "Second value should be 2");
    
    assert_eq!(value_3.kind(), Value::INT, "Third value should be INT");
    assert_eq!(value_3.data(), 3, "Third value should be 3");
    
    Ok(())
}

#[test]
fn test_parse_float_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[3.14 -2.5 0.0]")?;
    
    // Verify the result is a block
    assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
    assert!(result.data() > 0, "Expected a valid block address");
    
    // Verify the outer block - should contain one inner block
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1");
    
    // Get the inner block and verify it contains floats
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 3, "Inner block should contain 3 floats");
    
    // Verify the values: [3.14, -2.5, 0.0]
    let value_1 = *memory.get::<Value>(inner_block_addr + 2)?; // First float
    let value_2 = *memory.get::<Value>(inner_block_addr + 4)?; // Second float
    let value_3 = *memory.get::<Value>(inner_block_addr + 6)?; // Third float
    
    assert_eq!(value_1.kind(), Value::FLOAT, "First value should be FLOAT");
    assert!((value_1.as_float()? - 3.14).abs() < 0.0001, "First value should be 3.14");
    
    assert_eq!(value_2.kind(), Value::FLOAT, "Second value should be FLOAT");
    assert!((value_2.as_float()? - (-2.5)).abs() < 0.0001, "Second value should be -2.5");
    
    assert_eq!(value_3.kind(), Value::FLOAT, "Third value should be FLOAT");
    assert!((value_3.as_float()? - 0.0).abs() < 0.0001, "Third value should be 0.0");
    
    Ok(())
}

#[test]
fn test_parse_mixed_numeric_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    // Block with mixed integer and float values
    let result = process.parse_block("[42 3.14159 -10 -0.5]")?;
    
    // Verify the result is a block
    assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
    assert!(result.data() > 0, "Expected a valid block address");
    
    // Verify the outer block - should contain one inner block
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1");
    
    // Get the inner block and verify it contains mixed values
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 4, "Inner block should contain 4 values");
    
    // Check the values
    let value_1 = *memory.get::<Value>(inner_block_addr + 2)?;
    let value_2 = *memory.get::<Value>(inner_block_addr + 4)?;
    let value_3 = *memory.get::<Value>(inner_block_addr + 6)?;
    let value_4 = *memory.get::<Value>(inner_block_addr + 8)?;
    
    assert_eq!(value_1.kind(), Value::INT, "First value should be INT");
    assert_eq!(value_1.data(), 42, "First value should be 42");
    
    assert_eq!(value_2.kind(), Value::FLOAT, "Second value should be FLOAT");
    assert!((value_2.as_float()? - 3.14159).abs() < 0.0001, "Second value should be ~3.14159");
    
    assert_eq!(value_3.kind(), Value::INT, "Third value should be INT");
    assert_eq!(value_3.data(), 0xFFFFFFF6, "Third value should be -10 (as two's complement)");
    assert_eq!(value_3.as_int()?, -10, "Third value should be -10");
    
    assert_eq!(value_4.kind(), Value::FLOAT, "Fourth value should be FLOAT");
    assert!((value_4.as_float()? - (-0.5)).abs() < 0.0001, "Fourth value should be -0.5");
    
    Ok(())
}

#[test]
fn test_parse_string_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[\"hello\" \"world\"]")?;
    
    // Verify the result is a block
    assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
    assert!(result.data() > 0, "Expected a valid block address");
    
    // Verify the outer block - should contain one inner block
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1");
    
    // Get the inner block and verify it contains two strings
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 2, "Inner block should contain 2 strings");
    
    // Verify the string values
    let string1_value = *memory.get::<Value>(inner_block_addr + 2)?; // First string
    let string2_value = *memory.get::<Value>(inner_block_addr + 4)?; // Second string
    
    assert_eq!(string1_value.kind(), Value::STRING, "First value should be STRING");
    assert_eq!(string2_value.kind(), Value::STRING, "Second value should be STRING");
    
    // We can't easily check the string content directly, but we can verify they're different
    assert_ne!(string1_value.data(), string2_value.data(), "String addresses should be different");
    
    Ok(())
}

#[test]
fn test_parse_word_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[word set-word: :get-word]")?;
    
    // Verify the result is a block
    assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
    assert!(result.data() > 0, "Expected a valid block address");
    
    // Verify the outer block - should contain one inner block
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1");
    
    // Get the inner block and verify it contains 3 words of different types
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 3, "Inner block should contain 3 words");
    
    // Verify the word values
    let word1 = *memory.get::<Value>(inner_block_addr + 2)?; // Regular word
    let word2 = *memory.get::<Value>(inner_block_addr + 4)?; // Set word
    let word3 = *memory.get::<Value>(inner_block_addr + 6)?; // Get word
    
    assert_eq!(word1.kind(), Value::WORD, "First value should be WORD");
    assert_eq!(word2.kind(), Value::SET_WORD, "Second value should be SET_WORD");
    assert_eq!(word3.kind(), Value::GET_WORD, "Third value should be GET_WORD");
    
    Ok(())
}

#[test]
fn test_parse_nested_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[1 [2 3] 4]")?;
    
    // Verify the result is a block
    assert_eq!(result.kind(), Value::BLOCK, "Result should be a BLOCK");
    assert!(result.data() > 0, "Expected a valid block address");
    
    // Verify the outer block - should contain one inner block
    let outer_block_addr = result.data();
    let outer_block = memory.get::<rebel::mem::Block>(outer_block_addr)?;
    assert_eq!(outer_block.len(), 1, "Outer block should have length 1");
    
    // Get the inner block and verify it contains integers 1, [2 3], 4
    let inner_block_value = *memory.get::<Value>(outer_block_addr + 2)?;
    assert_eq!(inner_block_value.kind(), Value::BLOCK, "Inner value should be a BLOCK");
    
    let inner_block_addr = inner_block_value.data();
    let inner_block = memory.get::<rebel::mem::Block>(inner_block_addr)?;
    assert_eq!(inner_block.len(), 3, "Inner block should contain 3 values");
    
    // Verify the values: [1, [2 3], 4]
    let value_1 = *memory.get::<Value>(inner_block_addr + 2)?; // First integer
    let nested_block = *memory.get::<Value>(inner_block_addr + 4)?; // Nested block
    let value_4 = *memory.get::<Value>(inner_block_addr + 6)?; // Last integer
    
    assert_eq!(value_1.kind(), Value::INT, "First value should be INT");
    assert_eq!(value_1.data(), 1, "First value should be 1");
    
    assert_eq!(nested_block.kind(), Value::BLOCK, "Middle value should be BLOCK");
    assert_eq!(value_4.kind(), Value::INT, "Last value should be INT");
    assert_eq!(value_4.data(), 4, "Last value should be 4");
    
    // Verify the nested block [2 3]
    let nested_addr = nested_block.data();
    let nested = memory.get::<rebel::mem::Block>(nested_addr)?;
    assert_eq!(nested.len(), 2, "Nested block should contain 2 values");
    
    let nested_1 = *memory.get::<Value>(nested_addr + 2)?;
    let nested_2 = *memory.get::<Value>(nested_addr + 4)?;
    
    assert_eq!(nested_1.kind(), Value::INT, "First nested value should be INT");
    assert_eq!(nested_1.data(), 2, "First nested value should be 2");
    assert_eq!(nested_2.kind(), Value::INT, "Second nested value should be INT");
    assert_eq!(nested_2.data(), 3, "Second nested value should be 3");
    
    Ok(())
}

#[test]
fn test_parse_path_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    let result = process.parse_block("[a/b c/d/e]")?;
    // For path blocks, the kind is usually Value::BLOCK
    assert_eq!(result.kind(), Value::BLOCK);
    assert!(result.data() > 0, "Expected a valid block address");
    
    Ok(())
}

// Test error handling for specific parser errors
#[test]
fn test_parse_invalid_escape() {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory).unwrap();
    
    // Invalid escape sequence
    let result = process.parse_block("[\"invalid \\z escape\"]");
    assert!(result.is_err(), "Expected error for invalid escape");
}

// Test memory allocation and addresses
#[test]
fn test_parse_block_addresses() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    // Parse a couple of blocks
    let result1 = process.parse_block("[1 2 3]")?;
    let result2 = process.parse_block("[\"a\" \"b\" \"c\"]")?;
    
    // Verify they have different addresses
    assert_ne!(result1.data(), result2.data());
    
    Ok(())
}

// Test handling of larger inputs
#[test]
fn test_parse_larger_input() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    // Create a larger block
    let mut input = "[".to_string();
    for i in 1..15 {
        input.push_str(&format!("{} ", i));
    }
    input.push(']');
    
    let result = process.parse_block(&input)?;
    assert!(result.data() > 0, "Expected a valid block address");
    
    Ok(())
}

// Test handling of comments in the input
#[test]
fn test_parse_with_comments() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    // Parse a block with comments
    let input = "[\n    1 ; comment after integer\n    ; full line comment\n    \"text\" ; comment after string\n    word ; comment after word\n]";
    let result = process.parse_block(input)?;
    assert!(result.data() > 0, "Expected a valid block address");
    
    Ok(())
}

// Test handling of complex syntax
#[test]
fn test_parse_complex_syntax() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    // Test with escaped characters in strings
    let result1 = process.parse_block("[\"hello\\nworld\" \"quote\\\"inside\"]")?;
    assert!(result1.data() > 0, "Expected a valid block address");
    
    // Test with multiple mixed types
    let result2 = process.parse_block("[1 \"text\" word :get-word]")?;
    assert!(result2.data() > 0, "Expected a valid block address");
    
    Ok(())
}

// Diagnostic test to understand block size variations
#[test]
fn test_parse_block_size_variations() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    let mut process = Process::new(&mut memory)?;
    
    // Test with blocks of varying sizes
    let empty_block = process.parse_block("[]")?;
    println!("Empty block: kind={}, data={}", empty_block.kind(), empty_block.data());
    
    let one_item_block = process.parse_block("[1]")?;
    println!("One item block: kind={}, data={}", one_item_block.kind(), one_item_block.data());
    
    let two_item_block = process.parse_block("[1 2]")?;
    println!("Two item block: kind={}, data={}", two_item_block.kind(), two_item_block.data());
    
    // All should be block kind
    assert_eq!(empty_block.kind(), Value::BLOCK, "Empty block should have BLOCK kind");
    assert_eq!(one_item_block.kind(), Value::BLOCK, "One item block should have BLOCK kind");
    assert_eq!(two_item_block.kind(), Value::BLOCK, "Two item block should have BLOCK kind");
    
    Ok(())
}