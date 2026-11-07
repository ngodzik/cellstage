//! Skin in three dimensions, cell by cell, in micrometres: the horny,
//! granular, spiny and basal layers of the epidermis over the wavy line
//! where it meets the dermis, the pigment cells among the basal ones and
//! their arms reaching up between the others, and below, the collagen,
//! the fibroblasts that make it, and the loops of capillaries that rise
//! into each fold with red cells running through them.
//!
//! Its sizes are those of histology: basal cells about 9 µm apart, an
//! epidermis 45 to 110 µm deep over its ridges, one pigment cell for ten
//! or so basal cells, capillary loops 6 to 8 µm wide. The places are made
//! from a seed, so the same skin is built every time.
//!
//! Every position is given about one pigment cell at the bottom of a
//! ridge: the one the story of a mole begins with, at the origin, so that
//! the camera can come down to its DNA without running out of precision.

use std::collections::HashMap;

use bevy::prelude::*;

use super::shapes::{Builder, Path, smooth};

/// How far from the middle the tissue is built, in micrometres.
pub const RADIUS: f32 = 215.0;
/// Where the horny layer ends and the granular one ends, below the skin's
/// surface.
const HORNY: f32 = 12.0;
const GRANULAR: f32 = 25.0;
/// How deep the dermis is drawn, and where its plexus of vessels runs.
const DERMIS: f32 = 330.0;
const PLEXUS: f32 = 165.0;
/// The spacing of the dermal papillae, the folds of dermis that rise into
/// the epidermis.
const PAPILLAE: f32 = 100.0;

/// What a piece of the world is, which decides how it looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    Horny,
    Granular,
    Spiny,
    Basal,
    Nucleus,
    FadingNucleus,
    Melanin,
    Melanocyte,
    /// The melanocyte the story of a mole begins with, and its nucleus.
    First,
    FirstNucleus,
    Membrane,
    Collagen,
    Elastic,
    Fibroblast,
    Vessel,
    RedCell,
    Immune,
}

impl Layer {
    /// What it is, in words for the agent.
    pub fn what(self) -> &'static str {
        match self {
            Layer::Horny => {
                "the horny layer (stratum corneum): dead, flattened keratinocytes full of keratin, the skin's outer wall, shed and replaced about every month"
            }
            Layer::Granular => {
                "the granular layer: keratinocytes flattening and dying, filling with keratin and lipids that seal the skin"
            }
            Layer::Spiny => {
                "the spiny layer (stratum spinosum): keratinocytes held together by desmosomes, on their way up to the surface"
            }
            Layer::Basal => {
                "the basal layer: the keratinocytes that divide, on the basement membrane; their daughters move up and die over about four weeks"
            }
            Layer::Nucleus | Layer::FadingNucleus => {
                "a keratinocyte's nucleus; under its melanin cap when the cell has one"
            }
            Layer::Melanin => {
                "melanin: a cap of pigment granules (melanosomes) a keratinocyte received from a melanocyte, over its nucleus like a parasol against ultraviolet light"
            }
            Layer::Melanocyte => {
                "a melanocyte: a pigment cell of the basal layer; it makes melanin in melanosomes and hands them through its dendrites to about 36 keratinocytes"
            }
            Layer::First => {
                "the first melanocyte: the pigment cell in which, in this story, BRAF mutates and a mole begins"
            }
            Layer::FirstNucleus => "the first melanocyte's nucleus, where its DNA is",
            Layer::Membrane => {
                "the basement membrane: a thin sheet of collagen IV and laminin between epidermis and dermis; its ridges and folds anchor the two"
            }
            Layer::Collagen => {
                "collagen bundles of the dermis: type I collagen made by fibroblasts, giving skin its strength"
            }
            Layer::Elastic => "elastic fibres: elastin, which lets skin stretch and come back",
            Layer::Fibroblast => {
                "a fibroblast: the dermis's resident cell, which makes collagen and elastin"
            }
            Layer::Vessel => {
                "a capillary loop: blood rises into each dermal papilla and comes back down, feeding the epidermis, which has no vessels of its own"
            }
            Layer::RedCell => "a red blood cell, carrying oxygen through the capillary",
            Layer::Immune => "a lymphocyte, an immune cell patrolling the dermis",
        }
    }

    /// Every layer's name, once each.
    pub fn names() -> Vec<&'static str> {
        let mut names: Vec<&'static str> = super::LAYERS.iter().map(|l| l.name()).collect();
        names.dedup();
        names
    }

    /// Its name, as the agent points at it.
    pub fn name(self) -> &'static str {
        match self {
            Layer::Horny => "horny_layer",
            Layer::Granular => "granular_layer",
            Layer::Spiny => "spiny_layer",
            Layer::Basal => "basal_layer",
            Layer::Nucleus | Layer::FadingNucleus => "keratinocyte_nucleus",
            Layer::Melanin => "melanin_cap",
            Layer::Melanocyte => "melanocyte",
            Layer::First => "first_melanocyte",
            Layer::FirstNucleus => "nucleus",
            Layer::Membrane => "basement_membrane",
            Layer::Collagen => "collagen",
            Layer::Elastic => "elastic_fibre",
            Layer::Fibroblast => "fibroblast",
            Layer::Vessel => "capillary",
            Layer::RedCell => "red_blood_cell",
            Layer::Immune => "lymphocyte",
        }
    }
}

