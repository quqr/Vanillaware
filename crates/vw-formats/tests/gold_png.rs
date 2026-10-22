//! 对照 PHP 参考工具导出结果验证 FTX 解码。
//!
//! 参考数据由 `php FTEX_swit_unicorn.php <ftx>` 生成，存放于 `.tmp/phpout`：
//! 输出为 `RGBA` 容器（`RGBA` magic + w(4) + h(4) + w*h*4 像素）。该输出正是
//! 官方 PNG 的来源，可作为逐字节真值。目录缺失时本测试自动跳过。

use std::path::Path;

fn read_rgba_container(path: &Path) -> Option<(usize, usize, Vec<u8>)> {
    let raw = std::fs::read(path).ok()?;
    if raw.len() < 12 || &raw[0..4] != b"RGBA" {
        return None;
    }
    let w = u32::from_le_bytes(raw[4..8].try_into().ok()?) as usize;
    let h = u32::from_le_bytes(raw[8..12].try_into().ok()?) as usize;
    Some((w, h, raw[12..12 + w * h * 4].to_vec()))
}

#[test]
fn ftx_matches_php_reference() {
    let dir = Path::new(r"F:\Codes\Vanillaware\.tmp\phpout");
    if !dir.is_dir() {
        eprintln!("skip: 参考目录不存在");
        return;
    }

    let mut checked = 0usize;
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .expect("read dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ftx"))
        .collect();
    names.sort();

    for ftx in &names {
        let stem = ftx.file_stem().expect("stem").to_string_lossy().to_string();
        let frames = vw_formats::ftx::decode_file(ftx).expect("decode ftx");

        for (index, tex) in frames.iter().enumerate() {
            let php = dir.join(format!("{stem}.{index}.nvt.rgba"));
            let Some((pw, ph, reference)) = read_rgba_container(&php) else {
                continue;
            };

            assert_eq!(tex.width as usize, pw, "{stem} 帧 {index} 宽度");
            assert_eq!(tex.height as usize, ph, "{stem} 帧 {index} 高度");

            // 仅四通道格式可与 RGBA 参考逐字节比对（BC4 为灰度，见下方说明）。
            let ch = tex.channels as usize;
            if ch != 4 {
                eprintln!("{stem} 帧 {index}: ch={ch}，跳过 RGBA 比对");
                continue;
            }

            let mut exact = 0usize;
            for i in 0..pw * ph {
                let p = &tex.data[i * ch..];
                let rp = &reference[i * 4..];
                if p[0] == rp[0] && p[1] == rp[1] && p[2] == rp[2] {
                    exact += 1;
                }
            }
            eprintln!(
                "{stem} 帧 {index}: {}x{} fmt={:#x} → {:.2}%",
                tex.width,
                tex.height,
                tex.format,
                exact as f64 * 100.0 / (pw * ph) as f64
            );
            assert_eq!(exact, pw * ph, "{stem} 帧 {index} 未逐字节一致");
            checked += 1;
        }
    }

    eprintln!("共比对 {checked} 帧");
}
