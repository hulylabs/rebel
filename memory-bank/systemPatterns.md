# System Patterns

## Memory System Architecture

The Rebel interpreter uses a carefully structured memory system designed for both efficiency and safety. The memory system follows a hierarchical pattern of abstractions:

### Core Structures

1. **Memory (`Memory`)**: The base container for all memory operations
   - Manages multiple domains, each for a specific data type
   - Provides domain-specific operations through type-safe interfaces
   - Divides memory into logical domains for different types

2. **Addresses (`Addr<T>`)**: Memory location abstractions
   - Generic `Addr<T>` for type-safe memory addressing
   - Properly encapsulated with private fields
   - Provides bounds checking and safe access to memory

3. **Domains (`Domain<T>`)**: Type-specific memory regions
   - Each domain contains items of a single type
   - Operations for allocation, access, and manipulation
   - Safe access through address abstraction

4. **Data Structures**:
   - `Block<T>`: Sequence of items with random access
   - Interaction with domains through type-safe interfaces
   - Symbol table for string interning and word storage

### Memory Layout

Memory is organized into distinct regions:
- Symbol table: For string interning and efficient word lookup
- Parse stack: For building up parsed values
- Parse base: Helper stack for parsing nested structures
- Heap: General memory allocations

### Value Representation

Values in memory use an enum representation (`VmValue`):
- Enum variants for different value types (None, Int, Bool, String, etc.)
- Support for composite types (Block, Context, Path)
- Symbol representation (Word, SetWord, GetWord)
- Serialized to memory as [data: u32, tag: u32] pairs

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
