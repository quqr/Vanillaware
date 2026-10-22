//! FCMP 容器解压（LZSS 变体）。
//!
//! 逐行移植自 `VanillawareConverter/FTEXConverter/FcmpDecoder.cs`：
//! 12 位字典窗口预先定位在 `0xFEE`，标志位从低到高消费。

use crate::Error;

pub fn is_fcmp(data: &[u8]) -> bool {
    data.len() >= 4 && &data[..4] == b"FCMP"
}

/// 解压 FCMP。失败时返回空 `Vec`（与原实现一致，不抛异常）。
pub fn decode(data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.len() < 0x10 || !is_fcmp(data) {
        return Ok(Vec::new());
    }

    let dec_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut cmp_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    if cmp_size == 0 {
        cmp_size = data.len() - 0x10;
    }

    let mut result = vec![0u8; dec_size];
    let mut dict = [0u8; 0x1000];
    let mut dict_pos = 0xFEEusize;

    let mut src_pos = 0x10usize;
    let mut dst_pos = 0usize;
    let mut bit_pos = 0u8;
    let mut flags = 0u8;

    while src_pos < 0x10 + cmp_size && dst_pos < dec_size {
        if bit_pos == 0 {
            flags = data[src_pos];
            src_pos += 1;
            bit_pos = 8;
        }

        if flags & 1 != 0 {
            if src_pos >= data.len() || dst_pos >= dec_size {
                break;
            }

            let b = data[src_pos];
            src_pos += 1;
            result[dst_pos] = b;
            dst_pos += 1;
            dict[dict_pos] = b;
            dict_pos = (dict_pos + 1) & 0xFFF;
        } else {
            if src_pos + 1 >= data.len() {
                break;
            }

            let b1 = data[src_pos];
            let b2 = data[src_pos + 1];
            src_pos += 2;

            let mut offset = (((usize::from(b2) & 0xF0) << 4) | usize::from(b1)) & 0xFFF;
            let length = usize::from(b2 & 0x0F) + 3;

            for _ in 0..length {
                if dst_pos >= dec_size {
                    break;
                }

                let b = dict[offset];
                result[dst_pos] = b;
                dst_pos += 1;
                dict[dict_pos] = b;
                dict_pos = (dict_pos + 1) & 0xFFF;
                offset = (offset + 1) & 0xFFF;
            }
        }

        flags >>= 1;
        bit_pos -= 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_fcmp() {
        assert!(!is_fcmp(b"FTEX"));
        assert!(decode(&[0u8; 32]).unwrap().is_empty());
    }
}
