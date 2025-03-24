# Rebel Memory System Documentation

This document provides a comprehensive overview of the new domain-based Rebel memory system, including its architecture, components, and key operations.

## 1. Overview

The Rebel memory system has been completely redesigned with a domain-based architecture to provide better memory safety, type checking, and clarity. It provides low-level memory abstractions to support the language's execution environment, including value representation, memory allocation, and data structures.

The memory system is now organized around these core concepts:
- **Memory**: The root container managing multiple domains
- **Domains**: Type-safe memory regions for different types of data
- **Addresses**: Typed references to specific locations in memory
- **Blocks**: Higher-level data structures built on domains

## 2. Memory Architecture

### 2.1 Domain-Based Organization

The memory system now divides its space into distinct domains, each designed to store a specific type of data:

1. **Values Domain**: Stores all `VmValue` instances
2. **Blocks Domain**: Stores block metadata structures
3. **Strings Domain**: Stores string metadata structures
4. **Bytes Domain**: Raw byte storage for string data
5. **Words Domain**: Word-sized integer values for operations
6. **Pairs Domain**: Key-value pairs for contexts/objects
7. **Contexts Domain**: Block metadata for contexts

Each domain is a strongly-typed container with safe operations for its specific type.

### 2.2 Typed Address System

The new address system uses a generic type parameter to ensure type safety:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Addr<T>(Word, PhantomData<T>);
```

Note that the field is private, ensuring proper encapsulation. This design ensures that addresses are:
- Type-safe: An `Addr<Block<VmValue>>` cannot be used where an `Addr<u8>` is expected
- Clear in intent: The type parameter indicates what the address points to
- Consistent in implementation: All address operations follow the same patterns
- Properly encapsulated: Internal details are hidden through accessor methods

## 3. Core Components

### 3.1 Memory Values (VmValue)

The `VmValue` is the fundamental value type in the Rebel system. It is an enum with variants for different types:
- `None`: Represents no value/null
- `Int`: Integer value
- `Block`: Block of values
- `Context`: Context/scope
- `String`: String value
- `Word`, `SetWord`, `GetWord`: Symbol types
- `Path`: Path expression

### 3.2 Domain<T>

The `Domain<T>` type provides a type-safe storage area for values of type `T`:

```rust
pub struct Domain<T> {
    pub items: Box<[T]>,
    pub len: Word,
}
```

Each domain:
- Stores items of a single type
- Tracks its current length
- Provides safe access methods
- Manages memory allocation

### 3.3 Block<T>

The `Block<T>` type represents a resizable sequence of items:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block<T> {
    pub cap: Word,
    pub len: Word,
    pub data: Addr<T>,
}
```

A block provides:
- Fixed capacity with dynamic length
- Safe access to its items through a domain
- Operations for manipulating the sequence

## 4. Key Operations

### 4.1 Domain Operations

Domains provide these core operations:

- **push**: Add a single item to the domain
  ```rust
  pub fn push(&mut self, item: T) -> Option<Addr<T>>
  ```

- **push_all**: Add multiple items at once
  ```rust
  pub fn push_all(&mut self, items: &[T]) -> Option<Addr<T>>
  ```

- **alloc**: Allocate space for multiple items
  ```rust
  pub fn alloc(&mut self, items: Word) -> Option<Addr<T>>
  ```

- **get_item**: Get a reference to an item at a specified address
  ```rust
  pub fn get_item(&self, addr: Addr<T>) -> Option<&T>
  ```

- **get_item_mut**: Get a mutable reference to an item
  ```rust
  pub fn get_item_mut(&mut self, addr: Addr<T>) -> Option<&mut T>
  ```

- **move_items**: Move items between addresses
  ```rust
  pub fn move_items(&mut self, from: Addr<T>, to: Addr<T>, items: Word) -> Option<()>
  ```

### 4.2 Block Operations

Blocks provide these operations:

- **push**: Add an item to the end of the block
  ```rust
  pub fn push(&mut self, item: T, domain: &mut Domain<T>) -> Option<()>
  ```

- **push_all**: Add multiple items to the block
  ```rust
  pub fn push_all(&mut self, items: &[T], domain: &mut Domain<T>) -> Option<()>
  ```

- **trim_after**: Truncate the block at a specified offset and return the removed items
  ```rust
  pub fn trim_after<'a>(&mut self, offset: Word, domain: &'a mut Domain<T>) -> Option<&'a [T]>
  ```

- **pop**: Remove and return the last item
  ```rust
  pub fn pop(&mut self, domain: &mut Domain<T>) -> Option<T>
  ```

- **get_item**: Get a reference to an item by index
  ```rust
  pub fn get_item<'a>(&self, index: Word, domain: &'a Domain<T>) -> Option<&'a T>
  ```

