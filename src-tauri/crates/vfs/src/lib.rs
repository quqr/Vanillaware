mod mount;

pub use mount::{
    build_index, index_archive, is_archive_path, split, Error, Mount, Split, Stat, Target,
    ARCHIVE_SPILL_DEFAULT,
};
