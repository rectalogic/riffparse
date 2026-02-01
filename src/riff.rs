use alloc::rc::Rc;
use binrw::{
    BinRead, BinResult,
    io::{Read, Result, Seek},
};
use core::{cell::RefCell, fmt::Debug, iter::Iterator};

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
    type Item = BinResult<RiffItem<R>>;

    fn next(&mut self) -> Option<Self::Item> {
        let riff_type = match RiffType::read(&mut *self.reader.borrow_mut()) {
            Ok(riff_type) => riff_type,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(RiffItem::new(Rc::clone(&self.reader), riff_type)))

        //XXX need a stack of riff_type, and keep track of amount read each time and pop when consumed
        //XXX Seek::stream_position
    }
}

pub struct RiffItem<R: Read + Seek> {
    reader: Rc<RefCell<R>>,
    riff_type: RiffType,
}

impl<R: Read + Seek> Debug for RiffItem<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RiffItem")
            .field("riff_type", &self.riff_type)
            .finish()
    }
}

impl<R: Read + Seek> RiffItem<R> {
    fn new(reader: Rc<RefCell<R>>, riff_type: RiffType) -> Self {
        Self { reader, riff_type }
    }

    fn chunk(&self) -> Chunk {
        match self.riff_type {
            RiffType::Chunk(chunk) => chunk,
            RiffType::List(list) | RiffType::Riff(list) => list.chunk,
        }
    }
}

#[derive(BinRead, Debug, Copy, Clone)]
#[br(little)]
pub enum RiffType {
    #[br(magic = b"RIFF")]
    Riff(List),
    #[br(magic = b"LIST")]
    List(List),
    Chunk(Chunk),
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct Chunk {
    chunk_id: FourCC,
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

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    #[test]
    fn test_avi() {
        let file = File::open(
            "/Users/aw/Projects/rectalogic/experiments/vendor/esp32-tv/player/milk2.avi",
        )
        .unwrap();
        let mut parser = RiffParser::new(file);
        //XXX magic is not read into struct, need to remove it
        let riff = parser.next();
        dbg!(&riff);
    }
}
