//! A cell as a fluorescence microscope shows it: dark, with each stained
//! protein glowing where it sits in the cell.
//!
//! The cell's architecture is drawn once, from a seed: its outline, its
//! nucleus with chromatin and nucleoli, its mitochondria, its Golgi
//! apparatus and its vesicles, each a layer of where the cell holds that
//! compartment. What is not drawn is how bright each protein is: a protein
//! is a mix of the layers it lives in, as bright as the network running in
//! the dish says it is. The optics are a microscope's: every layer blurred by
//! the point spread function of the lens, a faint haze of out-of-focus
//! light, and the grain of a camera counting photons.

use std::f32::consts::{PI, TAU};

use bevy::math::Vec2;

use super::dish::Fate;

/// The image's side, in pixels.
pub const SIZE: usize = 720;

/// Micrometres per pixel: the cell is about 35 µm across, its nucleus 13,
/// as a cultured cell spread on glass is.
pub const MICROMETRES_PER_PIXEL: f32 = 0.0715;

/// Where in the cell something is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compartment {
    /// Not the cell.
    Outside,
    /// The membrane around the cell.
    Membrane,
    /// The fluid between the membrane and the nucleus.
    Cytosol,
    /// The nucleus, where the DNA is.
    Nucleus,
    /// The nucleolus, inside the nucleus, where ribosomes are made.
    Nucleolus,
    /// A mitochondrion.
    Mitochondrion,
    /// The Golgi apparatus, beside the nucleus.
    Golgi,
    /// A vesicle carrying something to the surface or from it.
    Vesicle,
}

impl Compartment {
    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Compartment::Outside => "outside the cell",
            Compartment::Membrane => "the cell membrane",
            Compartment::Cytosol => "the cytosol",
            Compartment::Nucleus => "the nucleus",
            Compartment::Nucleolus => "a nucleolus",
            Compartment::Mitochondrion => "a mitochondrion",
            Compartment::Golgi => "the Golgi apparatus",
            Compartment::Vesicle => "a vesicle",
        }
    }

    /// What it is, in a sentence.
    pub fn what(self) -> &'static str {
        match self {
            Compartment::Outside => "The glass the cell is spread on, and the medium above it.",
            Compartment::Membrane => {
                "A double layer of lipids, 5 nanometres thick, around the whole cell. Receptors such as EGFR cross it; RAS hangs on its inner face."
            }
            Compartment::Cytosol => {
                "The crowded fluid that fills the cell around the organelles, where RAF, MEK and the caspases work."
            }
            Compartment::Nucleus => {
                "Where the DNA is, inside a double membrane. Transcription factors such as p53, HIF-1α and E2F work here, switching genes on."
            }
            Compartment::Nucleolus => {
                "A dense region of the nucleus where ribosomes are assembled. It holds little DNA, so the DNA stain is dark there."
            }
            Compartment::Mitochondrion => {
                "Where the cell burns its fuel with oxygen to make ATP. BCL-2 sits in its outer membrane; activated BAX opens pores there, and cytochrome c leaks out to start apoptosis."
            }
            Compartment::Golgi => {
                "Stacks of membrane beside the nucleus that sort and package proteins made for export, such as VEGF, into vesicles."
            }
            Compartment::Vesicle => {
                "A small membrane bubble, carrying cargo to the surface, such as VEGF to be secreted, or in from it, such as receptors taken back."
            }
        }
    }
}

/// The layers a protein can be in, in the order the specimen holds them.
const DNA: usize = 0;
const NUCLEOPLASM: usize = 1;
const CYTOSOL: usize = 2;
const MEMBRANE: usize = 3;
const MITOCHONDRIA: usize = 4;
const GOLGI: usize = 5;
const VESICLES: usize = 6;
const FRAGMENTS: usize = 7;
const LAYERS: usize = 8;

/// One channel of the image: what is stained, how bright it is, and the
/// colour of its dye.
#[derive(Debug, Clone, Copy)]
pub struct Channel {
    /// How much of each layer the stained thing is in.
    pub layers: [f32; LAYERS],
    /// How much of it there is: its level in the network, or one for a
    /// stain of the structure itself.
    pub amount: f32,
    /// The dye's colour, linear RGB.
    pub colour: [f32; 3],
    /// The camera's gain for this channel.
    pub gain: f32,
}

