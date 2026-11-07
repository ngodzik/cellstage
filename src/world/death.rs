//! A cell whose DNA is broken, among its neighbours: the third of Hanahan
//! and Weinberg's hallmarks, the death a cancer cell escapes.
//!
//! The breaks set off p53, which builds up in the nucleus and turns on the
//! genes for BAX and its kin. BAX gathers on the mitochondria and opens
//! pores in their outer membrane; cytochrome c leaks out into the cytosol,
//! where it builds apoptosomes, wheels of seven APAF-1 that switch on
//! caspase 9, which switches on caspase 3, and the caspases take the cell
//! apart from inside. The cell shrinks, its chromatin condenses and its
//! nucleus breaks up, its surface bubbles, and it falls into apoptotic
//! bodies, wrapped in membrane, that a macrophage eats; the neighbours close
//! the gap. Nothing spills: no inflammation.
//!
//! The variants are how cancer cells escape it: with BCL2 high, BAX is held
//! back and the mitochondria stay whole; with p53 lost, the breaks are never
//! heard. Either way the cell lives on with its broken DNA, and divides.
//!
//! Sizes in micrometres: a cell 12 µm across, mitochondria about 0.5 µm
//! thick, an apoptosome about 25 nm across. The clock is in hours after the
//! DNA breaks; the order of the events and their rough pace are those of
//! the textbooks, the looks are simplified.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::engine::{Band, Groups, NEVER, NONE, Solids, Things, Tick, ease, set_shown};
use super::film::{Eye, Place};
use super::glow::Glow;
use super::shapes::{Builder, smooth};
use super::skin::Dice;

/// The stories the cell can tell.
pub const VARIANTS: [&str; 3] = ["normal", "bcl2_high", "p53_lost"];

/// The cell's radii.
const CELL: Vec3 = Vec3::new(6.0, 4.6, 6.0);
const NUCLEUS: f32 = 2.4;
const MITOCHONDRIA: usize = 26;
const APOPTOSOMES: usize = 9;
const BODIES: usize = 7;

/// When things happen, in hours after the DNA breaks.
const P53: (f32, f32) = (0.3, 2.5);
const BAX: (f32, f32) = (2.5, 3.5);
const LEAK: (f32, f32) = (3.0, 5.0);
const WHEELS: (f32, f32) = (4.0, 5.0);
const CASPASES: (f32, f32) = (5.0, 6.2);
const SHRINK: (f32, f32) = (6.0, 7.5);
const BLEBS: (f32, f32) = (6.4, 8.0);
const BREAK_UP: (f32, f32) = (8.0, 8.8);
const COMES: (f32, f32) = (8.5, 10.5);
const EATS: (f32, f32) = (10.5, 13.0);
const CLOSE: (f32, f32) = (12.0, 15.0);
const DIVIDES: (f32, f32) = (10.0, 12.0);

const CELLS: Band = Band::new((0.6, 2.0), NEVER);
const INSIDE: Band = Band::new((0.12, 0.4), NEVER);
const MOLECULES: Band = Band::new(NONE, (0.6, 2.0));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Neighbour,
    NeighbourNucleus,
    Cell,
    Cytosol,
    Nucleus,
    Chromatin,
    Break,
    Mitochondrion,
    Bax,
    Bcl2,
    CytochromeC,
    Wheel,
    WheelSpot,
    Bleb,
    Body,
    Fragment,
    Macrophage,
    MacrophageNucleus,
}

