use alloc::rc::Rc;
use binrw::{
    BinRead, BinResult, Error as BinError,
    io::{Error as IoError, ErrorKind, Read, Seek, SeekFrom},
};
use core::{cell::RefCell, fmt::Debug, iter::Iterator, mem::size_of, ops::Range};

use crate::fourcc::Fourcc;

pub struct RiffParser<R> {
    reader: Rc<RefCell<R>>,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Rc::new(RefCell::new(reader)),
        }
    }

    pub fn riff(&self) -> BinResult<ChunkType<R>> {
        let mut reader = self.reader.borrow_mut();
        let header = match Header::read(&mut *reader) {
            Ok(Header::Riff(header)) => header,
            Ok(_) => {
                return Err(BinError::Custom {
                    pos: 0,
                    err: Box::new("invalid RIFF file"),
                });
            }
            Err(e) => return Err(e),
        };
        let data_start = reader.stream_position().map_err(BinError::Io)?;
        Ok(ChunkType::List(List::new(
            header,
            Rc::clone(&self.reader),
            data_start,
        )))
    }
}

pub enum ChunkType<R> {
    List(List<R>),
    Chunk(Chunk<R>),
}

#[derive(BinRead, Debug, Copy, Clone)]
struct ChunkHeader {
    chunk_id: Fourcc,
    size: u32,
}

#[derive(BinRead, Debug, Copy, Clone)]
struct ListHeader {
    size: u32,
    list_id: Fourcc,
}

#[derive(BinRead, Debug, Copy, Clone)]
#[br(little)]
pub enum Header {
    #[br(magic = b"RIFF")]
    Riff(ListHeader),
    #[br(magic = b"LIST")]
    List(ListHeader),
    Chunk(ChunkHeader),
}

#[derive(Debug, Clone)]
struct Metadata<R> {
    reader: Rc<RefCell<R>>,
    consumed: bool,
    data_start: u64,
}

impl<R> Metadata<R> {
    fn new(reader: Rc<RefCell<R>>, data_start: u64) -> Self {
        Self {
            reader,
            consumed: false,
            data_start,
        }
    }
}

trait ChunkData<R: Read + Seek> {
    fn metadata(&mut self) -> &mut Metadata<R>;

    fn data_size(&self) -> u32;

    fn skip_data(&mut self) -> BinResult<()> {
        let data_size = self.data_size();
        let metadata = self.metadata();
        let mut data_end = metadata.data_start + data_size as u64;
        // Skip padding byte
        if !data_size.is_multiple_of(2) {
            data_end += 1;
        };
        metadata
            .reader
            .borrow_mut()
            .seek(SeekFrom::Start(data_end))
            .map_err(BinError::Io)?;
        metadata.consumed = true;
        Ok(())
    }

    fn read_data_vec(&mut self) -> BinResult<Vec<u8>> {
        let data_size = self.data_size();
        let mut buffer = vec![0u8; data_size as usize];
        self.read_data(&mut buffer)?;
        Ok(buffer)
    }

    fn read_data(&mut self, buffer: &mut [u8]) -> BinResult<()> {
        let data_size = self.data_size();
        let metadata = self.metadata();
        let mut reader = metadata.reader.borrow_mut();
        if buffer.len() != data_size as usize {
            let pos = reader.stream_position().unwrap_or(0);
            return Err(BinError::AssertFail {
                pos,
                message: "read buffer too small".into(),
            });
        }
        reader.read_exact(buffer).map_err(BinError::Io)?;
        // Skip padding byte
        if !data_size.is_multiple_of(2) {
            reader.read_exact(&mut [0u8; 1]).map_err(BinError::Io)?;
        }
        metadata.consumed = true;
        Ok(())
    }
}

pub struct Chunk<R> {
    header: ChunkHeader,
    metadata: Metadata<R>,
}

impl<R: Read + Seek> Chunk<R> {
    fn new(header: ChunkHeader, reader: Rc<RefCell<R>>, data_start: u64) -> Self {
        Self {
            header,
            metadata: Metadata::new(reader, data_start),
        }
    }
}

impl<R: Read + Seek> ChunkData<R> for Chunk<R> {
    fn metadata(&mut self) -> &mut Metadata<R> {
        &mut self.metadata
    }

    fn data_size(&self) -> u32 {
        self.header.size
    }
}

pub struct List<R> {
    header: ListHeader,
    metadata: Metadata<R>,
}

impl<R: Read + Seek> List<R> {
    fn new(header: ListHeader, reader: Rc<RefCell<R>>, data_start: u64) -> Self {
        Self {
            header,
            metadata: Metadata::new(reader, data_start),
        }
    }

    fn read_next(&mut self) -> BinResult<ChunkType<R>> {
        let mut reader = self.metadata.reader.borrow_mut();
        match Header::read(&mut *reader) {
            Ok(header) => {
                let data_start = reader.stream_position().map_err(BinError::Io)?;
                match header {
                    Header::List(list_header) => Ok(ChunkType::List(List::new(
                        list_header,
                        Rc::clone(&self.metadata.reader),
                        data_start,
                    ))),
                    Header::Chunk(chunk_header) => Ok(ChunkType::Chunk(Chunk::new(
                        chunk_header,
                        Rc::clone(&self.metadata.reader),
                        data_start,
                    ))),
                    Header::Riff(_) => Err(BinError::Custom {
                        pos: data_start,
                        err: Box::new("malformed RIFF file"),
                    }),
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl<R: Read + Seek> ChunkData<R> for List<R> {
    fn metadata(&mut self) -> &mut Metadata<R> {
        &mut self.metadata
    }

    fn data_size(&self) -> u32 {
        self.header.size
    }
}

impl<R: Read + Seek> Iterator for List<R> {
    type Item = BinResult<ChunkType<R>>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.read_next())
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