/// The few solids instances are made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Solid {
    Ball,
    Bead,
    Plate,
    RedCell,
}

/// One cell, or part of one, placed.
#[derive(Debug, Clone)]
pub struct Instance {
    pub solid: Solid,
    pub layer: Layer,
    pub transform: Transform,
}

impl Instance {
    /// Roughly how far it reaches from its middle, to be clicked.
    pub fn reach(&self) -> f32 {
        self.transform.scale.max_element()
    }
}

/// A pigment cell: where its body is, and the arms it reaches up with.
#[derive(Debug, Clone)]
pub struct Melanocyte {
    pub at: Vec3,
    pub dendrites: Vec<Path>,
}

/// The skin, ready to be put in the world.
pub struct Skin {
    /// The skin's surface, as a height, about the origin.
    pub surface: f32,
    pub junction: Junction,
    pub instances: Vec<Instance>,
    pub tubes: Vec<(Layer, Builder)>,
    pub melanocytes: Vec<Melanocyte>,
    /// The loops red cells run through.
    pub loops: Vec<Path>,
    /// Where the junction dips deepest, between the papillae: where the
    /// nests of a mole sit.
    pub pegs: Vec<Vec3>,
    /// The middle of each dermal papilla, at its top.
    pub papillae: Vec<Vec3>,
}

/// A small, fast, seeded source of numbers.
#[derive(Debug, Clone)]
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A number in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    /// A point in the disc of `radius`.
    pub fn disc(&mut self, radius: f32) -> Vec2 {
        let r = radius * self.unit().sqrt();
        let a = self.unit() * std::f32::consts::TAU;
        Vec2::new(r * a.cos(), r * a.sin())
    }

    /// A direction, evenly over the sphere.
    pub fn direction(&mut self) -> Vec3 {
        let z = self.range(-1.0, 1.0);
        let a = self.unit() * std::f32::consts::TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * a.cos(), z, r * a.sin())
    }
}

fn hash2(i: i32, j: i32, seed: u64) -> u64 {
    let mut d =
        Dice::new((i as u64).wrapping_mul(73_856_093) ^ (j as u64).wrapping_mul(19_349_663) ^ seed);
    d.next();
    d.next()
}

/// The dermo-epidermal junction: a height over the skin's plane, deepest
/// along the rete ridges and deeper still where they meet, highest over
/// the middle of each dermal papilla. The papillae are the cells of a
/// jittered grid's Voronoi diagram.
#[derive(Debug, Clone)]
pub struct Junction {
    seed: u64,
    /// Added to every height, so that the origin is where it should be.
    lift: f32,
    shift: Vec2,
}

