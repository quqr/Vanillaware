pub struct Page<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

impl<'a> Page<'a> {
    pub fn new(width: u32, height: u32, rgba: &'a [u8]) -> Option<Self> {
        if rgba.len() < (width as usize) * (height as usize) * 4 {
            return None;
        }
        Some(Self {
            width,
            height,
            rgba,
        })
    }

    fn texel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }

    pub fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        if self.width == 0 || self.height == 0 {
            return [0, 0, 0, 0];
        }
        let fx = (u * self.width as f32 - 0.5).clamp(0.0, self.width as f32 - 1.0);
        let fy = (v * self.height as f32 - 0.5).clamp(0.0, self.height as f32 - 1.0);
        let x0 = fx.floor() as u32;
        let y0 = fy.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let p00 = self.texel(x0, y0);
        let p10 = self.texel(x1, y0);
        let p01 = self.texel(x0, y1);
        let p11 = self.texel(x1, y1);

        let mut out = [0u8; 4];
        for channel in 0..4 {
            let top = p00[channel] as f32 * (1.0 - tx) + p10[channel] as f32 * tx;
            let bottom = p01[channel] as f32 * (1.0 - tx) + p11[channel] as f32 * tx;
            out[channel] = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
        }
        out
    }
}

#[derive(Clone)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Raster {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0u8; (width as usize) * (height as usize) * 4],
        }
    }

    pub fn filled(width: u32, height: u32, color: [u8; 4]) -> Self {
        let mut raster = Self::new(width, height);
        for pixel in raster.rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        raster
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn blend(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let at = ((y * self.width + x) * 4) as usize;
        let alpha = color[3] as f32 / 255.0;
        if alpha <= 0.0 {
            return;
        }
        if alpha >= 1.0 {
            self.rgba[at..at + 4].copy_from_slice(&color);
            return;
        }
        let under_alpha = self.rgba[at + 3] as f32 / 255.0;
        let out = alpha + under_alpha * (1.0 - alpha);
        if out <= 0.0 {
            return;
        }
        let scale = 1.0 / out;
        let keep = under_alpha * (1.0 - alpha);
        for (channel, value) in color.iter().take(3).enumerate() {
            let under = self.rgba[at + channel] as f32;
            let mixed = *value as f32 * alpha + under * keep;
            self.rgba[at + channel] = (mixed * scale).round().clamp(0.0, 255.0) as u8;
        }
        self.rgba[at + 3] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    /// Composite another raster over this one at `left, top`. Pixels are either
    /// clear (skipped) or opaque (copied), which is what a band of field panels
    /// and props is most of the time, so only edges cost arithmetic.
    pub fn over(&mut self, source: &Raster) {
        self.over_at(source, 0, 0);
    }

    pub fn over_at(&mut self, source: &Raster, left: u32, top: u32) {
        if left >= self.width || top >= self.height {
            return;
        }
        let width = (source.width).min(self.width - left);
        let height = (source.height).min(self.height - top);
        for y in 0..height {
            let start = (((top + y) * self.width + left) * 4) as usize;
            let end = start + width as usize * 4;
            let from = (y * source.width * 4) as usize;
            let row = &source.rgba[from..from + width as usize * 4];
            Self::over_row(&mut self.rgba[start..end], row);
        }
    }

    fn over_row(target: &mut [u8], source: &[u8]) {
        for (target, pixel) in target.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
            match pixel[3] {
                0 => continue,
                255 => target.copy_from_slice(pixel),
                alpha => {
                    let alpha = alpha as f32 / 255.0;
                    let under_alpha = target[3] as f32 / 255.0;
                    let out = alpha + under_alpha * (1.0 - alpha);
                    if out <= 0.0 {
                        continue;
                    }
                    let scale = 1.0 / out;
                    let keep = under_alpha * (1.0 - alpha);
                    for channel in 0..3 {
                        let under = target[channel] as f32;
                        let mixed = pixel[channel] as f32 * alpha + under * keep;
                        target[channel] = (mixed * scale).round().clamp(0.0, 255.0) as u8;
                    }
                    target[3] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }

    pub fn blit(&mut self, source: &Raster, left: u32, top: u32) {
        for y in 0..source.height {
            let target = top + y;
            if target >= self.height {
                break;
            }
            for x in 0..source.width {
                let column = left + x;
                if column >= self.width {
                    break;
                }
                let at = ((y * source.width + x) * 4) as usize;
                let color = [
                    source.rgba[at],
                    source.rgba[at + 1],
                    source.rgba[at + 2],
                    source.rgba[at + 3],
                ];
                self.blend(column, target, color);
            }
        }
    }

    pub fn downscale(&self, width: u32, height: u32) -> Raster {
        if self.is_empty() || width == 0 || height == 0 {
            return Raster::new(0, 0);
        }
        let mut out = Raster::new(width, height);
        for y in 0..height {
            let sy0 = (y as u64 * self.height as u64 / height as u64) as u32;
            let sy1 = (((y as u64 + 1) * self.height as u64 / height as u64) as u32)
                .max(sy0 + 1)
                .min(self.height);
            for x in 0..width {
                let sx0 = (x as u64 * self.width as u64 / width as u64) as u32;
                let sx1 = (((x as u64 + 1) * self.width as u64 / width as u64) as u32)
                    .max(sx0 + 1)
                    .min(self.width);
                let mut sum = [0u64; 4];
                let mut count = 0u64;
                for sy in sy0..sy1 {
                    for sx in sx0..sx1 {
                        let at = ((sy * self.width + sx) * 4) as usize;
                        for (channel, slot) in sum.iter_mut().enumerate() {
                            *slot += self.rgba[at + channel] as u64;
                        }
                        count += 1;
                    }
                }
                if count == 0 {
                    continue;
                }
                let at = ((y * width + x) * 4) as usize;
                for (channel, slot) in sum.iter().enumerate() {
                    out.rgba[at + channel] = (*slot / count) as u8;
                }
            }
        }
        out
    }
}

pub fn paint_quad(
    raster: &mut Raster,
    dst: [(f32, f32); 4],
    uv: [(f32, f32); 4],
    page: &Page<'_>,
    tint: [u8; 4],
) {
    for triangle in [[0usize, 1, 2], [0, 2, 3]] {
        let (p0, p1, p2) = (dst[triangle[0]], dst[triangle[1]], dst[triangle[2]]);
        let area = (p1.0 - p0.0) * (p2.1 - p0.1) - (p1.1 - p0.1) * (p2.0 - p0.0);
        if area.abs() < 1e-6 {
            continue;
        }

        let low_x = p0.0.min(p1.0).min(p2.0).floor().max(0.0);
        let low_y = p0.1.min(p1.1).min(p2.1).floor().max(0.0);
        let high_x =
            p0.0.max(p1.0)
                .max(p2.0)
                .ceil()
                .clamp(0.0, raster.width as f32);
        let high_y =
            p0.1.max(p1.1)
                .max(p2.1)
                .ceil()
                .clamp(0.0, raster.height as f32);
        if high_x <= low_x || high_y <= low_y {
            continue;
        }

        for py in low_y as u32..high_y as u32 {
            for px in low_x as u32..high_x as u32 {
                let point = (px as f32 + 0.5, py as f32 + 0.5);
                let w0 = ((p1.0 - point.0) * (p2.1 - point.1)
                    - (p1.1 - point.1) * (p2.0 - point.0))
                    / area;
                let w1 = ((p2.0 - point.0) * (p0.1 - point.1)
                    - (p2.1 - point.1) * (p0.0 - point.0))
                    / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                    continue;
                }

                let u = w0 * uv[triangle[0]].0 + w1 * uv[triangle[1]].0 + w2 * uv[triangle[2]].0;
                let v = w0 * uv[triangle[0]].1 + w1 * uv[triangle[1]].1 + w2 * uv[triangle[2]].1;
                let texel = page.sample(u, v);
                if texel[3] == 0 {
                    continue;
                }
                let color = [
                    (texel[0] as u32 * tint[0] as u32 / 255) as u8,
                    (texel[1] as u32 * tint[1] as u32 / 255) as u8,
                    (texel[2] as u32 * tint[2] as u32 / 255) as u8,
                    (texel[3] as u32 * tint[3] as u32 / 255) as u8,
                ];
                raster.blend(px, py, color);
            }
        }
    }
}
