// Common testing utilities for the memory system

use crate::mem::*;

// Default memory sizes used in tests
pub const MEMORY_SIZE: usize = 8192;
pub const SYMBOL_TABLE_SIZE: u32 = 1024;
pub const PARSE_STACK_SIZE: u32 = 1024;
pub const PARSE_BASE_SIZE: u32 = 256;
pub const HEAP_SIZE: u32 = 4096;
pub const REGION_SIZE: u32 = 1024; // For equal-sized region tests

/// Create a standard test memory instance with the specified region sizes
pub fn setup_memory<'a>(memory: &'a mut [u32], region_sizes: [u32; 4]) -> Memory<'a> {
    Memory::init(memory, region_sizes).expect("Failed to initialize memory")
}

/// Create a standard test memory instance with default region sizes
/// [SYMBOL_TABLE_SIZE, PARSE_STACK_SIZE, PARSE_BASE_SIZE, HEAP_SIZE]
pub fn new_test_memory<'a>(memory: &'a mut [u32]) -> Memory<'a> {
    setup_memory(
        memory,
        [
            SYMBOL_TABLE_SIZE,
            PARSE_STACK_SIZE,
            PARSE_BASE_SIZE,
            HEAP_SIZE,
        ],
    )
}

/// Create a test memory with equally sized regions
pub fn new_equal_region_memory<'a>(memory: &'a mut [u32]) -> Memory<'a> {
    setup_memory(memory, [REGION_SIZE, REGION_SIZE, REGION_SIZE, REGION_SIZE])
}
