use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_GOLDEN_ROOT: &str = r"F:\Codes\Vanillaware\spec\golden";

const FMT_BC3: u16 = 0x44;
const FMT_BC4: u16 = 0x49;
const FMT_BC7: u16 = 0x4D;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Manifest {
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Sample {
    source: String,
    relative: String,
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Frame {
    file: String,
    index: usize,
    width: usize,
    height: usize,
    channels: usize,
    payload_length: usize,
}

fn golden_root() -> PathBuf {
    std::env::var("GOLDEN_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_GOLDEN_ROOT))
}

#[test]
#[ignore = "the goldens come from the old C# tool (split-table swizzle); its block mapping for textures above 64 blocks is known wrong (see spec/golden/README.md). The Tegra X1 rearrangement now follows the rip-vanillaware PHP reference and the Tegra TRM bit patterns, so the goldens must be re-exported. Run with cargo test -- --ignored"]
fn switch_ftx_matches_golden_pixels() {
    let root = golden_root();
    let manifest_path = root.join("manifest.json");

    if !manifest_path.exists() {
        eprintln!("skip: {} not found", manifest_path.display());
        return;
    }

    let manifest: Manifest =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();

    let mut failures: Vec<String> = Vec::new();
    let mut exact_frames = 0usize;
    let mut structural_frames = 0usize;
    let mut exact_bytes = 0usize;
    let mut bc7_similarity = Vec::new();

    for sample in &manifest.samples {
        let decoded = match formats::ftx::decode_file(Path::new(&sample.source)) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{}: decode failed: {e}", sample.relative));
                continue;
            }
        };

        if decoded.len() != sample.frames.len() {
            failures.push(format!(
                "{}: frame count {} != expected {}",
                sample.relative,
                decoded.len(),
                sample.frames.len()
            ));
            continue;
        }

        for (tex, frame) in decoded.iter().zip(sample.frames.iter()) {
            let label = format!(
                "{}#{} {}x{}",
                sample.relative, frame.index, frame.width, frame.height
            );

            if tex.width as usize != frame.width
                || tex.height as usize != frame.height
                || tex.channels as usize != frame.channels
            {
                failures.push(format!(
                    "{label}: {}x{}x{} != expected {}x{}x{}",
                    tex.width, tex.height, tex.channels, frame.width, frame.height, frame.channels
                ));
                continue;
            }

            if tex.data.len() != frame.payload_length {
                failures.push(format!(
                    "{label}: payload {} bytes != expected {}",
                    tex.data.len(),
                    frame.payload_length
                ));
                continue;
            }

            let expected_path = root.join(sample_dir(&sample.relative)).join(&frame.file);
            let expected = match read_gzip(&expected_path) {
                Ok(b) => b,
                Err(e) => {
                    failures.push(format!(
                        "{label}: read {} failed: {e}",
                        expected_path.display()
                    ));
                    continue;
                }
            };

            if expected.len() != tex.data.len() {
                failures.push(format!(
                    "{label}: golden payload {} != decoded {}",
                    expected.len(),
                    tex.data.len()
                ));
                continue;
            }

            match tex.format {
                FMT_BC3 | FMT_BC4 => {
                    if let Some(diff) =
                        diff_pixels(&expected, &tex.data, frame.width, frame.channels)
                    {
                        failures.push(format!("{label} (0x{:02X}): {diff}", tex.format));
                    } else {
                        exact_frames += 1;
                        exact_bytes += expected.len();
                    }
                }
                FMT_BC7 => {
                    let (exact_pct, error) = compare_coverage(&expected, &tex.data, frame.channels);
                    bc7_similarity.push(exact_pct);
                    if let Some(msg) = error {
                        failures.push(format!("{label} (BC7): {msg}"));
                    } else {
                        structural_frames += 1;
                    }
                }
                other => failures.push(format!("{label}: unexpected format 0x{other:02X}")),
            }
        }
    }

    let avg_similarity = if bc7_similarity.is_empty() {
        0.0
    } else {
        bc7_similarity.iter().sum::<f64>() / bc7_similarity.len() as f64
    };

    eprintln!(
        "golden: {} frames byte-exact ({} MiB), {} frames structurally equal (BC7, byte similarity {:.1}%), {} failures",
        exact_frames,
        exact_bytes / (1024 * 1024),
        structural_frames,
        avg_similarity,
        failures.len()
    );

    assert!(
        failures.is_empty(),
        "golden mismatch:\n{}",
        failures.join("\n")
    );
}

fn sample_dir(relative: &str) -> String {
    let without_ext = relative
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(relative);
    without_ext.replace('/', "_")
}

fn read_gzip(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let mut decoder = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

fn diff_pixels(expected: &[u8], actual: &[u8], width: usize, channels: usize) -> Option<String> {
    let first = (0..expected.len()).find(|&i| expected[i] != actual[i])?;

    let pixel = first / channels;
    let x = pixel % width.max(1);
    let y = pixel / width.max(1);
    let channel = first % channels;

    let mut diff_count = 0usize;
    let mut max_delta = 0i32;
    for i in 0..expected.len() {
        if expected[i] != actual[i] {
            diff_count += 1;
            max_delta = max_delta.max((i32::from(actual[i]) - i32::from(expected[i])).abs());
        }
    }

    Some(format!(
        "first diff at ({x},{y}) ch{channel}: got {} expected {} | {diff_count} bytes differ, max delta {max_delta}",
        actual[first], expected[first]
    ))
}

fn compare_coverage(expected: &[u8], actual: &[u8], channels: usize) -> (f64, Option<String>) {
    let pixels = expected.len() / channels;
    let mut exact = 0usize;
    let mut expected_opaque = 0usize;
    let mut actual_opaque = 0usize;
    let mut expected_luma = 0u64;
    let mut actual_luma = 0u64;

    for p in 0..pixels {
        let e = &expected[p * channels..p * channels + channels];
        let a = &actual[p * channels..p * channels + channels];

        if e == a {
            exact += 1;
        }

        if channels == 4 {
            if e[3] != 0 {
                expected_opaque += 1;
            }
            if a[3] != 0 {
                actual_opaque += 1;
            }
        }

        expected_luma += e[0] as u64 + e[1] as u64 + e[2] as u64;
        actual_luma += a[0] as u64 + a[1] as u64 + a[2] as u64;
    }

    let pixels = pixels.max(1);
    let exact_pct = exact as f64 * 100.0 / pixels as f64;
    let expected_cov = expected_opaque as f64 / pixels as f64;
    let actual_cov = actual_opaque as f64 / pixels as f64;
    let expected_mean = expected_luma as f64 / (pixels * 3) as f64;
    let actual_mean = actual_luma as f64 / (pixels * 3) as f64;

    let error = if (expected_cov - actual_cov).abs() > 0.02 {
        Some(format!(
            "opaque coverage {:.1}% != expected {:.1}%",
            actual_cov * 100.0,
            expected_cov * 100.0
        ))
    } else if (expected_mean - actual_mean).abs() > 2.0 {
        Some(format!(
            "mean luma {:.1} != expected {:.1}",
            actual_mean, expected_mean
        ))
    } else {
        None
    };

    (exact_pct, error)
}
