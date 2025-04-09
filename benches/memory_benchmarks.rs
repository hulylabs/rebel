use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rebel::mem::{Memory, Value};

fn bench_value_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Value Operations");
    
    group.bench_function("value_creation", |b| {
        b.iter(|| {
            black_box(Value::int(black_box(42)))
        })
    });
    
    group.bench_function("value_is_type_check", |b| {
        let value = Value::int(42);
        b.iter(|| {
            black_box(value.is_int())
        })
    });
    
    group.bench_function("value_type_conversion", |b| {
        let value = Value::int(42);
        b.iter(|| {
            black_box(value.as_int().ok())
        })
    });
    
    group.finish();
}

criterion_group!(benches, bench_value_operations);
criterion_main!(benches);