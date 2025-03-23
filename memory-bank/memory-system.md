# Rebel Memory System Documentation

This document provides a comprehensive overview of the Rebel memory system, including its architecture, components, and key operations. The memory system is now fully operational with all tests passing.

## 1. Overview

The Rebel memory system is a custom memory management implementation designed for the Rebel programming language. It provides low-level memory abstractions to support the language's execution environment, including value representation, memory allocation, and data structures.

The memory system is organized around the following core concepts:
- **Memory**: The root container that manages all memory regions
- **Addresses**: References to specific locations in memory
- **Items**: Values that can be stored in and loaded from memory
- **Blocks and Stacks**: Higher-level data structures built on top of the memory system

## 2. Memory Architecture

### 2.1 Memory Layout

The memory system divides its space into four main regions:

1. **Symbol Table**: Stores interned strings for efficient symbol lookup
2. **Parse Stack**: Maintains values during parsing
3. **Parse Base**: Tracks block nesting during parsing
4. **Heap**: General-purpose memory for allocating blocks and strings

Each region is defined by a capacity and a current length. The memory layout is initialized with specific sizes for each region.

### 2.2 Address Types

Two key address types are used to refer to memory locations:

- **LenAddress**: Points to a length-prefixed block of memory (format: `[length: u32][data...]`)
- **CapAddress**: Points to a capacity-prefixed region (format: `[capacity: u32][length: u32][data...]`)

These address types enable efficient memory organization and access patterns.

## 3. Core Components

### 3.1 Memory Values (VmValue)

The `VmValue` is the fundamental value type in the Rebel system. It is an enum with variants for different types:
- None, Int, Bool for primitive values
- Block, Context, Path for composite types
- String for text data
- Word, SetWord, GetWord for symbols/identifiers

Tags defined in the system:
- `TAG_NONE`: Represents no value/null (0)
- `TAG_INT`: Integer value (1)
- `TAG_BOOL`: Boolean value (2)
- `TAG_BLOCK`: Block of values (3)
- `TAG_CONTEXT`: Context/scope (4)
- `TAG_PATH`: Path expression (5)
- `TAG_STRING`: String value (6)
- `TAG_WORD`: Symbol/identifier (7)
- `TAG_SET_WORD`: Assignment target (8)
- `TAG_GET_WORD`: Retrieval operator (9)

### 3.2 Data Structures

#### Blocks

Blocks are sequences of items in memory. They're represented by a length-prefixed memory region pointed to by a `LenAddress`. Operations:
- Get length of block
- Access items by index
- Memory-efficient storage of sequences

#### Stacks

Stacks allow push/pop operations on sequences of items. They're implemented as a capacity-constrained region with operations:
- Push: Add an item to the top
- Pop: Remove and return the top item
- Peek: View the top item without removing
- Cut: Extract a subset of items into a new block

#### Symbol Table

The symbol table provides efficient string interning using a hash table implementation with:
- Open addressing with linear probing
- xxHash-based hashing function
- Automatic string deduplication

### 3.3 Memory Operations

Key memory operations include:
- **Allocation**: Reserve memory for blocks, strings, or other data
- **Access**: Read from or write to memory locations
- **Conversion**: Convert between different data representations
- **Movement**: Move data between memory regions

## 4. Implementation Details

### 4.1 Memory Addressing

The memory system uses 32-bit words as its basic unit. Addresses and offsets are measured in words, while lengths can be specified in bytes or words depending on the context.

### 4.2 Memory Safety

The implementation uses Rust's type system to enforce memory safety. Key patterns:
- Optional return types to handle errors gracefully
- Bounds checking to prevent buffer overflows
- Clear ownership semantics through Rust's borrowing system
- Unsafe code only where necessary with careful documentation

### 4.3 Byte/Word Conversion

The system handles conversion between words and bytes:
- Words are 4 bytes each
- Byte-addressed data is aligned to word boundaries
- Careful handling of endianness in serialization/deserialization

## 5. Key Operations

### 5.1 Block Creation

Creating a block involves:
1. Reserving memory for the block in a capacity region
2. Setting up the length field at the beginning
3. Initializing the block content if necessary

