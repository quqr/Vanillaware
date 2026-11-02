use std::path::Path;

pub trait Source {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn peers(&self, path: &str) -> Vec<String>;
    fn walk_names(&self, dir: &str, depth: u8) -> Vec<String>;
}

pub struct DiskSource;

impl Source for DiskSource {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    fn peers(&self, path: &str) -> Vec<String> {
        let Some(directory) = Path::new(path).parent() else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(directory) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
            })
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }

    fn walk_names(&self, dir: &str, depth: u8) -> Vec<String> {
        walkdir::WalkDir::new(dir)
            .max_depth(depth as usize + 1)
            .follow_links(false)
            .into_iter()
            .flatten()
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| entry.file_name().to_str().map(|name| name.to_string()))
            .collect()
    }
}
