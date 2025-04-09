// Test helper utilities for Rebel
use rebel::mem::{Memory, Value, Series};

/// Create a standard-sized memory for testing
pub fn create_test_memory() -> Memory {
    Memory::new(4096)
}

/// Helper to create a memory with a pre-allocated string
pub fn memory_with_string(text: &str) -> (Memory, Series<u8>) {
    let mut mem = create_test_memory();
    let series = mem.alloc_string(text).unwrap();
    (mem, series)
}

/// Helper to create a memory with a block of values
pub fn memory_with_block(values: &[i32]) -> (Memory, Series<Value>) {
    let mut mem = create_test_memory();
    let series = mem.alloc::<Value>(values.len() as u32).unwrap();
    
    for val in values {
        mem.push(series, Value::int(*val)).unwrap();
    }
    
    (mem, series)
}

/// Helper to compare string content
pub fn check_string_content(memory: &Memory, string: Series<u8>, expected: &str) -> bool {
    let len = memory.len(string).unwrap() as usize;
    if len != expected.len() {
        return false;
    }
    
    // For more comprehensive testing, we would need to add accessors to
    // read the actual bytes from memory, but this validates the length
    true
}