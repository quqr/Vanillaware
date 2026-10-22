//! 原生二进制资源的格式注册表。
//!
//! **白名单语义**：只有登记在册的扩展名才算「库里的资源」。`.png` / `.json` / `.txt` /
//! `.quad` 这些要么是别的工具链的产物、要么是本项目的输出，一律不收录——库只呈现游戏自己的
//! 二进制资源。未登记的扩展名在目录列表里直接被过滤掉（`hidden_files` 只报数量）。
//!
//! `Kind` 是粗分类（图标 + 中文标签 + 分组），`Format` 才是扩展名级的能力声明。
//! 新增一种格式 = 这里加一行；查看器与缩略图能力跟随格式，不再跟随类别。

use serde::Serialize;

/// 资源类别。只列**原生游戏二进制资源**的类别；图片 / 文本不属于本工具的收录范围。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// FTEX 容器：压缩像素帧。
    Texture,
    /// MBS 族：骨骼-四边形动画。
    Animation,
    /// 场景装配 / 节点树。
    Scene,
    /// 模型 / 网格容器。
    Model,
    /// 脚本字节码。
    Script,
    /// 文本表 / 消息表。
    Text,
    /// 资源清单 / 表状数据。
    List,
    /// 索引 / 网格状数据。
    Grid,
    /// 已确认是原生资源但尚无解析器：只看元信息与头部字节。
    Binary,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Texture => "texture",
            Kind::Animation => "animation",
            Kind::Scene => "scene",
            Kind::Model => "model",
            Kind::Script => "script",
            Kind::Text => "text",
            Kind::List => "list",
            Kind::Grid => "grid",
            Kind::Binary => "binary",
        }
    }

    /// 列表里的图标语义名（前端映射到具体 SVG）。
    pub const fn icon(self) -> &'static str {
        self.as_str()
    }

    /// 类别中文名（查看器头部徽章用）。
    pub const fn label(self) -> &'static str {
        match self {
            Kind::Texture => "纹理",
            Kind::Animation => "动画",
            Kind::Scene => "场景",
            Kind::Model => "模型",
            Kind::Script => "脚本",
            Kind::Text => "文本",
            Kind::List => "列表",
            Kind::Grid => "网格",
            Kind::Binary => "二进制",
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
    /// 可用查看器 id，按展示顺序。
    pub viewers: &'static [&'static str],
    /// 是否支持缩略图。
    pub has_thumb: bool,
}

const DOC_HEX: &[&str] = &["doc", "hex"];
const ANIMATION_DOC_HEX: &[&str] = &["animation", "doc", "hex"];
const ATLAS_DOC_HEX: &[&str] = &["atlas", "doc", "hex"];

/// 注册表。扩展名小写、不含点。
pub const FORMATS: &[Format] = &[
    Format {
        ext: "ftx",
        kind: Kind::Texture,
        label: "FTEX",
        viewers: ATLAS_DOC_HEX,
        has_thumb: true,
    },
    Format {
        ext: "ftp",
        kind: Kind::Texture,
        label: "FTEX",
        viewers: ATLAS_DOC_HEX,
        has_thumb: true,
    },
    Format {
        ext: "mbs",
        kind: Kind::Animation,
        label: "MBS",
        viewers: ANIMATION_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "mbp",
        kind: Kind::Animation,
        label: "MBS",
        viewers: ANIMATION_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "osb",
        kind: Kind::Scene,
        label: "OSB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "bbs",
        kind: Kind::Scene,
        label: "BBS",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "asb",
        kind: Kind::Script,
        label: "ASB",
        viewers: &["asm", "doc", "hex"],
        has_thumb: false,
    },
    Format {
        ext: "fmd",
        kind: Kind::Model,
        label: "FMD",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "fsb",
        kind: Kind::List,
        label: "FSB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "fms",
        kind: Kind::Text,
        label: "FMS",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "rsc",
        kind: Kind::List,
        label: "RSC",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "otm",
        kind: Kind::Grid,
        label: "OTM",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "mpd",
        kind: Kind::Grid,
        label: "MPD",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "ctb",
        kind: Kind::Grid,
        label: "CTB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    // 已确认是原生资源、解析器尚未进场的族。
    Format {
        ext: "nnd",
        kind: Kind::Binary,
        label: "NND",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "drg",
        kind: Kind::Binary,
        label: "DRG",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "ufb",
        kind: Kind::Binary,
        label: "UFB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "ucb",
        kind: Kind::Binary,
        label: "UCB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "acb",
        kind: Kind::Binary,
        label: "ACB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "awb",
        kind: Kind::Binary,
        label: "AWB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "acf",
        kind: Kind::Binary,
        label: "ACF",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "cpk",
        kind: Kind::Binary,
        label: "CPK",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "usm",
        kind: Kind::Binary,
        label: "USM",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "vs2",
        kind: Kind::Binary,
        label: "VS2",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "inc",
        kind: Kind::Binary,
        label: "INC",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "scb",
        kind: Kind::Binary,
        label: "SCB",
        viewers: DOC_HEX,
        has_thumb: false,
    },
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
    fn every_format_has_viewers_and_icon() {
        for f in FORMATS {
            assert!(!f.viewers.is_empty(), "{} 没有查看器", f.ext);
            assert!(!f.kind.icon().is_empty());
            assert!(!f.kind.label().is_empty());
        }
    }

    #[test]
    fn extensions_are_unique_and_lowercase() {
        let mut seen = std::collections::HashSet::new();
        for f in FORMATS {
            assert_eq!(f.ext, f.ext.to_ascii_lowercase(), "{} 必须小写", f.ext);
            assert!(!f.ext.starts_with('.'), "{} 不能带点", f.ext);
            assert!(seen.insert(f.ext), "{} 重复登记", f.ext);
        }
    }

    #[test]
    fn every_kind_has_at_least_one_format() {
        for kind in [
            Kind::Texture,
            Kind::Animation,
            Kind::Scene,
            Kind::Model,
            Kind::Script,
            Kind::Text,
            Kind::List,
            Kind::Grid,
            Kind::Binary,
        ] {
            assert!(
                FORMATS.iter().any(|f| f.kind == kind),
                "{} 没有对应格式",
                kind.as_str()
            );
        }
    }

    #[test]
    fn only_textures_have_thumbnails() {
        for f in FORMATS {
            assert_eq!(f.has_thumb, f.kind == Kind::Texture, "{} 缩略图策略不对", f.ext);
        }
    }
}
