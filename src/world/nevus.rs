//! A common mole, as it grows from one melanocyte over the years.
//!
//! The first melanocyte divides; its daughters lose their arms, round up
//! and divide again, spreading along the junction to the bottoms of the
//! nearby rete ridges, where they gather in nests: a junctional naevus,
//! over its first years. Then the clone stops dividing, for decades: the
//! senescence induced by the oncogene its mutation turned on. Over the
//! decades after, the nests sink into the dermis and their cells grow
//! smaller and paler: a compound naevus, then an intradermal one.
//!
//! The clone is a tree of divisions laid out ahead of time. Each cell's
//! daughters head for where their own descendants will end up, so that
//! division by division the clone opens out from the first cell to its
//! nests. The times are those of the textbooks, read loosely: most of a
//! mole's growth in its first years, maturing into the dermis over the
//! following decades. It is an illustration of that course, not a
//! simulation of it.

use bevy::prelude::*;

use super::skin::{Dice, Skin};

/// How many nests, and cells in each.
const NESTS: usize = 8;
const PER_NEST: usize = 22;
/// Years between one generation's divisions and the next's, about: the
/// eight generations from one cell to the nests take some six years.
const GENERATION: f32 = 0.72;
/// Years a newborn cell takes to reach where it is going.
const SETTLE: f32 = 0.7;
/// When the clone stops dividing and has stopped, in years.
pub const BRAKE: (f32, f32) = (7.0, 9.5);
/// When the nests sink into the dermis, and how deep they end.
const SINK: (f32, f32) = (15.0, 50.0);
const DEPTH: f32 = 70.0;

/// A cell of the clone.
#[derive(Debug, Clone)]
pub struct Cell {
    pub parent: Option<usize>,
    pub children: Option<(usize, usize)>,
    /// When it is born and when it divides, in years.
    pub born: f32,
    pub divides: f32,
    /// Where it heads, and the nest it ends in when it is a leaf.
    pub goal: Vec3,
    pub nest: Option<usize>,
}

/// The clone: its cells, and the nests' middles.
#[derive(Debug, Clone)]
pub struct Mole {
    pub cells: Vec<Cell>,
    pub nests: Vec<Vec3>,
}

