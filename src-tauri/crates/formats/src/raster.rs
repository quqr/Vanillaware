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

    /// Bilinear sample in 8-bit fixed point.
    ///
    /// The weights are `u32` in 1/256 units and the four neighbours are read as
    /// four byte loads each; the two lerps stay in integers so nothing converts
    /// to `f32` and back per channel, which is what made a full-screen fog layer
    /// cost two dozen nanoseconds a pixel.
    #[inline(always)]
    pub fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        let width = self.width;
        let height = self.height;
        if width == 0 || height == 0 {
            return [0, 0, 0, 0];
        }
        let last_x = (width - 1) as f32;
        let last_y = (height - 1) as f32;
        let mut fx = u * width as f32 - 0.5;
        let mut fy = v * height as f32 - 0.5;
        if !(fx >= 0.0) {
            fx = 0.0;
        } else if fx > last_x {
            fx = last_x;
        }
        if !(fy >= 0.0) {
            fy = 0.0;
        } else if fy > last_y {
            fy = last_y;
        }
        let x0 = fx as u32;
        let y0 = fy as u32;
        let tx = ((fx - x0 as f32) * 256.0) as u32;
        let ty = ((fy - y0 as f32) * 256.0) as u32;
        let ix = 256 - tx;
        let iy = 256 - ty;

        let base = ((y0 * width + x0) * 4) as usize;
        let right = if x0 + 1 < width { 4 } else { 0 };
        let down = if y0 + 1 < height {
            width as usize * 4
        } else {
            0
        };
        let rgba = self.rgba;
        let load = |at: usize| -> u32 { u32::from_le_bytes(rgba[at..at + 4].try_into().unwrap()) };
        let p00 = load(base);
        let p10 = load(base + right);
        let p01 = load(base + down);
        let p11 = load(base + right + down);
        let mut out = [0u8; 4];
        for (channel, value) in out.iter_mut().enumerate() {
            let shift = channel * 8;
            let a = (p00 >> shift) & 0xff;
            let b = (p10 >> shift) & 0xff;
            let c = (p01 >> shift) & 0xff;
            let d = (p11 >> shift) & 0xff;
            let top = a * ix + b * tx;
            let bottom = c * ix + d * tx;
            *value = ((top * iy + bottom * ty + 0x8000) >> 16) as u8;
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
        Self::blend_pixel(&mut self.rgba[at..at + 4], color);
    }

    /// Composite one pixel, in integers.
    ///
    /// `out` is the composited alpha and `keep` the share of what was already
    /// there, both scaled by 255; the colour is `(source·a·255 + under·keep) / out`,
    /// which is the same `src·a + dst·u(1−a)` over `a + u(1−a)` the float path
    /// used, only without a reciprocal per pixel.
    #[inline]
    fn blend_pixel(target: &mut [u8], color: [u8; 4]) {
        let alpha = color[3] as u32;
        if alpha == 0 {
            return;
        }
        if alpha == 255 {
            target.copy_from_slice(&color);
            return;
        }
        let keep = target[3] as u32 * (255 - alpha);
        let out = alpha * 255 + keep;
        // One reciprocal for the three channels instead of three divisions.
        let scale = ((1u64 << 32) + out as u64 / 2) / out as u64;
        for (channel, value) in color.iter().take(3).enumerate() {
            let numerator = *value as u32 * alpha * 255 + target[channel] as u32 * keep;
            target[channel] = ((numerator as u64 * scale + (1 << 31)) >> 32) as u8;
        }
        target[3] = ((out + 127) / 255) as u8;
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

/// Fill one textured quad, one triangle at a time.
///
/// The barycentric weights and the texture coordinates are affine in screen
/// space, so their slopes are solved once per triangle and stepped across each
/// row; the horizontal span comes from solving the three edge constraints for
/// `x` instead of testing the corners of the bounding box. The inner loop is
/// then a bilinear sample and a composite, which is what a full-screen fog
/// layer needs.
pub fn paint_quad(
    raster: &mut Raster,
    dst: [(f32, f32); 4],
    uv: [(f32, f32); 4],
    page: &Page<'_>,
    tint: [u8; 4],
) {
    if page.width == 0 || page.height == 0 {
        return;
    }
    const EPS: f32 = -0.001;
    for triangle in [[0usize, 1, 2], [0, 2, 3]] {
        let (p0, p1, p2) = (dst[triangle[0]], dst[triangle[1]], dst[triangle[2]]);
        let det = (p1.0 - p0.0) * (p2.1 - p0.1) - (p2.0 - p0.0) * (p1.1 - p0.1);
        if det.abs() < 1e-6 {
            continue;
        }
        let low_y = p0.1.min(p1.1).min(p2.1).floor().max(0.0);
        let high_y =
            p0.1.max(p1.1)
                .max(p2.1)
                .ceil()
                .clamp(0.0, raster.height as f32);
        if high_y <= low_y {
            continue;
        }

        let det_inv = 1.0 / det;
        // w0 and w1 at the centre of pixel (0, 0) plus their per-pixel slopes.
        let corner = p1.0 * p2.1 - p1.1 * p2.0;
        let second = p2.0 * p0.1 - p2.1 * p0.0;
        let b0_x = (p1.1 - p2.1) * det_inv;
        let b0_y = (p2.0 - p1.0) * det_inv;
        let b1_x = (p2.1 - p0.1) * det_inv;
        let b1_y = (p0.0 - p2.0) * det_inv;
        let a0 = corner * det_inv + 0.5 * (b0_x + b0_y);
        let a1 = second * det_inv + 0.5 * (b1_x + b1_y);

        let (u0, v0) = uv[triangle[0]];
        let (u1, v1) = uv[triangle[1]];
        let (u2, v2) = uv[triangle[2]];
        let du0 = u0 - u2;
        let dv0 = v0 - v2;
        let du1 = u1 - u2;
        let dv1 = v1 - v2;
        let du_dx = b0_x * du0 + b1_x * du1;
        let dv_dx = b0_x * dv0 + b1_x * dv1;
        let alpha = raster.width as u32;

        for py in low_y as u32..high_y as u32 {
            let row = py as f32;
            let w0 = a0 + b0_y * row;
            let w1 = a1 + b1_y * row;
            let mut lo = 0.0f32;
            let mut hi = alpha as f32;
            let mut open = true;
            for (value, slope) in [(w0, b0_x), (w1, b1_x), (1.0 - w0 - w1, -b0_x - b1_x)] {
                if slope > 1e-9 {
                    lo = lo.max((EPS - value) / slope);
                } else if slope < -1e-9 {
                    hi = hi.min((EPS - value) / slope);
                } else if value < EPS {
                    open = false;
                    break;
                }
            }
            if !open {
                continue;
            }
            let start = (lo.ceil().max(0.0) as u32).min(alpha);
            let end = (hi.ceil().max(0.0) as u32).min(alpha);
            if end <= start {
                continue;
            }

            let mut u = u2 + w0 * du0 + w1 * du1 + du_dx * start as f32;
            let mut v = v2 + w0 * dv0 + w1 * dv1 + dv_dx * start as f32;
            let at = ((py * raster.width + start) * 4) as usize;
            let span = (end - start) as usize;
            let mut row_out = &mut raster.rgba[at..at + span * 4];
            for _ in 0..span {
                let texel = page.sample(u, v);
                if texel[3] != 0 {
                    let color = [
                        (texel[0] as u32 * tint[0] as u32 / 255) as u8,
                        (texel[1] as u32 * tint[1] as u32 / 255) as u8,
                        (texel[2] as u32 * tint[2] as u32 / 255) as u8,
                        (texel[3] as u32 * tint[3] as u32 / 255) as u8,
                    ];
                    Raster::blend_pixel(&mut row_out[..4], color);
                }
                row_out = &mut row_out[4..];
                u += du_dx;
                v += dv_dx;
            }
        }
    }
}
