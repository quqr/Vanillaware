#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub sp_at: u32,
    pub count_at: u32,
    pub count_bytes: u8,
    pub stride: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub tag: &'static str,
    pub id3: &'static str,
    pub kind: u8,
    pub version: u16,
    pub big_endian: bool,
    pub sections: [Section; 12],
}

impl Profile {
    pub fn read_counts(&self, data: &[u8]) -> crate::Result<[u32; 12]> {
        let mut out = [0u32; 12];
        for (index, section) in self.sections.iter().enumerate() {
            let at = section.count_at as usize;
            let end = at + section.count_bytes as usize;
            let slice = data.get(at..end).ok_or_else(|| {
                crate::Error::BadData(format!("section {index} count out of range: {at:#x}"))
            })?;
            out[index] = read_uint(slice, self.big_endian);
        }
        Ok(out)
    }
}

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
