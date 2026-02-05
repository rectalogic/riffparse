#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod avi;
pub mod fourcc;
#[cfg(not(feature = "std"))]
pub mod io;
mod riff;
pub use binrw::{
    Error,
    io::{Read, Seek, SeekFrom},
};
pub use riff::{Chunk, List, ListIter, Riff, RiffParser, RiffType};
