mod archive;
pub mod container;
mod crilayla;
mod table;
mod toc;

pub use archive::{CpkArchive, CpkEntry, CpkSummary};
pub use crilayla::{decompress, is_compressed};
pub use table::{Column, ColumnKind, Table};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    BadContainer {
        at: u64,
        reason: &'static str,
    },
    BadTable {
        reason: &'static str,
    },
    MissingColumn(&'static str),
    OutOfRange {
        what: &'static str,
        value: u64,
        limit: u64,
    },
    Corrupt(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(error) => write!(f, "io error: {error}"),
            Error::BadContainer { at, reason } => {
                write!(f, "bad table container at {at}: {reason}")
            }
            Error::BadTable { reason } => write!(f, "bad @UTF table: {reason}"),
            Error::MissingColumn(name) => write!(f, "@UTF table has no column named {name}"),
            Error::OutOfRange { what, value, limit } => {
                write!(f, "{what} is {value}, outside 0..{limit}")
            }
            Error::Corrupt(reason) => write!(f, "corrupt payload: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

pub(crate) fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub(crate) fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

pub(crate) fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub(crate) fn be_i32(bytes: &[u8], at: usize) -> Option<i32> {
    be_u32(bytes, at).map(|value| value as i32)
}

pub(crate) fn be_u64(bytes: &[u8], at: usize) -> Option<u64> {
    let slice = bytes.get(at..at + 8)?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(slice);
    Some(u64::from_be_bytes(buf))
}
