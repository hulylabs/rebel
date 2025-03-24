// Common testing utilities for the memory system

use crate::mem::*;

// Default memory sizes used in tests
pub const MEMORY_SIZE: usize = 8192;
pub const VALUES_SIZE: usize = 1024;
pub const BLOCKS_SIZE: usize = 256;
pub const STRINGS_SIZE: usize = 256;
pub const BYTES_SIZE: usize = 1024;
pub const WORDS_SIZE: usize = 256;
pub const PAIRS_SIZE: usize = 256;
pub const CONTEXTS_SIZE: usize = 256;

/// Create a test memory instance with the specified sizes
pub fn new_test_memory() -> Memory {
    let mut memory = Memory::new();
    memory.init().expect("Failed to initialize memory");
    memory
}
