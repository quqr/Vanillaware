use std::path::{Path, PathBuf};

use media::ThumbCache;
use vfs::Mount;

const CORPUS: &[&str] = &[
    r"F:\Codes\13\main\1\ROBO_CN.CPK",
    r"F:\Codes\UNICORN OVERLORD\extract\160d1db236ac9d15b509ef661bf18b7b_NCA\1\Unicorn.CPK",
];

fn cache_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join("vw-archive-preview").join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create cache root");
    root
}

fn extension(path: &str) -> String {
    path.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

fn split(path: &str) -> (String, String) {
    match path.rfind(['\\', '/']) {
        Some(at) => (path[..at].to_string(), path[at + 1..].to_string()),
        None => (String::new(), path.to_string()),
    }
}

fn find_with_ext(mount: &Mount, dir: &str, wanted: &str, depth: u8) -> Option<String> {
    if depth == 0 {
        return None;
    }
    let listing = mount.list(dir).ok()?;
    for file in &listing.files {
        if extension(&file.path) == wanted {
            return Some(file.path.clone());
        }
    }
    for nested in &listing.dirs {
        if let Some(found) = find_with_ext(mount, &nested.path, wanted, depth - 1) {
            return Some(found);
        }
    }
    None
}

fn same_family(candidate: &str, stem: &str) -> bool {
    if candidate.eq_ignore_ascii_case(stem) {
        return true;
    }
    let candidate = candidate.to_lowercase();
    let stem = stem.to_lowercase();
    candidate.len() > stem.len()
        && candidate.starts_with(&stem)
        && candidate[stem.len()..]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn find_scene(mount: &Mount, dir: &str, depth: u8, numeric_only: bool) -> Option<String> {
    if depth == 0 {
        return None;
    }
    let listing = mount.list(dir).ok()?;
    for file in &listing.files {
        if extension(&file.path) != "mbs" {
            continue;
        }
        let stem = file
            .name
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or("");
        let paired = listing.files.iter().any(|other| {
            let Some((other_stem, other_ext)) = other.name.rsplit_once('.') else {
                return false;
            };
            let texture =
                other_ext.eq_ignore_ascii_case("ftx") || other_ext.eq_ignore_ascii_case("ftp");
            texture
                && if numeric_only {
                    !other_stem.eq_ignore_ascii_case(stem) && same_family(other_stem, stem)
                } else {
                    same_family(other_stem, stem)
                }
        });
        if paired {
            return Some(file.path.clone());
        }
    }
    for nested in &listing.dirs {
        if let Some(found) = find_scene(mount, &nested.path, depth - 1, numeric_only) {
            return Some(found);
        }
    }
    None
}

#[test]
#[ignore]
fn a_real_archive_entry_can_be_thumbnailed_and_materialised() {
    for cpk in CORPUS {
        if !Path::new(cpk).is_file() {
            eprintln!("skip missing corpus {cpk}");
            continue;
        }

        let spill = cache_root("spill");
        let thumbs = cache_root("thumbs");
        let mount = Mount::new(Some(spill));
        let cache = ThumbCache::new(thumbs);

        let texture = find_with_ext(&mount, cpk, "ftx", 3)
            .unwrap_or_else(|| panic!("{cpk} holds no .ftx within three levels"));

        let stat = mount
            .stat(&texture)
            .unwrap_or_else(|error| panic!("stat {texture}: {error}"));
        assert!(!stat.is_dir);

        let bytes = mount
            .read(&texture)
            .unwrap_or_else(|error| panic!("read {texture}: {error}"));

        let png = cache.ensure_png(
            Path::new(&texture),
            media::cache::Stamp::new(stat.size, stat.mtime_ms),
            0,
            || formats::ftx::decode_bytes_frame(&bytes, 0),
        );
        match png {
            Ok(png) => assert!(!png.is_empty(), "{texture} produced an empty png"),
            Err(error) => panic!("thumbnail {texture}: {error}"),
        }

        let script = find_scene(&mount, cpk, 3, true)
            .or_else(|| find_scene(&mount, cpk, 3, false))
            .unwrap_or_else(|| panic!("{cpk} holds no .mbs with a texture container"));

        let real = mount
            .real(&script)
            .unwrap_or_else(|error| panic!("materialise {script}: {error}"));
        assert!(
            real.is_file(),
            "{script} produced {real:?} which is not a file"
        );

        let (_, name) = split(&script);
        let stem = name
            .rsplit_once('.')
            .map(|(value, _)| value)
            .unwrap_or(&name);
        let container = real
            .parent()
            .map(|parent| {
                std::fs::read_dir(parent)
                    .map(|entries| {
                        entries.flatten().any(|entry| {
                            let candidate = entry.file_name().to_string_lossy().into_owned();
                            let Some((other_stem, other_ext)) = candidate.rsplit_once('.') else {
                                return false;
                            };
                            same_family(other_stem, stem)
                                && (other_ext.eq_ignore_ascii_case("ftx")
                                    || other_ext.eq_ignore_ascii_case("ftp"))
                        })
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(
            container,
            "the texture container of {script} was not materialised"
        );

        let atlas = formats::atlas::Atlas::open(&real)
            .unwrap_or_else(|error| panic!("atlas of {script}: {error}"))
            .unwrap_or_else(|| panic!("no texture container was found for {script}"));
        assert!(!atlas.is_empty(), "the atlas of {script} holds no frames");

        let container = mount
            .sibling_container(&script)
            .unwrap_or_else(|| panic!("sibling_container could not resolve {script}"));
        let container_stat = mount
            .stat(&container)
            .unwrap_or_else(|error| panic!("stat {container}: {error}"));
        let container_bytes = mount
            .read(&container)
            .unwrap_or_else(|error| panic!("read {container}: {error}"));
        let frame = cache.ensure_png_full(
            Path::new(&container),
            media::cache::Stamp::new(container_stat.size, container_stat.mtime_ms),
            0,
            || formats::ftx::decode_bytes_frame(&container_bytes, 0),
        );
        match frame {
            Ok(png) => assert!(!png.is_empty(), "{container} produced an empty frame"),
            Err(error) => panic!("atlas frame of {script}: {error}"),
        }

        let meta = mount
            .file_meta(cpk)
            .unwrap_or_else(|error| panic!("metadata of the archive mount point: {error}"));
        assert_eq!(meta.entry.ext, "cpk");
        assert_eq!(meta.entry.size, std::fs::metadata(cpk).unwrap().len());

        if let Some(script) = find_with_ext(&mount, cpk, "rsc", 3) {
            let real = mount
                .real(&script)
                .unwrap_or_else(|error| panic!("materialise {script}: {error}"));
            if let Some(pool) = formats::rsc::resolve::locate(&real) {
                let loaded = formats::rsc::resolve::TextPool::load(&pool);
                assert!(
                    loaded.is_ok(),
                    "the text pool of {script} does not load: {:?}",
                    loaded.err()
                );
                assert!(
                    !loaded.unwrap().is_empty(),
                    "the text pool of {script} is empty"
                );
            } else {
                panic!("the text pool of {script} was not materialised next to {real:?}");
            }
        }
    }
}
