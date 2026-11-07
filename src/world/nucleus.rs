//! Inside the first melanocyte's nucleus, down to its DNA: the envelope
//! and its pores, chromatin filling the nucleus, then nearer, the string
//! of nucleosomes it is made of, DNA wound twice round each core of
//! histones, and nearer still, the double helix itself, base pair by base
//! pair, at the 600th codon of the BRAF gene.
//!
//! Sizes are the measured ones, in micrometres: a nucleosome's core 11 nm
//! across and 5.7 nm thick, the helix 2 nm wide, a base pair every
//! 0.34 nm, ten and a half to a turn. The bases are those of BRAF's exon
//! 15 on the coding strand, around the T that a common mutation turns
//! into an A (c.1799T>A), which makes the codon GTG, valine 600, into GAG,
//! glutamate: BRAF V600E, found in about four common moles in five.

use bevy::prelude::*;

use super::shapes::{Builder, smooth};
use super::skin::Dice;

/// BRAF's exon 15 and around it, coding strand.
const EXON: &str = "TCATGAAGACCTCACAGTAAAAATAGGTGATTTTGGTCTAGCTACAGTGAAATCTCGATGGAGTGGGTCCCATCAGTTTGAACAGTTGTCTGGATCCATTTTGTGGATGG";
/// How many base pairs either side of the mutated one are built.
const SIDE: usize = 35;
/// The double helix: its radius, its rise per base pair, base pairs per
/// turn, and how far round the second strand is from the first.
const HELIX: f32 = 0.001;
const RISE: f32 = 0.000_34;
const PER_TURN: f32 = 10.5;
const GROOVE: f32 = 0.78 * std::f32::consts::PI;

/// A base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Base {
    A,
    C,
    G,
    T,
}

impl Base {
    fn read(c: char) -> Option<Self> {
        match c {
            'A' => Some(Base::A),
            'C' => Some(Base::C),
            'G' => Some(Base::G),
            'T' => Some(Base::T),
            _ => None,
        }
    }

    pub fn pair(self) -> Self {
        match self {
            Base::A => Base::T,
            Base::T => Base::A,
            Base::C => Base::G,
            Base::G => Base::C,
        }
    }

    /// Its colour, as the convention of many textbooks has it.
    pub fn colour(self) -> [f32; 3] {
        match self {
            Base::A => [0.2, 1.0, 0.35],
            Base::T => [1.0, 0.22, 0.2],
            Base::G => [1.0, 0.78, 0.15],
            Base::C => [0.25, 0.55, 1.0],
        }
    }
}

/// Half a base pair: the base on one strand, reaching to the middle.
#[derive(Debug, Clone)]
pub struct HalfPair {
    pub base: Base,
    pub transform: Transform,
    /// Whether it is half of the pair that mutates.
    pub mutates: bool,
}

/// The inside of the nucleus, built.
pub struct Nucleus {
    /// The envelope's transform, the same as the nucleus seen from
    /// outside.
    pub envelope: Transform,
    pub pores: Vec<Transform>,
    pub chromatin: Builder,
    pub nucleolus: Transform,
    pub cores: Vec<Transform>,
    /// The DNA of the nucleosome string, as one tube.
    pub string: Builder,
    pub backbones: Builder,
    pub phosphates: Vec<Transform>,
    pub halves: Vec<HalfPair>,
    /// The codons either side, as letters, to say what changes.
    pub codon: (String, String),
}

/// The bases built, the mutated one in the middle.
fn bases() -> Vec<Base> {
    let t = EXON
        .find("CAGTGAAA")
        .map(|i| i + 3)
        .unwrap_or(EXON.len() / 2);
    let from = t.saturating_sub(SIDE);
    EXON[from..(t + SIDE + 1).min(EXON.len())]
        .chars()
        .filter_map(Base::read)
        .collect()
}

