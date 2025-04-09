use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rebel::parse::{Collector, Parser, WordKind};

// Simple no-op collector for benchmarking
struct BenchCollector;

impl Collector for BenchCollector {
    type Error = ();
    
    fn string(&mut self, _: &str) -> Result<(), Self::Error> { Ok(()) }
    fn word(&mut self, _: WordKind, _: &str) -> Result<(), Self::Error> { Ok(()) }
    fn integer(&mut self, _: i32) -> Result<(), Self::Error> { Ok(()) }
    fn begin_block(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn end_block(&mut self) -> Result<(), Self::Error> { Ok(()) } 
    fn begin_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn end_path(&mut self) -> Result<(), Self::Error> { Ok(()) }
}

fn bench_simple_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("Simple Parser");
    
    group.bench_function("parse_int", |b| {
        let mut collector = BenchCollector;
        b.iter(|| {
            black_box(Parser::parse(black_box("123"), &mut collector).unwrap())
        })
    });
    
    group.bench_function("parse_word", |b| {
        let mut collector = BenchCollector;
        b.iter(|| {
            black_box(Parser::parse(black_box("hello"), &mut collector).unwrap())
        })
    });
    
    group.bench_function("parse_string", |b| {
        let mut collector = BenchCollector;
        b.iter(|| {
            black_box(Parser::parse(black_box("\"Hello, World!\""), &mut collector).unwrap())
        })
    });
    
    group.bench_function("parse_simple_block", |b| {
        let mut collector = BenchCollector;
        b.iter(|| {
            black_box(Parser::parse(black_box("[1 2 3]"), &mut collector).unwrap())
        })
    });
    
    group.bench_function("parse_nested_block", |b| {
        let mut collector = BenchCollector;
        b.iter(|| {
            black_box(Parser::parse(black_box("[1 [2 3] 4]"), &mut collector).unwrap())
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_simple_parser
);
criterion_main!(benches);