use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rebel::mem::{Memory, Value};

fn bench_value_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Value Operations");

    group.bench_function("value_creation", |b| {
        b.iter(|| black_box(Value::int(black_box(42))))
    });

    group.bench_function("value_is_type_check", |b| {
        let value = Value::int(42);
        b.iter(|| black_box(value.is_int()))
    });

    group.bench_function("value_type_conversion", |b| {
        let value = Value::int(42);
        b.iter(|| black_box(value.as_int().ok()))
    });

    group.finish();
}

fn bench_realistic_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Realistic Memory Operations");

    // Benchmark a series of operations that would occur in a data structure manipulation scenario
    group.bench_function("build_and_access_list", |b| {
        b.iter_with_setup(
            || {
                // Create a new memory arena for each iteration
                Memory::new(1024 * 1024)
            },
            |mut memory| {
                // Allocate a series for a list of integers
                let int_list = memory.alloc::<Value>(50).unwrap();

                // Fill the list with integers 1-50
                for i in 1..=50 {
                    memory.push(int_list, Value::int(i)).unwrap();
                }

                // Create a block to hold the list
                let block_val = Value::block(int_list);

                // Simulate accessing values from the list
                let list = block_val.as_block().unwrap();
                let length = memory.len(list).unwrap();

                // Simulate a sum operation on the list
                let mut sum = 0;
                for _ in 0..length {
                    // We're not actually popping since that would modify the list
                    // but we simulate accessing each element
                    let idx = memory.len(list).unwrap() - 1;
                    if idx < length {
                        // Get the value at the index by peeking
                        let val = memory.get::<Value>(list.address() + 2 + idx * 2).unwrap();
                        if val.is_int() {
                            sum += val.as_int().unwrap();
                        }
                    }
                }

                black_box(sum)
            },
        )
    });

    // Benchmark a more complex scenario with nested data structures
    group.bench_function("nested_data_structures", |b| {
        b.iter_with_setup(
            || {
                // Create a new memory arena for each iteration
                Memory::new(1024 * 1024)
            },
            |mut memory| {
                // Create nested lists - a main list with 5 sublists
                let main_list = memory.alloc::<Value>(10).unwrap();

                // Create and fill 5 sublists
                for i in 0..5 {
                    let sublist = memory.alloc::<Value>(10).unwrap();

                    // Fill each sublist with 10 values
                    for j in 0..10 {
                        memory.push(sublist, Value::int(i * 10 + j)).unwrap();
                    }

                    // Add the sublist to the main list
                    memory.push(main_list, Value::block(sublist)).unwrap();
                }

                // Simulate traversing the nested structure
                let mut sum = 0;
                for i in 0..memory.len(main_list).unwrap() {
                    // Get sublist at index i
                    let val = memory
                        .get::<Value>(main_list.address() + 2 + i * 2)
                        .unwrap();

                    if val.is_block() {
                        let sublist = val.as_block().unwrap();
                        let sublist_len = memory.len(sublist).unwrap();

                        // Sum all values in the sublist
                        for j in 0..sublist_len {
                            let subval =
                                memory.get::<Value>(sublist.address() + 2 + j * 2).unwrap();
                            if subval.is_int() {
                                sum += subval.as_int().unwrap();
                            }
                        }
                    }
                }

                black_box(sum)
            },
        )
    });

    // Benchmark a scenario that simulates a stack-based calculation
    group.bench_function("stack_calculation", |b| {
        b.iter_with_setup(
            || {
                // Create a new memory arena for each iteration
                let mut memory = Memory::new(1024 * 1024);
                // Create a stack for operations
                let stack = memory.alloc::<Value>(100).unwrap();

                // Pre-fill with some operations (a simple RPN calculation)
                // This represents "3 4 + 5 * 2 -" which is (3+4)*5-2 = 33
                memory.push(stack, Value::int(3)).unwrap();
                memory.push(stack, Value::int(4)).unwrap();
                memory.push(stack, Value::int(5)).unwrap();
                memory.push(stack, Value::int(2)).unwrap();

                (memory, stack)
            },
            |(mut memory, stack)| {
                // Simulate a stack-based calculation engine
                // Pop values and perform operations
                let b = memory.pop(stack).unwrap().as_int().unwrap(); // 2
                let c = memory.pop(stack).unwrap().as_int().unwrap(); // 5
                let d = memory.pop(stack).unwrap().as_int().unwrap(); // 4
                let e = memory.pop(stack).unwrap().as_int().unwrap(); // 3

                // Perform calculation: (e + d) * c - b
                let result = (e + d) * c - b;

                // Push result back to stack
                memory.push(stack, Value::int(result)).unwrap();

                black_box(result)
            },
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_value_operations,
    bench_realistic_memory_operations
);
criterion_main!(benches);
