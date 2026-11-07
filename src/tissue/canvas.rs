//! A slide being stained: three density maps, haematoxylin, eosin and
//! melanin, drawn into shape by shape, and turned into the colour a
//! brightfield microscope shows by the Beer-Lambert law, with the stain
//! vectors Ruifrok and Johnston measured.

use bevy::math::Vec2;

/// Optical density per unit of each stain in red, green and blue: how much
/// of each colour a unit of stain takes out of the light. Haematoxylin and
/// eosin from Ruifrok and Johnston (Anal Quant Cytol Histol, 2001); melanin
/// a brown that takes blue most.
const HAEMATOXYLIN: [f32; 3] = [0.650, 0.704, 0.286];
const EOSIN: [f32; 3] = [0.072, 0.990, 0.105];
const MELANIN: [f32; 3] = [0.28, 0.48, 0.76];

/// Which stain a shape adds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stain {
    Haematoxylin,
    Eosin,
    Melanin,
}

/// The slide.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    layers: [Vec<f32>; 3],
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            layers: [
                vec![0.0; width * height],
                vec![0.0; width * height],
                vec![0.0; width * height],
            ],
        }
    }

    fn layer(&mut self, stain: Stain) -> &mut Vec<f32> {
        &mut self.layers[stain as usize]
    }

    /// Set the stain at every pixel from a function of the pixel.
    pub fn fill(&mut self, stain: Stain, f: impl Fn(usize, usize) -> f32 + Sync) {
        use rayon::prelude::*;
        let w = self.width;
        self.layer(stain)
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, v) in row.iter_mut().enumerate() {
                    *v += f(x, y);
                }
            });
    }

    /// An ellipse of stain with a soft edge, its density shaped by
    /// `texture` at each point, a function of the point relative to the
    /// centre.
    #[allow(clippy::too_many_arguments)]
    pub fn blob(
        &mut self,
        stain: Stain,
        centre: Vec2,
        radii: Vec2,
        angle: f32,
        density: f32,
        texture: &dyn Fn(Vec2) -> f32,
    ) {
        let reach = radii.max_element() + 1.5;
        let (x0, x1) = (
            (centre.x - reach).floor().max(0.0) as usize,
            (centre.x + reach).ceil().min(self.width as f32 - 1.0) as usize,
        );
        let (y0, y1) = (
            (centre.y - reach).floor().max(0.0) as usize,
            (centre.y + reach).ceil().min(self.height as f32 - 1.0) as usize,
        );
        if x0 > x1 || y0 > y1 {
            return;
        }
        let (s, c) = angle.sin_cos();
        let w = self.width;
        let layer = self.layer(stain);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = Vec2::new(x as f32, y as f32) - centre;
                let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
                let r = ((local.x / radii.x).powi(2) + (local.y / radii.y).powi(2)).sqrt();
                // About a pixel of soft edge, whatever the size.
                let edge = 1.0 / radii.min_element().max(1.0);
                let cover = ((1.0 - r) / edge).clamp(0.0, 1.0);
                if cover > 0.0 {
                    layer[y * w + x] += density * cover * texture(local);
                }
            }
        }
    }

    /// A stroke of stain along a path of points, `width` pixels across,
    /// as even where its segments meet as along them.
    pub fn stroke(&mut self, stain: Stain, path: &[Vec2], width: f32, density: f32) {
        if path.len() < 2 {
            return;
        }
        let half = width / 2.0;
        let (mut lo, mut hi) = (path[0], path[0]);
        for p in path {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let x0 = (lo.x - half - 1.0).floor().max(0.0) as usize;
        let y0 = (lo.y - half - 1.0).floor().max(0.0) as usize;
        let x1 = ((hi.x + half + 1.0).ceil() as usize).min(self.width.saturating_sub(1));
        let y1 = ((hi.y + half + 1.0).ceil() as usize).min(self.height.saturating_sub(1));
        if x0 > x1 || y0 > y1 {
            return;
        }
        let bw = x1 - x0 + 1;
        let mut cover = vec![0.0f32; bw * (y1 - y0 + 1)];
        for pair in path.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let ab = b - a;
            let length2 = ab.length_squared().max(1e-6);
            let sx0 = (a.x.min(b.x) - half - 1.0).floor().max(x0 as f32) as usize;
            let sx1 = ((a.x.max(b.x) + half + 1.0).ceil() as usize).min(x1);
            let sy0 = (a.y.min(b.y) - half - 1.0).floor().max(y0 as f32) as usize;
            let sy1 = ((a.y.max(b.y) + half + 1.0).ceil() as usize).min(y1);
            for y in sy0..=sy1 {
                for x in sx0..=sx1 {
                    let p = Vec2::new(x as f32, y as f32);
                    let t = ((p - a).dot(ab) / length2).clamp(0.0, 1.0);
                    let d = p.distance(a + ab * t);
                    let c = (half - d + 0.5).clamp(0.0, 1.0);
                    let cell = &mut cover[(y - y0) * bw + (x - x0)];
                    *cell = cell.max(c);
                }
            }
        }
        let w = self.width;
        let layer = self.layer(stain);
        for (j, row) in cover.chunks(bw).enumerate() {
            for (i, c) in row.iter().enumerate() {
                if *c > 0.0 {
                    layer[(y0 + j) * w + x0 + i] += density * c;
                }
            }
        }
    }

    /// The slide as a brightfield microscope shows it: each stain's density
    /// blurred a little by the lens, taken out of white light by the
    /// Beer-Lambert law, a little darker towards the edges of the field.
    pub fn develop(&self) -> Vec<u8> {
        use rayon::prelude::*;
        let soft: Vec<Vec<f32>> = self
            .layers
            .iter()
            .map(|l| blur(l, self.width, self.height))
            .collect();
        let (w, h) = (self.width, self.height);
        let mut out = vec![0u8; w * h * 4];
        out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
            for x in 0..w {
                let i = y * w + x;
                let (hx, ex, mx) = (soft[0][i], soft[1][i], soft[2][i]);
                let off = Vec2::new(x as f32 / w as f32 - 0.5, y as f32 / h as f32 - 0.5).length();
                let lamp = 1.0 - 0.12 * off * off;
                let px = &mut row[x * 4..x * 4 + 4];
                for k in 0..3 {
                    let density = hx * HAEMATOXYLIN[k] + ex * EOSIN[k] + mx * MELANIN[k];
                    let light = lamp * (-1.25 * density).exp();
                    px[k] = (srgb(light.clamp(0.0, 1.0)) * 255.0) as u8;
                }
                px[3] = 255;
            }
        });
        out
    }
}

