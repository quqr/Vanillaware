mod common;

use archive::CpkArchive;

#[test]
fn sample_toc_matches_the_reference_listing() {
    let archive = CpkArchive::open(&common::asset("SampleData.cpk")).expect("open");

    let summary = archive.summary();
    assert_eq!(summary.toc_offset, 174080);
    assert_eq!(summary.content_offset, 2048);
    assert!(!summary.header_encrypted);
    assert!(!summary.toc_encrypted);
    assert_eq!(summary.entries, 3);
    assert_eq!(summary.declared_files, 3);
    assert_eq!(summary.directories, 1);
    assert_eq!(summary.out_of_range, 0);
    assert_eq!(summary.toc_padding, 0);

    let entries = archive.entries();
    assert_eq!(entries[0].name, "Audio-NoCompression.flac");
    assert_eq!(entries[0].size, 48431);
    assert_eq!(entries[0].extract_size, 48431);
    assert_eq!(entries[0].offset, 2048);
    assert_eq!(entries[0].dir, "");

    assert_eq!(entries[1].name, "Image-NoCompression.jpg");
    assert_eq!(entries[1].offset, 50688);

    assert_eq!(entries[2].name, "Text-Compressed.txt");
    assert_eq!(entries[2].size, 2484);
    assert_eq!(entries[2].extract_size, 3592);
    assert_eq!(entries[2].offset, 171520);
}

#[test]
fn encrypted_sample_lists_the_same_entries() {
    let plain = CpkArchive::open(&common::asset("SampleData.cpk")).expect("plain");
    let encrypted =
        CpkArchive::open(&common::asset("SampleData-Encrypted.cpk")).expect("encrypted");

    assert!(encrypted.summary().header_encrypted);
    assert!(encrypted.summary().toc_encrypted);

    assert_eq!(plain.entries().len(), encrypted.entries().len());
    for (left, right) in plain.entries().iter().zip(encrypted.entries()) {
        assert_eq!(left.path(), right.path());
        assert_eq!(left.size, right.size);
        assert_eq!(left.extract_size, right.extract_size);
        assert_eq!(left.offset, right.offset);
    }
}

#[test]
fn compressed_entry_decompresses_to_the_expected_text() {
    let archive = CpkArchive::open(&common::asset("SampleData.cpk")).expect("open");
    let data = archive.read(2).expect("read");
    let expected =
        std::fs::read(common::asset("SampleData/Text-Compressed.txt")).expect("fixture missing");

    assert_eq!(data.len(), 3592);
    assert_eq!(data, expected);
}

#[test]
fn uncompressed_entries_are_returned_verbatim() {
    let archive = CpkArchive::open(&common::asset("SampleData.cpk")).expect("open");

    let audio = archive.read(0).expect("read audio");
    assert_eq!(audio.len(), 48431);
    assert_eq!(&audio[..4], b"fLaC");

    let image = archive.read(1).expect("read image");
    assert_eq!(image.len(), 120719);
    assert_eq!(&image[..2], [0xFF, 0xD8]);
}

#[test]
fn paths_and_directory_queries_are_normalized() {
    let archive = CpkArchive::open(&common::asset("SampleData.cpk")).expect("open");

    assert_eq!(archive.entries()[0].path(), "Audio-NoCompression.flac");
    assert_eq!(archive.find("Audio-NoCompression.flac"), Some(0));
    assert_eq!(archive.find("/Audio-NoCompression.flac"), Some(0));
    assert_eq!(archive.find("missing.flac"), None);

    assert!(archive.child_dirs("").is_empty());
    assert_eq!(archive.child_entries("").len(), 3);
}
