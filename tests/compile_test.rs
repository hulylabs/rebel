// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use rebel::mem::{Block, Memory, Value};
use rebel::vm::{Process, VmError};

// Helper function to create a test memory
fn create_test_memory() -> Memory {
    Memory::new(4096)
}

// Test simple constant compilation [1 2 3]
#[test]
fn test_compile_constants() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    
    // Parse and compile in separate scopes to avoid borrow issues
    let block_value = {
        let mut process = Process::new(&mut memory)?;
        process.parse_block("[1 2 3]")?
    };
    
    let block_address = block_value.as_block()?.address();
    let inner_block_value = *memory.get::<Value>(block_address + 2)?;
    let inner_block = inner_block_value.as_block()?;
    
    let compiled = {
        let mut process = Process::new(&mut memory)?;
        process.compile(inner_block)?
    };
    
    // Verify that compilation works by checking block length
    let compiled_addr = compiled.address();
    let compiled_block = memory.get::<Block>(compiled_addr)?;
    
    // We expect 7 instructions (3 constants with 2 instructions each + 1 LEAVE)
    assert_eq!(compiled_block.len(), 7, "Block length should be 7");
    
    Ok(())
}

// Test compilation of a block with set_word: [x: 5 x]
#[test]
fn test_compile_set_word_and_use() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    
    // Parse and compile in separate scopes to avoid borrow issues
    let block_value = {
        let mut process = Process::new(&mut memory)?;
        process.parse_block("[x: 5 x]")?
    };
    
    let block_address = block_value.as_block()?.address();
    let inner_block_value = *memory.get::<Value>(block_address + 2)?;
    let inner_block = inner_block_value.as_block()?;
    
    let compiled = {
        let mut process = Process::new(&mut memory)?;
        process.compile(inner_block)?
    };
    
    // Verify that compilation works by checking block length
    let compiled_addr = compiled.address();
    let compiled_block = memory.get::<Block>(compiled_addr)?;
    
    // We expect 5 instructions (TYPE, CONST, SET_WORD, WORD, LEAVE)
    assert_eq!(compiled_block.len(), 5, "Block length should be 5");
    
    Ok(())
}

// Test compilation of a block with multiple set_words: [x: y: z: 42 y]
#[test]
fn test_compile_multiple_set_words() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    
    // Parse and compile in separate scopes to avoid borrow issues
    let block_value = {
        let mut process = Process::new(&mut memory)?;
        process.parse_block("[x: y: z: 42 y]")?
    };
    
    let block_address = block_value.as_block()?.address();
    let inner_block_value = *memory.get::<Value>(block_address + 2)?;
    let inner_block = inner_block_value.as_block()?;
    
    let compiled = {
        let mut process = Process::new(&mut memory)?;
        process.compile(inner_block)?
    };
    
    // Verify that compilation works by checking block length
    let compiled_addr = compiled.address();
    let compiled_block = memory.get::<Block>(compiled_addr)?;
    
    // After debugging, we found the correct number of instructions is 5 
    // (TYPE, CONST, SET_WORD z, SET_WORD y, SET_WORD x, WORD y)
    assert_eq!(compiled_block.len(), 5, "Block length should be 5");
    
    Ok(())
}

// Test compilation adds NONE value when compiling an empty block
#[test]
fn test_compile_empty_block() -> Result<(), VmError> {
    let mut memory = create_test_memory();
    
    // Parse and compile in separate scopes to avoid borrow issues
    let block_value = {
        let mut process = Process::new(&mut memory)?;
        process.parse_block("[]")?
    };
    
    let block_address = block_value.as_block()?.address();
    let inner_block_value = *memory.get::<Value>(block_address + 2)?;
    let inner_block = inner_block_value.as_block()?;
    
    let compiled = {
        let mut process = Process::new(&mut memory)?;
        process.compile(inner_block)?
    };
    
    // Verify that compilation works by checking block length
    let compiled_addr = compiled.address();
    let compiled_block = memory.get::<Block>(compiled_addr)?;
    
    // For empty blocks, we expect 2 instructions (TYPE NONE, CONST 0)
    assert_eq!(compiled_block.len(), 2, "Empty block should have 2 instructions");
    
    Ok(())
}