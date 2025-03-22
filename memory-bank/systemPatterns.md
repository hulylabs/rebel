# System Patterns

## Memory System Architecture

The Rebel interpreter uses a carefully structured memory system designed for both efficiency and safety. The memory system follows a hierarchical pattern of abstractions:

### Core Structures

1. **Memory (`Memory`)**: The base container for all memory operations
   - Manages a raw buffer of 32-bit words (`u32`)
   - Provides low-level operations (get_word, set_word, get, get_mut)
   - Divides memory into logical regions for different purposes

2. **Addresses (`LenAddress`, `CapAddress`)**: Memory location abstractions
   - `LenAddress`: Points to a length-prefixed block of memory
   - `CapAddress`: Points to a capacity-prefixed region of memory
   - Provides bounds checking and safe access to memory

3. **Data Structures**:
   - `Stack<T>`: Generic stack implementation for storing items
   - `Block<T>`: Sequence of items with random access
   - `Str`: String representation in memory
   - `Arena`: Memory arena for allocating objects
   - `SymbolTable`: String interning system for efficient word storage

### Memory Layout

Memory is organized into distinct regions:
- Symbol table: For string interning and efficient word lookup
- Parse stack: For building up parsed values
- Parse base: Helper stack for parsing nested structures
- Heap: General memory allocations

### Value Representation

Values in memory use a tagged representation (`MemValue`):
- 32-bit word for data (typically an address or immediate value)
- 8-bit tag to indicate type
- Support for various types (int, bool, string, block, etc.)

### Key Patterns

1. **Memory Safety**: All operations return `Option<T>` to indicate success/failure
2. **Abstraction Layers**: Raw memory operations are abstracted behind safe interfaces
3. **Address Indirection**: Values typically point to memory addresses rather than containing data directly
4. **Generic Data Structures**: Stack and Block use generics with the `Item` trait
5. **Serialization Protocol**: The `Item` trait defines how types are stored in and loaded from memory

### API Design

The API follows these principles:
- Low-level operations are exposed through `Memory` methods
- Data structures are built on top of these operations
- Each structure provides specific operations relevant to its purpose
- The API aims to be comprehensive enough for testing while maintaining internal cohesion
- Helper functions provide convenient access to common operations

This architecture provides a solid foundation for the interpreter, balancing performance needs with safety and clarity.
