use crate::{Error, Result};

pub const MAGIC: &[u8; 8] = b"CRILAYLA";
pub const PREFIX_LEN: usize = 0x100;
pub const HEADER_LEN: usize = 0x10;

const MIN_COPY: usize = 3;

pub fn is_compressed(data: &[u8]) -> bool {
    data.len() >= MAGIC.len() && &data[..MAGIC.len()] == MAGIC
}

pub fn compressed_len(header: &[u8]) -> Option<u64> {
    if header.len() < HEADER_LEN || &header[..MAGIC.len()] != MAGIC {
        return None;
    }
    let stream = crate::le_u32(header, 12)? as u64;
    Some(HEADER_LEN as u64 + stream + PREFIX_LEN as u64)
}

pub fn decompress(input: &[u8]) -> Result<Vec<u8>> {
    if input.len() < HEADER_LEN {
        return Err(Error::Corrupt("CRILAYLA header is truncated"));
    }
    if &input[..MAGIC.len()] != MAGIC {
        return Err(Error::Corrupt("CRILAYLA magic is missing"));
    }

    let body_len =
        crate::le_u32(input, 8).ok_or(Error::Corrupt("CRILAYLA body length is missing"))? as usize;
    let stream_len = crate::le_u32(input, 12)
        .ok_or(Error::Corrupt("CRILAYLA stream length is missing"))? as usize;

    let total = PREFIX_LEN
        .checked_add(body_len)
        .ok_or(Error::Corrupt("CRILAYLA body length overflows"))?;
    let prefix_at = HEADER_LEN
        .checked_add(stream_len)
        .ok_or(Error::Corrupt("CRILAYLA stream length overflows"))?;

    if prefix_at + PREFIX_LEN > input.len() {
        return Err(Error::Corrupt("CRILAYLA prefix runs past the payload"));
    }

    let mut out = vec![0u8; total];
    out[..PREFIX_LEN].copy_from_slice(&input[prefix_at..prefix_at + PREFIX_LEN]);

    let mut bits = Bits::new(input, prefix_at);
    let mut write = total - 1;

    while write >= PREFIX_LEN {
        if bits.bit()? == 1 {
            let offset = bits.read(13)? as usize + MIN_COPY;
            let mut length = MIN_COPY;

            let mut level = bits.read(2)? as usize;
            length += level;
            if level == 3 {
                level = bits.read(3)? as usize;
                length += level;
                if level == 7 {
                    level = bits.read(5)? as usize;
                    length += level;
                    if level == 31 {
                        loop {
                            let step = bits.read(8)? as usize;
                            length += step;
                            if step != 0xFF {
                                break;
                            }
                        }
                    }
                }
            }

            for _ in 0..length {
                if write < PREFIX_LEN {
                    break;
                }
                let from = write + offset;
                if from >= total {
                    return Err(Error::Corrupt(
                        "CRILAYLA back reference runs past the output",
                    ));
                }
                out[write] = out[from];
                write -= 1;
            }
        } else {
            out[write] = bits.read(8)? as u8;
            write -= 1;
        }
    }

    Ok(out)
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    left: u8,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos, left: 0 }
    }

    fn bit(&mut self) -> Result<u32> {
        if self.left == 0 {
            self.pos = self
                .pos
                .checked_sub(1)
                .ok_or(Error::Corrupt("CRILAYLA bitstream underrun"))?;
            self.left = 7;
        } else {
            self.left -= 1;
        }
        let byte = *self
            .data
            .get(self.pos)
            .ok_or(Error::Corrupt("CRILAYLA bitstream underrun"))?;
        Ok(((byte >> self.left) & 1) as u32)
    }

    fn read(&mut self, count: u8) -> Result<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | self.bit()?;
        }
        Ok(value)
    }
}
