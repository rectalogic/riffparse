use binrw::io;
use embedded_io::{
    Error as EError, ErrorKind as EErrorKind, Read as ERead, Seek as ESeek, SeekFrom as ESeekFrom,
};

#[derive(Debug)]
pub struct EmbeddedAdapter<T>(pub T);

impl<T> From<T> for EmbeddedAdapter<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

fn map_kind(kind: EErrorKind) -> io::ErrorKind {
    match kind {
        EErrorKind::NotFound => io::ErrorKind::NotFound,
        EErrorKind::PermissionDenied => io::ErrorKind::PermissionDenied,
        EErrorKind::ConnectionRefused => io::ErrorKind::ConnectionRefused,
        EErrorKind::ConnectionReset => io::ErrorKind::ConnectionReset,
        EErrorKind::ConnectionAborted => io::ErrorKind::ConnectionAborted,
        EErrorKind::NotConnected => io::ErrorKind::NotConnected,
        EErrorKind::AddrInUse => io::ErrorKind::AddrInUse,
        EErrorKind::AddrNotAvailable => io::ErrorKind::AddrNotAvailable,
        EErrorKind::BrokenPipe => io::ErrorKind::BrokenPipe,
        EErrorKind::AlreadyExists => io::ErrorKind::AlreadyExists,
        EErrorKind::InvalidInput => io::ErrorKind::InvalidInput,
        EErrorKind::InvalidData => io::ErrorKind::InvalidData,
        EErrorKind::TimedOut => io::ErrorKind::TimedOut,
        EErrorKind::Interrupted => io::ErrorKind::Interrupted,
        EErrorKind::Unsupported => io::ErrorKind::Other, // no matching kind
        EErrorKind::OutOfMemory => io::ErrorKind::Other, // no matching kind
        EErrorKind::WriteZero => io::ErrorKind::WriteZero,
        EErrorKind::Other => io::ErrorKind::Other,
        _ => io::ErrorKind::Other, // ErrorKind is non_exhaustive
    }
}

fn map_error<E: EError>(e: E) -> io::Error {
    io::Error::from(map_kind(e.kind()))
}

impl<T> io::Read for EmbeddedAdapter<T>
where
    T: ERead,
    T::Error: EError,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf).map_err(map_error)
    }
}

impl<T> io::Seek for EmbeddedAdapter<T>
where
    T: ESeek,
    T::Error: EError,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let pos = match pos {
            io::SeekFrom::Start(n) => ESeekFrom::Start(n),
            io::SeekFrom::End(n) => ESeekFrom::End(n),
            io::SeekFrom::Current(n) => ESeekFrom::Current(n),
        };
        self.0.seek(pos).map_err(map_error)
    }
}
