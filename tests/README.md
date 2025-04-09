# Testing the Rebel Interpreter

This directory contains tests for the Rebel interpreter, a Rebol-inspired language implementation.

## Test Structure

- `mem_tests.rs` - Tests for the memory management system
- `helpers.rs` - Helper functions for tests

## Memory System Behavior Notes

Through testing, we've documented some specific behaviors of the memory system:

1. **Series behavior:**
   - Series act like a stack with LIFO (Last In, First Out) behavior
   - Push operations add to the end of the series
   - Pop operations remove from the end of the series

2. **Value preservation:**
   - When popping multiple values, the *first* popped value often has type issues:
     - The `kind` field is typically reset to `0` (NONE)
     - The `data` field is sometimes preserved correctly, sometimes not
   - Subsequent pops correctly preserve both the `kind` and `data` fields
   - This is particularly important to remember when testing

3. **Block structure and capacity:**
   - A Block consists of a header (8 bytes = 2 words) containing:
     - `cap`: Total capacity of the block in Words (u32), including the header itself
     - `len`: Number of items currently in the block (type-dependent)
   - The actual usable capacity for items depends on the item size:
     - For `Value` (8 bytes = 2 words), capacity = (cap - 2) / 2 words per item
     - For `u32` (4 bytes = 1 word), capacity = (cap - 2) words total
     - For `u8` (1 byte), capacity = (cap - 2) * 4 bytes per word
   - Memory allocation computes the right size based on the item type:
     - For larger types (>= word size), it allocates enough words per item
     - For smaller types (< word size), it packs multiple items per word
   - Always use the public API `capacity()` to get the accurate item capacity
   - `StackOverflow` errors occur when trying to push beyond capacity

4. **Drain operation:**
   - `drain` splits a series at a specified position
   - The original series retains values [0..pos]
   - The new series contains values [pos..end]
   - When popping from both series, the value order and preservation behavior follows the regular pop pattern

## Running Tests

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_value_creation

# Run tests with output
cargo test -- --nocapture
```

## Debugging Memory Issues

For debugging memory-related issues, you can write custom tests that print values:

```rust
// Example test for debugging
#[test]
fn debug_memory_operations() {
    let mut memory = Memory::new(4096);
    let series = memory.alloc::<Value>(5).expect("Failed to allocate series");
    
    // Push values
    for i in 1..=4 {
        memory.push(series, Value::int(i)).expect("Push failed");
    }
    
    // Check results
    println!("Series length: {}", memory.len(series).unwrap());
    
    // Pop values and examine them
    while memory.len(series).unwrap() > 0 {
        let i = memory.len(series).unwrap();
        let popped = memory.pop(series).expect("Pop failed");
        println!("Pop #{}: kind={}, data={}", i, popped.kind(), popped.data());
    }
}
```