/// The dyes the microscope's channels are read in.
pub const BLUE: [f32; 3] = [0.20, 0.42, 1.0];
pub const GREEN: [f32; 3] = [0.15, 1.0, 0.25];
pub const RED: [f32; 3] = [1.0, 0.12, 0.08];
pub const MAGENTA: [f32; 3] = [0.95, 0.15, 1.0];

/// The cell's architecture, drawn once.
pub struct Specimen {
    layers: Vec<Vec<f32>>,
    centre: Vec2,
    radius: f32,
    /// The outline's harmonics: amplitude and phase of each.
    harmonics: Vec<(f32, f32)>,
    /// The nucleus: centre, half axes, angle.
    nucleus: (Vec2, Vec2, f32),
    /// The nucleoli: centre and radius.
    nucleoli: Vec<(Vec2, f32)>,
}

impl Specimen {
    /// Draw a cell from a seed.
    pub fn new(seed: u32) -> Self {
        let s = SIZE as f32;
        let mut random = Random(seed.wrapping_mul(2_654_435_761).max(1));
        let centre = Vec2::splat(s / 2.0);
        let radius = 0.34 * s;
        let harmonics: Vec<(f32, f32)> = (2..8)
            .map(|k| {
                (
                    0.14 / k as f32 * random.between(0.6, 1.2),
                    random.between(0.0, TAU),
                )
            })
            .collect();
        let nucleus = (
            centre + Vec2::new(0.10 * radius, -0.06 * radius),
            Vec2::new(0.36 * radius, 0.28 * radius),
            random.between(0.0, PI),
        );
        let nucleoli: Vec<(Vec2, f32)> = (0..3)
            .map(|_| {
                let a = random.between(0.0, TAU);
                let d = random.between(0.1, 0.55);
                let offset = Vec2::new(a.cos() * nucleus.1.x * d, a.sin() * nucleus.1.y * d);
                (
                    nucleus.0 + rotate(offset, nucleus.2),
                    random.between(0.028, 0.045) * radius,
                )
            })
            .collect();
        let mut specimen = Self {
            layers: vec![vec![0.0; SIZE * SIZE]; LAYERS],
            centre,
            radius,
            harmonics,
            nucleus,
            nucleoli,
        };
        specimen.draw(&mut random, seed);
        for layer in &mut specimen.layers {
            *layer = optics(layer);
        }
        specimen
    }

    /// The outline's radius in direction `angle`.
    fn rim(&self, angle: f32) -> f32 {
        let wobble: f32 = self
            .harmonics
            .iter()
            .enumerate()
            .map(|(i, (a, phase))| a * ((i as f32 + 2.0) * angle + phase).cos())
            .sum();
        self.radius * (1.0 + wobble)
    }

    /// How far `p` is inside the nucleus: below one inside, one on its
    /// envelope.
    fn in_nucleus(&self, p: Vec2) -> f32 {
        let (c, axes, angle) = self.nucleus;
        let q = rotate(p - c, -angle);
        ((q.x / axes.x).powi(2) + (q.y / axes.y).powi(2)).sqrt()
    }

    /// How deep `p` is in the cell: one at its centre, zero on its outline,
    /// negative outside; and the distance to the outline in pixels.
    fn depth(&self, p: Vec2) -> (f32, f32) {
        let d = p - self.centre;
        let rim = self.rim(d.y.atan2(d.x));
        let r = d.length();
        (1.0 - r / rim, rim - r)
    }

