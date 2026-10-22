//! 缩略图编码：RGBA8（或单通道灰度）→ 最长边 ≤256 的 PNG。

use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};

use vw_formats::DecodedTexture;
use vw_formats::Error;
use vw_formats::Result;

/// 缩略图最长边。列表图标 20px、网格卡片 112px，128 足够。
pub const THUMBNAIL_MAX_EDGE: u32 = 128;

/// 把解码结果渲染成**原始尺寸**的 PNG（查看器用，绝不能压）。
///
/// 图集是 UV 采样目标，缩放会同时损失精度与颜色，必须保持原尺寸。
pub fn render_full_png(texture: &DecodedTexture) -> Result<Vec<u8>> {
    let buffer = if texture.channels == 4 {
        texture.data.clone()
    } else {
        texture.rgba()
    };

    let image_buffer =
        RgbaImage::from_raw(texture.width, texture.height, buffer).ok_or_else(|| {
            Error::BadData(format!(
                "pixel buffer size mismatch: expected {}x{}x4",
                texture.width, texture.height
            ))
        })?;

    let mut out = Vec::new();
    DynamicImage::ImageRgba8(image_buffer)
        .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| Error::BadData(format!("PNG encode failed: {e}")))?;

    Ok(out)
}

/// 把解码结果渲染成缩略图 PNG（最长边不超过 [`THUMBNAIL_MAX_EDGE`]，小图不放大）。
///
/// BC4（单通道）在这里通过 [`DecodedTexture::rgba`] 统一扩成 RGBA8，
/// 编码器只面对一种颜色布局。
pub fn render_thumb_png(texture: &DecodedTexture) -> Result<Vec<u8>> {
    let width = texture.width;
    let height = texture.height;

    let buffer = if texture.channels == 4 {
        texture.data.clone()
    } else {
        texture.rgba()
    };

    let image_buffer = RgbaImage::from_raw(width, height, buffer).ok_or_else(|| {
        Error::BadData(format!(
            "pixel buffer size mismatch: expected {}x{}x4",
            width, height
        ))
    })?;

    // 用 box 采样（thumbnail）而不是 Triangle：缩略图只看轮廓，box 快一个量级。
    // thumbnail 不做等比约束，先按最长边算出目标尺寸。
    let thumb = if width <= THUMBNAIL_MAX_EDGE && height <= THUMBNAIL_MAX_EDGE {
        DynamicImage::ImageRgba8(image_buffer)
    } else {
        let scale = THUMBNAIL_MAX_EDGE as f32 / width.max(height) as f32;
        let target_w = ((width as f32 * scale).round() as u32).max(1);
        let target_h = ((height as f32 * scale).round() as u32).max(1);
        DynamicImage::ImageRgba8(image::imageops::thumbnail(
            &image_buffer,
            target_w,
            target_h,
        ))
    };

    // 缩略图是滚动时的高频产物：用快速压缩档换编码速度，体积变大无所谓（本地缓存）。
    let mut cursor = std::io::Cursor::new(Vec::new());
    let encoder =
        PngEncoder::new_with_quality(&mut cursor, CompressionType::Fast, PngFilterType::NoFilter);
    encoder
        .write_image(
            thumb.as_bytes(),
            thumb.width(),
            thumb.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| Error::BadData(format!("PNG encode failed: {e}")))?;

    Ok(cursor.into_inner())
}
