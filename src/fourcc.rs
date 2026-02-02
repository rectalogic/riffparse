use core::{fmt::Debug, ops::Deref};

use binrw::BinRead;

pub mod tag {
    use super::Fourcc;
    pub const RIFF: Fourcc = Fourcc::new(*b"RIFF");
    pub const LIST: Fourcc = Fourcc::new(*b"LIST");
}

#[derive(BinRead, Copy, Clone, Eq, PartialEq)]
pub struct Fourcc(u32);

impl Fourcc {
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }

    pub fn bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

impl Debug for Fourcc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let bytes = self.bytes();
        if let Ok(s) = str::from_utf8(&bytes) {
            write!(f, "Fourcc({s})")
        } else {
            write!(f, "Fourcc({bytes:?})")
        }
    }
}

impl Deref for Fourcc {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 4]> for Fourcc {
    fn from(bytes: [u8; 4]) -> Self {
        Self::new(bytes)
    }
}