    /// Draw every layer, before the optics.
    fn draw(&mut self, random: &mut Random, seed: u32) {
        let s = seed;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let p = Vec2::new(x as f32, y as f32);
                let i = y * SIZE + x;
                let (depth, to_rim) = self.depth(p);
                if depth <= -0.02 {
                    continue;
                }
                let inside = smooth(-1.0, 1.0, to_rim);
                let e = self.in_nucleus(p);
                let in_nucleus = 1.0 - smooth(0.96, 1.02, e);
                let nucleolus = self
                    .nucleoli
                    .iter()
                    .map(|(c, r)| 1.0 - smooth(0.8 * r, 1.1 * r, p.distance(*c)))
                    .fold(0.0, f32::max);
                // Chromatin: grainy, denser against the envelope, thin in
                // the nucleoli.
                let grain = fbm(p * 0.11, s, 4);
                let clumps = fbm(p * 0.035, s + 3, 3);
                let chromatin =
                    (0.15 + 1.4 * (grain - 0.35).max(0.0) + 0.6 * (clumps - 0.4).max(0.0)).min(1.2);
                let envelope = (-((1.0 - e) / 0.07).powi(2)).exp();
                self.layers[DNA][i] = in_nucleus
                    * (0.25 + 0.75 * chromatin + 0.35 * envelope)
                    * (1.0 - 0.75 * nucleolus);
                // Proteins of the nucleoplasm: fairly even, finely speckled,
                // fewer in the nucleoli.
                let speckle = 0.75 + 0.25 * fbm(p * 0.21, s + 7, 2);
                self.layers[NUCLEOPLASM][i] = in_nucleus * speckle * (1.0 - 0.6 * nucleolus);
                // The cytosol: thicker, so brighter, towards the middle of
                // the cell, thin at its edges.
                let thickness = depth.max(0.0).sqrt();
                let texture = 0.8 + 0.2 * fbm(p * 0.03, s + 11, 3);
                self.layers[CYTOSOL][i] =
                    inside * (1.0 - in_nucleus) * (0.25 + 0.75 * thickness) * texture;
                // The membrane: bright where it is seen edge on, at the
                // outline, faint over the top of the cell.
                let edge = (-(to_rim / 1.8).powi(2)).exp();
                let patchy = 0.7 + 0.3 * fbm(p * 0.15, s + 13, 2);
                self.layers[MEMBRANE][i] = (edge + 0.12 * inside) * patchy;
            }
        }
        self.draw_mitochondria(random);
        self.draw_golgi(random);
        self.draw_vesicles(random);
        self.draw_fragments(random);
    }

    /// A point in the cytosol, nearer the nucleus more often than not.
    fn cytosolic_point(&self, random: &mut Random) -> Vec2 {
        for _ in 0..200 {
            let a = random.between(0.0, TAU);
            let r = self.rim(a) * random.between(0.15, 0.92).powf(0.8);
            let p = self.centre + Vec2::new(a.cos(), a.sin()) * r;
            if self.in_nucleus(p) > 1.08 {
                return p;
            }
        }
        self.centre
    }

    /// Mitochondria: tubes a few micrometres long, wandering through the
    /// cytosol, thickest around the nucleus.
    fn draw_mitochondria(&mut self, random: &mut Random) {
        for _ in 0..300 {
            let mut p = self.cytosolic_point(random);
            let mut heading = random.between(0.0, TAU);
            let length = random.between(10.0, 42.0) as usize;
            let brightness = random.between(0.6, 1.0);
            for _ in 0..length {
                heading += random.between(-0.35, 0.35);
                let next = p + Vec2::new(heading.cos(), heading.sin()) * 1.4;
                if self.depth(next).0 < 0.06 || self.in_nucleus(next) < 1.05 {
                    heading += 2.0;
                    continue;
                }
                p = next;
                splat(&mut self.layers[MITOCHONDRIA], p, 1.5, brightness);
            }
        }
        cap(&mut self.layers[MITOCHONDRIA], 1.3);
    }

    /// The Golgi apparatus: curved stacks against one side of the nucleus.
    fn draw_golgi(&mut self, random: &mut Random) {
        let (c, axes, angle) = self.nucleus;
        let side = (self.centre - c).y.atan2((self.centre - c).x) + PI;
        for _ in 0..34 {
            let a = side + random.between(-0.5, 0.5);
            let out = random.between(1.12, 1.45);
            let start = c + rotate(Vec2::new(a.cos() * axes.x, a.sin() * axes.y) * out, angle);
            let along = Vec2::new(-a.sin(), a.cos());
            let length = random.between(6.0, 16.0);
            let mut t = -length / 2.0;
            while t < length / 2.0 {
                let bend = 0.04 * t * t;
                let p = start + rotate(along * t, angle) + (start - c).normalize_or_zero() * bend;
                splat(&mut self.layers[GOLGI], p, 1.4, 0.8);
                t += 1.0;
            }
        }
        cap(&mut self.layers[GOLGI], 1.3);
    }

    /// Vesicles: small bright points through the cytosol.
    fn draw_vesicles(&mut self, random: &mut Random) {
        for _ in 0..260 {
            let p = self.cytosolic_point(random);
            splat(&mut self.layers[VESICLES], p, 1.3, random.between(0.5, 1.0));
        }
        cap(&mut self.layers[VESICLES], 1.2);
    }

    /// The nucleus of an apoptotic cell, condensed and broken into round
    /// pieces, what a section shows as karyorrhexis.
    fn draw_fragments(&mut self, random: &mut Random) {
        let (c, axes, angle) = self.nucleus;
        for _ in 0..6 {
            let a = random.between(0.0, TAU);
            let d = random.between(0.0, 0.6);
            let p = c + rotate(Vec2::new(a.cos() * axes.x, a.sin() * axes.y) * d, angle);
            let r = random.between(0.05, 0.09) * self.radius;
            let (x0, x1) = (
                (p.x - r * 1.5).max(0.0) as usize,
                (p.x + r * 1.5).min(SIZE as f32 - 1.0) as usize,
            );
            let (y0, y1) = (
                (p.y - r * 1.5).max(0.0) as usize,
                (p.y + r * 1.5).min(SIZE as f32 - 1.0) as usize,
            );
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let d = Vec2::new(x as f32, y as f32).distance(p);
                    let v = 1.4 * (1.0 - smooth(0.85 * r, r, d));
                    let cell = &mut self.layers[FRAGMENTS][y * SIZE + x];
                    *cell = cell.max(v);
                }
            }
        }
    }

    /// Where pixel `(x, y)` of the image is in the cell, as it is when
    /// alive.
    pub fn compartment(&self, x: f32, y: f32) -> Compartment {
        let p = Vec2::new(x, y);
        let (_, to_rim) = self.depth(p);
        if to_rim < -2.5 {
            return Compartment::Outside;
        }
        if to_rim < 4.0 {
            return Compartment::Membrane;
        }
        if self.in_nucleus(p) < 1.0 {
            let in_nucleolus = self.nucleoli.iter().any(|(c, r)| p.distance(*c) < *r);
            return if in_nucleolus {
                Compartment::Nucleolus
            } else {
                Compartment::Nucleus
            };
        }
        let at = |layer: usize| {
            let (xi, yi) = (x as usize, y as usize);
            if xi < SIZE && yi < SIZE {
                self.layers[layer][yi * SIZE + xi]
            } else {
                0.0
            }
        };
        if at(GOLGI) > 0.45 {
            Compartment::Golgi
        } else if at(MITOCHONDRIA) > 0.5 {
            Compartment::Mitochondrion
        } else if at(VESICLES) > 0.45 {
            Compartment::Vesicle
        } else {
            Compartment::Cytosol
        }
    }

    /// Paint the image the camera records: every channel, through the
    /// cell's fate, with this frame's grain, into an sRGB RGBA buffer.
    pub fn expose(&self, channels: &[Channel], fate: Fate, frame: u32, out: &mut [u8]) {
        use rayon::prelude::*;
        let warp = Warp::of(fate, self.centre);
        out.par_chunks_mut(SIZE * 4)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..SIZE {
                    let p = Vec2::new(x as f32, y as f32);
                    let (q, fade) = warp.source(p);
                    // Light falls off towards the edges of the field, as it
                    // does through any objective.
                    let off = (p - Vec2::splat(SIZE as f32 / 2.0)).length() / SIZE as f32;
                    let field = 1.0 - 0.35 * off * off;
                    let mut colour = [0.006f32, 0.007, 0.012];
                    for channel in channels {
                        if channel.amount <= 0.0 && channel.layers[FRAGMENTS] == 0.0 {
                            continue;
                        }
                        let mut signal = 0.0;
                        for (l, &w) in channel.layers.iter().enumerate() {
                            if w == 0.0 {
                                continue;
                            }
                            let mut v = self.sample(l, q);
                            if l == DNA {
                                v = warp.dna(v, self.sample(FRAGMENTS, q));
                            }
                            signal += w * v;
                        }
                        let signal = signal * channel.amount * channel.gain * fade * field;
                        for (c, dye) in colour.iter_mut().zip(channel.colour) {
                            *c += signal * dye;
                        }
                    }
                    // The camera's grain: photon counts scatter as their square
                    // root, over a floor of read noise.
                    let seed = (y * SIZE + x) as u32 ^ frame.wrapping_mul(0x9E37_79B9);
                    let shot = hash(seed) - 0.5;
                    let read = hash(seed ^ 0x5bd1_e995) - 0.5;
                    let px = &mut row[x * 4..x * 4 + 4];
                    for k in 0..3 {
                        let c = colour[k];
                        let c = (c + shot * 0.09 * c.sqrt() + read * 0.012).max(0.0);
                        // A soft shoulder rather than a hard clip, as a display
                        // lookup table gives.
                        let c = 1.0 - (-1.6 * c).exp();
                        px[k] = (srgb(c) * 255.0) as u8;
                    }
                    px[3] = 255;
                }
            });
    }

    /// Bilinear sample of a layer.
    fn sample(&self, layer: usize, q: Vec2) -> f32 {
        if q.x < 0.0 || q.y < 0.0 || q.x >= (SIZE - 1) as f32 || q.y >= (SIZE - 1) as f32 {
            return 0.0;
        }
        let (x0, y0) = (q.x as usize, q.y as usize);
        let (fx, fy) = (q.x - x0 as f32, q.y - y0 as f32);
        let l = &self.layers[layer];
        let i = y0 * SIZE + x0;
        let top = l[i] * (1.0 - fx) + l[i + 1] * fx;
        let bottom = l[i + SIZE] * (1.0 - fx) + l[i + SIZE + 1] * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

/// How a dying cell's image is moved and faded.
struct Warp {
    centre: Vec2,
    fate: Fate,
}

impl Warp {
    fn of(fate: Fate, centre: Vec2) -> Self {
        Self { centre, fate }
    }

    /// Where pixel `p` of the image takes its light from in the living
    /// cell, and how much of that light is left.
    fn source(&self, p: Vec2) -> (Vec2, f32) {
        let d = p - self.centre;
        match self.fate {
            Fate::Apoptotic(a) => {
                // An apoptotic cell shrinks, and its membrane bulges into
                // blebs that pinch off as the cell breaks up.
                let angle = d.y.atan2(d.x);
                let blebs = 0.13
                    * a
                    * ((7.0 * angle + 1.3).sin().max(0.0).powi(3)
                        + 0.6 * (11.0 * angle + 0.4).sin().max(0.0).powi(4));
                let scale = (1.0 - 0.3 * a) * (1.0 + blebs);
                (self.centre + d / scale, 1.0)
            }
            Fate::Necrotic(n) => {
                // A necrotic cell swells as water rushes in, and its
                // contents leak out and fade.
                let scale = 1.0 + 0.2 * n;
                (self.centre + d / scale, 1.0 - 0.75 * n)
            }
            _ => (p, 1.0),
        }
    }

    /// The DNA stain at a point: whole, broken into fragments, or fading.
    fn dna(&self, whole: f32, fragments: f32) -> f32 {
        match self.fate {
            Fate::Apoptotic(a) => whole * (1.0 - a) + fragments * a,
            Fate::Necrotic(n) => whole * (1.0 - 0.8 * n),
            _ => whole,
        }
    }
}

/// What a stain of `protein` lights up, by its name in a network file: the
/// compartments it lives in, given the levels it and its partners are at.
pub fn pattern(protein: &str, level: f32) -> [f32; LAYERS] {
    let mut w = [0.0f32; LAYERS];
    match protein {
        "egfr" => {
            w[MEMBRANE] = 1.0;
            w[VESICLES] = 0.3;
        }
        "ras" => {
            w[MEMBRANE] = 1.0;
            w[CYTOSOL] = 0.08;
        }
        "erk" => {
            // Switched on, ERK goes into the nucleus: the more active, the
            // more of it there.
            let inside = 0.15 + 0.75 * level.clamp(0.0, 1.0);
            w[CYTOSOL] = 1.0 - inside;
            w[NUCLEOPLASM] = inside * 1.3;
        }
        "vegf" => {
            w[GOLGI] = 1.0;
            w[VESICLES] = 0.7;
        }
        "bcl2" => w[MITOCHONDRIA] = 1.0,
        "bax" => {
            // Inactive BAX floats in the cytosol; switched on, it gathers on
            // the mitochondria.
            let gathered = (level * 4.0).clamp(0.0, 1.0);
            w[CYTOSOL] = 0.8 * (1.0 - gathered);
            w[MITOCHONDRIA] = 1.3 * gathered;
        }
        "caspase" | "raf" | "mek" => w[CYTOSOL] = 1.0,
        "cyclin_d" => {
            w[NUCLEOPLASM] = 0.9;
            w[CYTOSOL] = 0.2;
        }
        "hif1" | "p53" | "mdm2" | "p21" | "p27" | "rb" | "e2f" | "cyclin_e" => {
            w[NUCLEOPLASM] = 1.0;
            w[CYTOSOL] = 0.04;
        }
        _ => w[CYTOSOL] = 1.0,
    }
    w
}

/// Where each protein of the fuller cell lives, in a sentence.
pub fn place_of(protein: &str) -> &'static str {
    match protein {
        "egfr" => "In the cell membrane, and in vesicles once taken back in.",
        "ras" => "On the inner face of the cell membrane.",
        "raf" | "mek" => "In the cytosol.",
        "erk" => "In the cytosol; switched on, it moves into the nucleus.",
        "hif1" => {
            "Destroyed as soon as it is made while there is oxygen; without it, it builds up in the nucleus."
        }
        "vegf" => "Packed by the Golgi apparatus into vesicles, and secreted.",
        "bcl2" => "In the outer membrane of the mitochondria.",
        "bax" => "In the cytosol; switched on, it gathers on the mitochondria.",
        "caspase" => "In the cytosol.",
        "cyclin_d" => "Made in the cytosol, it works in the nucleus.",
        _ => "In the nucleus.",
    }
}

