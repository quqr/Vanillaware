use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};

use formats::DecodedTexture;
use formats::Error;
use formats::Result;

pub const THUMBNAIL_MAX_EDGE: u32 = 128;

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
