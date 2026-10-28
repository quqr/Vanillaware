//! 内容视图渲染：把 `vw-formats` 的投影转成前端 DTO。
//!
//! 位图在这里编成 PNG 并以 data URL 内联，免得再开一条二进制命令；
//! 灰度位图压缩后通常只有几 KB。

use std::io::Cursor;

use serde::Serialize;

use vw_formats::content::{self, Projection};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDto {
    pub title: String,
    pub note: Option<String>,
    pub sections: Vec<SectionDto>,
    pub series: Option<SeriesDto>,
    pub text: Option<Vec<String>>,
    /// `data:image/png;base64,...`
    pub image: Option<String>,
    /// 位图的坐标口径（横轴/纵轴/灰度含义）。
    pub image_note: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionDto {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDto {
    pub label: String,
    pub points: Vec<f32>,
    pub note: Option<String>,
}

/// 投影并渲染；该扩展名没有内容视图时返回 `None`。
pub fn open(bytes: &[u8], ext: &str) -> Option<ContentDto> {
    let projection = content::project(bytes, ext)?;
    Some(render(projection))
}

fn render(projection: Projection) -> ContentDto {
    let image = projection.bitmap.as_ref().and_then(render_gray_png);
    let projection_legend = projection
        .bitmap
        .as_ref()
        .map(|bitmap| bitmap.legend.clone());
    ContentDto {
        title: projection.title,
        note: projection.note,
        sections: projection
            .sections
            .into_iter()
            .map(|section| SectionDto {
                title: section.title,
                columns: section.columns,
                rows: section.rows,
                total: section.total,
            })
            .collect(),
        series: projection.series.map(|series| SeriesDto {
            label: series.label,
            points: series.points,
            note: series.note,
        }),
        text: projection.text,
        image,
        image_note: projection_legend,
    }
}

fn render_gray_png(bitmap: &vw_formats::content::GrayMap) -> Option<String> {
    if bitmap.width == 0 || bitmap.height == 0 {
        return None;
    }
    if bitmap.values.len() != (bitmap.width as usize) * (bitmap.height as usize) {
        return None;
    }
    let image = image::GrayImage::from_raw(bitmap.width, bitmap.height, bitmap.values.clone())?;
    let mut buffer = Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64(&buffer.into_inner())
    ))
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0] as u32;
        let second = *chunk.get(1).unwrap_or(&0) as u32;
        let third = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (first << 16) | (second << 8) | third;
        out.push(TABLE[(triple >> 18) as usize & 0x3f] as char);
        out.push(TABLE[(triple >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
    }
}
