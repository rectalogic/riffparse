use alloc::rc::Rc;
use binrw::{
    BinRead, BinResult, Error as BinError,
    io::{Read, Seek, SeekFrom},
    meta::ReadEndian,
};
use core::{cell::RefCell, fmt::Debug, iter::Iterator, mem::size_of};

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

#[derive(Debug)]
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
enum Header {
    #[br(magic = b"RIFF")]
    Riff(ListHeader),
    #[br(magic = b"LIST")]
    List(ListHeader),
    Chunk(ChunkHeader),
}

pub struct Metadata<R> {
    reader: Rc<RefCell<R>>,
    data_start: u64,
}

impl<R> Metadata<R> {
    fn new(reader: Rc<RefCell<R>>, data_start: u64) -> Self {
        Self { reader, data_start }
    }
}

impl<R> Debug for Metadata<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Metadata")
            .field("data_start", &self.data_start)
            .finish()
    }
}

mod private {
    use super::*;
    pub trait ChunkData<R: Read + Seek> {
        fn metadata(&mut self) -> &mut Metadata<R>;

        fn data_size(&self) -> u32;

        fn data_pad(&self) -> u32 {
            if self.data_size().is_multiple_of(2) {
                0
            } else {
                1
            }
        }
    }
}
use private::ChunkData;

pub trait ChunkRead<R: Read + Seek>: ChunkData<R> {
    fn read_data_struct<S>(&mut self) -> BinResult<S>
    where
        S: BinRead + ReadEndian + Sized,
        for<'a> <S as BinRead>::Args<'a>: Default,
    {
        S::read(&mut *self.metadata().reader.borrow_mut())
    }

    fn read_data_vec(&mut self) -> BinResult<Vec<u8>> {
        let data_size = self.data_size();
        let mut buffer = vec![0u8; data_size as usize];
        self.read_data(&mut buffer)?;
        Ok(buffer)
    }

    fn read_data(&mut self, buffer: &mut [u8]) -> BinResult<()> {
        let data_size = self.data_size();
        let data_pad = self.data_pad();
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
        if data_pad == 1 {
            reader.read_exact(&mut [0u8; 1]).map_err(BinError::Io)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
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

    pub fn id(&self) -> Fourcc {
        self.header.chunk_id
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

impl<R: Read + Seek> ChunkRead<R> for Chunk<R> {}

#[derive(Debug)]
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

    pub fn id(&self) -> Fourcc {
        self.header.list_id
    }

    pub fn iter(&'_ self) -> ListIter<'_, R> {
        ListIter::new(self)
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

impl<R: Read + Seek> ChunkRead<R> for List<R> {}

pub struct ListIter<'a, R> {
    list: &'a List<R>,
    next_position: u64,
}

impl<'a, R: Read + Seek> ListIter<'a, R> {
    fn new(list: &'a List<R>) -> Self {
        Self {
            next_position: list.metadata.data_start,
            list,
        }
    }

    fn read_next(&mut self) -> BinResult<ChunkType<R>> {
        let mut reader = self.list.metadata.reader.borrow_mut();
        reader
            .seek(SeekFrom::Start(self.next_position))
            .map_err(BinError::Io)?;
        match Header::read(&mut *reader) {
            Ok(header) => {
                let data_start = reader.stream_position().map_err(BinError::Io)?;
                match header {
                    Header::List(list_header) => {
                        let list = List::new(
                            list_header,
                            Rc::clone(&self.list.metadata.reader),
                            data_start,
                        );
                        // list_id fourcc is part of the data size, so subtract it since we already read it
                        self.next_position = data_start
                            + (list_header.size + list.data_pad()) as u64
                            - size_of::<Fourcc>() as u64;
                        Ok(ChunkType::List(list))
                    }
                    Header::Chunk(chunk_header) => {
                        let chunk = Chunk::new(
                            chunk_header,
                            Rc::clone(&self.list.metadata.reader),
                            data_start,
                        );
                        self.next_position =
                            data_start + (chunk_header.size + chunk.data_pad()) as u64;
                        Ok(ChunkType::Chunk(chunk))
                    }
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

impl<'a, R: Read + Seek> Iterator for ListIter<'a, R> {
    type Item = BinResult<ChunkType<R>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_position
            >= self.list.metadata.data_start + (self.list.header.size + self.list.data_pad()) as u64
                - size_of::<Fourcc>() as u64
        {
            None
        } else {
            Some(self.read_next())
        }
    }
}

impl<'a, R: Read + Seek> IntoIterator for &'a List<R> {
    type Item = BinResult<ChunkType<R>>;
    type IntoIter = ListIter<'a, R>;

    fn into_iter(self) -> ListIter<'a, R> {
        self.iter()
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
        let parser = RiffParser::new(file);
        if let ChunkType::List(riff) = parser.riff().unwrap() {
            dbg!(&riff);
            riff.iter()
                .try_for_each(|chunk| -> Result<(), BinError> {
                    let chunk = chunk?;
                    match chunk {
                        ChunkType::List(list) => {
                            dbg!(&list);
                            list.iter()
                                .try_for_each(|subchunk| -> Result<(), BinError> {
                                    let subchunk = subchunk?;
                                    match subchunk {
                                        ChunkType::List(sublist) => {
                                            dbg!(sublist);
                                        }
                                        ChunkType::Chunk(subsubchunk) => {
                                            dbg!(subsubchunk);
                                        }
                                    };
                                    Ok(())
                                })?;
                        }
                        ChunkType::Chunk(chunk) => {
                            dbg!(chunk);
                        }
                    };
                    Ok(())
                })
                .unwrap();
        };
    }
}
