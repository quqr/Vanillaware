pub mod atlas;
pub mod codec;
pub mod content;
pub mod document;
pub mod field;
pub mod ftx;
pub mod mbs;
pub mod primitives;
pub mod raster;
pub mod rsc;
pub mod source;

pub use source::{DiskSource, Source};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Kaitai(kaitai::KError),
    Unsupported { what: &'static str, detail: String },
    BadData(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Kaitai(e) => write!(f, "kaitai parse error: {e:?}"),
            Error::Unsupported { what, detail } => write!(f, "unsupported {what}: {detail}"),
            Error::BadData(msg) => write!(f, "bad data: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<kaitai::KError> for Error {
    fn from(e: kaitai::KError) -> Self {
        Error::Kaitai(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct DecodedTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: u16,
    pub channels: u8,
    pub data: Vec<u8>,
}

impl DecodedTexture {
    pub fn rgba(&self) -> Vec<u8> {
        match self.channels {
            4 => self.data.clone(),
            1 => {
                let mut out = Vec::with_capacity(self.data.len() * 4);
                for &v in &self.data {
                    out.extend_from_slice(&[v, v, v, 255]);
                }
                out
            }
            other => panic!("unsupported channel count: {other}"),
        }
    }
}
