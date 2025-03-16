//

use std::marker::PhantomData;

type Addr = u32;

pub trait Item: Sized {
    const SIZE: usize;

    fn load(data: &[u8]) -> Option<Self>;
}

impl Item for [u32; 2] {
    const SIZE: usize = 8;

    fn load(data: &[u8]) -> Option<Self> {
        let a = data[0..4].try_into().ok().map(u32::from_le_bytes)?;
        let b = data[4..8].try_into().ok().map(u32::from_le_bytes)?;
        Some([a, b])
    }
}

pub struct Box(Addr);

impl<'a> Box {
    pub fn open<T: AsRef<[u8]> + ?Sized>(self, memory: &'a T) -> Option<&'a [u8]> {
        let addr = self.0 as usize;
        let memory = memory.as_ref();
        let header = memory.get(addr..addr + 4)?;
        let size = header.try_into().ok().map(u32::from_le_bytes)? as usize;
        memory.get(addr + 4..addr + size)
    }
}

pub struct Series<'a, I: Item>(&'a [u8], PhantomData<I>);

impl<'a, I> Series<'a, I>
where
    I: Item,
{
    const HEADER_SIZE: usize = 4;

    pub fn len(&self) -> usize {
        self.0.len() / I::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn load<T: AsRef<[u8]> + ?Sized>(memory: &'a T, bx: Box) -> Option<Series<'a, I>> {
        let memory = bx.open(memory)?;
        let (header, data) = memory.split_at_checked(Self::HEADER_SIZE)?;
        let items = header
            .try_into()
            .ok()
            .map(u32::from_le_bytes)
            .map(|x| x as usize)?;
        data.get(..items * I::SIZE)
            .map(|data| Series(data, PhantomData))
    }

    pub fn get<T: Item>(&self, index: usize) -> Option<T> {
        let start = T::SIZE * index;
        self.0.get(start..start + T::SIZE).and_then(T::load)
    }
}

pub fn load_series(memory: &[u8], bx: Box, pos: usize) -> Option<[u32; 2]> {
    Series::<[u32; 2]>::load(memory, bx).and_then(|series| series.get(pos))
}
