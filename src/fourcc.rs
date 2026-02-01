use core::{fmt::Debug, ops::Deref};

use binrw::BinRead;

pub mod tag {
    use super::FourCC;
    pub const RIFF: FourCC = FourCC::new(*b"RIFF");
    pub const LIST: FourCC = FourCC::new(*b"LIST");
}

#[derive(BinRead, Copy, Clone, Eq, PartialEq)]
pub struct FourCC(u32);

impl FourCC {
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }
}

impl Debug for FourCC {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let bytes = self.0.to_le_bytes();
        if let Ok(s) = str::from_utf8(&bytes) {
            write!(f, "FourCC({s})")
        } else {
            write!(f, "FourCC({bytes:?})")
        }
    }
}

impl Deref for FourCC {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 4]> for FourCC {
    fn from(bytes: [u8; 4]) -> Self {
        Self::new(bytes)
    }
}