fn look(key: Key) -> (Glow, Band) {
    let g = Glow::new;
    match key {
        Key::Neighbour => (g([0.15, 0.6, 1.0], 0.35, 0.0).edges(2.8).varied(8.0), CELLS),
        Key::NeighbourNucleus => (
            g([0.35, 0.5, 1.0], 0.3, 0.0)
                .faces([0.3, 0.4, 1.0], 0.05)
                .edges(1.8),
            CELLS,
        ),
        Key::Cell => (g([0.3, 0.85, 1.0], 0.6, 0.0).edges(2.6), CELLS),
        Key::Cytosol => (
            g([0.2, 0.5, 1.0], 0.0, 0.0).faces([0.25, 0.55, 1.0], 0.02),
            CELLS,
        ),
        Key::Nucleus => (
            g([0.4, 0.55, 1.0], 0.55, 0.0)
                .faces([0.3, 0.45, 1.0], 0.06)
                .edges(2.0),
            INSIDE,
        ),
        Key::Chromatin => (
            g([0.35, 0.45, 1.0], 0.3, 0.0)
                .faces([0.3, 0.35, 1.0], 0.03)
                .edges(1.6),
            INSIDE,
        ),
        Key::Break => (
            g([1.0, 1.0, 1.0], 1.0, 0.0)
                .faces([1.0, 0.95, 0.85], 0.8)
                .edges(1.3),
            INSIDE,
        ),
        Key::Mitochondrion => (
            g([1.0, 0.55, 0.15], 0.55, 0.0)
                .faces([1.0, 0.45, 0.1], 0.05)
                .edges(1.9),
            INSIDE,
        ),
        Key::Bax => (
            g([1.0, 0.25, 0.3], 1.0, 0.0)
                .faces([1.0, 0.2, 0.25], 0.5)
                .edges(1.4),
            Band::new(NONE, (4.0, 10.0)),
        ),
        Key::Bcl2 => (
            g([1.0, 0.95, 0.35], 1.0, 0.0)
                .faces([1.0, 0.9, 0.3], 0.5)
                .edges(1.4),
            Band::new(NONE, (4.0, 10.0)),
        ),
        Key::CytochromeC => (
            g([1.0, 0.3, 0.2], 0.9, 0.0)
                .faces([1.0, 0.25, 0.15], 0.5)
                .edges(1.4),
            Band::new(NONE, (10.0, 30.0)),
        ),
        Key::Wheel => (
            g([0.6, 1.0, 0.4], 0.9, 0.0)
                .faces([0.5, 1.0, 0.35], 0.35)
                .edges(1.5),
            MOLECULES,
        ),
        Key::WheelSpot => (
            g([0.6, 1.0, 0.4], 0.9, 0.0)
                .faces([0.55, 1.0, 0.35], 0.7)
                .edges(1.4),
            Band::new((0.5, 1.5), NEVER),
        ),
        Key::Bleb => (g([0.5, 0.75, 1.0], 0.6, 0.0).edges(2.2), CELLS),
        Key::Body => (
            g([0.65, 0.6, 1.0], 0.6, 0.0)
                .faces([0.5, 0.45, 1.0], 0.04)
                .edges(2.2),
            CELLS,
        ),
        Key::Fragment => (
            g([0.85, 0.6, 1.0], 0.8, 0.0)
                .faces([0.75, 0.5, 1.0], 0.35)
                .edges(1.5),
            CELLS,
        ),
        Key::Macrophage => (
            g([0.4, 1.0, 0.55], 0.45, 0.0)
                .faces([0.35, 1.0, 0.5], 0.03)
                .edges(2.4)
                .varied(10.0),
            CELLS,
        ),
        Key::MacrophageNucleus => (
            g([0.4, 0.9, 0.6], 0.35, 0.0)
                .faces([0.35, 0.8, 0.55], 0.06)
                .edges(1.8),
            CELLS,
        ),
    }
}

fn picks(name: &str, key: Key) -> bool {
    match name {
        "p53" | "nucleus" => matches!(key, Key::Nucleus | Key::Chromatin | Key::Break),
        "dna_damage" => key == Key::Break,
        "mitochondria" => matches!(
            key,
            Key::Mitochondrion | Key::Bax | Key::Bcl2 | Key::CytochromeC
        ),
        "bax" => matches!(key, Key::Bax | Key::Mitochondrion),
        "bcl2" => matches!(key, Key::Bcl2 | Key::Mitochondrion),
        "cytochrome_c" => matches!(key, Key::CytochromeC | Key::Mitochondrion),
        "apoptosome" | "caspases" => matches!(key, Key::Wheel | Key::WheelSpot | Key::Cytosol),
        "apoptotic_bodies" => matches!(key, Key::Body | Key::Fragment | Key::Bleb),
        "macrophage" => matches!(
            key,
            Key::Macrophage | Key::MacrophageNucleus | Key::Body | Key::Fragment
        ),
        "dying_cell" => !matches!(
            key,
            Key::Neighbour | Key::NeighbourNucleus | Key::Macrophage | Key::MacrophageNucleus
        ),
        "neighbours" => matches!(key, Key::Neighbour | Key::NeighbourNucleus),
        _ => false,
    }
}

const HIGHLIGHTS: [&str; 12] = [
    "dying_cell",
    "p53",
    "dna_damage",
    "mitochondria",
    "bax",
    "bcl2",
    "cytochrome_c",
    "apoptosome",
    "caspases",
    "apoptotic_bodies",
    "macrophage",
    "neighbours",
];

pub fn highlights() -> Vec<&'static str> {
    HIGHLIGHTS.to_vec()
}

pub const PLACES: [&str; 8] = [
    "tissue",
    "cell",
    "nucleus",
    "mitochondria",
    "pore",
    "apoptosome",
    "bodies",
    "macrophage",
];

const THINGS: [&str; 10] = [
    "dying_cell",
    "nucleus",
    "dna_break",
    "mitochondrion",
    "bax_pore",
    "cytochrome_c",
    "apoptosome",
    "apoptotic_body",
    "macrophage",
    "neighbour",
];

pub fn things() -> Vec<&'static str> {
    THINGS.to_vec()
}