/// One cell as seen at a moment.
#[derive(Debug, Clone, Copy)]
pub struct Seen {
    pub at: Vec3,
    pub size: f32,
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Mole {
    pub fn build(skin: &Skin, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        // Nests at the pegs nearest the first cell, a little above the
        // junction, the first one at the first cell's own peg.
        let nests: Vec<Vec3> = skin
            .pegs
            .iter()
            .take(NESTS)
            .map(|p| *p + skin.junction.up(p.x, p.z) * 9.0)
            .collect();
        // Each nest a ball of cells, packed from the middle out.
        let mut leaves = Vec::new();
        for (n, c) in nests.iter().enumerate() {
            for k in 0..PER_NEST {
                let f = (k as f32 + 0.5) / PER_NEST as f32;
                let r = 13.0 * f.cbrt();
                let y = 1.0 - 2.0 * f;
                let a = k as f32 * 2.399_963;
                let ring = (1.0 - y * y).max(0.0).sqrt();
                let offset = Vec3::new(ring * a.cos(), y * 0.7, ring * a.sin()) * r;
                leaves.push((*c + offset, n));
            }
        }
        let mut cells = Vec::new();
        // Split the leaves in two along their widest spread, again and
        // again: each split is a division.
        fn split(
            cells: &mut Vec<Cell>,
            leaves: &mut [(Vec3, usize)],
            parent: Option<usize>,
            born: f32,
            depth: u32,
            dice: &mut Dice,
        ) -> usize {
            let me = cells.len();
            let goal = leaves.iter().map(|l| l.0).sum::<Vec3>() / leaves.len() as f32;
            cells.push(Cell {
                parent,
                children: None,
                born,
                divides: f32::MAX,
                goal,
                nest: (leaves.len() == 1).then(|| leaves[0].1),
            });
            if leaves.len() == 1 {
                cells[me].goal = leaves[0].0;
                return me;
            }
            let (lo, hi) = leaves.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), l| {
                (lo.min(l.0), hi.max(l.0))
            });
            let spread = hi - lo;
            let axis = if spread.x >= spread.z { 0 } else { 2 };
            leaves.sort_by(|a, b| a.0[axis].total_cmp(&b.0[axis]));
            let half = leaves.len() / 2;
            let divides = born + GENERATION * dice.range(0.85, 1.15) + 0.01 * depth as f32;
            cells[me].divides = divides;
            let (left, right) = leaves.split_at_mut(half);
            let a = split(cells, left, Some(me), divides, depth + 1, dice);
            let b = split(cells, right, Some(me), divides, depth + 1, dice);
            cells[me].children = Some((a, b));
            me
        }
        split(&mut cells, &mut leaves, None, 0.0, 0, &mut dice);
        // The first cell is the first melanocyte, where it is.
        cells[0].goal = Vec3::ZERO;
        Self { cells, nests }
    }

    /// How far the nests have sunk at `years`, as a fraction.
    fn sunk(years: f32) -> f32 {
        ease((years - SINK.0) / (SINK.1 - SINK.0))
    }

    /// Where a cell is at `years`, if it is alive then.
    pub fn seen(&self, i: usize, years: f32, skin: &Skin) -> Option<Seen> {
        let cell = &self.cells[i];
        if years < cell.born || years >= cell.divides {
            return None;
        }
        let start = match cell.parent {
            Some(p) => {
                let parent = &self.cells[p];
                self.position(p, parent.divides, skin)
            }
            None => Vec3::ZERO,
        };
        let t = ease((years - cell.born) / SETTLE);
        let mut goal = cell.goal;
        if cell.nest.is_none() {
            // On the way: along the junction, not through the dermis.
            goal.y = skin.junction.height(goal.x, goal.z) + 5.0;
        }
        let mut at = start.lerp(goal, t);
        if cell.parent.is_some() && cell.nest.is_none() {
            at.y = at.y.max(skin.junction.height(at.x, at.z) + 3.0);
        }
        // Just born: the two daughters swell apart from where their mother
        // was.
        let fresh = ((years - cell.born) / 0.12).clamp(0.0, 1.0);
        let sink = Self::sunk(years);
        if cell.nest.is_some() {
            at.y -= DEPTH * sink;
        }
        // Maturing: smaller as they go deeper.
        let size = 4.3 * (0.72 + 0.28 * fresh) * (1.0 - 0.28 * sink);
        Some(Seen { at, size })
    }

    /// Where a cell is, alive or at the moment it divides.
    fn position(&self, i: usize, years: f32, skin: &Skin) -> Vec3 {
        let cell = &self.cells[i];
        let start = match cell.parent {
            Some(p) => self.position(p, self.cells[p].divides, skin),
            None => Vec3::ZERO,
        };
        let t = ease((years - cell.born) / SETTLE);
        let mut goal = cell.goal;
        if cell.nest.is_none() && cell.parent.is_some() {
            goal.y = skin.junction.height(goal.x, goal.z) + 5.0;
        }
        start.lerp(goal, t)
    }

    /// How many cells are alive at `years`.
    #[cfg(test)]
    pub fn count(&self, years: f32) -> usize {
        self.cells
            .iter()
            .filter(|c| years >= c.born && years < c.divides)
            .count()
    }

    /// The nests' middles at `years`, sinking with the decades.
    pub fn nest_at(&self, n: usize, years: f32) -> Option<Vec3> {
        self.nests
            .get(n)
            .map(|c| *c - Vec3::Y * DEPTH * Self::sunk(years))
    }

    /// How settled into a nest the clone is at `years`, 0 to 1: how much
    /// the nests crowd out the basal cells where they sit.
    pub fn crowding(&self, years: f32) -> f32 {
        ease((years - 3.0) / 3.0)
    }
}

/// How the clone looks at `years`: its cells' colour, from the gold of a
/// dividing melanocyte to the dusk of a senescent one, and paler as they
/// mature into the dermis.
pub fn colour(years: f32) -> ([f32; 3], f32) {
    let brake = ease((years - BRAKE.0) / (BRAKE.1 - BRAKE.0));
    let sink = Mole::sunk(years);
    let gold = Vec3::new(1.0, 0.55, 0.12);
    let dusk = Vec3::new(0.75, 0.42, 0.62);
    let pale = Vec3::new(0.85, 0.7, 0.75);
    let c = gold.lerp(dusk, brake).lerp(pale, sink * 0.6);
    (c.to_array(), 1.0 - 0.35 * sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clone_opens_out_from_one_cell_to_its_nests_and_then_stops() {
        let skin = Skin::build(7);
        let clone = Mole::build(&skin, 3);
        assert_eq!(clone.count(-0.5), 0);
        assert_eq!(clone.count(0.0), 1);
        // One cell at the start, where the first melanocyte is.
        let first = clone.seen(0, 0.0, &skin).expect("the first cell");
        assert!(first.at.length() < 1e-3);
        // Every leaf at its nest once the divisions are over.
        let leaves = clone.cells.iter().filter(|c| c.nest.is_some()).count();
        assert_eq!(leaves, NESTS.min(skin.pegs.len()) * PER_NEST);
        assert_eq!(clone.count(BRAKE.0 + 1.0), leaves);
        let last_division = clone
            .cells
            .iter()
            .filter(|c| c.divides < f32::MAX)
            .map(|c| c.divides)
            .fold(0.0, f32::max);
        assert!(last_division < BRAKE.0, "{last_division}");
        // Grown in between, never shrinking.
        let mut last = 0;
        for k in 0..40 {
            let n = clone.count(k as f32 * 0.2);
            assert!(n >= last);
            last = n;
        }
        // Sunk into the dermis after decades.
        let (young, old) = (clone.nest_at(0, 10.0), clone.nest_at(0, 50.0));
        assert!(old.unwrap().y < young.unwrap().y - 50.0);
    }
}
