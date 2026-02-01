use alloc::rc::Rc;
use binrw::{
    BinRead, BinResult,
    io::{Read, Seek},
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

    fn data_size(&self) -> u32 {
        match self.riff_type {
            RiffType::Chunk(chunk) => chunk.size,
            // Subtract the size of list_id
            RiffType::List(list) | RiffType::Riff(list) => list.size - 4,
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
    size: u32,
    list_id: FourCC,
}

impl<R: Read + Seek> RiffItem<R> {
    pub fn skip(&mut self) -> BinResult<()> {
        let mut size = self.data_size();
        // Include padding byte
        if !size.is_multiple_of(2) {
            size += 1;
        };
        self.reader
            .borrow_mut()
            .seek_relative(size as i64)
            .map_err(binrw::Error::Io)
    }

    pub fn read_vec(&mut self) -> BinResult<Vec<u8>> {
        let mut buffer = vec![0u8; self.data_size() as usize];
        self.read(&mut buffer)?;
        Ok(buffer)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> BinResult<()> {
        let mut reader = self.reader.borrow_mut();
        let size = self.data_size();
        if buffer.len() != size as usize {
            let pos = reader.stream_position().unwrap_or(0);
            return Err(binrw::Error::AssertFail {
                pos,
                message: "read buffer too small".into(),
            });
        }
        reader.read_exact(buffer).map_err(binrw::Error::Io)?;
        // Skip padding byte
        if !size.is_multiple_of(2) {
            reader.read_exact(&mut [0u8; 1]).map_err(binrw::Error::Io)?;
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

        let riff = parser.next().unwrap().unwrap();
        dbg!(&riff);
        let mut hdr = parser.next().unwrap().unwrap();
        dbg!(&hdr);
        hdr.skip().unwrap();
        let chunk = parser.next().unwrap().unwrap();
        dbg!(&chunk);
    }
}