pub fn clock_label(t: f32) -> Option<String> {
    (t >= 0.0).then(|| {
        if t < 1.0 {
            format!("minute {:.0}", t * 60.0)
        } else {
            format!("hour {t:.1}")
        }
    })
}

/// A mitochondrion: its curve and where its pores and BCL2 sit.
struct Mito {
    points: Vec<Vec3>,
    pores: Vec<Vec3>,
}

/// The tissue, built once.
struct Tissue {
    neighbours: Vec<Vec3>,
    mitos: Vec<Mito>,
    chromatin: Builder,
    breaks: Vec<Vec3>,
    /// Cytochrome c: where each leaves from and drifts to.
    leaks: Vec<(Vec3, Vec3, f32)>,
    wheels: Vec<Vec3>,
    blebs: Vec<Vec3>,
    bodies: Vec<Vec3>,
    /// The macrophage's arms, as grown from its body towards the bodies.
    arms: Vec<Vec<Vec3>>,
}

/// Where the macrophage ends up, beside the dying cell.
const EATER: Vec3 = Vec3::new(10.5, 0.5, 2.0);

impl Tissue {
    fn build(seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let neighbours = (0..7)
            .map(|k| {
                let a = k as f32 / 7.0 * std::f32::consts::TAU + 0.3;
                Vec3::new(a.cos(), 0.0, a.sin()) * 11.2 + Vec3::new(0.0, dice.range(-0.6, 0.6), 0.0)
            })
            .filter(|p| p.distance(EATER) > 8.0)
            .collect();
        let mut mitos = Vec::new();
        for _ in 0..MITOCHONDRIA {
            let dir = dice.direction();
            let at = dir * dice.range(3.0, 4.6) * Vec3::new(1.0, 0.75, 1.0);
            let along = dice.direction().cross(dir).normalize_or(Vec3::X);
            let length = dice.range(1.6, 3.2);
            let bend = dir * dice.range(-0.3, 0.3);
            let points = smooth(
                &[
                    at - along * length * 0.5,
                    at + bend,
                    at + along * length * 0.5,
                ],
                8,
            );
            let pores = (0..3)
                .map(|k| {
                    let p = points[(points.len() - 1) * (k + 1) / 4];
                    p + dice.direction() * 0.24
                })
                .collect();
            mitos.push(Mito { points, pores });
        }
        let mut chromatin = Builder::default();
        for _ in 0..22 {
            let mut p = dice.direction() * NUCLEUS * 0.5;
            let mut heading = dice.direction();
            let mut points = Vec::new();
            for _ in 0..140 {
                heading = (heading + dice.direction() * 0.45).normalize();
                let next = p + heading * 0.07;
                if next.length() < NUCLEUS - 0.12 {
                    p = next;
                } else {
                    heading = -p.normalize();
                }
                points.push(p);
            }
            chromatin.tube(&smooth(&points, 2), |_| 0.03, 5);
        }
        let breaks = (0..9)
            .map(|_| dice.direction() * NUCLEUS * dice.unit().cbrt() * 0.85)
            .collect();
        let mut leaks = Vec::new();
        for (m, mito) in mitos.iter().enumerate() {
            for k in 0..8 {
                let from = mito.points[(k * 3) % mito.points.len()];
                let to = from + dice.direction() * dice.range(0.6, 2.2);
                leaks.push((
                    from,
                    to,
                    LEAK.0 + (m as f32 * 0.07 + k as f32 * 0.2) % (LEAK.1 - LEAK.0),
                ));
            }
        }
        let wheels = (0..APOPTOSOMES)
            .map(|_| dice.direction() * dice.range(3.0, 4.4) * Vec3::new(1.0, 0.7, 1.0))
            .collect();
        let blebs = (0..14).map(|_| dice.direction() * CELL * 0.82).collect();
        let bodies = (0..BODIES)
            .map(|k| {
                let a = k as f32 / BODIES as f32 * std::f32::consts::TAU;
                Vec3::new(a.cos() * 2.6, dice.range(-1.2, 1.2), a.sin() * 2.6)
            })
            .collect::<Vec<_>>();
        let arms = (0..4)
            .map(|k| {
                let a = (k as f32 - 1.5) * 0.5;
                let start = EATER + Vec3::new(-6.5 * a.cos(), dice.range(-1.5, 1.5), 6.5 * a.sin());
                let end = Vec3::new(1.5 * a.cos(), dice.range(-1.0, 1.0), 4.0 * a.sin());
                smooth(
                    &[
                        start,
                        (start + end) * 0.5 + Vec3::Y * dice.range(-1.0, 1.0),
                        end,
                    ],
                    10,
                )
            })
            .collect();
        Self {
            neighbours,
            mitos,
            chromatin,
            breaks,
            leaks,
            wheels,
            blebs,
            bodies,
            arms,
        }
    }
}