```
pub fn reserve_block(&self, size_bytes: Word, memory: &mut Memory) -> Option<LenAddress> {
    // Allocate space for length field (4 bytes) + data
    let total_bytes = 4 + size_bytes;
    let _slot = self.alloc_slot(total_bytes, memory)?;
    
    // Calculate block address
    let len_addr = self.len_address();
    let current_len = len_addr.get_len(memory)?;
    let offset_to_block = current_len - total_bytes;
    let words_offset = offset_to_block / 4;
    let block_addr = self.data_address() + words_offset;
    
    // Create and initialize the block
    let block = LenAddress(block_addr);
    memory.set_word(block.address(), size_bytes)?;
    
    Some(block)
}
```

### 5.2 Stack Operations

Stack operations follow these patterns:

**Stack Allocation** (create a new stack):
1. Calculate memory needed based on item size and requested capacity
2. Round up to the nearest word boundary for alignment
3. Allocate memory region with appropriate capacity
4. Initialize the stack with zero length
5. Return a Stack handle for future operations

**Push** (add item to stack):
1. Calculate new length after push
2. Ensure capacity is sufficient
3. Write the item at the end of the stack
4. Update stack length

**Pop** (remove item from stack):
1. Check if stack is not empty
2. Calculate new length after pop
3. Read the item from the end of the stack
4. Update stack length
5. Return the item

### 5.3 Symbol Table Lookup

Symbol table operations use a hash-based lookup:
1. Calculate hash of the symbol
2. Find bucket in hash table
3. Check for existing entry
4. If not found, create new entry
5. Return address of the symbol

## 6. Performance Considerations

### 6.1 Memory Layout

The memory layout is designed to optimize for:
- Locality: Related data is stored close together
- Allocation speed: Simple bump allocator pattern
- Access speed: Direct addressing with minimal indirection

### 6.2 Optimization Techniques

Several optimization techniques are employed:
- Word-aligned memory access for performance
- Stack-based allocation pattern for fast LIFO operations
- String interning to reduce memory usage and comparison cost
- Reuse of memory blocks where possible

## 7. Testing

The memory system is thoroughly tested through unit tests covering:
- Basic memory operations
- Stack operations
- Block creation and access
- String handling
- Symbol table functionality

Tests ensure correct behavior and memory safety across all operations.

## 8. Best Practices

When working with the memory system, follow these best practices:

1. **Memory Access**
   - Always check the return value of memory operations (they return `Option<T>`)
   - Use high-level abstractions (Stack, Block) when possible instead of direct memory access
   - Keep track of capacity constraints to prevent allocation failures

2. **API Usage**
   - Prefer using domain-specific methods over generic memory access
   - Use the appropriate address type (LenAddress vs. CapAddress) for each use case
   - Release memory resources when they're no longer needed

3. **Testing**
   - Test memory operations with edge cases (empty, maximum size)
   - Verify both successful and failure conditions
   - Test interactions between different memory regions

4. **Documentation**
   - Document memory requirements for functions that use the memory system
   - Clearly comment any unsafe operations or assumptions
   - Document lifetime requirements for borrowed memory references

## 9. Troubleshooting

Common issues that may arise and their solutions:

1. **Memory bounds errors**
   - Check capacity calculations
   - Verify word/byte conversion
   - Ensure proper alignment
   - Confirm all regions are properly initialized with sufficient size

2. **Invalid address errors**
   - Confirm address calculation is using the right offset
   - Check if memory has been properly initialized
   - Verify pointer arithmetic, especially when converting between bytes and words
   - Ensure method visibility is correct for test access

3. **Stack corruption**
   - Review push/pop balance
   - Check length updates
   - Verify data copying operations
   - Validate memory state before and after operations

## 10. Future Enhancements

Potential areas for improvement:
- Garbage collection for automatic memory management
- Reference counting for shared resources
- Memory compaction to reduce fragmentation
- Region-based allocation for temporary computations
- More sophisticated error handling with specific error types
- Memory usage statistics and monitoring
- Optimized memory layout for specific operation patterns
