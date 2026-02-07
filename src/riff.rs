use alloc::rc::Rc;
use alloc::{boxed::Box, vec, vec::Vec};
use binrw::io::TakeSeekExt;
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

    pub fn riff(&self) -> BinResult<Riff<List>> {
        let mut reader = self.reader.borrow_mut();
        let header = match HeaderType::read(&mut *reader) {
            Ok(HeaderType::Riff(header)) => header,
            Ok(_) => {
                return Err(BinError::Custom {
                    pos: 0,
                    err: Box::new("invalid RIFF file"),
                });
            }
            Err(e) => return Err(e),
        };
        let data_start = reader.stream_position().map_err(BinError::Io)?;
        Ok(Riff::new(header, data_start))
    }

    pub fn chunks(&self, chunk: Riff<List>) -> ListIter<R> {
        ListIter::new(chunk, Rc::clone(&self.reader))
    }

    pub fn read_data_struct<S>(&self, chunk: Riff<Chunk>) -> BinResult<S>
    where
        S: BinRead + ReadEndian + Sized,
        for<'a> <S as BinRead>::Args<'a>: Default,
    {
        let mut reader = self.reader.borrow_mut();
        reader
            .seek(SeekFrom::Start(chunk.data_start))
            .map_err(BinError::Io)?;
        let mut limited_reader = reader.by_ref().take_seek(chunk.data_size() as u64);
        S::read(&mut limited_reader)
    }

    pub fn read_data_vec<H: Header>(&self, chunk: Riff<H>) -> BinResult<Vec<u8>> {
        let data_size = chunk.data_size();
        let mut buffer = vec![0u8; data_size as usize];
        self.read_data(chunk, &mut buffer)?;
        Ok(buffer)
    }

    pub fn read_data<H: Header>(&self, chunk: Riff<H>, buffer: &mut [u8]) -> BinResult<()> {
        let data_size = chunk.data_size();
        let data_pad = chunk.data_pad();
        let mut reader = self.reader.borrow_mut();
        if buffer.len() > data_size as usize {
            return Err(BinError::AssertFail {
                pos: chunk.data_start,
                message: "buffer too large".into(),
            });
        }
        reader
            .seek(SeekFrom::Start(chunk.data_start))
            .map_err(BinError::Io)?;
        reader.read_exact(buffer).map_err(BinError::Io)?;
        // Skip padding byte
        if data_pad == 1 {
            reader.read_exact(&mut [0u8; 1]).map_err(BinError::Io)?;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum RiffType {
    List(Riff<List>),
    Chunk(Riff<Chunk>),
}

impl Header for RiffType {
    fn id(&self) -> Fourcc {
        match self {
            RiffType::List(list) => list.id(),
            RiffType::Chunk(chunk) => chunk.id(),
        }
    }

    fn data_size(&self) -> u32 {
        match self {
            RiffType::List(list) => list.data_size(),
            RiffType::Chunk(chunk) => chunk.data_size(),
        }
    }
}

pub trait Header: Copy + Clone + Debug {
    fn id(&self) -> Fourcc;
    fn data_size(&self) -> u32;
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct Chunk {
    chunk_id: Fourcc,
    size: u32,
}

impl Header for Chunk {
    fn id(&self) -> Fourcc {
        self.chunk_id
    }

    fn data_size(&self) -> u32 {
        self.size
    }
}

#[derive(BinRead, Debug, Copy, Clone)]
pub struct List {
    size: u32,
    list_id: Fourcc,
}

impl Header for List {
    fn id(&self) -> Fourcc {
        self.list_id
    }

    fn data_size(&self) -> u32 {
        // The list_id is part of the data, but we read it as part of the header
        self.size - size_of::<Fourcc>() as u32
    }
}

#[derive(BinRead, Debug, Copy, Clone)]
#[br(little)]
enum HeaderType {
    #[br(magic = b"RIFF")]
    Riff(List),
    #[br(magic = b"LIST")]
    List(List),
    Chunk(Chunk),
}

#[derive(Debug, Copy, Clone)]
pub struct Riff<H> {
    header: H,
    data_start: u64,
}

impl<H: Header> Riff<H> {
    fn new(header: H, data_start: u64) -> Self {
        Self { header, data_start }
    }

    fn data_pad(&self) -> u32 {
        if self.data_size().is_multiple_of(2) {
            0
        } else {
            1
        }
    }

    pub fn position(&self) -> u64 {
        self.data_start
    }

    pub fn data_size(&self) -> u32 {
        self.header.data_size()
    }

    pub fn id(&self) -> Fourcc {
        self.header.id()
    }
}

pub struct ListIter<R> {
    reader: Rc<RefCell<R>>,
    list: Riff<List>,
    next_position: u64,
}

impl<R: Read + Seek> ListIter<R> {
    fn new(list: Riff<List>, reader: Rc<RefCell<R>>) -> Self {
        Self {
            reader,
            next_position: list.data_start,
            list,
        }
    }

    pub fn position(&self) -> u64 {
        self.next_position
    }

    fn read_next(&mut self) -> BinResult<RiffType> {
        let mut reader = self.reader.borrow_mut();
        reader
            .seek(SeekFrom::Start(self.next_position))
            .map_err(BinError::Io)?;
        match HeaderType::read(&mut *reader) {
            Ok(header) => {
                let data_start = reader.stream_position().map_err(BinError::Io)?;
                match header {
                    HeaderType::List(list_header) => {
                        let list = Riff::new(list_header, data_start);
                        self.next_position =
                            data_start + (list.data_size() + list.data_pad()) as u64;
                        Ok(RiffType::List(list))
                    }
                    HeaderType::Chunk(chunk_header) => {
                        let chunk = Riff::new(chunk_header, data_start);
                        self.next_position =
                            data_start + (chunk.data_size() + chunk.data_pad()) as u64;
                        Ok(RiffType::Chunk(chunk))
                    }
                    HeaderType::Riff(_) => Err(BinError::Custom {
                        pos: data_start,
                        err: Box::new("malformed RIFF file"),
                    }),
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl<R: Read + Seek> Iterator for ListIter<R> {
    type Item = BinResult<RiffType>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_position
            >= self.list.data_start + (self.list.data_size() + self.list.data_pad()) as u64
                - size_of::<Fourcc>() as u64
        {
            None
        } else {
            Some(self.read_next())
        }
    }
}