fn tissue() -> &'static Tissue {
    static TISSUE: std::sync::OnceLock<Tissue> = std::sync::OnceLock::new();
    TISSUE.get_or_init(|| Tissue::build(31))
}

fn places() -> Vec<Place> {
    let t = tissue();
    let place = |name, what, focus: Vec3, distance, yaw: f32, pitch: f32| Place {
        name,
        what,
        eye: Eye {
            focus,
            distance,
            yaw,
            pitch,
        },
    };
    let mito = t
        .mitos
        .first()
        .map_or(Vec3::ZERO, |m| m.points[m.points.len() / 2]);
    let pore = t.mitos.first().map_or(Vec3::ZERO, |m| m.pores[1]);
    let wheel = t.wheels.first().copied().unwrap_or(Vec3::ZERO);
    vec![
        place(
            "tissue",
            "the dying cell among its neighbours",
            Vec3::ZERO,
            60.0,
            0.5,
            0.6,
        ),
        place(
            "cell",
            "the cell whose DNA is broken, its nucleus and mitochondria inside",
            Vec3::ZERO,
            26.0,
            0.7,
            0.35,
        ),
        place(
            "nucleus",
            "the nucleus: breaks in the DNA, p53 building up",
            Vec3::ZERO,
            8.5,
            0.9,
            0.3,
        ),
        place(
            "mitochondria",
            "mitochondria in the cytoplasm, where the decision to die is made",
            mito,
            7.0,
            0.4,
            0.3,
        ),
        place(
            "pore",
            "a mitochondrion up close: BAX pores in its outer membrane, cytochrome c leaking out (or BCL2 holding BAX back)",
            pore,
            1.4,
            0.6,
            0.25,
        ),
        place(
            "apoptosome",
            "an apoptosome: a wheel of seven APAF-1 built round cytochrome c, switching caspase 9 on",
            wheel,
            0.09,
            0.5,
            0.5,
        ),
        place(
            "bodies",
            "the cell falling apart into apoptotic bodies",
            Vec3::ZERO,
            24.0,
            0.3,
            0.5,
        ),
        place(
            "macrophage",
            "a macrophage coming to eat the apoptotic bodies",
            Vec3::new(5.0, 0.0, 1.0),
            40.0,
            0.2,
            0.45,
        ),
    ]
}

/// The tissue, as shown in a panel.
pub struct DeathWorld {
    groups: Groups<Key>,
    neighbours: Vec<(Entity, Entity, Vec3)>,
    cell: (Entity, Entity),
    nucleus: (Entity, Entity),
    breaks: Vec<Entity>,
    mitos: Entity,
    bax: Vec<Entity>,
    bcl2: Vec<Entity>,
    leaks: Vec<Entity>,
    wheels: Vec<(Entity, [Entity; 8])>,
    blebs: Vec<Entity>,
    bodies: Vec<(Entity, Entity)>,
    macrophage: (Entity, Entity, Entity),
    arms: Vec<Entity>,
    /// The two daughters when the cell survives and divides.
    daughters: [(Entity, Entity); 2],
    places: Vec<Place>,
    variant: usize,
}

