use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rebel::parse::{Collector, Parser, WordKind};

// Simple no-op collector for benchmarking
struct BenchCollector {
    count: usize,  // For more realistic benchmarks, track number of tokens processed
}

impl BenchCollector {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Collector for BenchCollector {
    type Error = ();
    
    fn string(&mut self, _: &str) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    }
    
    fn word(&mut self, _: WordKind, _: &str) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    }
    
    fn integer(&mut self, _: i32) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    }
    
    fn begin_block(&mut self) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    }
    
    fn end_block(&mut self) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    } 
    
    fn begin_path(&mut self) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    }
    
    fn end_path(&mut self) -> Result<(), Self::Error> { 
        self.count += 1;
        Ok(()) 
    }
}

fn bench_simple_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("Simple Parser");
    
    group.bench_function("parse_int", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box("123"), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("parse_word", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box("hello"), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("parse_string", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box("\"Hello, World!\""), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("parse_simple_block", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box("[1 2 3]"), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("parse_nested_block", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box("[1 [2 3] 4]"), &mut collector).unwrap())
            }
        )
    });
    
    group.finish();
}

fn bench_realistic_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Realistic Parsing");
    
    // Benchmark parsing a small program with multiple statements
    group.bench_function("small_program", |b| {
        // Keep it simple and make sure it parses correctly
        let program = r#"[set x 10 set y 20 print "The sum is:" print "result" if true [print "x is greater than 5"]]"#;
        
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                // Parse the program and handle any errors
                match Parser::parse(black_box(program), &mut collector) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("Error parsing program: {:?}", e);
                        panic!("Small program should parse successfully");
                    }
                }
            }
        )
    });
    
    // Benchmark parsing a data structure definition
    group.bench_function("data_structure", |b| {
        let data = r#"[name: "Product" id: 12345 price: 9995 in-stock: true tags: ["electronics" "gadget" "smartphone"] dimensions: [width: 5 height: 2 depth: 1] reviews: [[name: "User1" rating: 5 comment: "Great product!"] [name: "User2" rating: 4 comment: "Good value for money"] [name: "User3" rating: 3 comment: "Decent, but could be better"]]]"#;
        
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                match Parser::parse(black_box(data), &mut collector) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("Error parsing data: {:?}", e);
                        panic!("Data structure should parse successfully");
                    }
                }
            }
        )
    });
    
    // Benchmark parsing expressions
    group.bench_function("expressions", |b| {
        let expressions = r#"[1 2 3 4 5 6 7 8 9 10]"#;
        
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                match Parser::parse(black_box(expressions), &mut collector) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("Error parsing expressions: {:?}", e);
                        panic!("Expressions should parse successfully");
                    }
                }
            }
        )
    });
    
    // Benchmark parsing a function definition with body
    group.bench_function("function_definition", |b| {
        let function = r#"[func [x] [print x] func [a b] [print a print b] print "Function example"]"#;
        
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                match Parser::parse(black_box(function), &mut collector) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("Error parsing function: {:?}", e);
                        panic!("Function definition should parse successfully");
                    }
                }
            }
        )
    });
    
    // Benchmark parsing with a mix of paths, words, and values
    group.bench_function("mixed_paths", |b| {
        let paths = r#"[system/options/path: "path" user/name: "John" options/width: 800 options/height: 600]"#;
        
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                match Parser::parse(black_box(paths), &mut collector) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("Error parsing paths: {:?}", e);
                        panic!("Paths should parse successfully");
                    }
                }
            }
        )
    });
    
    group.finish();
}

fn bench_parser_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("Parser Scalability");
    
    // Generate lists of different sizes to test scalability
    let list_small = generate_list(10);
    let list_medium = generate_list(100);
    let list_large = generate_list(1000);
    
    // Benchmark parsing lists of different sizes
    group.bench_function("list_size_10", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box(&list_small), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("list_size_100", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box(&list_medium), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("list_size_1000", |b| {
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box(&list_large), &mut collector).unwrap())
            }
        )
    });
    
    // Benchmark with different nested depths
    group.bench_function("nested_depth_5", |b| {
        let nested = generate_nested_blocks(5);
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box(&nested), &mut collector).unwrap())
            }
        )
    });
    
    group.bench_function("nested_depth_10", |b| {
        let nested = generate_nested_blocks(10);
        b.iter_with_setup(
            || BenchCollector::new(),
            |mut collector| {
                black_box(Parser::parse(black_box(&nested), &mut collector).unwrap())
            }
        )
    });
    
    group.finish();
}

// Helper function to generate a list of integers of given size
fn generate_list(size: usize) -> String {
    let mut list = String::from("[");
    for i in 0..size {
        list.push_str(&i.to_string());
        list.push(' ');
    }
    list.push(']');
    list
}

// Helper function to generate nested blocks of given depth
fn generate_nested_blocks(depth: usize) -> String {
    let mut nested = String::from("1 ");
    for _ in 0..depth {
        nested = format!("[{nested}]");
    }
    nested
}

criterion_group!(
    benches,
    bench_simple_parser,
    bench_realistic_parsing,
    bench_parser_scalability
);
criterion_main!(benches);