- **move_to**: Move items from this block to another block
  ```rust
  pub fn move_to(&mut self, dest: &Block<T>, items: Word, domain: &mut Domain<T>) -> Option<()>
  ```

### 4.3 Memory Stack Operations

The Memory struct provides a stack for VM operations:

- **stack_push**: Push a value onto the stack
  ```rust
  pub fn stack_push(&mut self, value: VmValue) -> Option<()>
  ```

- **stack_pop**: Pop a value from the stack
  ```rust
  pub fn stack_pop(&mut self) -> Option<VmValue>
  ```

- **stack_len**: Get the current stack length
  ```rust
  pub fn stack_len(&self) -> Word
  ```

### 4.4 Block Creation

The Memory struct provides methods for creating blocks:

- **alloc_empty_block**: Allocate an empty block with specified capacity
  ```rust
  pub fn alloc_empty_block(&mut self, cap: Word) -> Option<Addr<Block<VmValue>>>
  ```

- **alloc_block**: Allocate a block and initialize it with values
  ```rust
  pub fn alloc_block(&mut self, items: &[VmValue]) -> Option<Addr<Block<VmValue>>>
  ```

## 5. Implementation Details

### 5.1 Memory Safety

The new implementation leverages Rust's type system to enforce memory safety:

- Generic types ensure address type safety
- Option return types handle errors without exceptions
- Domain boundaries prevent out-of-bounds access
- Clear ownership semantics through Rust's borrowing system

### 5.2 The `trim_after` Method

The `trim_after` method (previously named `pop_all`) has an improved implementation:

```rust
pub fn trim_after<'a>(&mut self, offset: Word, domain: &'a mut Domain<T>) -> Option<&'a [T]> {
    let items = self.len.checked_sub(offset)?;
    let result = domain.get(self.data.capped_next(offset, self.cap)?, items);
    // Update the block length to be equal to the offset
    self.len = offset;
    result
}
```

This method:
- Keeps elements [0..offset] in the block
- Returns elements [offset..len] that were removed
- Reduces the block's length to `offset`

For example, a block containing [1,2,3,4,5] with trim_after(2) would keep [1,2] in the block and return [3,4,5].

### 5.3 Symbol Table

The symbol table uses a HashMap to map strings to their addresses:

```rust
pub symbols: HashMap<SmolStr, Addr<Block<u8>>>,
```

This provides:
- Efficient symbol lookup by name
- String interning to reduce memory usage
- Fast symbol equality comparison

## 6. Performance Considerations

### 6.1 Memory Layout

The domain-based design optimizes for:
- Type safety: Each domain contains a single type of item
- Allocation efficiency: Simple bump allocation within domains
- Access speed: Direct addressing with minimal indirection
- Memory usage: Compact representation of values

### 6.2 Optimization Techniques

Several optimization techniques are employed:
- Word-aligned memory access for performance
- Type-based memory organization for better safety and locality
- Reuse of memory allocation patterns for consistency
- Clear ownership semantics to avoid unnecessary copying

## 7. Testing

The memory system is thoroughly tested through unit tests covering:
- Basic domain operations (push, get, alloc)
- Block operations (push, trim_after, move_to)
- Memory stack operations (push, pop)
- String and symbol operations
- Error conditions and boundary cases

Tests verify both the correctness of operations and memory safety across all components.

## 8. Best Practices

When working with the new memory system, follow these practices:

1. **Type Safety**
   - Use the correct address type (`Addr<T>`) for each operation
   - Let the compiler help verify type correctness
   - Avoid type casting between address types

2. **Error Handling**
   - Always check the `Option<T>` return value of operations
   - Use the `?` operator for clean error propagation
   - Handle None returns appropriately

3. **Memory Usage**
   - Be mindful of domain capacities
   - Release resources when no longer needed
   - Use block operations for manipulating sequences

4. **API Usage**
   - Prefer high-level operations (stack_push, alloc_block) over direct domain access
   - Use the block API for sequence operations rather than direct memory manipulation
   - Follow the ownership model for mutable access

## 9. Troubleshooting

Common issues and solutions:

1. **Option returns None**
   - Check domain capacity limits
   - Verify address calculations
   - Ensure indices are within bounds

2. **Type mismatch errors**
   - Verify address types match the domain
   - Check generic type parameters
   - Review function signatures for correct types

3. **Block operations fail**
   - Confirm block capacity is sufficient
   - Check address validity
   - Verify block length and capacity constraints

## 10. Future Enhancements

Potential areas for improvement:
- Garbage collection for automatic memory management
- More specialized domain types for specific use cases
- Improved error reporting with detailed failure reasons
- Memory usage statistics and monitoring
- Optimized domain layout based on access patterns
- Safer access patterns with better borrow checking