impl DeathWorld {
    pub fn spawn(
        world: &mut World,
        layer: RenderLayers,
        variant: &str,
        solids: &Solids,
    ) -> Result<Self, String> {
        let variant = if variant.is_empty() {
            0
        } else {
            VARIANTS.iter().position(|v| *v == variant).ok_or_else(|| {
                format!(
                    "no story {variant} in the dying cell; there are {}",
                    VARIANTS.join(", ")
                )
            })?
        };
        let t = tissue();
        let mut groups = Groups::new(layer);
        {
            let mut materials = world.resource_mut::<Assets<Glow>>();
            for key in [
                Key::Neighbour,
                Key::NeighbourNucleus,
                Key::Cell,
                Key::Cytosol,
                Key::Nucleus,
                Key::Chromatin,
                Key::Break,
                Key::Mitochondrion,
                Key::Bax,
                Key::Bcl2,
                Key::CytochromeC,
                Key::Wheel,
                Key::WheelSpot,
                Key::Bleb,
                Key::Body,
                Key::Fragment,
                Key::Macrophage,
                Key::MacrophageNucleus,
            ] {
                let (glow, band) = look(key);
                groups.add(&mut materials, key, glow, band, Vec4::ZERO, true);
            }
        }
        let hidden = Transform::from_scale(Vec3::ZERO);
        let mut add =
            |world: &mut World, key, mesh: &Handle<Mesh>| groups.spawn(world, key, mesh, hidden);
        let (globe, ball, bead) = (&solids.globe, &solids.ball, &solids.bead);
        let neighbours = t
            .neighbours
            .iter()
            .map(|p| {
                (
                    add(world, Key::Neighbour, globe),
                    add(world, Key::NeighbourNucleus, ball),
                    *p,
                )
            })
            .collect();
        let cell = (
            add(world, Key::Cell, globe),
            add(world, Key::Cytosol, globe),
        );
        let chromatin = world.resource_mut::<Assets<Mesh>>().add(t.chromatin.mesh());
        let nucleus = (
            add(world, Key::Nucleus, globe),
            add(world, Key::Chromatin, &chromatin),
        );
        let breaks = t
            .breaks
            .iter()
            .map(|_| add(world, Key::Break, bead))
            .collect();
        let mitos = {
            let mut b = Builder::default();
            for m in &t.mitos {
                b.tube(
                    &m.points,
                    |s| 0.24 * (1.0 - (2.0 * s - 1.0).powi(8)).max(0.3),
                    10,
                );
            }
            let mesh = world.resource_mut::<Assets<Mesh>>().add(b.mesh());
            add(world, Key::Mitochondrion, &mesh)
        };
        let pore_count: usize = t.mitos.iter().map(|m| m.pores.len()).sum();
        let bax = (0..pore_count)
            .map(|_| add(world, Key::Bax, &solids.ring))
            .collect();
        let bcl2 = (0..pore_count)
            .map(|_| add(world, Key::Bcl2, bead))
            .collect();
        let leaks = t
            .leaks
            .iter()
            .map(|_| add(world, Key::CytochromeC, bead))
            .collect();
        let wheels = t
            .wheels
            .iter()
            .map(|_| {
                let spot = add(world, Key::WheelSpot, ball);
                let mut spokes = [Entity::PLACEHOLDER; 8];
                for s in spokes.iter_mut() {
                    *s = add(world, Key::Wheel, ball);
                }
                (spot, spokes)
            })
            .collect();
        let blebs = t
            .blebs
            .iter()
            .map(|_| add(world, Key::Bleb, ball))
            .collect();
        let bodies = t
            .bodies
            .iter()
            .map(|_| (add(world, Key::Body, ball), add(world, Key::Fragment, ball)))
            .collect();
        let macrophage = (
            add(world, Key::Macrophage, globe),
            add(world, Key::MacrophageNucleus, ball),
            add(world, Key::MacrophageNucleus, ball),
        );
        let arms = t
            .arms
            .iter()
            .map(|points| {
                let mut b = Builder::default();
                b.tube(points, |s| 1.1 - 0.7 * s, 10);
                let mesh = world.resource_mut::<Assets<Mesh>>().add(b.mesh());
                add(world, Key::Macrophage, &mesh)
            })
            .collect();
        let daughters = [
            (
                add(world, Key::Cell, globe),
                add(world, Key::Nucleus, globe),
            ),
            (
                add(world, Key::Cell, globe),
                add(world, Key::Nucleus, globe),
            ),
        ];
        Ok(Self {
            groups,
            neighbours,
            cell,
            nucleus,
            breaks,
            mitos,
            bax,
            bcl2,
            leaks,
            wheels,
            blebs,
            bodies,
            macrophage,
            arms,
            daughters,
            places: places(),
            variant,
        })
    }

    pub fn places(&self) -> &[Place] {
        &self.places
    }

    /// Whether the cell dies, as a normal cell does, or lives on.
    fn dies(&self) -> bool {
        self.variant == 0
    }

    pub fn locate(&self, name: &str, clock: f32, _focus: Vec3) -> Option<Vec3> {
        let t = tissue();
        Some(match name {
            "dying_cell" | "nucleus" => Vec3::ZERO,
            "dna_break" => t.breaks.first().copied()?,
            "mitochondrion" => t.mitos.first().map(|m| m.points[m.points.len() / 2])?,
            "bax_pore" => t.mitos.first().map(|m| m.pores[1])?,
            "cytochrome_c" => t.leaks.first().map(|l| l.0.lerp(l.1, 0.5))?,
            "apoptosome" => t.wheels.first().copied()?,
            "apoptotic_body" => t.bodies.first().copied()?,
            "macrophage" => self.eater_at(clock),
            "neighbour" => t.neighbours.first().copied()?,
            _ => return None,
        })
    }

    fn eater_at(&self, clock: f32) -> Vec3 {
        let come = ease((clock - COMES.0) / (COMES.1 - COMES.0));
        (EATER + Vec3::new(30.0, 0.0, 8.0)).lerp(EATER, come)
    }

