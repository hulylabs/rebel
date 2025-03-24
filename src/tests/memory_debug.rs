use crate::mem::*;
use crate::tests::test_utils::*;

// This test is for debugging the memory system behavior with blocks and the stack
#[test]
fn debug_block_stack_behavior() {
    let mut memory = new_test_memory();

    // Create a block [1, 2, 3]
    let block_addr = memory.alloc_empty_block(3).unwrap();
    memory.push_to_block(block_addr, VmValue::Int(1)).unwrap();
    memory.push_to_block(block_addr, VmValue::Int(2)).unwrap();
    memory.push_to_block(block_addr, VmValue::Int(3)).unwrap();

    // First verify what's in our block initially
    println!("Original block contents:");
    let block = memory.get_block(block_addr).unwrap();
    println!("Length: {}", block.len());
    for i in 0..block.len() {
        println!(
            "Item {}: {:?}",
            i,
            memory.get_block_item(block_addr, i).unwrap()
        );
    }

    // Push a value to the stack
    memory.stack_push(VmValue::Int(42)).unwrap();

    // Push the block to the stack
    memory.stack_push(VmValue::Block(block_addr)).unwrap();

    // Print stack state
    println!("\nStack state after pushes:");
    println!("Stack length: {}", memory.stack_len());

    // Pop the block from the stack
    let popped = memory.stack_pop().unwrap();
    println!("\nPopped from stack: {:?}", popped);

    // Verify it's a block
    if let VmValue::Block(addr) = popped {
        println!("\nPopped block contents:");
        let block = memory.get_block(addr).unwrap();
        println!("Length: {}", block.len());
        for i in 0..block.len() {
            println!("Item {}: {:?}", i, memory.get_block_item(addr, i).unwrap());
        }

        // Also check the original block address
        println!("\nOriginal block address contents after pop:");
        let orig_block = memory.get_block(block_addr).unwrap();
        println!("Length: {}", orig_block.len());
        for i in 0..orig_block.len() {
            println!(
                "Item {}: {:?}",
                i,
                memory.get_block_item(block_addr, i).unwrap()
            );
        }
    }
}
