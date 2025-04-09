# Rebel Memory System

This document explains the Rebel memory management system and how to work with it properly.

## Overview

The Rebel memory system provides a simple, efficient way to manage blocks of homogeneous data items, similar to arrays or vectors in other languages. The fundamental data structure is the `Series<T>`, which represents a reference to a block of memory containing items of the same type.

## Memory Structure

### Block Header

Every series begins with a Block header containing:

```
+------------------+
| Block (8 bytes)  |
| - cap: u32       | <- Series.address points here
| - len: u32       |
+------------------+
| Data area...     | <- Items are stored here
| (cap - 2) words  |
+------------------+
```

- `cap`: Total capacity of the block in Words (u32), including the header itself
- `len`: Number of items currently in the block (type-dependent)

### Memory Usage

The memory system uses byte-based calculations internally for simplicity and consistency:

1. **Allocation Process:**
   - Calculate total bytes needed for all items (item_size * capacity)
   - Round up to whole words (4-byte units)
   - Add space for the Block header (2 words = 8 bytes)

2. **Capacity Calculation:**
   - Get total data bytes (excluding header)
   - Divide by the item size to get maximum item capacity

### Important Observations

From our testing, we discovered some interesting behaviors:

1. For `Value` objects (8 bytes):
   - Capacity is calculated as: (total_bytes - header_bytes) / sizeof(Value)
   - Due to word alignment, sometimes you get slightly more capacity than requested

2. Stack-like behavior:
   - Push adds items to the end of the series
   - Pop removes items from the end (LIFO - Last In, First Out)

3. Memory initialization:
   - With newly allocated series, initial length is 0

## API Guidelines

### Use the Public API

Always use the provided public API to work with memory, not direct access to Block fields:

```rust
// Good - use the appropriate API
let capacity = rebel::mem::capacity(&memory, series);
let length = memory.len(series);

// Bad - direct access to Block fields (error-prone and may break)
let block = memory.get::<Block>(series.address);
let cap = block.cap;
let len = block.len;
```

### Capacity Management

When allocating series, consider the item type:

```rust
// For larger items (like Value), request the number you need
let value_series = memory.alloc::<Value>(5);  // Can store 5 Value items

// For smaller items (like u8), remember that multiple items fit per word
let u8_series = memory.alloc::<u8>(20);  // Can store 20 u8 items

// Always check the real capacity if needed
let actual_capacity = rebel::mem::capacity(&memory, series);
```

### Error Handling

Always handle the various memory errors appropriately:

- `StackOverflow`: Occurs when pushing beyond capacity
- `StackUnderflow`: Occurs when popping from an empty series
- `OutOfMemory`: Occurs when allocating beyond available memory
- `OutOfBounds`: Occurs when accessing invalid addresses
- `TypeMismatch`: Occurs when converting between incompatible types

## Testing Notes

When writing tests, be aware of some memory behavior quirks:
1. Capacity rounding:
   - The actual capacity may be slightly different than requested
   - Be aware of how item sizes affect actual capacity