    pub fn pick(&self, origin: Vec3, direction: Vec3, clock: f32, d: f32) -> Option<String> {
        let t = tissue();
        let mut best: Option<(f32, String)> = None;
        let mut consider = |centre: Vec3, r: f32, what: &dyn Fn() -> String| {
            let o = centre - origin;
            let along = o.dot(direction);
            let miss = o.length_squared() - along * along;
            if along > 0.0 && miss < r * r {
                let hit = along - (r * r - miss).sqrt();
                if best.as_ref().is_none_or(|(b, _)| hit < *b) {
                    best = Some((hit, what()));
                }
            }
        };
        if self.groups.seen(Key::Mitochondrion, d) {
            for m in &t.mitos {
                consider(m.points[m.points.len() / 2], 0.6, &|| {
                    if self.dies() {
                        "mitochondrion: where the cell's energy is made, and where it decides to die: BAX opens its outer membrane and cytochrome c leaks out".to_owned()
                    } else if self.variant == 1 {
                        "mitochondrion: BCL2 sits on it and holds BAX back, so it stays whole and the cell lives on despite its broken DNA".to_owned()
                    } else {
                        "mitochondrion: with p53 lost, nothing tells BAX to gather here; it stays whole".to_owned()
                    }
                });
            }
        }
        if self.groups.seen(Key::Wheel, d) {
            for w in &t.wheels {
                consider(*w, 0.02, &|| {
                    "apoptosome: seven APAF-1 round cytochrome c; it switches on caspase 9, which switches on caspase 3, the executioner".to_owned()
                });
            }
        }
        if self.groups.seen(Key::Neighbour, d) {
            for n in &t.neighbours {
                consider(*n, 5.0, &|| {
                    "neighbour: a healthy cell; it will close the gap the dying one leaves"
                        .to_owned()
                });
            }
            consider(self.eater_at(clock), 7.0, &|| {
                "macrophage: an immune cell that eats apoptotic bodies whole, so nothing spills and nothing inflames".to_owned()
            });
            consider(Vec3::ZERO, 5.5, &|| {
                if clock < P53.0 {
                    "the cell whose DNA has just been broken".to_owned()
                } else if self.dies() && clock < BREAK_UP.0 {
                    "the dying cell: p53 has turned on BAX, the mitochondria are leaking, the caspases are at work".to_owned()
                } else if self.dies() {
                    "what is left of the cell: apoptotic bodies, wrapped in membrane".to_owned()
                } else {
                    "the cell that should have died: it lives on with its broken DNA".to_owned()
                }
            });
        }
        best.map(|(_, w)| w)
    }

