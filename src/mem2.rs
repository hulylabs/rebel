// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::value;

pub type Word = u32;
pub type MemValue = [Word; 2];

#[derive(Copy, Clone, Debug)]
pub struct Addr(Word);

impl Addr {
    pub fn address(self) -> usize {
        self.0 as usize
    }

    pub fn prev(self) -> Option<Addr> {
        self.0.checked_sub(1).map(Addr)
    }

    pub fn prev_n(self, n: Word) -> Option<Addr> {
        self.0.checked_sub(n).map(Addr)
    }

    pub fn next(self) -> Option<Addr> {
        self.0.checked_add(1).map(Addr)
    }

    pub fn next_n(self, n: Word) -> Option<Addr> {
        self.0.checked_add(n).map(Addr)
    }
}

/// Block representation in memory
/// [Capacity (Words)] [Length (Items)] [Tag (Addr point here)] [Data (point to data some memory bank, this is not an Addr)]
pub struct Block(Addr);

impl Block {
    pub fn get(self, index: Word, memory: &Memory) -> Option<MemValue> {
        let data = self.data(memory)?;
        memory.data.get_data(data, index)
    }

    pub fn set(self, index: u32, value: Word, memory: &mut Memory) -> Option<Word> {
        unimplemented!()
    }

    pub fn push<const N: usize>(self, values: [Word; N], memory: &mut Memory) -> Option<Word> {
        let (heap, data_region) = memory.get_regions_mut();
        let [cap, len, _, data] = heap.get_array_mut(self.0)?;
        let new_len = *len + N as Word;
        if new_len > *cap {
            None
        } else {
            *len = new_len;
            data_region.get_array_mut(Addr(*data)).map(|data| {
                *data = values;
            });
            Some(new_len)
        }
    }

    fn data(self, memory: &Memory) -> Option<Word> {
        memory.data.get_word(self.0.next()?)
    }

    pub fn len(self, memory: &Memory) -> Option<Word> {
        memory.data.get_word(self.0.prev()?)
    }

    pub fn cap(self, memory: &Memory) -> Option<Word> {
        memory.data.get_word(self.0.prev_n(2)?)
    }

    // fn get_all<'a>(self, heap: &'a mut Region) -> Option<&'a mut [Word; 4]> {
    //     heap.get_range_mut(self.0.prev_n(2)?, self.0.next_n(2)?)?
    //         .try_into()
    //         .ok()
    // }
}

pub struct Region<'a> {
    data: &'a mut [Word],
    len: Addr,
}

impl<'a> Region<'a> {
    pub fn new(data: &'a mut [Word]) -> Self {
        Self { data, len: Addr(0) }
    }

    fn get_mut(&mut self, begin: Addr, end: Addr) -> Option<&mut [Word]> {
        self.data.get_mut(begin.address()..end.address())
    }

    pub fn alloc(&mut self, capacity: Word) -> Option<Addr> {
        let len = self.len;
        self.len = self.len.next_n(capacity)?;
        if self.len.address() > self.data.len() {
            None
        } else {
            Some(len)
        }
    }

    pub fn push<const N: usize>(&mut self, values: [Word; N]) -> Option<Addr> {
        let len = self.len;
        let new_len = self.len.next_n(N as Word)?;
        let target = self.get_mut(len, new_len)?;
        target.copy_from_slice(&values);
        self.len = new_len;
        Some(len)
    }

    pub fn get_word(&self, addr: Addr) -> Option<Word> {
        self.data.get(addr.address()).copied()
    }

    pub fn get_range(&self, begin: Addr, end: Addr) -> Option<&[Word]> {
        self.data.get(begin.address()..end.address())
    }

    pub fn get_range_mut(&mut self, begin: Addr, end: Addr) -> Option<&mut [Word]> {
        self.data.get_mut(begin.address()..end.address())
    }

    pub fn get_array_mut<const N: usize>(&mut self, addr: Addr) -> Option<&mut [Word; N]> {
        self.data
            .get_mut(addr.address()..addr.next_n(N as Word)?.address())?
            .try_into()
            .ok()
    }

    fn get_data(&self, data: Word, index: Word) -> Option<MemValue> {
        let data = data as usize;
        let index = index as usize;
        let start = data + index * 2;
        let end = start + 2;
        let slot = self.data.get(start..end)?;
        slot.try_into().ok()
    }
}

pub struct Memory<'a> {
    heap: Region<'a>,
    data: Region<'a>,
}

impl<'a> Memory<'a> {
    pub fn new(heap: Region<'a>, data: Region<'a>) -> Self {
        Self { heap, data }
    }

    fn get_regions_mut(&mut self) -> (&mut Region<'a>, &mut Region<'a>) {
        (&mut self.heap, &mut self.data)
    }

    pub fn alloc_block(&mut self, capacity: Word) -> Option<Block> {
        let data = self.data.alloc(capacity)?;
        let addr = self.heap.push([capacity, 0, 0, data.0])?;
        Some(Block(addr.next_n(2)?))
    }
}

//

#[inline(never)]
pub fn len2(memory: &Memory, block: Block) -> Option<Word> {
    block.len(memory)
}

pub fn alloc2(memory: &mut Memory, capacity: Word) -> Option<Block> {
    memory.alloc_block(capacity)
}

pub fn get2(memory: &Memory, block: Block, index: Word) -> Option<MemValue> {
    block.get(index, memory)
}

pub fn push2(memory: &mut Memory, block: Block, values: [Word; 2]) -> Option<Word> {
    block.push(values, memory)
}
