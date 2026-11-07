//! A melanoma, as it grows out of a mole: the course Clark described and
//! Hanahan and Weinberg's hallmarks name, laid out ahead of time over the
//! world's clock, in years since BRAF first mutated.
//!
//! - Years 0 to 9: the mole, as in `nevus`.
//! - About year 10: one cell of the first nest loses p16 (CDKN2A), the
//!   brake that held the clone, and its descendants keep dividing: deaf to
//!   the signals that stop growth. They spread along the junction, singly
//!   and in uneven nests, and climb up into the epidermis: the radial
//!   growth phase, a melanoma in situ.
//! - About year 13: TERT is switched on, and the clone can divide without
//!   end.
//! - Years 14 to 16: the basement membrane gives way under it, digested by
//!   the proteases the cells make.
//! - Years 15 to 21: the cells fill a nodule in the dermis, the vertical
//!   growth phase. Its middle grows short of oxygen; from year 16 the
//!   nearest capillaries send out sprouts towards it, which meet and carry
//!   blood through it from year 19.
//! - From year 19.5: cells leave the nodule through the new vessels,
//!   carried off in the blood: the first step of metastasis.
//! - All along: mutations pile up in the DNA, most of them the C to T
//!   changes ultraviolet light leaves.
//!
//! The times are compressed and illustrative; real melanomas take any time
//! from months to decades from one step to the next, and many moles never
//! take any.

use bevy::prelude::*;

use super::nevus::Mole;
use super::shapes::{Path, smooth};
use super::skin::{Dice, Skin};

/// When the brake is lost, when the membrane gives way and how wide the
/// breach grows, when the sprouts set out and meet, when cells leave.
pub const BRAKE_LOST: f32 = 10.0;
pub const IMMORTAL: f32 = 13.0;
pub const BREACH: (f32, f32) = (14.0, 16.5);
const BREACH_WIDTH: f32 = 55.0;
pub const SPROUTS: (f32, f32) = (16.0, 19.0);
pub const LEAVING: f32 = 19.5;
/// How many cells spread in the epidermis, and fill the nodule.
const SPREAD: usize = 260;
const NODULE: usize = 650;
/// The nodule's size when full, in micrometres: across, and deep.
const NODULE_RADIUS: f32 = 70.0;

/// A cell of the melanoma.
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub born: f32,
    /// Where it comes from: the cell that made it, or itself for the first.
    pub parent: usize,
    pub at: Vec3,
    pub size: f32,
    /// Whether it is in the nodule's middle, short of oxygen.
    pub core: bool,
}

/// A sprout: from one capillary to another through the nodule, grown from
/// both ends at once.
#[derive(Debug, Clone)]
pub struct Sprout {
    pub path: Path,
}

/// The melanoma, built.
pub struct Tumour {
    pub cells: Vec<Cell>,
    /// Where the clone starts: the first nest.
    pub start: Vec3,
    /// The breach in the basement membrane.
    pub breach: Vec3,
    /// The nodule's middle.
    pub nodule: Vec3,
    pub sprouts: Vec<Sprout>,
    /// Base pairs of the helix that mutate, by their index, and when.
    pub scars: Vec<(usize, f32)>,
}

