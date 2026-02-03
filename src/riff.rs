use binrw::{
    BinRead, BinResult,
    io::{Error as IoError, ErrorKind, Read, Seek},
};
use core::{fmt::Debug, iter::Iterator, mem::size_of};

use crate::fourcc::Fourcc;

pub struct RiffParser<R> {
    reader: R,
    chunk_stack: Vec<RiffChunk>,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(mut reader: R) -> BinResult<Self> {
        let riff = match ChunkType::read(&mut reader) {
            Ok(ChunkType::Riff(riff)) => ChunkType::Riff(riff),
            Ok(_) => {
                return Err(binrw::error::Error::Custom {
                    pos: 0,
                    err: Box::new("invalid RIFF file"),
                });
            }
            Err(e) => return Err(e),
        };
        let mut chunk_stack = Vec::with_capacity(4);
        chunk_stack.push(RiffChunk::new(riff));
        Ok(Self {
            reader,
            chunk_stack,
        })
    }

    pub fn skip_chunk(&mut self) -> BinResult<()> {
        let mut chunk = self.current_chunk()?;
        chunk.chunk_type.skip_data(&mut self.reader)?;
        self.consume_current(&mut chunk);
        Ok(())
    }

    pub fn read_chunk_vec(&mut self) -> BinResult<Vec<u8>> {
        let mut chunk = self.current_chunk()?;
        let data = chunk.chunk_type.read_data_vec(&mut self.reader)?;
        self.consume_current(&mut chunk);
        Ok(data)
    }

    pub fn read_chunk(&mut self, buffer: &mut [u8]) -> BinResult<()> {
        let mut chunk = self.current_chunk()?;
        chunk.chunk_type.read_data(&mut self.reader, buffer)?;
        self.consume_current(&mut chunk);
        Ok(())
    }

    fn consume_current(&mut self, chunk: &mut RiffChunk) {
        chunk.consume_all();
        if let Some(current_chunk) = self.chunk_stack.last_mut() {
            *current_chunk = *chunk;
        }
    }

    fn current_chunk(&self) -> BinResult<RiffChunk> {
        self.chunk_stack.last().copied().ok_or_else(|| {
            binrw::Error::Io(IoError::new(ErrorKind::UnexpectedEof, "no more chunks"))
        })
    }
}

impl<R: Read + Seek> Iterator for RiffParser<R> {
    type Item = BinResult<ChunkType>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current_chunk = self.chunk_stack.last()?;

        if let ChunkType::Chunk(chunk) = current_chunk.chunk_type {
            if !current_chunk.is_consumed() {
                current_chunk.chunk_type.skip_data(&mut self.reader)?;
            }
            self.chunk_stack.pop();
            // Add the chunk size to the parent list consumed size
            let consumed = current_chunk.chunk_type.chunk_size();
            current_chunk = self.chunk_stack.last()?;
            current_chunk.consume(consumed);
        }

        //XXX now current_chunk must be List/Riff - check if consumed
        //

        // first, if chunk consume if needed, pop either way - record chunk_size
        // then, last() must be List, consume(chunk_size) (may be zero)
        //    if list now consumed, pop and record chunk_size - do this in a loop until we find unconsumed list
        // read next chunk and return it
        match current_chunk.chunk_type {
            ChunkType::Riff(list) | ChunkType::List(list) => {
                if current_chunk.is_consumed() {
                    self.chunk_stack.pop();
                }
            }
            ChunkType::Chunk(chunk) => {
                if !current_chunk.is_consumed() {
                    current_chunk.chunk_type.skip_data(&mut self.reader)?;
                    self.chunk_stack.pop();
                }
            }
        }

        //XXX check stack, if Chunk then skip if not consumed, and pop - add size to parent list consumed
        // if stack now empty, read and push
        // if stack has Riff/List check if consumed - need to track bytes
        let chunk_type = match ChunkType::read(&mut self.reader) {
            Ok(chunk_type) => chunk_type,
            Err(e) => return Some(Err(e)),
        };

        Some(Ok(chunk_type))
        //XXX need a stack of chunk_type, and keep track of amount read each time and pop when consumed
        //XXX Seek::stream_position
        // caller may want the stack too so it knows where it is - api to fetch immutable view on stack
        // also need to prevent RiffChunk::skip/read from being called more than one (need to track consumed on it, so if already consumed disallow)
        // track consumed in RiffChunk, make iter return &mut RiffChunk (and keep on stack) - so read/skip updates that instance
        //
        // do we need to return RiffChunk to user? read/skip could just always operate on top of stack
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RiffChunk {
    consumed: usize,
    chunk_type: ChunkType,
}

impl RiffChunk {
    fn new(chunk_type: ChunkType) -> Self {
        Self {
            consumed: 0,
            chunk_type,
        }
    }

    fn consume(&mut self, amount: usize) {
        self.consumed += amount;
    }

    fn consume_all(&mut self) {
        self.consume(self.chunk_type.chunk_size());
    }

    fn is_consumed(&self) -> bool {
        self.consumed == self.chunk_type.chunk_size()
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

    fn chunk_size(&self) -> usize {
        let mut data_size = self.data_size();
        if !data_size.is_multiple_of(2) {
            data_size += 1;
        };
        data_size
            + match self {
                ChunkType::Riff(_) | ChunkType::List(_) => size_of::<Fourcc>() + size_of::<List>(),
                ChunkType::Chunk(_) => size_of::<Chunk>(),
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
