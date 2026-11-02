use std::io::{Read, Seek, SeekFrom};

use crate::{Error, Result};

pub const LEN: u64 = 16;

const MAX_TABLE_BYTES: u32 = 64 * 1024 * 1024;
const ENCRYPTED_PREFIX: [u8; 4] = [0x1F, 0x9E, 0xF3, 0xF5];
const XOR_SEED: u8 = 0x5F;
const XOR_STEP: u8 = 0x15;

#[derive(Debug)]
pub struct Container {
    pub body: Vec<u8>,
    pub magic: [u8; 4],
    pub encrypted: bool,
}

pub fn read<R: Read + Seek>(reader: &mut R, at: u64, file_len: u64) -> Result<Container> {
    if at + LEN > file_len {
        return Err(Error::BadContainer {
            at,
            reason: "header runs past the end of the file",
        });
    }

    let mut header = [0u8; LEN as usize];
    reader.seek(SeekFrom::Start(at))?;
    reader.read_exact(&mut header)?;

    let size = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    if size == 0 || size > MAX_TABLE_BYTES {
        return Err(Error::BadContainer {
            at,
            reason:
                "implausible table length, the container length is not a sane little-endian u32",
        });
    }
    if at + LEN + size as u64 > file_len {
        return Err(Error::BadContainer {
            at,
            reason: "table body runs past the end of the file",
        });
    }

    let mut body = vec![0u8; size as usize];
    reader.seek(SeekFrom::Start(at + LEN))?;
    reader.read_exact(&mut body)?;

    let encrypted = body.starts_with(&ENCRYPTED_PREFIX);
    if encrypted {
        decrypt(&mut body);
    }

    let mut magic = [0u8; 4];
    magic.copy_from_slice(&header[..4]);

    Ok(Container {
        body,
        magic,
        encrypted,
    })
}

fn decrypt(body: &mut [u8]) {
    let mut key = XOR_SEED;
    for byte in body.iter_mut() {
        *byte ^= key;
        key = key.wrapping_mul(XOR_STEP);
    }
}