    pub fn update(&mut self, tick: &Tick, things: &mut Things, glows: &mut Assets<Glow>) {
        let t = tissue();
        let h = tick.clock;
        let dies = self.dies();
        self.groups.update(tick, picks, |_, s| s, things, glows);
        // p53 tints the nucleus; the caspases set the cytosol alight.
        let p53 = if self.variant == 2 {
            0.0
        } else {
            ease((h - P53.0) / (P53.1 - P53.0))
        };
        self.groups.restyle(Key::Nucleus, p53, glows, |m| {
            let c = Vec3::new(0.4, 0.55, 1.0).lerp(Vec3::new(1.0, 0.35, 0.45), p53);
            m.rim = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.55);
            m.body = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.06 + 0.08 * p53);
        });
        let caspases = if dies {
            ease((h - CASPASES.0) / (CASPASES.1 - CASPASES.0))
        } else {
            0.0
        };
        self.groups.restyle(Key::Cytosol, caspases, glows, |m| {
            let c = Vec3::new(0.25, 0.55, 1.0).lerp(Vec3::new(0.85, 0.2, 1.0), caspases);
            m.body = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.02 + 0.06 * caspases);
        });

        let life = tick.life;
        let shown = |g: &Groups<Key>, k| !g.hidden(k);
        let cells = shown(&self.groups, Key::Neighbour);
        let inside = shown(&self.groups, Key::Mitochondrion);
        let put = |things: &mut Things, e: Entity, on: bool, tr: Transform| {
            set_shown(things, e, on);
            if on && let Ok((mut t, _)) = things.get_mut(e) {
                *t = tr;
            }
        };
        // Neighbours close the gap once the bodies are gone.
        let close = if dies {
            ease((h - CLOSE.0) / (CLOSE.1 - CLOSE.0))
        } else {
            0.0
        };
        for (body, core, p) in &self.neighbours {
            let at = *p * (1.0 - 0.18 * close);
            let stretch = Vec3::new(5.8, 4.4, 5.8) * (1.0 + 0.12 * close);
            put(
                things,
                *body,
                cells,
                Transform::from_translation(at).with_scale(stretch),
            );
            put(
                things,
                *core,
                cells,
                Transform::from_translation(at).with_scale(Vec3::splat(2.2)),
            );
        }
        // The cell shrinks, then is gone into its bodies; or, living on,
        // divides into two.
        let shrink = if dies {
            ease((h - SHRINK.0) / (SHRINK.1 - SHRINK.0))
        } else {
            0.0
        };
        let broken = if dies {
            ease((h - BREAK_UP.0) / (BREAK_UP.1 - BREAK_UP.0))
        } else {
            0.0
        };
        let divide = if dies {
            0.0
        } else {
            ease((h - DIVIDES.0) / (DIVIDES.1 - DIVIDES.0))
        };
        let whole = 1.0 - broken;
        let size = 1.0 - 0.3 * shrink;
        let one = divide < 0.02;
        put(
            things,
            self.cell.0,
            cells && whole > 0.02 && one,
            Transform::from_scale(CELL * size),
        );
        put(
            things,
            self.cell.1,
            cells && whole > 0.02 && one,
            Transform::from_scale(CELL * size * 0.97),
        );
        // The nucleus condenses, then breaks up into the bodies.
        let condense = 1.0 - 0.45 * shrink;
        put(
            things,
            self.nucleus.0,
            inside && whole > 0.02 && one,
            Transform::from_scale(Vec3::splat(NUCLEUS * condense)),
        );
        put(
            things,
            self.nucleus.1,
            inside && whole > 0.02 && one,
            Transform::from_scale(Vec3::splat(condense)),
        );
        for (k, (e, p)) in self.breaks.iter().zip(&t.breaks).enumerate() {
            // The breaks: flashes at first, then marks that stay while
            // nothing mends them.
            let flash = (-(h.max(0.0)) * 3.0).exp();
            let on = inside && h >= 0.0 && whole > 0.02;
            let pulse = 1.0 + 0.25 * (life * 3.0 + k as f32).sin();
            let at = if one {
                *p * condense
            } else {
                *p * 0.5 + Vec3::X * (if k % 2 == 0 { 1.0 } else { -1.0 }) * 6.0 * divide
            };
            put(
                things,
                *e,
                on,
                Transform::from_translation(at)
                    .with_scale(Vec3::splat(0.1 * (1.0 + 3.0 * flash) * pulse)),
            );
        }
        put(
            things,
            self.mitos,
            inside && whole > 0.02 && one,
            Transform::from_scale(Vec3::splat(size)),
        );
        // BAX pores, or BCL2 holding them back.
        let pores = if dies {
            ease((h - BAX.0) / (BAX.1 - BAX.0))
        } else {
            0.0
        };
        let held = if self.variant == 1 {
            ease((h - P53.0) / 1.5)
        } else {
            0.0
        };
        let mut n = 0;
        for m in &t.mitos {
            for p in &m.pores {
                let normal = (p - m.points[m.points.len() / 2]).normalize_or(Vec3::Y);
                put(
                    things,
                    self.bax[n],
                    inside && pores > 0.05 && whole > 0.02,
                    Transform::from_translation(*p * size)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, normal))
                        .with_scale(Vec3::new(0.08, 0.03, 0.08) * pores),
                );
                put(
                    things,
                    self.bcl2[n],
                    inside && held > 0.05 && one,
                    Transform::from_translation(*p * size).with_scale(Vec3::splat(0.06 * held)),
                );
                n += 1;
            }
        }
        // Cytochrome c leaking out and drifting.
        for ((from, to, when), e) in t.leaks.iter().zip(&self.leaks) {
            let out = if dies { ease((h - when) / 1.2) } else { 0.0 };
            let drift = Vec3::new(
                (life * 0.9 + when * 7.0).sin(),
                (life * 0.7 + when * 3.0).cos(),
                (life * 0.8 + when * 5.0).sin(),
            ) * 0.08
                * out;
            put(
                things,
                *e,
                inside && out > 0.02 && whole > 0.02,
                Transform::from_translation((from.lerp(*to, out) + drift) * size)
                    .with_scale(Vec3::splat(0.035)),
            );
        }
        // Apoptosomes building, wheel by wheel.
        let built = if dies {
            ease((h - WHEELS.0) / (WHEELS.1 - WHEELS.0))
        } else {
            0.0
        };
        for (k, ((spot, spokes), at)) in self.wheels.iter().zip(&t.wheels).enumerate() {
            let here = (built * APOPTOSOMES as f32 - k as f32).clamp(0.0, 1.0);
            let on = here > 0.02 && whole > 0.02;
            let at = *at * size;
            put(
                things,
                *spot,
                on && cells,
                Transform::from_translation(at).with_scale(Vec3::splat(0.18 * here)),
            );
            let spin = Quat::from_rotation_y(life * 0.3 + k as f32);
            for (s, e) in spokes.iter().enumerate() {
                let (offset, r) = if s == 7 {
                    (Vec3::ZERO, 0.0035)
                } else {
                    let a = s as f32 / 7.0 * std::f32::consts::TAU;
                    (
                        spin * Vec3::new(a.cos(), 0.0, a.sin()) * 0.0095 * here,
                        0.0028,
                    )
                };
                put(
                    things,
                    *e,
                    on,
                    Transform::from_translation(at + offset).with_scale(Vec3::new(
                        r * 1.3,
                        r * 0.7,
                        r,
                    )),
                );
            }
        }
        // Blebs bubbling at the surface.
        let blebbing = if dies {
            ease((h - BLEBS.0) / (BLEBS.1 - BLEBS.0))
        } else {
            0.0
        };
        for (k, (e, p)) in self.blebs.iter().zip(&t.blebs).enumerate() {
            let swell = blebbing * (0.6 + 0.4 * (life * 1.7 + k as f32 * 1.3).sin().abs());
            put(
                things,
                *e,
                cells && blebbing > 0.02 && whole > 0.02,
                Transform::from_translation(*p * size).with_scale(Vec3::splat(1.3 * swell)),
            );
        }
        // The bodies: drifting a little apart, then eaten.
        let eaten = if dies {
            ease((h - EATS.0) / (EATS.1 - EATS.0))
        } else {
            0.0
        };
        let eater = self.eater_at(h);
        for (k, ((body, frag), p)) in self.bodies.iter().zip(&t.bodies).enumerate() {
            let swallowed = (eaten * 1.4 - k as f32 * 0.06).clamp(0.0, 1.0);
            let loose = *p * (1.0 + 0.25 * broken);
            let inside_eater = eater + (*p * 0.35);
            let at = loose.lerp(inside_eater, swallowed);
            let gone = 1.0 - ease((swallowed - 0.7) / 0.3);
            let on = cells && broken > 0.02 && gone > 0.02;
            put(
                things,
                *body,
                on,
                Transform::from_translation(at).with_scale(Vec3::splat(1.7 * broken * gone)),
            );
            put(
                things,
                *frag,
                on,
                Transform::from_translation(at).with_scale(Vec3::splat(0.7 * broken * gone)),
            );
        }
        // The macrophage: coming, reaching out, eating; gone on its way.
        let come = if dies {
            ease((h - COMES.0) / (COMES.1 - COMES.0))
        } else {
            0.0
        };
        let leave = if dies { ease((h - EATS.1) / 2.0) } else { 0.0 };
        let eater = eater + Vec3::new(20.0, 0.0, 10.0) * leave;
        let here = cells && come > 0.01 && leave < 0.99;
        let squash = Vec3::new(7.5, 5.0, 7.0) * (1.0 + 0.05 * (life * 0.8).sin());
        put(
            things,
            self.macrophage.0,
            here,
            Transform::from_translation(eater).with_scale(squash),
        );
        put(
            things,
            self.macrophage.1,
            here,
            Transform::from_translation(eater + Vec3::new(1.2, 0.4, -0.8))
                .with_scale(Vec3::new(2.4, 1.6, 1.8)),
        );
        put(
            things,
            self.macrophage.2,
            here,
            Transform::from_translation(eater + Vec3::new(-0.6, 0.2, 1.4))
                .with_scale(Vec3::new(2.0, 1.5, 1.6)),
        );
        let reach = ease((h - EATS.0 + 0.5) / 1.5) * (1.0 - ease((h - EATS.1 + 0.8) / 1.0));
        for e in &self.arms {
            put(
                things,
                *e,
                here && dies && reach > 0.02,
                Transform::from_translation((eater - EATER) + EATER * (1.0 - reach))
                    .with_scale(Vec3::splat(reach.max(0.001))),
            );
        }
        // The survivor divides, its broken DNA in both daughters.
        for (k, (body, core)) in self.daughters.iter().enumerate() {
            let side = if k == 0 { 1.0 } else { -1.0 };
            let at = Vec3::X * side * 4.0 * divide;
            let on = cells && !dies && divide > 0.02;
            put(
                things,
                *body,
                on,
                Transform::from_translation(at).with_scale(CELL * Vec3::new(0.72, 0.9, 0.8)),
            );
            put(
                things,
                *core,
                on && inside,
                Transform::from_translation(at).with_scale(Vec3::splat(NUCLEUS * 0.8)),
            );
        }
    }

    pub fn entities(self) -> Vec<Entity> {
        self.groups.entities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cell_dies_in_the_textbooks_order_and_its_places_are_offered() {
        let steps = [
            P53, BAX, LEAK, WHEELS, CASPASES, SHRINK, BLEBS, BREAK_UP, COMES, EATS, CLOSE,
        ];
        for w in steps.windows(2) {
            assert!(w[0].0 <= w[1].0, "{:?} before {:?}", w[0], w[1]);
        }
        let t = tissue();
        assert_eq!(t.mitos.len(), MITOCHONDRIA);
        assert!(t.neighbours.iter().all(|n| n.distance(EATER) > 8.0));
        let names: Vec<&str> = places().iter().map(|p| p.name).collect();
        assert_eq!(names, PLACES.to_vec());
    }
}
