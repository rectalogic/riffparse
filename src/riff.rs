use alloc::rc::Rc;
use binrw::{
    BinRead,
    io::{Read, Result, Seek},
};
use core::{cell::RefCell, iter::Iterator};

use crate::fourcc::FourCC;

pub struct RiffParser<R> {
    reader: Rc<RefCell<R>>,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Rc::new(RefCell::new(reader)),
        }
    }
}

impl<R: Read + Seek> Iterator for RiffParser<R> {
    type Item = Result<RiffItem<R>>;

    fn next(&mut self) -> Option<Self::Item> {
        let fourcc = FourCC::read(&mut *self.reader.borrow_mut());
        //RiffItem::new(Rc::clone(&self.reader));
        None
        // track consumed in Chunk - skip/read update it and incrementally update as we iter List - propagate updates from sublists - so List knows when iteration is finished
        // but we will have multiple refs to reader - parent List immutable, sublist mutable
        // can we do flat iter? return List, it can be skipped, if not we iterate it's children etc. - caller can set flag to skip, or read (which effectively skips if List) otherwise we go into?
    }
}

pub struct RiffItem<R: Read + Seek> {
    reader: Rc<RefCell<R>>,
    riff_type: RiffType,
}

impl<R: Read + Seek> RiffItem<R> {
    fn new(reader: Rc<RefCell<R>>, riff_type: RiffType) -> Self {
        Self { reader, riff_type }
    }

    fn chunk(&self) -> Chunk {
        match self.riff_type {
            RiffType::Chunk(chunk) => chunk,
            RiffType::List(list) => list.chunk,
        }
    }
}

#[derive(BinRead, Debug, Copy, Clone)]
pub enum RiffType {
    #[br(magic = b"LIST")]
    List(List),
    Chunk(Chunk),
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct Chunk {
    chunk_id: FourCC,
    #[br(little)]
    size: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct List {
    chunk: Chunk,
    list_id: FourCC,
}

impl<R: Read + Seek> RiffItem<R> {
    pub fn skip(&mut self) -> Result<()> {
        let chunk = self.chunk();
        // Include padding byte
        let size = if !chunk.size.is_multiple_of(2) {
            chunk.size + 1
        } else {
            chunk.size
        };
        self.reader.borrow_mut().seek_relative(size as i64)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<()> {
        let chunk = self.chunk();
        if buffer.len() != chunk.size as usize {
            //XXX return error
        }
        self.reader.borrow_mut().read_exact(buffer)?;
        // Skip padding byte
        if !chunk.size.is_multiple_of(2) {
            self.reader.borrow_mut().read_exact(&mut [0u8; 1])?;
        }
        Ok(())
    }
}
