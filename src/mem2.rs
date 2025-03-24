// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

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
        memory.content.get_data(data, index)
    }

    pub fn set(self, index: u32, value: Word, memory: &mut Memory) -> Option<Word> {
        unimplemented!()
    }

    pub fn push<const N: usize>(self, values: [Word; N], memory: &mut Memory) -> Option<Word> {
        let (heap, data_region) = memory.get_regions_mut();
        let [cap, len, data] = heap.get_array_mut(self.0)?;
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
        memory.content.get_word(self.0.next()?)
    }

    pub fn len(self, memory: &Memory) -> Option<Word> {
        memory.content.get_word(self.0.prev()?)
    }

    pub fn cap(self, memory: &Memory) -> Option<Word> {
        memory.content.get_word(self.0.prev_n(2)?)
    }
}

pub struct Region<'a> {
    data: &'a mut [Word],
}

impl<'a> Region<'a> {
    pub fn new(data: &'a mut [Word]) -> Self {
        Self { data }
    }

    fn init(&mut self) -> Option<()> {
        self.set_len(Addr(1))
    }

    fn len(&self) -> Option<Addr> {
        self.data.first().copied().map(Addr)
    }

    fn set_len(&mut self, len: Addr) -> Option<()> {
        self.data.first_mut().map(|first| *first = len.0)
    }

    fn get_mut(&mut self, begin: Addr, len: Word) -> Option<&mut [Word]> {
        let begin = begin.address();
        let end = begin + len as usize;
        self.data.get_mut(begin..end)
    }

    pub fn alloc(&mut self, capacity: Word) -> Option<Addr> {
        let len = self.len()?;
        let new_len = len.next_n(capacity)?;
        if new_len.address() > self.data.len() {
            None
        } else {
            self.set_len(new_len);
            Some(len)
        }
    }

    pub fn push<const N: usize>(&mut self, values: [Word; N]) -> Option<Addr> {
        let len = self.len()?;
        let new_len = len.next_n(N as Word)?;
        self.get_array_mut(len).map(|slot| {
            *slot = values;
        })?;
        self.set_len(new_len);
        Some(len)
    }

    pub fn get_word(&self, addr: Addr) -> Option<Word> {
        self.data.get(addr.address()).copied()
    }

    pub fn get(&self, begin: Addr, len: Word) -> Option<&[Word]> {
        let begin = begin.address();
        let end = begin + len as usize;
        self.data.get(begin..end)
    }

    pub fn get_array_mut<const N: usize>(&mut self, addr: Addr) -> Option<&mut [Word; N]> {
        self.get_mut(addr, N as Word)?.try_into().ok()
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
    objects: Region<'a>,
    content: Region<'a>,
}

impl<'a> Memory<'a> {
    pub fn new(objects: Region<'a>, content: Region<'a>) -> Self {
        Self { objects, content }
    }

    fn get_regions_mut(&mut self) -> (&mut Region<'a>, &mut Region<'a>) {
        (&mut self.objects, &mut self.content)
    }

    pub fn alloc_block(&mut self, capacity: Word) -> Option<Block> {
        let data = self.content.alloc(capacity)?;
        let addr = self.objects.push([capacity, 0, data.0])?;
        Some(Block(addr))
    }

    pub fn move_items(&mut self, from: Block, to: Block, count: Word) -> Option<()> {
        let (heap, data_region) = self.get_regions_mut();

        let [from_cap, from_len, from_data] = *heap.get_array_mut(from.0)?;
        let from_start = from_len.checked_sub(count)?;

        let [to_cap, to_len, to_data] = *heap.get_array_mut(to.0)?;
        let new_to_len = to_len.checked_add(count)?;
        if new_to_len > to_cap {
            return None;
        }

        let items = count as usize * 2;
        let attr_to = (to_data + to_len * 2) as usize;
        let attr_from = (from_data + from_start * 2) as usize;

        let data = data_region

        for i in 0..items {

        }

        Some(())
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
