const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn fnv1a64_path(path_lower: &str) -> String {
    let bytes = path_lower.to_lowercase();

    let mut hash = OFFSET_BASIS;
    for byte in bytes.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }

    format!("{hash:016x}")
}
