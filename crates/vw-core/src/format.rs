//! 原生二进制资源的格式注册表。
//!
//! **白名单语义**：只有登记在册的扩展名才算「库里的资源」。`.png` / `.json` / `.txt` /
//! `.quad` 这些要么是别的工具链的产物、要么是本项目的输出，一律不收录——库只呈现游戏自己的
//! 二进制资源。未登记的扩展名在目录列表里直接被过滤掉（`hidden_files` 只报数量）。
//!
//! 新增一种格式 = 这里加一行 + 一个查看器组件；`kind` 决定用哪个查看器。

use serde::Serialize;

/// 资源类别。只列**原生游戏二进制资源**的类别；图片 / 文本不属于本工具的收录范围。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// FTEX 容器：压缩像素帧。
    Texture,
    /// MBS 族：骨骼-四边形动画。
    Animation,
    /// 已确认是原生资源但尚无解析器：只看元信息与头部字节。
    Binary,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Texture => "texture",
            Kind::Animation => "animation",
            Kind::Binary => "binary",
        }
    }

    /// 列表里的图标语义名（前端映射到具体 SVG）。
    pub const fn icon(self) -> &'static str {
        match self {
            Kind::Texture => "texture",
            Kind::Animation => "animation",
            Kind::Binary => "binary",
        }
    }

    /// 该类别可用的查看器 id，按展示顺序。
    pub const fn viewers(self) -> &'static [&'static str] {
        match self {
            Kind::Texture => &["atlas"],
            Kind::Animation => &["animation"],
            Kind::Binary => &["hex"],
        }
    }
}

/// 一条格式声明。
#[derive(Clone, Copy, Debug)]
pub struct Format {
    pub ext: &'static str,
    pub kind: Kind,
    /// 给人看的格式名（列表的类型列）。
    pub label: &'static str,
}

/// 注册表。扩展名小写、不含点。
pub const FORMATS: &[Format] = &[
    Format { ext: "ftx", kind: Kind::Texture, label: "FTEX" },
    Format { ext: "ftp", kind: Kind::Texture, label: "FTEX" },
    Format { ext: "mbs", kind: Kind::Animation, label: "MBS" },
    Format { ext: "mbp", kind: Kind::Animation, label: "MBS" },
    // 已确认是原生资源、解析器尚未进场的族。
    Format { ext: "asb", kind: Kind::Binary, label: "ASB" },
    Format { ext: "nnd", kind: Kind::Binary, label: "NND" },
    Format { ext: "bbs", kind: Kind::Binary, label: "BBS" },
    Format { ext: "fms", kind: Kind::Binary, label: "FMS" },
    Format { ext: "mpd", kind: Kind::Binary, label: "MPD" },
    Format { ext: "inc", kind: Kind::Binary, label: "INC" },
    Format { ext: "acb", kind: Kind::Binary, label: "ACB" },
    Format { ext: "awb", kind: Kind::Binary, label: "AWB" },
    Format { ext: "cpk", kind: Kind::Binary, label: "CPK" },
    Format { ext: "drg", kind: Kind::Binary, label: "DRG" },
    Format { ext: "ufb", kind: Kind::Binary, label: "UFB" },
    Format { ext: "ucb", kind: Kind::Binary, label: "UCB" },
    Format { ext: "acf", kind: Kind::Binary, label: "ACF" },
    Format { ext: "vs2", kind: Kind::Binary, label: "VS2" },
];

/// 按扩展名查表；`ext` 大小写不敏感、不含点。
pub fn lookup(ext: &str) -> Option<&'static Format> {
    let needle = ext.trim_start_matches('.').to_ascii_lowercase();
    FORMATS.iter().find(|f| f.ext == needle)
}

/// 该扩展名是否被收录。
pub fn is_catalogued(ext: &str) -> bool {
    lookup(ext).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_case_and_dot_insensitive() {
        assert_eq!(lookup("FTX").map(|f| f.kind), Some(Kind::Texture));
        assert_eq!(lookup(".Mbs").map(|f| f.kind), Some(Kind::Animation));
    }

    #[test]
    fn derived_outputs_are_not_catalogued() {
        for ext in ["png", "json", "txt", "quad", "rgba", "7z"] {
            assert!(!is_catalogued(ext), "{ext} 不应被收录");
        }
    }

    #[test]
    fn every_kind_has_a_viewer() {
        for f in FORMATS {
            assert!(!f.kind.viewers().is_empty(), "{} 没有查看器", f.ext);
            assert!(!f.kind.icon().is_empty());
        }
    }
}
