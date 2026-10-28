//! MBS 平台档案：分节表与平台标签。
//!
//! 数据逐行对齐权威 PHP 参考 `rip-vanillaware/vanillaware.inc` 的
//! `$gp_data['ver72' | 'ver76' | 'ver77']`（见 ADR-0004：真值来源是 PHP）。
//!
//! 每个分节的读法只有三件事：
//! 1. `sp`：从文件 `sp_at` 处读 4 字节得到该分节数据的**绝对起始偏移**；
//! 2. `count`：从 `count_at` 处读 `count_bytes` 字节得到记录条数；
//! 3. `stride`：单条记录的字节步长。
//!
//! 范围仅含 v72 / v76 / v77 家族（Odin、Dragon、13 Sentinels、Unicorn Overlord）。
//! 更早的 ver55 / ver66 / ver6e 不在本轮范围（ADR-0004）。

/// 一个分节在文件中的读法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// `sp` 的读取位置：从这里读 4 字节得到分节数据的绝对偏移。
    pub sp_at: u32,
    /// `count` 的读取位置。
    pub count_at: u32,
    /// `count` 的字节宽度（v77 的 s0..s4 为 4，其余为 2）。
    pub count_bytes: u8,
    /// 单条记录的字节步长。
    pub stride: u32,
}

/// 一个平台标签对应的档案。
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    /// 平台标识串（见 `CONTEXT.md` 的 tag 条目）。
    pub tag: &'static str,
    /// 该平台的 `idtag`（PHP `$gp_data[..]['idtag']`），原样写进 V55 顶层的 `id3`。
    pub id3: &'static str,
    /// 分节记录布局代际，喂给 ksy 的 `kind` 参数：
    /// `0` = v72（s4 步长 `0x10`），`1` = v76，`2` = v77（s4 步长 `0x14`）。
    /// 只有 s4 的两份记录定义需要它来二选一。
    pub kind: u8,
    /// 该平台文件 `0x14` 处应当出现的版本号（`0x72` / `0x76` / `0x77`），仅用于自校验。
    pub version: u16,
    /// 该平台是否为大端（PHP 的 `bigend`）。
    pub big_endian: bool,
    /// 12 个分节 s0..sb 的读法表。
    pub sections: [Section; 12],
}

impl Profile {
    /// 按分节表读出 12 个分节的记录条数（PHP `sect_head` 的 `$sc`）。
    ///
    /// 越界即报错：`count` 字段本身缺失说明文件不是本格式，不应静默按 0 处理。
    pub fn read_counts(&self, data: &[u8]) -> crate::Result<[u32; 12]> {
        let mut out = [0u32; 12];
        for (index, section) in self.sections.iter().enumerate() {
            let at = section.count_at as usize;
            let end = at + section.count_bytes as usize;
            let slice = data.get(at..end).ok_or_else(|| {
                crate::Error::BadData(format!("分节 {index} 的 count 越界：{at:#x}"))
            })?;
            out[index] = read_uint(slice, self.big_endian);
        }
        Ok(out)
    }
}

/// 读 `1..=4` 字节的无符号整数；对应 PHP `van_int` 的大小端两个分支。
fn read_uint(bytes: &[u8], big_endian: bool) -> u32 {
    let mut value = 0u32;
    if big_endian {
        for &byte in bytes {
            value = (value << 8) | u32::from(byte);
        }
    } else {
        for (shift, &byte) in bytes.iter().enumerate() {
            value |= u32::from(byte) << (shift * 8);
        }
    }
    value
}

/// v72 家族（ps3_odin / ps4_odin / ps4_drag）的分节表。
const VER72: [Section; 12] = [
    Section {
        sp_at: 0x0b0,
        count_at: 0x54,
        count_bytes: 2,
        stride: 0x18,
    }, // s0
    Section {
        sp_at: 0x0b8,
        count_at: 0x56,
        count_bytes: 2,
        stride: 0x30,
    }, // s1
    Section {
        sp_at: 0x0c0,
        count_at: 0x58,
        count_bytes: 2,
        stride: 0x30,
    }, // s2
    Section {
        sp_at: 0x0c8,
        count_at: 0x5a,
        count_bytes: 2,
        stride: 0x50,
    }, // s3
    Section {
        sp_at: 0x0d0,
        count_at: 0x50,
        count_bytes: 4,
        stride: 0x10,
    }, // s4
    Section {
        sp_at: 0x0d8,
        count_at: 0x5c,
        count_bytes: 2,
        stride: 0x08,
    }, // s5
    Section {
        sp_at: 0x0e0,
        count_at: 0x62,
        count_bytes: 2,
        stride: 0x1c,
    }, // s6
    Section {
        sp_at: 0x0e8,
        count_at: 0x5e,
        count_bytes: 2,
        stride: 0x24,
    }, // s7
    Section {
        sp_at: 0x0f0,
        count_at: 0x60,
        count_bytes: 2,
        stride: 0x20,
    }, // s8
    Section {
        sp_at: 0x0f8,
        count_at: 0x64,
        count_bytes: 2,
        stride: 0x30,
    }, // s9
    Section {
        sp_at: 0x100,
        count_at: 0x66,
        count_bytes: 2,
        stride: 0x18,
    }, // sa
    Section {
        sp_at: 0x108,
        count_at: 0x6a,
        count_bytes: 2,
        stride: 0x14,
    }, // sb
];

