//! Vanillaware 格式编排层。
//!
//! 分工：
//! - **结构**：由 `spec/ksy/*.ksy` 生成到 `vw-gen`，这里只消费生成物的字段。
//! - **像素**：Kaitai 不承担块解码、地址重排、熵解压，全部在 [`codec`] 手写。

pub mod atlas;
pub mod codec;
pub mod content;
pub mod doc;
pub mod ftx;
pub mod mbs;
pub mod viz;

/// 统一的错误类型（不引入 thiserror，保持依赖面最小）。
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

/// 一张解码完成的纹理。
///
/// `channels` 为 1（BC4 灰度索引）或 4（RGBA8）。之所以不统一成 RGBA，
/// 是因为旧实现里 BC4 存的是单通道索引 + 调色板，黄金对比必须按同形比较。
#[derive(Debug, Clone)]
pub struct DecodedTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// 源文件里的格式码（0x44 BC3 / 0x49 BC4 / 0x4D BC7 …）
    pub format: u16,
    pub channels: u8,
    /// 行主序、无 padding；长度 = width * height * channels
    pub data: Vec<u8>,
}

impl DecodedTexture {
    /// 展开成 RGBA8（单通道按灰度复制，alpha 固定 255）。
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
