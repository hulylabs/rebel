use crate::mem::{CapAddress, LenAddress, Memory};

const MEMORY_SIZE: usize = 1024;

// Helper function to create a small memory instance for targeted tests
fn setup_memory<'a>(memory_array: &'a mut [u32]) -> Memory<'a> {
    // Initialize memory with a custom layout
    let mut memory = Memory::init(
        memory_array,
        [100, 100, 100, 100], // Need all 4 regions for testing
    )
    .unwrap();

    // Set a magic number at start
    memory.set_word(0, 0x0BAD_F00D).unwrap();

    memory
}

#[test]
fn test_len_address_basics() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = setup_memory(&mut memory_vec);

    // Create a LenAddress and test basic operations
    let addr = LenAddress(10);

    // Test length operations
    memory.set_word(addr.address(), 42).unwrap();
    assert_eq!(addr.get_len(&memory), Some(42));

    addr.set_len(24, &mut memory).unwrap();
    assert_eq!(addr.get_len(&memory), Some(24));

    // Test address calculations
    assert_eq!(addr.address(), 10);
    assert_eq!(addr.data_address(), 11);
}

#[test]
fn test_cap_address_basics() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = setup_memory(&mut memory_vec);

    // Create a CapAddress and test basic operations
    let addr = CapAddress(20);

    // Test capacity operations
    memory.set_word(addr.address(), 100).unwrap();
    assert_eq!(addr.get_cap(&memory), Some(100));

    // Test address calculations
    assert_eq!(addr.address(), 20);
    assert_eq!(addr.data_address(), 22);

    // Test len_address conversion
    let len_addr = addr.len_address();
    assert_eq!(len_addr.address(), 21);
}

#[test]
fn test_block_creation_and_read() {
    let mut memory_vec = vec![0u32; MEMORY_SIZE];
    let mut memory = setup_memory(&mut memory_vec);

    // Create a capacity address with enough space
    let cap_addr = memory.make_cap(100, 10).unwrap();

    // Reserve a block in the capacity address
    let test_data_size: u32 = 16; // 16 bytes
    let block_addr = cap_addr.reserve_block(test_data_size, &mut memory).unwrap();

    // Verify the block length was set correctly
    assert_eq!(block_addr.get_len(&memory), Some(test_data_size));

    // Get a mutable reference to the block data
    let data = block_addr.get_data_mut(&mut memory).unwrap();

    // Write some data to the block
    for i in 0..test_data_size as usize {
        data[i] = (i % 256) as u8;
    }

    // Read the data back and verify it matches
    let read_data = block_addr.get_data(&memory).unwrap();
    for i in 0..test_data_size as usize {
        assert_eq!(read_data[i], (i % 256) as u8);
    }
}
