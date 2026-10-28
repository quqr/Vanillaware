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
    /// 音频容器（cue sheet / wave bank）。
    Audio,
    /// 影片容器。
    Video,
    /// 容器可读、载荷不透明的原生资源（如 CRI CPK）。
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
            Kind::Audio => "audio",
            Kind::Video => "video",
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
            Kind::Audio => "音频",
            Kind::Video => "影片",
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
const GRAPH_DOC_HEX: &[&str] = &["graph", "doc", "hex"];
const ANIMATION_DOC_HEX: &[&str] = &["animation", "doc", "hex"];
const ATLAS_DOC_HEX: &[&str] = &["atlas", "doc", "hex"];
const AUDIO_DOC_HEX: &[&str] = &["audio", "doc", "hex"];
const VIDEO_DOC_HEX: &[&str] = &["video", "doc", "hex"];
const CONTENT_DOC_HEX: &[&str] = &["content", "doc", "hex"];

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
        viewers: &["scene", "doc", "hex"],
        has_thumb: false,
    },
    Format {
        ext: "bbs",
        kind: Kind::Scene,
        label: "BBS",
        viewers: &["layout", "graph", "doc", "hex"],
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
        viewers: &["mesh", "doc", "hex"],
        has_thumb: false,
    },
    Format {
        ext: "fsb",
        kind: Kind::List,
        label: "FSB",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "fms",
        kind: Kind::Text,
        label: "FMS",
        viewers: &["text", "doc", "hex"],
        has_thumb: false,
    },
    Format {
        ext: "rsc",
        kind: Kind::List,
        label: "RSC",
        viewers: &["graph", "doc", "hex"],
        has_thumb: false,
    },
    Format {
        ext: "otm",
        kind: Kind::Grid,
        label: "OTM",
        viewers: GRAPH_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "mpd",
        kind: Kind::Grid,
        label: "MPD",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "ctb",
        kind: Kind::Grid,
        label: "CTB",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    // 13 侧的 bundle 容器（与 OSB 同族）与战斗关卡。
    // 注意：`.bin` 不收 —— 语料里的 `.bin` 除 `Other/Battle*List.bin` 外都是
    // `fmd_extract.py` 导出的 glTF 缓冲（同目录必有 `.gltf`），属派生产物。
    Format {
        ext: "oeb",
        kind: Kind::Scene,
        label: "OEB",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "orb",
        kind: Kind::List,
        label: "ORB",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "bst",
        kind: Kind::Scene,
        label: "BST",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    // 大地图与调试参数库：结构已逐字段闭合，查看器给出全量字段。
    Format {
        ext: "nnd",
        kind: Kind::Grid,
        label: "NND",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "drg",
        kind: Kind::List,
        label: "DRG",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "ufb",
        kind: Kind::Scene,
        label: "UFB",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "ucb",
        kind: Kind::Grid,
        label: "UCB",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "vs2",
        kind: Kind::List,
        label: "VS2",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "inc",
        kind: Kind::Text,
        label: "INC",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    // CRI 音频 / 影片：容器层闭合，载荷（HCA / Sofdec2）按字节保留。
    Format {
        ext: "acb",
        kind: Kind::Audio,
        label: "ACB",
        viewers: AUDIO_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "awb",
        kind: Kind::Audio,
        label: "AFS2",
        viewers: AUDIO_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "acf",
        kind: Kind::Audio,
        label: "ACF",
        viewers: CONTENT_DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "usm",
        kind: Kind::Video,
        label: "USM",
        viewers: VIDEO_DOC_HEX,
        has_thumb: false,
    },
    // CPK 是 CRI 封包，目录表加密且 romfs 已全解包：只报元信息。
    Format {
        ext: "cpk",
        kind: Kind::Binary,
        label: "CPK",
        viewers: DOC_HEX,
        has_thumb: false,
    },
    Format {
        ext: "scb",
        kind: Kind::Text,
        label: "SCB",
        viewers: CONTENT_DOC_HEX,
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
