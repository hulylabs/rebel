// Debug tests for specific failing cases
use rebel::mem::{Memory, MemoryError, Value};

#[test]
fn debug_drain_popping() {
    let mut memory = Memory::new(4096);
    let series = memory.alloc::<Value>(5).expect("Failed to allocate series");

    // Push values
    for i in 1..=4 {
        memory.push(series, Value::int(i)).expect("Push failed");
    }

    println!(
        "Series length after pushing: {}",
        memory.len(series).unwrap()
    );

    // Drain from position 2
    let drain_pos = 2u32;
    println!("Draining values from position {}", drain_pos);
    let drained = memory.drain(series, drain_pos).expect("Drain failed");

    println!(
        "Original series length after drain: {}",
        memory.len(series).unwrap()
    );
    println!("Drained series length: {}", memory.len(drained).unwrap());

    // Check original series values
    println!("\nPopping from original series:");
    while memory.len(series).unwrap() > 0 {
        let i = memory.len(series).unwrap();
        let popped = memory.pop(series).expect("Pop failed");
        println!("Pop #{}: kind={}, data={}", i, popped.kind(), popped.data());
    }

    // Check drained series values
    println!("\nPopping from drained series:");
    while memory.len(drained).unwrap() > 0 {
        let i = memory.len(drained).unwrap();
        let popped = memory.pop(drained).expect("Pop failed");
        println!("Pop #{}: kind={}, data={}", i, popped.kind(), popped.data());
    }
}

#[test]
fn debug_capacity() {
    let mut memory = Memory::new(4096);

    // Test with different capacities to find pattern
    for capacity in 1..6 {
        // Create a new series with the given capacity
        let series = memory
            .alloc::<Value>(capacity)
            .expect("Failed to allocate series");

        // Get actual capacity
        let actual_capacity =
            rebel::mem::capacity(&memory, series).expect("Failed to get capacity");
        let block = memory
            .get::<rebel::mem::Block>(series.address())
            .expect("Failed to get block");

        println!(
            "\nRequested capacity: {}, Actual capacity: {}",
            capacity, actual_capacity
        );
        println!("Block.cap: {}, Block.len: {}", block.cap(), block.len());
        println!("Block::SIZE_IN_WORDS: {}", rebel::mem::Block::SIZE_IN_WORDS);

        // Push values until it fails
        println!("Pushing values:");
        let mut pushed = 0;
        for i in 0..10 {
            let result = memory.push(series, Value::int(i as i32));
            match result {
                Ok(_) => {
                    println!("  Push #{} - OK", i + 1);
                    pushed += 1;
                }
                Err(e) => {
                    println!("  Push #{} - Error: {:?}", i + 1, e);
                    break;
                }
            }
        }
        println!(
            "Successfully pushed {} values with capacity {}",
            pushed, capacity
        );
    }
}

#[test]
fn debug_simple_push_pop() {
    let mut memory = Memory::new(4096);
    let series = memory.alloc::<Value>(5).expect("Failed to allocate series");

    println!("Pushing single value...");
    memory.push(series, Value::int(42)).expect("Push failed");

    println!("Series length: {}", memory.len(series).unwrap());

    let popped = memory.pop(series).expect("Pop failed");
    println!(
        "Popped value - kind: {}, data: {}",
        popped.kind(),
        popped.data()
    );
}

#[test]
fn debug_push_all_function() {
    let mut memory = Memory::new(4096);
    let series = memory
        .alloc::<Value>(10)
        .expect("Failed to allocate series");

    // Values to push
    let values = [Value::int(10), Value::int(20), Value::int(30)];

    println!("Pushing array of {} values", values.len());
    // Push all values at once
    memory.push_all(series, &values).expect("Push all failed");

    println!(
        "Series length after push_all: {}",
        memory.len(series).unwrap()
    );

    // Pop them all
    for i in 0..values.len() {
        let popped = memory.pop(series).expect("Pop failed");
        println!(
            "Pop #{}: kind={}, data={}",
            i + 1,
            popped.kind(),
            popped.data()
        );
    }

    println!(
        "Series length after popping all: {}",
        memory.len(series).unwrap()
    );
}
