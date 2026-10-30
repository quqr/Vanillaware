use std::io::Cursor;

use serde::Serialize;

use formats::content::{self, Projection};

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

pub fn open(bytes: &[u8], ext: &str) -> Option<ContentDto> {
    let projection = content::project(bytes, ext)?;
    Some(render(projection))
}

fn render(projection: Projection) -> ContentDto {
    let color = projection.color.as_ref();
    let (image, projection_legend) = match color {
        Some(map) => (render_color_png(map), Some(map.legend.clone())),
        None => (
            projection.bitmap.as_ref().and_then(render_gray_png),
            projection
                .bitmap
                .as_ref()
                .map(|bitmap| bitmap.legend.clone()),
        ),
    };
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

fn render_color_png(map: &formats::content::ColorMap) -> Option<String> {
    if map.width == 0 || map.height == 0 {
        return None;
    }
    if map.rgba.len() != (map.width as usize) * (map.height as usize) * 4 {
        return None;
    }
    let image = image::RgbaImage::from_raw(map.width, map.height, map.rgba.clone())?;
    let mut buffer = Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64(&buffer.into_inner())
    ))
}

fn render_gray_png(bitmap: &formats::content::GrayMap) -> Option<String> {
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

pub fn base64(bytes: &[u8]) -> String {
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
