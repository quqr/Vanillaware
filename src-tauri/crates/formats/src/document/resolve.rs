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

    fn ensure(&mut self, key: &str) {
        if self
            .index
            .as_ref()
            .is_some_and(|index| index.contains_key(key))
        {
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
