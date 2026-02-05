#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod avi;
pub mod fourcc;
mod riff;
pub use binrw::{
    Error,
    io::{Read, Seek, SeekFrom},
};
pub use riff::{Chunk, ChunkType, List, ListIter, RiffItem, RiffParser};
