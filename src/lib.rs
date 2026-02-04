#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod fourcc;
mod riff;
pub use riff::{Chunk, ChunkRead, ChunkType, List, ListIter, RiffParser};
