//! 预览场景的结构检查：模型 → 视图的搬运必须保持引用闭合。
//!
//! 只用一份真实 `.mbs`（`VW_MBS_SAMPLE`），核对：
//! - 每个时间线帧的 `attach` 都能落到一个**实填**的 keyframe / hitbox 上；
//! - 图层数组里非空项的 `texId` 落在图集帧数范围内；
//! - `dst` / `src` 恒为 8 个分量。
//!
//! ```text
//! $env:VW_MBS_SAMPLE="F:\Codes\UNICORN OVERLORD\extract\000\Chara\Fighter_M.mbs"
//! cargo test -p vanillaware --test scene_build -- --nocapture
//! ```

use std::path::PathBuf;

use vanillaware_lib::service::scene;

#[test]
fn scene_references_are_closed() {
    let Ok(sample) = std::env::var("VW_MBS_SAMPLE") else {
        eprintln!("skip: 未设置 VW_MBS_SAMPLE");
        return;
    };
    let path = PathBuf::from(sample);
    if !path.is_file() {
        eprintln!("skip: {} 不存在", path.display());
        return;
    }

    let loaded = scene::load(&path, "swi_unic").expect("解析并转换 mbs");
    let dto = loaded.scene(&path, "swi_unic");

    assert!(!dto.animations.is_empty(), "至少应当有一个动画");
    assert!(!dto.keyframe.is_empty(), "至少应当有一帧判定框");

    let mut frames = 0usize;
    let mut layers = 0usize;
    for animation in &dto.animations {
        for frame in &animation.frames {
            frames += 1;
            let Some(attach) = &frame.attach else {
                continue;
            };
            let filled = match attach.kind.as_str() {
                "keyframe" | "slot" => dto
                    .keyframe
                    .get(attach.id as usize)
                    .is_some_and(Option::is_some),
                "hitbox" => dto
                    .hitbox
                    .get(attach.id as usize)
                    .is_some_and(Option::is_some),
                other => panic!("未知的 attach 类型：{other}"),
            };
            assert!(filled, "{} 指向不存在的帧", attach.kind);
        }
        assert!(
            animation.loop_id >= -1,
            "loop_id 不能小于 -1：{}",
            animation.loop_id
        );
    }

    for keyframe in dto.keyframe.iter().flatten() {
        for layer in keyframe.layers.iter().flatten() {
            layers += 1;
            assert_eq!(layer.dst.len(), 8, "dstquad 必须是 8 个分量");
            if let Some(src) = &layer.src {
                assert_eq!(src.len(), 8, "srcquad 必须是 8 个分量");
            }
            if let Some(tex_id) = layer.tex_id {
                assert!(
                    tex_id >= 0 && (tex_id as usize) < dto.atlas.len(),
                    "tex_id {tex_id} 超出图集 {}",
                    dto.atlas.len()
                );
            }
        }
    }

    eprintln!(
        "场景：{} 个动画 / {frames} 帧 / {layers} 图层 / 图集 {} 帧",
        dto.animations.len(),
        dto.atlas.len()
    );
    assert!(frames > 0 && layers > 0, "动画或图层为空");
}
