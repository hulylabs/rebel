// Common testing utilities for the memory system

use crate::mem::*;

/// Create a test memory instance with default sizes and initialized stacks
///
/// This helper function creates a Memory instance and calls init() to set up stacks
/// for use in tests, reducing boilerplate code in test cases.
pub fn new_test_memory() -> Memory {
    let mut memory = Memory::new();
    memory.init().expect("Failed to initialize memory");
    memory
}