/// v76 家族（ps4_sent）的分节表。
///
/// 与 [`VER72`] **仅差 s4 的步长**：v72 为 `0x10`，v76 为 `0x14`
/// （PHP 注释：`ver72 -> ver76: s4 *10 -> *14`）。
const VER76: [Section; 12] = [
    Section {
        sp_at: 0x0b0,
        count_at: 0x54,
        count_bytes: 2,
        stride: 0x18,
    }, // s0
    Section {
        sp_at: 0x0b8,
        count_at: 0x56,
        count_bytes: 2,
        stride: 0x30,
    }, // s1
    Section {
        sp_at: 0x0c0,
        count_at: 0x58,
        count_bytes: 2,
        stride: 0x30,
    }, // s2
    Section {
        sp_at: 0x0c8,
        count_at: 0x5a,
        count_bytes: 2,
        stride: 0x50,
    }, // s3
    Section {
        sp_at: 0x0d0,
        count_at: 0x50,
        count_bytes: 4,
        stride: 0x14,
    }, // s4
    Section {
        sp_at: 0x0d8,
        count_at: 0x5c,
        count_bytes: 2,
        stride: 0x08,
    }, // s5
    Section {
        sp_at: 0x0e0,
        count_at: 0x62,
        count_bytes: 2,
        stride: 0x1c,
    }, // s6
    Section {
        sp_at: 0x0e8,
        count_at: 0x5e,
        count_bytes: 2,
        stride: 0x24,
    }, // s7
    Section {
        sp_at: 0x0f0,
        count_at: 0x60,
        count_bytes: 2,
        stride: 0x20,
    }, // s8
    Section {
        sp_at: 0x0f8,
        count_at: 0x64,
        count_bytes: 2,
        stride: 0x30,
    }, // s9
    Section {
        sp_at: 0x100,
        count_at: 0x66,
        count_bytes: 2,
        stride: 0x18,
    }, // sa
    Section {
        sp_at: 0x108,
        count_at: 0x6a,
        count_bytes: 2,
        stride: 0x14,
    }, // sb
];

/// v77 家族（swi/ps4/ps5/xbx 的 unic、sent、grim）的分节表。
///
/// 与 v76 的差别：s0..s4 的 `count` 加宽为 4 字节，且各 `count_at` 整体后移
/// （PHP 注释：`ver76 -> ver77: head 0 1 2 3 -> 0+ 1+ 2+ 3+`）。
const VER77: [Section; 12] = [
    Section {
        sp_at: 0x0b0,
        count_at: 0x54,
        count_bytes: 4,
        stride: 0x18,
    }, // s0
    Section {
        sp_at: 0x0b8,
        count_at: 0x58,
        count_bytes: 4,
        stride: 0x30,
    }, // s1
    Section {
        sp_at: 0x0c0,
        count_at: 0x5c,
        count_bytes: 4,
        stride: 0x30,
    }, // s2
    Section {
        sp_at: 0x0c8,
        count_at: 0x60,
        count_bytes: 4,
        stride: 0x50,
    }, // s3
    Section {
        sp_at: 0x0d0,
        count_at: 0x50,
        count_bytes: 4,
        stride: 0x14,
    }, // s4
    Section {
        sp_at: 0x0d8,
        count_at: 0x64,
        count_bytes: 2,
        stride: 0x08,
    }, // s5
    Section {
        sp_at: 0x0e0,
        count_at: 0x6a,
        count_bytes: 2,
        stride: 0x1c,
    }, // s6
    Section {
        sp_at: 0x0e8,
        count_at: 0x66,
        count_bytes: 2,
        stride: 0x24,
    }, // s7
    Section {
        sp_at: 0x0f0,
        count_at: 0x68,
        count_bytes: 2,
        stride: 0x20,
    }, // s8
    Section {
        sp_at: 0x0f8,
        count_at: 0x6c,
        count_bytes: 2,
        stride: 0x30,
    }, // s9
    Section {
        sp_at: 0x100,
        count_at: 0x6e,
        count_bytes: 2,
        stride: 0x18,
    }, // sa
    Section {
        sp_at: 0x108,
        count_at: 0x72,
        count_bytes: 2,
        stride: 0x14,
    }, // sb
];

/// 按平台标签取出档案；不在本轮范围内返回 `None`。
///
/// 标签全集与 PHP `$gp_data` 的键一一对应（仅取 v72/v76/v77 家族）。
/// `id3` 逐字抄自 PHP 的 `idtag`。
pub fn lookup(tag: &str) -> Option<Profile> {
    let (name, id3, kind, version, big_endian, sections) = match tag {
        "ps3_odin" => ("ps3_odin", "ps3 odin sphere", 0, 0x72, true, VER72),
        "ps4_odin" => ("ps4_odin", "ps4 odin sphere", 0, 0x72, false, VER72),
        "ps4_drag" => ("ps4_drag", "ps4 dragon crown", 0, 0x72, false, VER72),
        "ps4_sent" => ("ps4_sent", "ps4 13 sentinels", 1, 0x76, false, VER76),
        "swi_sent" => ("swi_sent", "swit 13 sentinels", 2, 0x77, false, VER77),
        "swi_grim" => ("swi_grim", "swit grim grimoire", 2, 0x77, false, VER77),
        "ps4_grim" => ("ps4_grim", "ps4 grim grimoire", 2, 0x77, false, VER77),
        "ps5_grim" => ("ps5_grim", "ps5 grim grimoire", 2, 0x77, false, VER77),
        "swi_unic" => ("swi_unic", "swit unicorn overlord", 2, 0x77, false, VER77),
        "ps4_unic" => ("ps4_unic", "ps4 unicorn overlord", 2, 0x77, false, VER77),
        "ps5_unic" => ("ps5_unic", "ps5 unicorn overlord", 2, 0x77, false, VER77),
        "xbx_unic" => ("xbx_unic", "xbsx unicorn overlord", 2, 0x77, false, VER77),
        _ => return None,
    };
    Some(Profile {
        tag: name,
        id3,
        kind,
        version,
        big_endian,
        sections,
    })
}
