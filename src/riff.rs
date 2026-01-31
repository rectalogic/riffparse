use binrw::io::{Read, Result, Seek};
use core::iter::Iterator;

mod tag {
    use super::fourcc;
    const RIFF: u32 = fourcc(*b"RIFF");
    const LIST: u32 = fourcc(*b"LIST");
}

const fn fourcc(bytes: [u8; 4]) -> u32 {
    u32::from_le_bytes(bytes)
}

pub struct RiffParser<R> {
    reader: R,
}

impl<R: Read + Seek> RiffParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn iter(&mut self) -> ListIter<R> {
        //XXX parse main RIFF List and return it's iter
    }
}

pub enum Riff<R: Read + Seek> {
    Chunk(Chunk<R>),
    List(List<R>),
}

pub struct Chunk<R: Read + Seek> {
    reader: R,
    chunk_id: u32,
    size: u32,
}

pub struct List<R: Read + Seek> {
    chunk: Chunk<R>,
    list_id: u32,
}

impl<R: Read + Seek> Chunk<R> {
    pub fn skip(&mut self) -> Result<()> {
        let size = if !self.size.is_multiple_of(2) {
            self.size + 1
        } else {
            self.size
        };
        self.reader.seek_relative(size as i64)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<()> {
        if buffer.len() != self.size as usize {
            //XXX return error
        }
        self.reader.read_exact(buffer)?;
        if !self.size.is_multiple_of(2) {
            self.reader.read_exact(&mut [0u8; 1])?;
        }
        Ok(())
    }
}

impl<R: Read + Seek> List<R> {
    pub fn as_chunk(&self) -> &Chunk<R> {
        &self.chunk
    }

    pub fn as_mut_chunk(&mut self) -> &mut Chunk<R> {
        &mut self.chunk
    }

    pub fn iter(&mut self) {}
}

pub struct ListIter<'a, R: Read + Seek> {
    list: &'a List<R>,
}

impl<'a, R: Read + Seek> Iterator for ListIter<'a, R> {
    type Item = Result<Riff<R>>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