/// The DNA stain, and the stains of the structure itself, which a
/// microscopist adds to see where things are.
pub fn dna_stain() -> [f32; LAYERS] {
    let mut w = [0.0; LAYERS];
    w[DNA] = 1.0;
    w
}

/// The lens and the camera: a sharp image of each point spread over about
/// a pixel and a half, and a faint haze of light from above and below the
/// plane in focus.
fn optics(layer: &[f32]) -> Vec<f32> {
    let sharp = blur(layer, 1.1);
    let haze = blur(layer, 7.0);
    sharp.iter().zip(&haze).map(|(s, h)| s + 0.14 * h).collect()
}

/// A Gaussian blur, separable: rows, then columns.
fn blur(layer: &[f32], sigma: f32) -> Vec<f32> {
    use rayon::prelude::*;
    let reach = (3.0 * sigma).ceil() as isize;
    let kernel: Vec<f32> = (-reach..=reach)
        .map(|i| (-(i as f32).powi(2) / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f32 = kernel.iter().sum();
    let kernel: Vec<f32> = kernel.iter().map(|k| k / total).collect();
    let pass = |source: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0; SIZE * SIZE];
        out.par_chunks_mut(SIZE).enumerate().for_each(|(a, line)| {
            for (b, cell) in line.iter_mut().enumerate() {
                let mut sum = 0.0;
                for (k, w) in kernel.iter().enumerate() {
                    let o = b as isize + k as isize - reach;
                    if o < 0 || o >= SIZE as isize {
                        continue;
                    }
                    let o = o as usize;
                    sum += w * if horizontal {
                        source[a * SIZE + o]
                    } else {
                        source[o * SIZE + a]
                    };
                }
                *cell = sum;
            }
        });
        out
    };
    let rows = pass(layer, true);
    // The second pass reads columns and writes them as rows: transpose
    // back.
    let columns = pass(&rows, false);
    let mut out = vec![0.0; SIZE * SIZE];
    for a in 0..SIZE {
        for b in 0..SIZE {
            out[b * SIZE + a] = columns[a * SIZE + b];
        }
    }
    out
}

