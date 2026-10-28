//! 路径哈希：FNV-1a 64，输入为小写路径，输出 16 位 hex。
//!
//! 刻意不引入哈希库：这里只需要「稳定 + 分布均匀」，不需要密码学强度。

const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn fnv1a64_path(path_lower: &str) -> String {
    // 防御性再小写一次：Windows 路径大小写不敏感，key 必须对调用方的疏忽免疫。
    let bytes = path_lower.to_lowercase();

    let mut hash = OFFSET_BASIS;
    for byte in bytes.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }

    format!("{hash:016x}")
}
