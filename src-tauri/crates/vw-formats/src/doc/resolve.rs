//! 资源名解析：清单/名表里只有文件名，真实文件常在同级或兄弟目录。
//!
//! 解析顺序（由便宜到贵）：
//! 1. 同目录与上溯 3 层目录直接拼路径；
//! 2. 每个祖先做一次**有界**的文件名索引（深度 ≤ 2、条目上限 8000、大小写不敏感）。
//!
//! 第 2 步只在必要时建一次并缓存，单次 `graph()` 调用内复用。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

const ANCESTOR_LEVELS: usize = 3;
const WALK_DEPTH: usize = 2;
const WALK_LIMIT: usize = 8000;

pub struct Resolver {
    base: PathBuf,
    index: Option<HashMap<String, PathBuf>>,
}

impl Resolver {
    pub fn new(base: &Path) -> Self {
        Self {
            base: base.to_path_buf(),
            index: None,
        }
    }

    /// 返回（候选路径, 是否真实存在）。
    pub fn resolve(&mut self, name: &str) -> (PathBuf, bool) {
        if name.is_empty() {
            return (self.base.clone(), false);
        }

        for dir in self.base.ancestors().take(ANCESTOR_LEVELS) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return (candidate, true);
            }
        }

        let key = name.to_ascii_lowercase();
        self.ensure(&key);
        match self.index.as_ref().and_then(|index| index.get(&key)) {
            Some(found) => (found.clone(), true),
            None => (self.base.join(name), false),
        }
    }

    /// 逐层祖先建索引，**命中即停**：先付出小目录的代价，只有当名字确实不在
    /// 近处时才继续往外走。
    fn ensure(&mut self, key: &str) {
        if self.index.as_ref().is_some_and(|index| index.contains_key(key)) {
            return;
        }

        let dirs: Vec<PathBuf> = self
            .base
            .ancestors()
            .take(ANCESTOR_LEVELS)
            .map(Path::to_path_buf)
            .collect();
        let index = self.index.get_or_insert_with(HashMap::new);
        let mut budget = WALK_LIMIT;

        for dir in &dirs {
            collect(dir, 0, index, &mut budget);
            if index.contains_key(key) || budget == 0 {
                break;
            }
        }
    }
}

fn collect(dir: &Path, depth: usize, index: &mut HashMap<String, PathBuf>, budget: &mut usize) {
    if depth > WALK_DEPTH || *budget == 0 {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;

        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };

        if kind.is_dir() {
            collect(&path, depth + 1, index, budget);
        } else if kind.is_file() {
            if let Some(name) = path.file_name() {
                index
                    .entry(name.to_string_lossy().to_ascii_lowercase())
                    .or_insert(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_same_directory_first() {
        let dir = std::env::temp_dir().join("vw-resolver-test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Target.mbs");
        std::fs::write(&file, b"x").unwrap();

        let (path, ok) = Resolver::new(&dir).resolve("Target.mbs");
        assert!(ok);
        assert_eq!(path, file);
    }

    #[test]
    fn missing_name_reports_false_without_panicking() {
        let (_, ok) = Resolver::new(Path::new(".")).resolve("no-such-file.mbs");
        assert!(!ok);
    }
}
