// A simple test file to debug memory behavior
use rebel::mem::{Memory, Value};

// Debug helper: Print a Value's contents
fn print_value(v: Value) {
    println!(
        "Value - Kind: {}, Data: {}",
        v.kind(),
        v.data()
    );
}

#[test]
fn debug_value_behavior() {
    // Create values
    let int_value = Value::int(42);
    println!("Int value - Kind: {}, Data: {}", int_value.kind(), int_value.data());
    
    let bool_value = Value::bool(true);
    println!("Bool value - Kind: {}, Data: {}", bool_value.kind(), bool_value.data());
    
    println!("\nTesting equality:");
    let int1 = Value::int(42);
    let int2 = Value::int(42);
    println!("int1.kind() == int2.kind(): {}", int1.kind() == int2.kind());
    println!("int1.data() == int2.data(): {}", int1.data() == int2.data());
}

#[test]
fn debug_memory_push_pop() {
    let mut memory = Memory::new(1024);
    
    // Create a series
    let series = memory.alloc::<Value>(5).expect("Failed to allocate series");
    
    println!("Initial series length: {}", memory.len(series).unwrap());
    
    // Push a value
    let val1 = Value::int(42);
    println!("\nPushing: Kind={}, Data={}", val1.kind(), val1.data());
    memory.push(series, val1).expect("Push failed");
    
    println!("Series length after push: {}", memory.len(series).unwrap());
    
    // Pop the value
    let popped = memory.pop(series).expect("Pop failed");
    println!("Popped value: Kind={}, Data={}", popped.kind(), popped.data());
    
    println!("Series length after pop: {}", memory.len(series).unwrap());
    
    // Try multiple pushes and pops
    println!("\nPushing multiple values:");
    memory.push(series, Value::int(10)).expect("Push failed");
    println!("Pushed: 10");
    memory.push(series, Value::int(20)).expect("Push failed");
    println!("Pushed: 20");
    memory.push(series, Value::int(30)).expect("Push failed");
    println!("Pushed: 30");
    
    println!("Series length after 3 pushes: {}", memory.len(series).unwrap());
    
    println!("\nPopping values:");
    for i in 0..3 {
        let popped = memory.pop(series).expect("Pop failed");
        println!("Pop #{}: Kind={}, Data={}", i+1, popped.kind(), popped.data());
    }
    
    println!("Series length after all pops: {}", memory.len(series).unwrap());
}