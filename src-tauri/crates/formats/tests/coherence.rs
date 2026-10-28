use std::path::Path;

const SAMPLE_ROOTS: &[&str] = &[
    r"F:\Codes\13\13rim ex\EX DATA\Other\Font.ftx",
    r"F:\Codes\13\13rim ex\EX DATA\Other\FontBt.ftx",
    r"F:\Codes\13\13rim ex\EX DATA\GUI\Fontback.ftx",
    r"F:\Codes\UNICORN OVERLORD\extract\000\Font_4K\AiramFont.ftx",
    r"F:\Codes\UNICORN OVERLORD\extract\000\Font_4K\KleeFont.ftx",
];

#[test]
fn decoded_texture_is_spatially_coherent() {
    let mut files = Vec::new();
    for root in SAMPLE_ROOTS {
        let p = Path::new(root);
        if p.is_file() {
            files.push(p.to_path_buf());
        } else {
            collect_ftx(p, &mut files);
        }
    }
    eprintln!("found {} .ftx files", files.len());
    assert!(!files.is_empty(), "no samples");

    let mut checked = 0usize;
    let mut worst = 0f64;
    for f in &files {
        let Ok(frames) = formats::ftx::decode_file(f) else {
            continue;
        };
        for tex in &frames {
            let w = tex.width as usize;
            if w < 2 {
                continue;
            }
            let ch = tex.channels as usize;
            let mut total = 0f64;
            let mut n = 0usize;
            for y in 0..tex.height as usize {
                for x in 0..w - 1 {
                    let a = &tex.data[(y * w + x) * ch..];
                    let b = &tex.data[(y * w + x + 1) * ch..];
                    for c in 0..ch {
                        total += (i32::from(a[c]) - i32::from(b[c])).abs() as f64;
                        n += 1;
                    }
                }
            }
            let avg = total / n as f64;
            worst = worst.max(avg);
            eprintln!(
                "{:?} #{} {}x{} fmt={:#x}: adjacent delta = {avg:.2}",
                f.file_name().unwrap(),
                tex.name,
                tex.width,
                tex.height,
                tex.format
            );
            assert!(avg < 40.0, "{} looks like noise (avg {avg:.2})", tex.name);
        }
        checked += 1;
    }
    eprintln!("checked {checked} files, worst avg delta = {worst:.2}");
}

fn collect_ftx(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if out.len() >= 200 {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dirs = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ftx"))
        {
            out.push(path);
            if out.len() >= 200 {
                return;
            }
        }
    }
    for d in dirs {
        collect_ftx(&d, out);
    }
}