/// A light blur, what the lens leaves of a sharp edge.
fn blur(layer: &[f32], w: usize, h: usize) -> Vec<f32> {
    let k = [0.25f32, 0.5, 0.25];
    let mut rows = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (j, kv) in k.iter().enumerate() {
                let xx = (x + j).saturating_sub(1).min(w - 1);
                s += kv * layer[y * w + xx];
            }
            rows[y * w + x] = s;
        }
    }
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (j, kv) in k.iter().enumerate() {
                let yy = (y + j).saturating_sub(1).min(h - 1);
                s += kv * rows[yy * w + x];
            }
            out[y * w + x] = s;
        }
    }
    out
}

/// A linear value as sRGB.
pub fn srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// A hash of two integers and a seed to a uniform value in [0, 1).
pub fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth noise in [0, 1).
pub fn noise(p: Vec2, seed: u32) -> f32 {
    let (x0, y0) = (p.x.floor(), p.y.floor());
    let (fx, fy) = (p.x - x0, p.y - y0);
    let (fx, fy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let at = |dx: i32, dy: i32| hash(x0 as i32 + dx, y0 as i32 + dy, seed);
    let top = at(0, 0) * (1.0 - fx) + at(1, 0) * fx;
    let bottom = at(0, 1) * (1.0 - fx) + at(1, 1) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Noise over octaves.
pub fn fbm(p: Vec2, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amplitude, mut scale, mut total) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amplitude * noise(p * scale, seed.wrapping_add(o * 977));
        total += amplitude;
        amplitude *= 0.5;
        scale *= 2.0;
    }
    sum / total
}

/// A slide's own random draws, from its seed.
pub struct Random(u32);

impl Random {
    pub fn new(seed: u32) -> Self {
        Self(seed.wrapping_mul(2_654_435_761).max(1))
    }

    pub fn between(&mut self, low: f32, high: f32) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        low + (high - low) * ((self.0 >> 8) as f32 / (1u32 << 24) as f32)
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.between(0.0, 1.0) < p
    }
}
