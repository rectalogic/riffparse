#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod avi;
pub mod fourcc;
#[cfg(feature = "embedded-io")]
mod io;
#[cfg(feature = "embedded-io")]
pub use io::EmbeddedAdapter;

mod riff;
pub use binrw::{
    self, Error,
    io::{Read, Seek, SeekFrom},
};
pub use riff::{Chunk, List, ListIter, Riff, RiffParser, RiffType};