impl Junction {
    fn seed_point(&self, i: i32, j: i32) -> Vec2 {
        let h = hash2(i, j, self.seed);
        let a = (h & 0xFFFF) as f32 / 65_535.0;
        let b = ((h >> 16) & 0xFFFF) as f32 / 65_535.0;
        Vec2::new(
            (i as f32 + 0.15 + 0.7 * a) * PAPILLAE,
            (j as f32 + 0.15 + 0.7 * b) * PAPILLAE,
        )
    }

    /// The three nearest papilla middles' distances, and the nearest one.
    fn nearest(&self, p: Vec2) -> ([f32; 3], Vec2) {
        let ci = (p.x / PAPILLAE).floor() as i32;
        let cj = (p.y / PAPILLAE).floor() as i32;
        let mut d = [f32::MAX; 3];
        let mut best = Vec2::ZERO;
        for i in ci - 2..=ci + 2 {
            for j in cj - 2..=cj + 2 {
                let s = self.seed_point(i, j);
                let e = s.distance(p);
                if e < d[0] {
                    d = [e, d[0], d[1]];
                    best = s;
                } else if e < d[1] {
                    d = [d[0], e, d[1]];
                } else if e < d[2] {
                    d[2] = e;
                }
            }
        }
        (d, best)
    }

    fn raw(&self, p: Vec2) -> f32 {
        let ([f1, f2, f3], _) = self.nearest(p);
        let edge = (f2 - f1) * 0.5;
        let node = (f3 - f1) * 0.5;
        let ridge = (-(edge / 13.0).powi(2)).exp();
        let peg = (-(node / 17.0).powi(2)).exp();
        let dome = (f1 / (0.5 * PAPILLAE)).min(1.3);
        -44.0 - 10.0 * dome * dome - 30.0 * ridge - 22.0 * peg
    }

    /// The junction's height at `x`, `z`, about the origin.
    pub fn height(&self, x: f32, z: f32) -> f32 {
        self.raw(Vec2::new(x, z) + self.shift) + self.lift
    }

    /// The way into the epidermis, square to the junction.
    pub fn up(&self, x: f32, z: f32) -> Vec3 {
        let e = 0.6;
        let dx = self.height(x + e, z) - self.height(x - e, z);
        let dz = self.height(x, z + e) - self.height(x, z - e);
        Vec3::new(-dx, 2.0 * e, -dz).normalize_or(Vec3::Y)
    }

    pub fn point(&self, x: f32, z: f32) -> Vec3 {
        Vec3::new(x, self.height(x, z), z)
    }

    /// The middle of the papilla nearest `x`, `z`, about the origin.
    pub fn papilla(&self, x: f32, z: f32) -> Vec2 {
        self.nearest(Vec2::new(x, z) + self.shift).1 - self.shift
    }
}

/// Points kept apart: a grid of buckets to ask quickly what is near.
struct Spacing {
    cell: f32,
    buckets: HashMap<(i32, i32, i32), Vec<(Vec3, f32)>>,
}

impl Spacing {
    fn new(cell: f32) -> Self {
        Self {
            cell,
            buckets: HashMap::new(),
        }
    }

    fn key(&self, p: Vec3) -> (i32, i32, i32) {
        (
            (p.x / self.cell).floor() as i32,
            (p.y / self.cell).floor() as i32,
            (p.z / self.cell).floor() as i32,
        )
    }

