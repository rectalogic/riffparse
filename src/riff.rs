use alloc::rc::Rc;
use binrw::{
    BinRead, BinResult,
    io::{Read, Seek},
};
use core::{cell::RefCell, fmt::Debug, iter::Iterator};

use crate::fourcc::FourCC;

pub struct RiffParser<R> {
    reader: RefCell<R>,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: RefCell::new(reader),
        }
    }

    pub fn skip_item(&mut self, item: &mut RiffItem) -> BinResult<()> {
        item.skip(&mut *self.reader.borrow_mut())
    }

    pub fn read_item_vec(&mut self, item: &mut RiffItem) -> BinResult<Vec<u8>> {
        item.read_vec(&mut *self.reader.borrow_mut())
    }

    pub fn read_item(&mut self, item: &mut RiffItem, buffer: &mut [u8]) -> BinResult<()> {
        item.read(&mut *self.reader.borrow_mut(), buffer)
    }
}

impl<R: Read + Seek> Iterator for RiffParser<R> {
    type Item = BinResult<RiffItem>;

    fn next(&mut self) -> Option<Self::Item> {
        let riff_type = match RiffType::read(&mut *self.reader.borrow_mut()) {
            Ok(riff_type) => riff_type,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(RiffItem::new(riff_type)))

        //XXX need a stack of riff_type, and keep track of amount read each time and pop when consumed
        //XXX Seek::stream_position
        // caller may want the stack too so it knows where it is - api to fetch immutable view on stack
        // also need to prevent RiffItem::skip/read from being called more than one (need to track consumed on it, so if already consumed disallow)
        // track consumed in RiffItem, make iter return &mut RiffItem (and keep on stack) - so read/skip updates that instance
    }
}

#[derive(Debug)]
pub struct RiffItem {
    riff_type: RiffType,
}

impl RiffItem {
    fn new(riff_type: RiffType) -> Self {
        Self { riff_type }
    }

    fn data_size(&self) -> u32 {
        match self.riff_type {
            RiffType::Chunk(chunk) => chunk.size,
            // list_id is counted in size, but we already read it. So subtract from size.
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

impl RiffItem {
    fn skip<R: Read + Seek>(&mut self, reader: &mut R) -> BinResult<()> {
        let mut size = self.data_size();
        // Include padding byte
        if !size.is_multiple_of(2) {
            size += 1;
        };
        reader.seek_relative(size as i64).map_err(binrw::Error::Io)
    }

    fn read_vec<R: Read + Seek>(&mut self, reader: &mut R) -> BinResult<Vec<u8>> {
        let mut buffer = vec![0u8; self.data_size() as usize];
        self.read(reader, &mut buffer)?;
        Ok(buffer)
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R, buffer: &mut [u8]) -> BinResult<()> {
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
        parser.skip_item(&mut hdr).unwrap();
        let chunk = parser.next().unwrap().unwrap();
        dbg!(&chunk);
    }
}
