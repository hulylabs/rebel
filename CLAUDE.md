# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands
- Build: `cargo build`
- Build with optimizations: `cargo build --release`
- Run tests: `cargo test`
- Run a specific test: `cargo test test_name`
- Run a specific test in verbose mode: `cargo test test_name -- --nocapture`
- Run tests with output: `cargo test -- --nocapture`
- Lint with Clippy: `cargo clippy`
- Format code: `cargo fmt`

## Project Structure
- **src/lib.rs**: Main library entry point
- **src/mem.rs**: Memory management system (Series, Block, allocation)
- **src/parse.rs**: Parser for the REBOL-inspired language
- **src/vm.rs**: Virtual machine implementation
- **tests/**: Comprehensive test suite for all components
  - **tests/mem_tests.rs**: Memory system tests
  - **tests/parse_test.rs**: Parser tests
  - **tests/helpers.rs**: Helper utilities for tests
- **MEMORY.md**: Detailed explanation of the memory system

## Memory System Guidelines
- Use byte-based calculations for memory operations (allocation, capacity)
- Never access Block fields directly - always use the public Memory methods
- Series operations follow LIFO (stack-like) behavior - push adds to end, pop removes from end
- Be aware of memory error types: StackOverflow, StackUnderflow, OutOfMemory, etc.
- See MEMORY.md for detailed memory system documentation

## Code Style Guidelines
- **Imports**: Group imports by crate, with std first, then external crates, then internal modules.
- **Formatting**: Follow rustfmt conventions - 4 space indentation, no tabs, max 100 columns.
- **Types**: Use descriptive type aliases (like `Word`, `Address`, `Offset`) for domain concepts.
- **Error Handling**: Use thiserror for error types with descriptive error messages.
- **Documentation**: Add doc comments to public items with description, examples, and parameters.
- **Memory Safety**: Use bytemuck for safe type casting with proper error handling.
- **Naming Conventions**: Use snake_case for functions/variables, CamelCase for types, SCREAMING_CASE for constants.
- **Code Structure**: Follow Rust's module system with clear separation of concerns (parse, mem, vm).
- **License**: Preserve the copyright header in all source files.

## Testing Guidelines
- Keep tests in the `tests/` directory
- Follow existing test patterns for consistency
- Test both success and error conditions
- For debugging complex behavior, write custom tests with println! statements
- When testing memory operations, be aware of implementation details