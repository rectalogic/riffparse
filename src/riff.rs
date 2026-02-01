use binrw::{
    BinRead, BinResult,
    io::{Read, Seek},
};
use core::{fmt::Debug, iter::Iterator};

use crate::fourcc::FourCC;

pub struct RiffParser<R> {
    reader: R,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn skip_chunk(&mut self, chunk: &mut RiffChunk) -> BinResult<()> {
        chunk.skip(&mut self.reader)
    }

    pub fn read_chunk_vec(&mut self, chunk: &mut RiffChunk) -> BinResult<Vec<u8>> {
        chunk.read_vec(&mut self.reader)
    }

    pub fn read_chunk(&mut self, chunk: &mut RiffChunk, buffer: &mut [u8]) -> BinResult<()> {
        chunk.read(&mut self.reader, buffer)
    }
}

impl<R: Read + Seek> Iterator for RiffParser<R> {
    type Item = BinResult<RiffChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk_type = match ChunkType::read(&mut self.reader) {
            Ok(chunk_type) => chunk_type,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(RiffChunk::new(chunk_type)))

        //XXX need a stack of chunk_type, and keep track of amount read each time and pop when consumed
        //XXX Seek::stream_position
        // caller may want the stack too so it knows where it is - api to fetch immutable view on stack
        // also need to prevent RiffChunk::skip/read from being called more than one (need to track consumed on it, so if already consumed disallow)
        // track consumed in RiffChunk, make iter return &mut RiffChunk (and keep on stack) - so read/skip updates that instance
        //
        // do we need to return RiffChunk to user? read/skip could just always operate on top of stack
    }
}

#[derive(Debug)]
pub struct RiffChunk {
    chunk_type: ChunkType,
}

impl RiffChunk {
    fn new(chunk_type: ChunkType) -> Self {
        Self { chunk_type }
    }

    fn data_size(&self) -> u32 {
        match self.chunk_type {
            ChunkType::Chunk(chunk) => chunk.size,
            // list_id is counted in size, but we already read it. So subtract from size.
            ChunkType::List(list) | ChunkType::Riff(list) => list.size - 4,
        }
    }
}

#[derive(BinRead, Debug, Copy, Clone)]
#[br(little)]
pub enum ChunkType {
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

impl RiffChunk {
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
        parser.skip_chunk(&mut hdr).unwrap();
        let chunk = parser.next().unwrap().unwrap();
        dbg!(&chunk);
    }
}