/// Add a small Gaussian spot to a layer, keeping the brightest of what is
/// there and the spot.
fn splat(layer: &mut [f32], p: Vec2, sigma: f32, brightness: f32) {
    let reach = (2.5 * sigma).ceil() as i32;
    let (cx, cy) = (p.x.round() as i32, p.y.round() as i32);
    for y in cy - reach..=cy + reach {
        for x in cx - reach..=cx + reach {
            if x < 0 || y < 0 || x >= SIZE as i32 || y >= SIZE as i32 {
                continue;
            }
            let d2 = (x as f32 - p.x).powi(2) + (y as f32 - p.y).powi(2);
            let v = brightness * (-d2 / (2.0 * sigma * sigma)).exp();
            let cell = &mut layer[y as usize * SIZE + x as usize];
            *cell = cell.max(v);
        }
    }
}

fn cap(layer: &mut [f32], top: f32) {
    for v in layer {
        *v = v.min(top);
    }
}

fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

fn smooth(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A linear value as sRGB.
fn srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// A hash of an integer to a uniform value in [0, 1).
fn hash(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth noise in [0, 1): the hash on a grid, interpolated.
fn noise(p: Vec2, seed: u32) -> f32 {
    let (x0, y0) = (p.x.floor(), p.y.floor());
    let (fx, fy) = (p.x - x0, p.y - y0);
    let (fx, fy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let at = |dx: i32, dy: i32| {
        let (x, y) = (x0 as i32 + dx, y0 as i32 + dy);
        hash((x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663) ^ seed)
    };
    let top = at(0, 0) * (1.0 - fx) + at(1, 0) * fx;
    let bottom = at(0, 1) * (1.0 - fx) + at(1, 1) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Noise summed over octaves, each twice as fine and half as strong.
fn fbm(p: Vec2, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amplitude, mut scale, mut total) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amplitude * noise(p * scale, seed.wrapping_add(o * 1013));
        total += amplitude;
        amplitude *= 0.5;
        scale *= 2.0;
    }
    sum / total
}

/// The specimen's own random draws.
struct Random(u32);

impl Random {
    fn between(&mut self, low: f32, high: f32) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        low + (high - low) * ((self.0 >> 8) as f32 / (1u32 << 24) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_of_the_nucleus_is_nucleus_and_the_corner_is_outside() {
        let specimen = Specimen::new(7);
        let (c, _, _) = specimen.nucleus;
        // The nucleus's centre may fall in a nucleolus; a point beside it
        // on the long axis is nucleoplasm or nucleolus, never cytosol.
        assert!(matches!(
            specimen.compartment(c.x, c.y),
            Compartment::Nucleus | Compartment::Nucleolus
        ));
        assert_eq!(specimen.compartment(2.0, 2.0), Compartment::Outside);
    }

    #[test]
    fn a_protein_that_is_absent_leaves_the_image_dark() {
        let specimen = Specimen::new(7);
        let mut out = vec![0u8; SIZE * SIZE * 4];
        let channel = Channel {
            layers: pattern("hif1", 0.0),
            amount: 0.0,
            colour: GREEN,
            gain: 1.0,
        };
        specimen.expose(&[channel], Fate::Cycling(0.0), 1, &mut out);
        let brightest = out.chunks(4).map(|p| p[1]).max().unwrap_or(0);
        assert!(brightest < 60, "only grain: {brightest}");
        let on = Channel {
            amount: 0.8,
            ..channel
        };
        specimen.expose(&[on], Fate::Cycling(0.0), 1, &mut out);
        let brightest = out.chunks(4).map(|p| p[1]).max().unwrap_or(0);
        assert!(brightest > 150, "lit: {brightest}");
    }
}
