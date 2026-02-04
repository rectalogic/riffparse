#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod fourcc;
mod riff;
pub use binrw::{
    Error,
    io::{Read, Seek, SeekFrom},
};
pub use riff::{ChunkRead, ChunkType, ListIter, RiffItem, RiffParser};
