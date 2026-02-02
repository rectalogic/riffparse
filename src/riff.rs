use binrw::{
    BinRead, BinResult,
    io::{Read, Seek},
};
use core::{fmt::Debug, iter::Iterator};

use crate::fourcc::Fourcc;

pub struct RiffParser<R> {
    reader: R,
    chunk_stack: Vec<RiffChunk>,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            chunk_stack: Vec::with_capacity(4),
        }
    }

    pub fn skip_chunk(&mut self, chunk: &mut RiffChunk) -> BinResult<()> {
        chunk.chunk_type.skip_data(&mut self.reader)
    }

    pub fn read_chunk_vec(&mut self, chunk: &mut RiffChunk) -> BinResult<Vec<u8>> {
        chunk.chunk_type.read_data_vec(&mut self.reader)
    }

    pub fn read_chunk(&mut self, chunk: &mut RiffChunk, buffer: &mut [u8]) -> BinResult<()> {
        chunk.chunk_type.read_data(&mut self.reader, buffer)
    }
}

impl<R: Read + Seek> Iterator for RiffParser<R> {
    type Item = BinResult<RiffChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = match self.reader.stream_position() {
            Ok(pos) => pos as usize,
            Err(e) => return Some(Err(binrw::Error::Io(e))),
        };
        let token = ChunkToken(pos);
        let chunk_type = match ChunkType::read(&mut self.reader) {
            Ok(chunk_type) => chunk_type,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(RiffChunk::new(token, chunk_type)))

        //XXX need a stack of chunk_type, and keep track of amount read each time and pop when consumed
        //XXX Seek::stream_position
        // caller may want the stack too so it knows where it is - api to fetch immutable view on stack
        // also need to prevent RiffChunk::skip/read from being called more than one (need to track consumed on it, so if already consumed disallow)
        // track consumed in RiffChunk, make iter return &mut RiffChunk (and keep on stack) - so read/skip updates that instance
        //
        // do we need to return RiffChunk to user? read/skip could just always operate on top of stack
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ChunkToken(usize);

#[derive(Debug)]
pub struct RiffChunk {
    token: ChunkToken,
    chunk_type: ChunkType,
}

impl RiffChunk {
    fn new(token: ChunkToken, chunk_type: ChunkType) -> Self {
        Self { token, chunk_type }
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

impl ChunkType {
    pub fn chunk_id(&self) -> Fourcc {
        match self {
            ChunkType::Chunk(chunk) => chunk.chunk_id,
            ChunkType::List(list) | ChunkType::Riff(list) => list.list_id,
        }
    }

    pub fn data_size(&self) -> usize {
        match self {
            ChunkType::Chunk(chunk) => chunk.size as usize,
            // list_id is counted in size, but we already read it. So subtract from size.
            ChunkType::List(list) | ChunkType::Riff(list) => list.size as usize - 4,
        }
    }

    fn skip_data<R: Read + Seek>(&mut self, reader: &mut R) -> BinResult<()> {
        let mut size = self.data_size();
        // Skip padding byte
        if !size.is_multiple_of(2) {
            size += 1;
        };
        reader.seek_relative(size as i64).map_err(binrw::Error::Io)
    }

    fn read_data_vec<R: Read + Seek>(&mut self, reader: &mut R) -> BinResult<Vec<u8>> {
        let mut buffer = vec![0u8; self.data_size()];
        self.read_data(reader, &mut buffer)?;
        Ok(buffer)
    }

    fn read_data<R: Read + Seek>(&mut self, reader: &mut R, buffer: &mut [u8]) -> BinResult<()> {
        let size = self.data_size();
        if buffer.len() != size {
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

#[derive(BinRead, Debug, Copy, Clone)]
pub struct Chunk {
    chunk_id: Fourcc,
    size: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct List {
    size: u32,
    list_id: Fourcc,
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
