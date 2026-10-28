use catalog::format::*;

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
        Kind::Audio,
        Kind::Video,
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
        assert_eq!(
            f.has_thumb,
            f.kind == Kind::Texture,
            "{} 缩略图策略不对",
            f.ext
        );
    }
}
