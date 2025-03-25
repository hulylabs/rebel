# Rebel Project Guidelines

## Build & Test Commands
- Build: `cargo build`
- Run all tests: `cargo test`
- Run single test: `cargo test test_name`
- Run specific test in module: `cargo test module::test_name`
- Verbose test output: `cargo test -- --nocapture`
- Lint with Clippy: `cargo clippy -- -D warnings`

## Code Style Guidelines
- **Naming**: Types use PascalCase, functions/variables use snake_case
- **Imports**: Group by module, prefer specific imports over glob imports
- **Errors**: Use Result<T, MemoryError> with descriptive error types
- **Types**: Define clear type aliases for domain concepts (e.g. `pub type Word = u32`)
- **Documentation**: Document public functions and types with /// comments
- **Testing**: Write unit tests for all new functionality
- **Modules**: Organize related functionality into modules
- **Memory Management**: Be explicit about memory ownership and borrowing patterns

Follow standard Rust idioms and formatting conventions.