    /// Whether `p`, `room` wide, is clear of every point kept: further
    /// from each than the larger of their rooms, with the vertical
    /// stretched by `squash`.
    fn clear(&self, p: Vec3, room: f32, squash: f32) -> bool {
        let (i, j, k) = self.key(p);
        // Rooms are never wider than a bucket, so the next buckets are
        // enough.
        for a in i - 1..=i + 1 {
            for b in j - 1..=j + 1 {
                for c in k - 1..=k + 1 {
                    if let Some(list) = self.buckets.get(&(a, b, c)) {
                        for (q, r) in list {
                            let mut d = *q - p;
                            d.y *= squash;
                            let need = room.max(*r);
                            if d.length_squared() < need * need {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn keep(&mut self, p: Vec3, room: f32) {
        let key = self.key(p);
        self.buckets.entry(key).or_default().push((p, room));
    }
}

/// A transform at `at` whose up is `up`, of radii `size`.
fn oriented(at: Vec3, up: Vec3, size: Vec3, spin: f32) -> Transform {
    let turn = Quat::from_rotation_arc(Vec3::Y, up) * Quat::from_rotation_y(spin);
    Transform::from_translation(at)
        .with_rotation(turn)
        .with_scale(size)
}

/// A transform at `at` whose long axis, x, lies along `along`.
fn lying(at: Vec3, along: Vec3, size: Vec3) -> Transform {
    Transform::from_translation(at)
        .with_rotation(Quat::from_rotation_arc(
            Vec3::X,
            along.normalize_or(Vec3::X),
        ))
        .with_scale(size)
}

impl Skin {
    /// Build the skin from `seed`.
    pub fn build(seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let mut junction = Junction {
            seed,
            lift: 0.0,
            shift: Vec2::ZERO,
        };
        // The deepest point near the middle, where ridges meet: the pigment
        // cell the story begins with sits there.
        let mut deepest = (f32::MAX, Vec2::ZERO);
        for i in -40..=40 {
            for j in -40..=40 {
                let p = Vec2::new(i as f32 * 2.0, j as f32 * 2.0);
                let h = junction.raw(p);
                if h < deepest.0 {
                    deepest = (h, p);
                }
            }
        }
        junction.shift = deepest.1;
        let body = 4.5;
        let up0 = junction.up(0.0, 0.0);
        let first = junction.point(0.0, 0.0) + up0 * body;
        // Everything about that cell's middle.
        junction.lift = -first.y;
        let surface = -first.y;
        let junction = junction;

        let mut instances = Vec::new();
        let mut tubes: Vec<(Layer, Builder)> = Vec::new();
        let mut spacing = Spacing::new(12.0);

        // Pigment cells first, one for every ten or so basal cells, kept
        // about 28 µm apart; the first at the origin.
        let mut melanocytes = vec![(Vec3::ZERO, junction.up(0.0, 0.0))];
        spacing.keep(Vec3::ZERO, 9.5);
        for _ in 0..6000 {
            let d = dice.disc(RADIUS);
            let up = junction.up(d.x, d.y);
            let at = junction.point(d.x, d.y) + up * body;
            if melanocytes.iter().all(|(m, _)| m.distance(at) > 28.0) {
                melanocytes.push((at, up));
                spacing.keep(at, 9.5);
            }
        }
        // The basal layer: one sheet of cells on the junction, columnar.
        let mut basal = Vec::new();
        for _ in 0..60_000 {
            let d = dice.disc(RADIUS);
            let up = junction.up(d.x, d.y);
            let at = junction.point(d.x, d.y) + up * 5.6;
            if spacing.clear(at, 8.4, 1.0) {
                spacing.keep(at, 8.4);
                basal.push((at, up));
            }
        }
        // The spiny layer, from just above the basal cells to the granular.
        let mut spiny = Vec::new();
        for _ in 0..160_000 {
            let d = dice.disc(RADIUS);
            let floor = junction.height(d.x, d.y) + 12.0;
            let ceiling = surface - GRANULAR - 4.0;
            if floor >= ceiling {
                continue;
            }
            let at = Vec3::new(d.x, dice.range(floor, ceiling), d.y);
            if spacing.clear(at, 10.2, 1.0) {
                spacing.keep(at, 10.2);
                spiny.push(at);
            }
        }
        // The granular layer, flattened, two or three cells deep.
        let mut granular = Spacing::new(12.0);
        let mut flat = Vec::new();
        for _ in 0..30_000 {
            let d = dice.disc(RADIUS);
            let at = Vec3::new(
                d.x,
                dice.range(surface - GRANULAR, surface - HORNY - 1.5),
                d.y,
            );
            if granular.clear(at, 11.5, 2.6) {
                granular.keep(at, 11.5);
                flat.push(at);
            }
        }
        // The horny layer: thin plates in loose sheets.
        for k in 0..7 {
            let mut sheet = Spacing::new(18.0);
            let y = surface - HORNY + 0.6 + k as f32 * 1.65;
            for _ in 0..2500 {
                let d = dice.disc(RADIUS * 1.02);
                let at = Vec3::new(d.x, y + dice.range(-0.3, 0.3), d.y);
                if sheet.clear(at, 17.0, 1.0) {
                    sheet.keep(at, 17.0);
                    let tilt = Vec3::new(dice.range(-0.05, 0.05), 1.0, dice.range(-0.05, 0.05));
                    instances.push(Instance {
                        solid: Solid::Plate,
                        layer: Layer::Horny,
                        transform: oriented(
                            at,
                            tilt.normalize(),
                            Vec3::new(dice.range(11.0, 14.5), 0.7, dice.range(11.0, 14.5)),
                            dice.unit() * 6.3,
                        ),
                    });
                }
            }
        }

        for (at, up) in &basal {
            let spin = dice.unit() * 6.3;
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Basal,
                transform: oriented(*at, *up, Vec3::new(4.0, 5.8, 4.0), spin),
            });
            let nucleus = *at - *up * 0.6;
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Nucleus,
                transform: oriented(nucleus, *up, Vec3::new(2.2, 2.9, 2.2), spin),
            });
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Melanin,
                transform: oriented(nucleus + *up * 3.1, *up, Vec3::new(2.3, 0.75, 2.3), spin),
            });
        }
        for at in &spiny {
            let depth = (surface - GRANULAR - at.y) / 60.0;
            let squash = 0.78 + 0.22 * depth.clamp(0.0, 1.0);
            let size = Vec3::new(dice.range(4.6, 5.3), 4.9 * squash, dice.range(4.6, 5.3));
            let spin = dice.unit() * 6.3;
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Spiny,
                transform: oriented(*at, Vec3::Y, size, spin),
            });
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Nucleus,
                transform: oriented(*at, Vec3::Y, Vec3::new(2.5, 2.5 * squash, 2.5), spin),
            });
            // The lower spiny cells still carry their caps of pigment.
            if at.y - junction.height(at.x, at.z) < 26.0 {
                instances.push(Instance {
                    solid: Solid::Ball,
                    layer: Layer::Melanin,
                    transform: oriented(
                        *at + Vec3::Y * 2.7 * squash,
                        Vec3::Y,
                        Vec3::new(2.4, 0.7, 2.4),
                        spin,
                    ),
                });
            }
        }
        for at in &flat {
            let spin = dice.unit() * 6.3;
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Granular,
                transform: oriented(
                    *at,
                    Vec3::Y,
                    Vec3::new(dice.range(6.0, 7.5), 2.1, dice.range(6.0, 7.5)),
                    spin,
                ),
            });
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::FadingNucleus,
                transform: oriented(*at, Vec3::Y, Vec3::new(1.9, 0.8, 1.9), spin),
            });
        }

        // Pigment cells: a body on the junction and four to six arms that
        // run out a little and then up between the keratinocytes.
        let mut pigment = Builder::default();
        let mut first = Builder::default();
        let mut kept = Vec::new();
        for (n, (at, up)) in melanocytes.iter().enumerate() {
            let spin = dice.unit() * 6.3;
            let (body_layer, nucleus_layer) = if n == 0 {
                (Layer::First, Layer::FirstNucleus)
            } else {
                (Layer::Melanocyte, Layer::Nucleus)
            };
            instances.push(Instance {
                solid: Solid::Ball,
                layer: body_layer,
                transform: oriented(*at, *up, Vec3::new(5.0, 4.2, 5.0), spin),
            });
            instances.push(Instance {
                solid: Solid::Ball,
                layer: nucleus_layer,
                transform: oriented(*at, *up, Vec3::new(2.6, 2.2, 2.6), spin),
            });
            let (arms_to, sides) = if n == 0 {
                (&mut first, 12)
            } else {
                (&mut pigment, 7)
            };
            let arms = 4 + (dice.next() % 3) as usize;
            let side = up.any_orthonormal_vector();
            let mut dendrites = Vec::new();
            for a in 0..arms {
                let angle =
                    (a as f32 + dice.range(-0.3, 0.3)) / arms as f32 * std::f32::consts::TAU;
                let out = Quat::from_axis_angle(*up, angle) * side;
                let reach = dice.range(22.0, 38.0) * if n == 0 { 1.1 } else { 1.0 };
                let mut points = vec![*at + out * 3.5 + *up * 0.5];
                let steps = 7;
                for s in 1..=steps {
                    let f = s as f32 / steps as f32;
                    let outward = 4.0 + 12.0 * f.sqrt();
                    let rise = reach * f * f.sqrt();
                    let wobble = Vec3::new(
                        dice.range(-1.6, 1.6),
                        dice.range(-0.8, 0.8),
                        dice.range(-1.6, 1.6),
                    );
                    points.push(*at + out * outward + *up * rise + wobble);
                }
                let path = smooth(&points, 5);
                arms_to.tube(&path, |s| 1.15 - 0.85 * s, sides);
                // A branch from partway up.
                if dice.unit() < 0.7 {
                    let k = path.len() / 2 + (dice.next() % 6) as usize;
                    let k = k.min(path.len() - 2);
                    let from = path[k];
                    let away = Quat::from_axis_angle(*up, dice.range(0.6, 1.2)) * out;
                    let branch = smooth(
                        &[
                            from,
                            from + away * 4.0 + *up * 4.0,
                            from + away * 7.0 + *up * dice.range(9.0, 14.0),
                        ],
                        6,
                    );
                    arms_to.tube(&branch, |s| 0.6 - 0.4 * s, 6);
                    dendrites.push(Path::new(branch));
                }
                dendrites.push(Path::new(path));
            }
            kept.push(Melanocyte { at: *at, dendrites });
        }
        tubes.push((Layer::Melanocyte, pigment));
        tubes.push((Layer::First, first));

        // The basement membrane.
        let mut membrane = Builder::default();
        membrane.sheet(RADIUS * 1.05, 2.4, RADIUS * 1.05, |x, z| {
            junction.height(x, z) - 0.6
        });
        tubes.push((Layer::Membrane, membrane));

        // Collagen: fine and loose under the junction, thick and wavy deeper.
        let mut collagen = Builder::default();
        let mut elastic = Builder::default();
        let bottom = surface - DERMIS;
        for f in 0..300 {
            let fine = f < 150;
            let d = dice.disc(RADIUS * 1.05);
            let top = junction.height(d.x, d.y) - 4.0;
            let y = if fine {
                dice.range(top - 55.0, top)
            } else {
                dice.range(bottom, top - 50.0)
            };
            let mut p = Vec3::new(d.x, y, d.y);
            let mut heading = dice.unit() * std::f32::consts::TAU;
            let length = dice.range(70.0, 220.0);
            let mut points = Vec::new();
            let mut walked = 0.0;
            let step = 4.0;
            let crimp = dice.range(0.0, 6.3);
            while walked < length {
                heading += dice.range(-0.12, 0.12);
                let along = Vec3::new(heading.cos(), dice.range(-0.12, 0.12), heading.sin());
                p += along * step;
                let ceiling = junction.height(p.x, p.z) - 3.0;
                p.y = p.y.min(ceiling).max(bottom);
                let side = Vec3::new(-heading.sin(), 0.0, heading.cos());
                let wave = (walked / 11.0 + crimp).sin() * if fine { 0.8 } else { 2.0 };
                points.push(p + side * wave);
                walked += step;
            }
            let r = if fine {
                dice.range(0.6, 1.3)
            } else {
                dice.range(2.0, 4.6)
            };
            collagen.tube(&smooth(&points, 2), |_| r, if fine { 5 } else { 8 });
        }
        for _ in 0..60 {
            let d = dice.disc(RADIUS);
            let top = junction.height(d.x, d.y) - 5.0;
            let mut p = Vec3::new(d.x, dice.range(bottom, top), d.y);
            let mut points = Vec::new();
            let mut heading = dice.direction();
            for _ in 0..40 {
                heading = (heading + dice.direction() * 0.5).normalize();
                heading.y *= 0.4;
                p += heading.normalize_or(Vec3::X) * 3.0;
                p.y = p.y.min(junction.height(p.x, p.z) - 3.0).max(bottom);
                points.push(p);
            }
            elastic.tube(&smooth(&points, 2), |_| 0.35, 4);
        }

        // Capillary loops, one rising into each papilla, and the plexus
        // they come from.
        let mut vessels = Builder::default();
        let mut loops = Vec::new();
        let mut papillae = Vec::new();
        let mut centres: Vec<Vec2> = Vec::new();
        for i in -4..=4 {
            for j in -4..=4 {
                let probe = Vec2::new(i as f32, j as f32) * PAPILLAE * 0.5;
                let c = junction.papilla(probe.x, probe.y);
                if c.length() < RADIUS * 0.95 && !centres.iter().any(|o| o.distance(c) < 5.0) {
                    centres.push(c);
                }
            }
        }
        let plexus = surface - PLEXUS;
        for c in &centres {
            let top = junction.height(c.x, c.y) - 7.0;
            papillae.push(Vec3::new(c.x, top + 7.0, c.y));
            let side = Vec2::from_angle(dice.unit() * 6.3);
            let s3 = Vec3::new(side.x, 0.0, side.y);
            let base = Vec3::new(c.x, plexus, c.y);
            let mut points = Vec::new();
            let rise = top - plexus;
            for k in 0..=8 {
                let f = k as f32 / 8.0;
                let meander = Vec3::new(dice.range(-1.5, 1.5), 0.0, dice.range(-1.5, 1.5));
                points.push(base - s3 * 4.0 + Vec3::Y * rise * f + meander);
            }
            for k in 1..6 {
                let a = k as f32 / 6.0 * std::f32::consts::PI;
                points.push(base + Vec3::Y * (rise + 3.0 * a.sin()) - s3 * 4.0 * a.cos());
            }
            for k in (0..=8).rev() {
                let f = k as f32 / 8.0;
                let meander = Vec3::new(dice.range(-1.5, 1.5), 0.0, dice.range(-1.5, 1.5));
                points.push(base + s3 * 4.5 + Vec3::Y * rise * f + meander);
            }
            let path = smooth(&points, 4);
            vessels.tube(&path, |_| 3.3, 10);
            loops.push(Path::new(path));
        }
        // The plexus: each loop's foot joined to the next nearest's.
        for (i, a) in centres.iter().enumerate() {
            if let Some(b) = centres
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .min_by(|x, y| a.distance(*x.1).total_cmp(&a.distance(*y.1)))
                .map(|(_, b)| *b)
            {
                let mid =
                    (*a + b) * 0.5 + Vec2::new(dice.range(-12.0, 12.0), dice.range(-12.0, 12.0));
                let points = smooth(
                    &[
                        Vec3::new(a.x, plexus, a.y),
                        Vec3::new(mid.x, plexus - dice.range(4.0, 14.0), mid.y),
                        Vec3::new(b.x, plexus, b.y),
                    ],
                    10,
                );
                vessels.tube(&points, |_| 4.2, 10);
                loops.push(Path::new(points));
            }
        }
        tubes.push((Layer::Vessel, vessels));
        tubes.push((Layer::Collagen, collagen));
        tubes.push((Layer::Elastic, elastic));

        // Fibroblasts, long and thin among the fibres, and a few lymphocytes.
        for _ in 0..90 {
            let d = dice.disc(RADIUS);
            let top = junction.height(d.x, d.y) - 10.0;
            let at = Vec3::new(d.x, dice.range(bottom + 10.0, top), d.y);
            let a = dice.unit() * 6.3;
            let along = Vec3::new(a.cos(), dice.range(-0.15, 0.15), a.sin());
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Fibroblast,
                transform: lying(at, along, Vec3::new(dice.range(11.0, 16.0), 2.0, 3.0)),
            });
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Nucleus,
                transform: lying(at, along, Vec3::new(4.5, 1.3, 1.9)),
            });
        }
        for _ in 0..16 {
            let d = dice.disc(RADIUS * 0.9);
            let top = junction.height(d.x, d.y) - 8.0;
            let at = Vec3::new(d.x, dice.range(top - 70.0, top), d.y);
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Immune,
                transform: Transform::from_translation(at).with_scale(Vec3::splat(3.4)),
            });
            instances.push(Instance {
                solid: Solid::Ball,
                layer: Layer::Nucleus,
                transform: Transform::from_translation(at).with_scale(Vec3::splat(2.6)),
            });
        }

        // The pegs: where three ridges meet, deeper than the ridges.
        let mut pegs: Vec<Vec3> = Vec::new();
        let step = 4.0;
        let n = (RADIUS * 0.8 / step) as i32;
        for i in -n..=n {
            for j in -n..=n {
                let (x, z) = (i as f32 * step, j as f32 * step);
                let h = junction.height(x, z);
                let lowest = [
                    (-1, 0),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (1, 1),
                    (-1, 1),
                    (1, -1),
                ]
                .iter()
                .all(|(a, b)| junction.height(x + *a as f32 * step, z + *b as f32 * step) >= h);
                let p = Vec3::new(x, h, z);
                if lowest && p.length() < RADIUS * 0.8 && pegs.iter().all(|q| q.distance(p) > 30.0)
                {
                    pegs.push(p);
                }
            }
        }
        pegs.sort_by(|a, b| a.xz().length().total_cmp(&b.xz().length()));

        Self {
            surface,
            junction,
            instances,
            tubes,
            melanocytes: kept,
            loops,
            pegs,
            papillae,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skin_has_its_layers_in_order_and_the_first_pigment_cell_at_the_origin() {
        let skin = Skin::build(7);
        assert!(skin.melanocytes[0].at.length() < 1e-3);
        let mean = |layer: Layer| {
            let ys: Vec<f32> = skin
                .instances
                .iter()
                .filter(|i| i.layer == layer)
                .map(|i| i.transform.translation.y)
                .collect();
            assert!(!ys.is_empty(), "{layer:?} is empty");
            ys.iter().sum::<f32>() / ys.len() as f32
        };
        let (horny, granular, spiny, basal) = (
            mean(Layer::Horny),
            mean(Layer::Granular),
            mean(Layer::Spiny),
            mean(Layer::Basal),
        );
        assert!(horny > granular && granular > spiny && spiny > basal);
        assert!(horny < skin.surface && skin.surface - horny < HORNY);
        // About one pigment cell for ten basal cells.
        let basal_count = skin
            .instances
            .iter()
            .filter(|i| i.layer == Layer::Basal)
            .count();
        let ratio = basal_count as f32 / skin.melanocytes.len() as f32;
        assert!((5.0..20.0).contains(&ratio), "{ratio}");
        // The epidermis between 40 and 120 µm deep.
        let depth = skin.surface - skin.junction.height(0.0, 0.0);
        assert!((40.0..125.0).contains(&depth), "{depth}");
        assert!(!skin.pegs.is_empty() && !skin.loops.is_empty());
    }
}