impl Nucleus {
    /// The nucleus whose shape is `envelope`, centred on the origin.
    pub fn build(envelope: Transform, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let radii = envelope.scale;
        let inside = |p: Vec3, margin: f32| {
            let q = envelope.rotation.inverse() * (p - envelope.translation);
            let s = q / (radii - Vec3::splat(margin));
            s.length_squared() < 1.0
        };
        let on_envelope = |d: Vec3| envelope.translation + envelope.rotation * (d * radii);

        // Pores: about ten in a square micrometre of envelope.
        let mut pores = Vec::new();
        let area = 4.0 * std::f32::consts::PI * radii.x * radii.y;
        let n = (area * 10.0) as usize;
        for k in 0..n {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / n as f32;
            let a = k as f32 * 2.399_963;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let d = Vec3::new(r * a.cos(), y, r * a.sin());
            let at = on_envelope(d);
            let normal = (envelope.rotation * (d / radii)).normalize();
            pores.push(
                Transform::from_translation(at)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, normal))
                    .with_scale(Vec3::new(0.06, 0.02, 0.06)),
            );
        }

        // Chromatin: threads wandering through the nucleus, kept inside.
        let mut chromatin = Builder::default();
        for _ in 0..42 {
            let mut p = envelope.translation + dice.direction() * radii.min_element() * 0.6;
            let mut heading = dice.direction();
            let mut points = Vec::new();
            for _ in 0..260 {
                heading = (heading + dice.direction() * 0.45).normalize();
                let next = p + heading * 0.06;
                if inside(next, 0.12) {
                    p = next;
                } else {
                    heading = (envelope.translation - p).normalize_or(Vec3::Y);
                }
                points.push(p);
            }
            chromatin.tube(&smooth(&points, 2), |_| 0.022, 5);
        }
        let nucleolus = Transform::from_translation(
            envelope.translation + envelope.rotation * Vec3::new(0.9, 0.3, -0.6),
        )
        .with_scale(Vec3::splat(0.65));

        // The nucleosome string through the origin: cores every 17 nm, the
        // DNA wound round each one and running on between, straight
        // through the middle where the helix is shown.
        let mut cores = Vec::new();
        let mut string = Builder::default();
        let mut axis = Vec::new();
        let span = 22;
        for k in -span..=span {
            let x = k as f32 * 0.017;
            let bend = 1.0 - (-(x / 0.03).powi(2)).exp();
            axis.push(Vec3::new(
                x,
                0.012 * (k as f32 * 1.1).sin() * bend,
                0.014 * (k as f32 * 0.7).cos() * bend - 0.014 * bend,
            ));
        }
        let axis = smooth(&axis, 8);
        let path = super::shapes::Path::new(axis.clone());
        let mut dna_points = Vec::new();
        let steps = 900;
        let core_every = 0.017;
        for s in 0..=steps {
            let f = s as f32 / steps as f32;
            let (p, heading) = path.at(f);
            let along = f * path.length();
            let phase = (along / core_every).fract();
            let near_middle = p.x.abs() < 0.02;
            if near_middle {
                dna_points.push(p);
                continue;
            }
            // Round the core: 1.65 turns while the string moves on by the
            // core's thickness, the rest of the way straight.
            let wrapping = phase < 0.6;
            if wrapping {
                let turn = phase / 0.6 * 1.65 * std::f32::consts::TAU;
                let side = heading.any_orthonormal_vector();
                let other = heading.cross(side);
                let centre = p + side * 0.0048;
                let out = side * -turn.cos() + other * turn.sin();
                dna_points.push(centre + out * 0.0048 + heading * (phase - 0.3) * 0.004);
            } else {
                dna_points.push(p);
            }
        }
        string.tube(&dna_points, |_| 0.001, 6);
        let mut last = f32::MIN;
        for s in 0..=steps {
            let f = s as f32 / steps as f32;
            let (p, heading) = path.at(f);
            let along = f * path.length();
            if p.x.abs() < 0.02 || along - last < core_every * 0.9 {
                continue;
            }
            if (along / core_every).fract() < 0.05 {
                last = along;
                let side = heading.any_orthonormal_vector();
                cores.push(
                    Transform::from_translation(p + side * 0.0048)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, heading))
                        .with_scale(Vec3::new(0.0040, 0.0029, 0.0040)),
                );
            }
        }

        // The double helix, along x through the origin.
        let bases = bases();
        let middle = bases.len() / 2;
        let mut backbones = Builder::default();
        let mut phosphates = Vec::new();
        let mut halves = Vec::new();
        let mut strands = (Vec::new(), Vec::new());
        let at = |i: usize, second: bool| {
            let x = (i as f32 - middle as f32) * RISE;
            let a = i as f32 / PER_TURN * std::f32::consts::TAU + if second { GROOVE } else { 0.0 };
            Vec3::new(x, HELIX * a.cos(), HELIX * a.sin())
        };
        for (i, base) in bases.iter().enumerate() {
            let (one, two) = (at(i, false), at(i, true));
            strands.0.push(one);
            strands.1.push(two);
            for p in [one, two] {
                phosphates.push(Transform::from_translation(p).with_scale(Vec3::splat(0.000_28)));
            }
            let mid = (one + two) * 0.5;
            for (from, b) in [(one, *base), (two, base.pair())] {
                let reach = mid - from;
                halves.push(HalfPair {
                    base: b,
                    transform: Transform::from_translation(from + reach * 0.5)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, reach.normalize()))
                        .with_scale(Vec3::new(0.000_32, reach.length() * 0.5, 0.000_32)),
                    mutates: i == middle,
                });
            }
        }
        backbones.tube(&smooth(&strands.0, 4), |_| 0.000_3, 8);
        backbones.tube(&smooth(&strands.1, 4), |_| 0.000_3, 8);
        let letters: String = bases.iter().map(|b| format!("{b:?}")).collect();
        let codon = (letters[middle - 1..middle + 2].to_owned(), {
            let mut c: Vec<char> = letters[middle - 1..middle + 2].chars().collect();
            c[1] = 'A';
            c.into_iter().collect()
        });

        Self {
            envelope,
            pores,
            chromatin,
            nucleolus,
            cores,
            string,
            backbones,
            phosphates,
            halves,
            codon,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helix_is_brafs_codon_600_with_its_t_in_the_middle() {
        let bases = bases();
        assert_eq!(bases.len(), 2 * SIDE + 1);
        let middle = bases.len() / 2;
        assert_eq!(bases[middle], Base::T);
        assert_eq!(
            (bases[middle - 1], bases[middle + 1]),
            (Base::G, Base::G),
            "GTG, valine"
        );
        let n = Nucleus::build(Transform::from_scale(Vec3::new(2.6, 2.2, 2.6)), 1);
        assert_eq!(n.codon, ("GTG".to_owned(), "GAG".to_owned()));
        assert_eq!(n.halves.iter().filter(|h| h.mutates).count(), 2);
        // A base pair every 0.34 nm, the helix 2 nm across.
        let xs: Vec<f32> = n
            .phosphates
            .iter()
            .step_by(2)
            .map(|t| t.translation.x)
            .collect();
        assert!((xs[1] - xs[0] - 0.000_34).abs() < 1e-7);
        assert!(!n.cores.is_empty() && n.cores.iter().all(|c| c.translation.x.abs() > 0.015));
    }
}
