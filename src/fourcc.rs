use core::ops::Deref;

use binrw::BinRead;

pub mod tag {
    use super::FourCC;
    pub const RIFF: FourCC = FourCC::new(*b"RIFF");
    pub const LIST: FourCC = FourCC::new(*b"LIST");
}

#[derive(BinRead, Debug, Copy, Clone, Eq, PartialEq)]
#[br(little)]
pub struct FourCC(u32);

impl FourCC {
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(bytes))
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
