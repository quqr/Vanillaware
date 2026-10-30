//! ```text
//! $env:VW_RSC_SAMPLE="F:\Codes\13\13rim ex\extract_data\RoundScript\Ogata00_00.rsc"
//! cargo test -p vanillaware --test dialogue_service -- --nocapture
//! ```

use std::path::PathBuf;

use vanillaware_lib::service::DialogueService;

#[test]
fn service_resolves_the_text_pool_and_reuses_it() {
    let Ok(sample) = std::env::var("VW_RSC_SAMPLE") else {
        eprintln!("skip: VW_RSC_SAMPLE is not set");
        return;
    };
    let path = PathBuf::from(sample);
    if !path.is_file() {
        eprintln!("skip: {} is missing", path.display());
        return;
    }

    let service = DialogueService::new();
    let first = service.open(&path).expect("first open");
    assert!(first.pool_path.is_some());
    assert!(first.pool_count > 0);
    assert!(first.stats.text_commands > 0);
    assert_eq!(first.stats.missing_text, 0);
    assert!(first.warnings.is_empty());

    let second = service.open(&path).expect("second open");
    assert_eq!(second.stats.text_commands, first.stats.text_commands);
    assert_eq!(
        second.entries[0].lines.len(),
        first.entries[0].lines.len()
    );
}
