use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Texture,
    Animation,
    Scene,
    Model,
    Script,
    Text,
    List,
    Grid,
    Audio,
    Video,
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

    pub const fn icon(self) -> &'static str {
        self.as_str()
    }

    pub const fn label(self) -> &'static str {
        match self {
            Kind::Texture => "Texture",
            Kind::Animation => "Animation",
            Kind::Scene => "Scene",
            Kind::Model => "Model",
            Kind::Script => "Script",
            Kind::Text => "Text",
            Kind::List => "List",
            Kind::Grid => "Grid",
            Kind::Audio => "Audio",
            Kind::Video => "Video",
            Kind::Binary => "Binary",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Format {
    pub ext: &'static str,
    pub kind: Kind,
    pub label: &'static str,
    pub viewers: &'static [&'static str],
    pub has_thumb: bool,
}

const DOC_HEX: &[&str] = &["doc", "hex"];
const GRAPH_DOC_HEX: &[&str] = &["graph", "doc", "hex"];
const ANIMATION_DOC_HEX: &[&str] = &["animation", "doc", "hex"];
const ATLAS_DOC_HEX: &[&str] = &["atlas", "doc", "hex"];
const AUDIO_DOC_HEX: &[&str] = &["audio", "doc", "hex"];
const VIDEO_DOC_HEX: &[&str] = &["video", "doc", "hex"];
const CONTENT_DOC_HEX: &[&str] = &["content", "doc", "hex"];

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
        kind: Kind::Script,
        label: "RSC",
        viewers: &["dialogue", "graph", "doc", "hex"],
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

pub fn lookup(ext: &str) -> Option<&'static Format> {
    let needle = ext.trim_start_matches('.').to_ascii_lowercase();
    FORMATS.iter().find(|f| f.ext == needle)
}

pub fn is_catalogued(ext: &str) -> bool {
    lookup(ext).is_some()
}