impl Tumour {
    pub fn build(skin: &Skin, mole: &Mole, pairs: usize, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let start = mole.nests.first().copied().unwrap_or(Vec3::ZERO);
        let j = &skin.junction;
        let mut cells: Vec<Cell> = vec![Cell {
            born: BRAKE_LOST,
            parent: 0,
            at: start,
            size: 4.8,
            core: false,
        }];
        // The radial phase: along the junction, more and more, some
        // climbing into the epidermis, each from the nearest cell already
        // there.
        let mut tries = 0;
        while cells.len() < SPREAD && tries < 20_000 {
            tries += 1;
            let k = cells.len() as f32 / SPREAD as f32;
            let reach = 30.0 + 150.0 * k.sqrt();
            // Uneven: farther one way than the other, as melanomas grow.
            let d = dice.disc(reach) * Vec2::new(1.25, 0.8);
            let (x, z) = (start.x + d.x, start.z + d.y);
            if Vec2::new(x, z).length() > super::skin::RADIUS * 0.82 {
                continue;
            }
            let base = j.point(x, z) + j.up(x, z) * 5.0;
            let climbing = dice.unit() < 0.3 * k;
            let at = if climbing {
                let top = skin.surface - 26.0;
                Vec3::new(x, base.y + dice.range(8.0, (top - base.y).max(9.0)), z)
            } else {
                base
            };
            if cells.iter().any(|c| c.at.distance(at) < 7.5) {
                continue;
            }
            let parent = nearest(&cells, at);
            cells.push(Cell {
                born: BRAKE_LOST + 4.5 * k + dice.range(0.0, 0.4),
                parent,
                at,
                size: dice.range(4.0, 6.2),
                core: false,
            });
        }
        // The breach: under the middle of the spread, at the junction.
        let breach = j.point(start.x + 25.0, start.z) + Vec3::Y * 2.0;
        // The nodule: a mass below the breach, filled from the top down.
        let nodule = breach - Vec3::Y * (NODULE_RADIUS + 12.0);
        let mut spots = Vec::new();
        let mut tries = 0;
        while spots.len() < NODULE && tries < 60_000 {
            tries += 1;
            let p = nodule
                + dice.direction()
                    * NODULE_RADIUS
                    * dice.unit().cbrt()
                    * Vec3::new(1.15, 0.85, 1.0);
            if p.y > j.height(p.x, p.z) - 3.0 {
                continue;
            }
            if spots.iter().any(|q: &Vec3| q.distance(p) < 8.0) {
                continue;
            }
            spots.push(p);
        }
        // Filled from the breach outwards.
        spots.sort_by(|a, b| a.distance(breach).total_cmp(&b.distance(breach)));
        let first_in_nodule = cells.len();
        let n = spots.len().max(1) as f32;
        for (i, p) in spots.iter().enumerate() {
            let f = i as f32 / n;
            let parent = if i == 0 {
                nearest(&cells, breach)
            } else {
                first_in_nodule + nearest(&cells[first_in_nodule..], *p)
            };
            cells.push(Cell {
                born: BREACH.0 + 0.6 + 5.5 * f.powf(0.8),
                parent,
                at: *p,
                size: dice.range(4.2, 6.8),
                core: p.distance(nodule) < NODULE_RADIUS * 0.5,
            });
        }

        // Sprouts: from the capillary loops nearest the nodule, each pair
        // joined through it.
        let mut feet: Vec<Vec3> = skin
            .loops
            .iter()
            .filter_map(|l| {
                l.points
                    .iter()
                    .copied()
                    .min_by(|a, b| a.distance(nodule).total_cmp(&b.distance(nodule)))
            })
            .collect();
        feet.sort_by(|a, b| a.distance(nodule).total_cmp(&b.distance(nodule)));
        feet.truncate(6);
        let mut sprouts = Vec::new();
        for pair in feet.chunks(2) {
            if let [a, b] = pair {
                let through = nodule
                    + Vec3::new(
                        dice.range(-25.0, 25.0),
                        dice.range(-20.0, 20.0),
                        dice.range(-25.0, 25.0),
                    );
                let mut bend = |p: Vec3, q: Vec3| {
                    (p + q) * 0.5
                        + Vec3::new(
                            dice.range(-12.0, 12.0),
                            dice.range(-8.0, 8.0),
                            dice.range(-12.0, 12.0),
                        )
                };
                let (m1, m2) = (bend(*a, through), bend(through, *b));
                sprouts.push(Sprout {
                    path: Path::new(smooth(&[*a, m1, through, m2, *b], 10)),
                });
            }
        }

        // Mutations in the helix: the C:G pairs, a dozen of them, turning
        // over the years.
        let mut scars = Vec::new();
        let middle = pairs / 2;
        let mut order: Vec<usize> = (0..pairs).filter(|i| *i != middle).collect();
        for i in (1..order.len()).rev() {
            let k = (dice.next() % (i as u64 + 1)) as usize;
            order.swap(i, k);
        }
        for (n, i) in order.into_iter().take(12).enumerate() {
            scars.push((i, 3.0 + n as f32 * 1.5 + dice.range(0.0, 1.0)));
        }

        Self {
            cells,
            start,
            breach,
            nodule,
            sprouts,
            scars,
        }
    }

    /// Where a cell is at `years`, and how big, if it is there yet.
    pub fn seen(&self, i: usize, years: f32) -> Option<(Vec3, f32)> {
        let cell = &self.cells[i];
        if years < cell.born {
            return None;
        }
        let from = if cell.parent == i {
            cell.at
        } else {
            self.cells[cell.parent].at
        };
        let t = super::engine::ease((years - cell.born) / 0.5);
        let swell = 0.7 + 0.3 * ((years - cell.born) / 0.1).clamp(0.0, 1.0);
        Some((from.lerp(cell.at, t), cell.size * swell))
    }

    /// How wide the breach is at `years`.
    pub fn breach_width(&self, years: f32) -> f32 {
        BREACH_WIDTH * super::engine::ease((years - BREACH.0) / (BREACH.1 - BREACH.0))
    }

    /// How far each sprout has grown at `years`, 0 to 1, from both ends.
    pub fn sprouted(years: f32) -> f32 {
        super::engine::ease((years - SPROUTS.0) / (SPROUTS.1 - SPROUTS.0))
    }
}

fn nearest(cells: &[Cell], at: Vec3) -> usize {
    cells
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.at.distance(at).total_cmp(&b.1.at.distance(at)))
        .map_or(0, |(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_melanoma_spreads_breaks_through_and_fills_a_nodule_fed_by_new_vessels() {
        let skin = Skin::build(7);
        let mole = Mole::build(&skin, 9);
        let t = Tumour::build(&skin, &mole, 71, 11);
        let alive = |years: f32| {
            (0..t.cells.len())
                .filter(|i| t.seen(*i, years).is_some())
                .count()
        };
        assert_eq!(alive(BRAKE_LOST - 0.1), 0);
        assert!(alive(14.0) > 150);
        assert_eq!(alive(30.0), t.cells.len());
        // The nodule below the junction, the spread above it.
        let below = t
            .cells
            .iter()
            .filter(|c| c.at.y < skin.junction.height(c.at.x, c.at.z))
            .count();
        assert!(below > 400, "{below}");
        assert!(t.breach_width(BREACH.0) < 1.0 && t.breach_width(30.0) > 50.0);
        assert!(!t.sprouts.is_empty());
        assert_eq!(Tumour::sprouted(SPROUTS.1), 1.0);
        assert_eq!(t.scars.len(), 12);
        assert!(t.scars.iter().all(|(i, _)| *i != 35));
    }
}
