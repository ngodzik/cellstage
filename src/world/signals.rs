//! A cell reading a growth factor, from its surface to its genes: the
//! first two of Hanahan and Weinberg's hallmarks, the growth signals a
//! cancer cell makes for itself and the stop signals it no longer hears.
//!
//! EGF arrives at the cell's surface and binds its receptor, EGFR, which
//! pairs up and phosphorylates its tails. The adaptor GRB2 and SOS come to
//! the tails, and SOS switches RAS on, from GDP to GTP, at the inner face
//! of the membrane. RAS calls RAF, RAF switches MEK on, MEK switches ERK
//! on, and ERK goes into the nucleus, where it turns on the gene for
//! cyclin D. Cyclin D with CDK4 phosphorylates RB, which lets go of E2F,
//! and E2F turns on the genes for copying the DNA: the cell is committed
//! to divide. A stop signal makes p16, which holds CDK4 back: RB stays on
//! E2F, and the cell does not divide.
//!
//! The variants are the hallmarks: RAS stuck on (as a G12V mutation leaves
//! it) runs the cascade with no growth factor at all; p16 lost lets RB go
//! despite the stop signal.
//!
//! Sizes are the molecules' own, in micrometres (EGFR's outer part about
//! 10 nm tall, the membrane 4 nm thick, a nuclear pore 0.1 µm across); the
//! cell is 16 µm across. The clock is in minutes after the growth factor
//! arrives; the order and rough times of the events are those of the
//! textbooks, the molecules' looks are simplified.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::engine::{Band, Groups, NEVER, NONE, Solids, Things, Tick, ease, set_shown};
use super::film::{Eye, Place};
use super::glow::Glow;
use super::shapes::{Builder, Path, smooth};
use super::skin::Dice;

/// The stories the cell can tell.
pub const VARIANTS: [&str; 4] = ["normal", "ras_stuck", "stop_signal", "p16_lost"];

/// The cell's radius, and its nucleus's middle and radius.
const CELL: f32 = 8.0;
const NUCLEUS: f32 = 3.2;
const NUCLEUS_AT: Vec3 = Vec3::new(0.0, -8.0, 0.0);
/// Where the genes are, just inside the nucleus's top.
const GENES: Vec3 = Vec3::new(0.0, -6.3, 0.0);
/// How many receptors in the patch, ligands, RAS, ERK.
const RECEPTORS: usize = 28;
const ERKS: usize = 160;
/// Half the membrane's thickness.
const LEAFLET: f32 = 0.002;

/// When things happen, in minutes after the growth factor arrives.
const BIND: (f32, f32) = (0.0, 2.0);
const RAS_ON: (f32, f32) = (2.0, 5.0);
const ERK_ON: (f32, f32) = (5.0, 10.0);
const ERK_IN: (f32, f32) = (10.0, 30.0);
const CYCLIN_D: (f32, f32) = (30.0, 120.0);
const P16: (f32, f32) = (60.0, 150.0);
const RB_OFF: (f32, f32) = (150.0, 320.0);
const S_GENES: (f32, f32) = (320.0, 600.0);

const MOLECULES: Band = Band::new(NONE, (0.5, 1.4));
const MEMBRANE: Band = Band::new((0.004, 0.012), (1.5, 4.0));
const CYTOPLASM: Band = Band::new((0.05, 0.15), (3.0, 8.0));
const WHOLE: Band = Band::new((0.6, 2.0), NEVER);
const NUCLEAR: Band = Band::new((0.3, 0.9), NEVER);
const GENE_BAND: Band = Band::new(NONE, (0.6, 1.6));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Cell,
    Cytoplasm,
    Nucleus,
    Pore,
    Chromatin,
    Membrane,
    ReceptorOut,
    ReceptorIn,
    Tail,
    Phospho,
    Egf,
    Grb2,
    RasOff,
    RasOn,
    Raf,
    ErkOff,
    ErkOn,
    Pulse,
    Dna,
    Rb,
    E2f,
    Cdk4,
    P16,
    Rna,
    Tgf,
    Dots,
}

fn look(key: Key) -> (Glow, Band) {
    let g = Glow::new;
    match key {
        Key::Cell => (g([0.3, 0.75, 1.0], 0.5, 0.0).edges(3.5), WHOLE),
        Key::Cytoplasm => (
            g([0.2, 0.5, 0.9], 0.0, 0.0).faces([0.25, 0.6, 1.0], 0.02),
            WHOLE,
        ),
        Key::Nucleus => (
            g([0.45, 0.55, 1.0], 0.5, 0.0)
                .faces([0.3, 0.4, 1.0], 0.04)
                .edges(3.0),
            NUCLEAR,
        ),
        Key::Pore => (
            g([0.85, 0.92, 1.0], 0.6, 0.0)
                .faces([0.7, 0.8, 1.0], 0.15)
                .edges(1.5),
            Band::new((0.04, 0.12), (8.0, 20.0)),
        ),
        Key::Chromatin => (
            g([0.35, 0.45, 1.0], 0.25, 0.0)
                .faces([0.3, 0.35, 1.0], 0.03)
                .edges(1.6)
                .varied(0.8),
            Band::new((0.15, 0.4), (14.0, 35.0)),
        ),
        Key::Membrane => (
            g([0.35, 0.8, 1.0], 0.5, 0.0)
                .faces([0.3, 0.7, 1.0], 0.05)
                .edges(3.0)
                .varied(0.01),
            MEMBRANE,
        ),
        Key::ReceptorOut => (
            g([0.3, 1.0, 0.65], 0.7, 0.0)
                .faces([0.25, 0.9, 0.55], 0.12)
                .edges(1.7),
            MOLECULES,
        ),
        Key::ReceptorIn => (
            g([0.3, 1.0, 0.65], 0.6, 0.0)
                .faces([0.25, 0.9, 0.55], 0.1)
                .edges(1.7),
            MOLECULES,
        ),
        Key::Tail => (g([0.5, 0.9, 0.7], 0.5, 0.0).edges(1.6), MOLECULES),
        Key::Phospho => (
            g([1.0, 0.9, 0.3], 1.0, 0.0)
                .faces([1.0, 0.85, 0.3], 0.6)
                .edges(1.4),
            MOLECULES,
        ),
        Key::Egf => (
            g([1.0, 0.75, 0.25], 0.8, 0.0)
                .faces([1.0, 0.65, 0.2], 0.3)
                .edges(1.5),
            MOLECULES,
        ),
        Key::Grb2 => (
            g([0.85, 0.55, 1.0], 0.8, 0.0)
                .faces([0.75, 0.45, 1.0], 0.2)
                .edges(1.6),
            MOLECULES,
        ),
        Key::RasOff => (g([0.35, 0.45, 0.9], 0.45, 0.0).edges(1.8), MOLECULES),
        Key::RasOn => (
            g([0.4, 1.0, 0.3], 1.1, 0.0)
                .faces([0.35, 1.0, 0.25], 0.45)
                .edges(1.4),
            MOLECULES,
        ),
        Key::Raf => (
            g([1.0, 0.45, 0.2], 0.9, 0.0)
                .faces([1.0, 0.4, 0.15], 0.3)
                .edges(1.5),
            MOLECULES,
        ),
        Key::ErkOff => (g([0.35, 0.4, 0.75], 0.35, 0.0).edges(1.8), CYTOPLASM),
        Key::ErkOn => (
            g([0.3, 1.0, 0.6], 1.0, 0.0)
                .faces([0.3, 1.0, 0.55], 0.45)
                .edges(1.4),
            CYTOPLASM,
        ),
        // The signal as a whole, seen from afar: light running from the
        // surface to the nucleus.
        Key::Pulse => (
            g([0.35, 1.0, 0.6], 0.8, 0.0)
                .faces([0.3, 1.0, 0.55], 0.5)
                .edges(1.4),
            Band::new((0.8, 2.5), NEVER),
        ),
        Key::Dna => (
            g([0.45, 0.75, 1.0], 0.6, 0.0)
                .faces([0.35, 0.65, 1.0], 0.1)
                .edges(1.7),
            GENE_BAND,
        ),
        Key::Rb => (
            g([1.0, 0.35, 0.4], 0.8, 0.0)
                .faces([1.0, 0.3, 0.35], 0.18)
                .edges(1.6),
            GENE_BAND,
        ),
        Key::E2f => (
            g([0.3, 0.9, 1.0], 0.8, 0.0)
                .faces([0.25, 0.8, 1.0], 0.2)
                .edges(1.6),
            GENE_BAND,
        ),
        Key::Cdk4 => (
            g([1.0, 0.65, 0.2], 0.8, 0.0)
                .faces([1.0, 0.55, 0.15], 0.2)
                .edges(1.6),
            GENE_BAND,
        ),
        Key::P16 => (
            g([1.0, 0.2, 0.25], 1.0, 0.0)
                .faces([1.0, 0.15, 0.2], 0.4)
                .edges(1.4),
            GENE_BAND,
        ),
        Key::Rna => (
            g([1.0, 0.9, 0.5], 0.7, 0.0)
                .faces([1.0, 0.85, 0.4], 0.2)
                .edges(1.6),
            GENE_BAND,
        ),
        Key::Tgf => (
            g([0.85, 0.35, 1.0], 0.8, 0.0)
                .faces([0.75, 0.3, 1.0], 0.25)
                .edges(1.5),
            MOLECULES,
        ),
        // Receptors over the whole cell, seen from afar.
        Key::Dots => (
            g([0.3, 1.0, 0.65], 0.6, 0.0)
                .faces([0.3, 1.0, 0.6], 0.25)
                .edges(1.5),
            Band::new((1.5, 4.0), NEVER),
        ),
    }
}

fn picks(name: &str, key: Key) -> bool {
    match name {
        "receptors" | "egfr" => matches!(
            key,
            Key::ReceptorOut | Key::ReceptorIn | Key::Tail | Key::Phospho | Key::Dots
        ),
        "growth_factor" | "egf" => key == Key::Egf,
        "ras" => matches!(key, Key::RasOff | Key::RasOn | Key::Grb2),
        "cascade" | "erk" => matches!(key, Key::Raf | Key::ErkOff | Key::ErkOn | Key::Pulse),
        "rb" | "brake" => matches!(key, Key::Rb | Key::E2f | Key::Cdk4 | Key::P16),
        "p16" => matches!(key, Key::P16 | Key::Cdk4),
        "genes" => matches!(key, Key::Dna | Key::Rna | Key::E2f),
        "stop_signal" => matches!(key, Key::Tgf | Key::P16),
        "membrane" => matches!(key, Key::Membrane | Key::Cell),
        "nucleus" => matches!(key, Key::Nucleus | Key::Pore | Key::Chromatin),
        _ => false,
    }
}

const HIGHLIGHTS: [&str; 11] = [
    "receptors",
    "growth_factor",
    "ras",
    "cascade",
    "rb",
    "p16",
    "genes",
    "stop_signal",
    "membrane",
    "nucleus",
    "erk",
];

pub fn highlights() -> Vec<&'static str> {
    HIGHLIGHTS.to_vec()
}

pub const PLACES: [&str; 8] = [
    "cell", "membrane", "receptor", "ras", "cascade", "pore", "genes", "nucleus",
];

const THINGS: [&str; 14] = [
    "egf",
    "egfr",
    "phosphate",
    "grb2_sos",
    "ras",
    "raf",
    "erk",
    "nuclear_pore",
    "rb",
    "e2f",
    "cyclin_d_cdk4",
    "p16",
    "cyclin_d_gene",
    "s_phase_genes",
];

pub fn things() -> Vec<&'static str> {
    THINGS.to_vec()
}

pub fn clock_label(t: f32) -> Option<String> {
    if t < 0.0 {
        None
    } else if t < 60.0 {
        Some(format!("minute {t:.0}"))
    } else {
        Some(format!("hour {:.1}", t / 60.0))
    }
}

/// A receptor: where it stands on the membrane, its partner, when it binds.
#[derive(Debug, Clone, Copy)]
struct Receptor {
    at: Vec3,
    partner: Option<usize>,
    binds: f32,
}

/// The cell, built once.
struct Cell {
    receptors: Vec<Receptor>,
    /// Where each RAS sits, under which receptor pair, and when it is on.
    ras: Vec<(Vec3, f32)>,
    /// Each ERK: where it waits, its way to the nucleus, when it goes.
    erks: Vec<(Vec3, Path, f32)>,
    pores: Vec<Transform>,
    chromatin: Builder,
    /// The paths the signal runs along, seen from afar.
    pulses: Vec<Path>,
    /// The genes: the cyclin D gene's place on the DNA, and the E2F sites.
    dna: Path,
    cyclin_gene: Vec3,
    sites: Vec<Vec3>,
    /// Where the stop signal's receptors are.
    tgf: Vec<Vec3>,
    dots: Vec<Vec3>,
}

/// A point on the cell's surface above `x`, `z`, and the way out there.
fn surface(x: f32, z: f32) -> (Vec3, Vec3) {
    let c = Vec3::new(0.0, -CELL, 0.0);
    let up = (Vec3::new(x, 0.0, z) - c).normalize();
    (c + up * CELL, up)
}

impl Cell {
    fn build(seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let mut receptors: Vec<Receptor> = Vec::new();
        let mut tries = 0;
        while receptors.len() < RECEPTORS && tries < 5000 {
            tries += 1;
            let d = dice.disc(0.42);
            let (at, _) = surface(d.x, d.y);
            if receptors.iter().any(|r| r.at.distance(at) < 0.045) {
                continue;
            }
            receptors.push(Receptor {
                at,
                partner: None,
                binds: 0.0,
            });
        }
        // The first one at the middle, where the camera looks.
        if let Some(r) = receptors.first_mut() {
            r.at = Vec3::ZERO;
        }
        // Pairs: each with its nearest free neighbour.
        for i in 0..receptors.len() {
            if receptors[i].partner.is_some() {
                continue;
            }
            let near = (0..receptors.len())
                .filter(|j| *j != i && receptors[*j].partner.is_none())
                .min_by(|a, b| {
                    receptors[*a]
                        .at
                        .distance(receptors[i].at)
                        .total_cmp(&receptors[*b].at.distance(receptors[i].at))
                });
            if let Some(j) = near {
                receptors[i].partner = Some(j);
                receptors[j].partner = Some(i);
                let binds =
                    BIND.0 + dice.range(0.0, BIND.1 - BIND.0) * (i as f32 / RECEPTORS as f32 + 0.2);
                receptors[i].binds = binds;
                receptors[j].binds = binds;
            }
        }
        let ras = receptors
            .iter()
            .map(|r| {
                let (_, up) = surface(r.at.x, r.at.z);
                let side = Vec3::new(dice.range(-1.0, 1.0), 0.0, dice.range(-1.0, 1.0))
                    .normalize_or(Vec3::X);
                (
                    r.at - up * (LEAFLET * 2.5) + side * 0.012,
                    r.binds + dice.range(RAS_ON.0, RAS_ON.1),
                )
            })
            .collect();
        // Nuclear pores, about ten in a square micrometre.
        let mut pores = Vec::new();
        let area = 4.0 * std::f32::consts::PI * NUCLEUS * NUCLEUS;
        let n = (area * 10.0) as usize;
        for k in 0..n {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / n as f32;
            let a = k as f32 * 2.399_963;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let d = Vec3::new(r * a.cos(), y, r * a.sin());
            pores.push(
                Transform::from_translation(NUCLEUS_AT + d * NUCLEUS)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, d))
                    .with_scale(Vec3::new(0.05, 0.016, 0.05)),
            );
        }
        let top_pores: Vec<Vec3> = pores
            .iter()
            .map(|t| t.translation)
            .filter(|p| p.y > NUCLEUS_AT.y + NUCLEUS * 0.9)
            .collect();
        // ERK waiting in the cytoplasm under the patch, each with its way
        // to one of the pores at the top of the nucleus.
        let mut erks = Vec::new();
        for _ in 0..ERKS {
            let d = dice.disc(0.5);
            let depth = dice.range(0.02, 0.5);
            let (top, up) = surface(d.x, d.y);
            let at = top - up * depth;
            let pore = top_pores
                .get((dice.next() as usize) % top_pores.len().max(1))
                .copied()
                .unwrap_or(GENES);
            let mid =
                (at + pore) * 0.5 + Vec3::new(dice.range(-0.6, 0.6), 0.0, dice.range(-0.6, 0.6));
            let inside = pore + (GENES - pore).normalize_or(Vec3::NEG_Y) * 0.3;
            let path = Path::new(smooth(&[at, mid, pore, inside], 12));
            erks.push((at, path, ERK_IN.0 + dice.range(-5.0, ERK_IN.1 - ERK_IN.0)));
        }
        // Chromatin through the nucleus.
        let mut chromatin = Builder::default();
        for _ in 0..30 {
            let mut p = NUCLEUS_AT + dice.direction() * NUCLEUS * 0.5;
            let mut heading = dice.direction();
            let mut points = Vec::new();
            for _ in 0..200 {
                heading = (heading + dice.direction() * 0.45).normalize();
                let next = p + heading * 0.07;
                if next.distance(NUCLEUS_AT) < NUCLEUS - 0.15 {
                    p = next;
                } else {
                    heading = (NUCLEUS_AT - p).normalize();
                }
                points.push(p);
            }
            chromatin.tube(&smooth(&points, 2), |_| 0.025, 5);
        }
        // The signal's paths, seen from afar: from the patch round the
        // surface, down through the cytoplasm, to the nucleus.
        let mut pulses = Vec::new();
        for k in 0..14 {
            let a = k as f32 / 14.0 * std::f32::consts::TAU;
            let d = Vec2::from_angle(a) * dice.range(0.2, 2.5);
            let (top, up) = surface(d.x, d.y);
            let start = top - up * 0.15;
            let end = NUCLEUS_AT + (start - NUCLEUS_AT).normalize() * (NUCLEUS + 0.05);
            let mid =
                (start + end) * 0.5 + Vec3::new(dice.range(-1.0, 1.0), 0.0, dice.range(-1.0, 1.0));
            pulses.push(Path::new(smooth(&[start, mid, end], 14)));
        }
        // A stretch of DNA under the nuclear envelope: the cyclin D gene at
        // one end, three genes E2F turns on along the rest.
        let dna_points: Vec<Vec3> = (0..=40)
            .map(|i| {
                let x = -0.16 + i as f32 * 0.008;
                GENES + Vec3::new(x, 0.01 * (x * 30.0).sin(), 0.012 * (x * 21.0).cos())
            })
            .collect();
        let dna = Path::new(smooth(&dna_points, 3));
        let cyclin_gene = dna.at(0.12).0;
        let sites = [0.45, 0.65, 0.85].iter().map(|f| dna.at(*f).0).collect();
        let tgf = (0..6)
            .map(|_| {
                let d = dice.disc(0.4) + Vec2::new(0.25, 0.25);
                surface(d.x, d.y).0
            })
            .collect();
        let mut dots = Vec::new();
        for k in 0..900 {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / 900.0;
            let a = k as f32 * 2.399_963;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let d = Vec3::new(r * a.cos(), y, r * a.sin());
            dots.push(Vec3::new(0.0, -CELL, 0.0) + d * CELL);
        }
        Self {
            receptors,
            ras,
            erks,
            pores,
            chromatin,
            pulses,
            dna,
            cyclin_gene,
            sites,
            tgf,
            dots,
        }
    }
}

fn cell() -> &'static Cell {
    static CELL_BUILT: std::sync::OnceLock<Cell> = std::sync::OnceLock::new();
    CELL_BUILT.get_or_init(|| Cell::build(21))
}

fn places() -> Vec<Place> {
    let c = cell();
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
    let pore = c
        .pores
        .iter()
        .map(|t| t.translation)
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap_or(NUCLEUS_AT);
    vec![
        place(
            "cell",
            "the whole cell, 16 µm across, its nucleus inside, receptors over its surface",
            Vec3::new(0.0, -6.0, 0.0),
            34.0,
            0.6,
            0.35,
        ),
        place(
            "membrane",
            "a patch of the cell's surface: EGF receptors standing in the membrane, growth factor arriving",
            Vec3::new(0.0, 0.0, 0.0),
            0.32,
            0.5,
            0.32,
        ),
        place(
            "receptor",
            "one pair of EGF receptors up close, growth factor bound, their tails phosphorylated",
            Vec3::new(0.0, 0.004, 0.0),
            0.07,
            0.7,
            0.15,
        ),
        place(
            "ras",
            "the inner face of the membrane: GRB2 and SOS at the receptors' tails, RAS switching on, RAF arriving",
            Vec3::new(0.0, -0.012, 0.0),
            0.08,
            0.9,
            -0.25,
        ),
        place(
            "cascade",
            "the cytoplasm under the membrane: ERK switched on, setting off for the nucleus",
            Vec3::new(0.0, -0.3, 0.0),
            1.1,
            0.6,
            0.1,
        ),
        place(
            "pore",
            "a nuclear pore, ERK passing through into the nucleus",
            pore,
            0.3,
            0.4,
            0.6,
        ),
        place(
            "genes",
            "the genes inside the nucleus: the cyclin D gene, and RB holding E2F on the genes for copying DNA",
            GENES,
            0.2,
            0.2,
            0.15,
        ),
        place(
            "nucleus",
            "the nucleus, its envelope and pores, chromatin inside",
            NUCLEUS_AT,
            10.0,
            0.8,
            0.3,
        ),
    ]
}

/// One receptor's parts, to move as it pairs.
struct ReceptorParts {
    outside: Entity,
    stalk: Entity,
    inside: Entity,
    tail: Entity,
    phospho: [Entity; 3],
    egf: Entity,
    grb2: Entity,
}

/// The cell, as shown in a panel.
pub struct SignalWorld {
    groups: Groups<Key>,
    receptors: Vec<ReceptorParts>,
    ras: Vec<(Entity, Entity, Entity)>,
    erks: Vec<(Entity, Entity)>,
    pulses: Vec<(Entity, usize, f32)>,
    sites: Vec<(Entity, Entity, Entity, Entity, Entity)>,
    cyclin_rna: Vec<Entity>,
    s_rna: Vec<(usize, Entity)>,
    tgf: Vec<(Entity, Entity)>,
    places: Vec<Place>,
    variant: usize,
    placed: Option<f32>,
}

impl SignalWorld {
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
                    "no story {variant} in the cell; there are {}",
                    VARIANTS.join(", ")
                )
            })?
        };
        let c = cell();
        let mut groups = Groups::new(layer);
        {
            let mut materials = world.resource_mut::<Assets<Glow>>();
            for key in [
                Key::Cell,
                Key::Cytoplasm,
                Key::Nucleus,
                Key::Pore,
                Key::Chromatin,
                Key::Membrane,
                Key::ReceptorOut,
                Key::ReceptorIn,
                Key::Tail,
                Key::Phospho,
                Key::Egf,
                Key::Grb2,
                Key::RasOff,
                Key::RasOn,
                Key::Raf,
                Key::ErkOff,
                Key::ErkOn,
                Key::Pulse,
                Key::Dna,
                Key::Rb,
                Key::E2f,
                Key::Cdk4,
                Key::P16,
                Key::Rna,
                Key::Tgf,
                Key::Dots,
            ] {
                let (glow, band) = look(key);
                let spot = match key {
                    Key::Membrane => Vec4::new(0.0, 0.0, 0.0, 0.55),
                    Key::ErkOff | Key::ErkOn => Vec4::new(0.0, 0.0, 0.0, 0.0),
                    _ => Vec4::ZERO,
                };
                groups.add(&mut materials, key, glow, band, spot, true);
            }
        }
        let hidden = Transform::from_scale(Vec3::ZERO);
        let (ball, bead, rod, ring, globe) = (
            &solids.ball,
            &solids.bead,
            &solids.rod,
            &solids.ring,
            &solids.globe,
        );
        let still = |groups: &mut Groups<Key>, world: &mut World, key, mesh: &Handle<Mesh>, t| {
            groups.spawn(world, key, mesh, t)
        };
        still(
            &mut groups,
            world,
            Key::Cell,
            globe,
            Transform::from_xyz(0.0, -CELL, 0.0).with_scale(Vec3::splat(CELL)),
        );
        still(
            &mut groups,
            world,
            Key::Cytoplasm,
            globe,
            Transform::from_xyz(0.0, -CELL, 0.0).with_scale(Vec3::splat(CELL * 0.98)),
        );
        still(
            &mut groups,
            world,
            Key::Nucleus,
            globe,
            Transform::from_translation(NUCLEUS_AT).with_scale(Vec3::splat(NUCLEUS)),
        );
        for p in &c.pores {
            still(&mut groups, world, Key::Pore, ring, *p);
        }
        let chromatin = world.resource_mut::<Assets<Mesh>>().add(c.chromatin.mesh());
        still(
            &mut groups,
            world,
            Key::Chromatin,
            &chromatin,
            Transform::default(),
        );
        // The membrane: two leaflets, curving with the cell.
        let mut membrane = Builder::default();
        for side in [LEAFLET, -LEAFLET] {
            membrane.sheet(0.6, 0.008, 0.6, |x, z| surface(x, z).0.y + side);
        }
        let membrane = world.resource_mut::<Assets<Mesh>>().add(membrane.mesh());
        still(
            &mut groups,
            world,
            Key::Membrane,
            &membrane,
            Transform::default(),
        );
        for d in &c.dots {
            still(
                &mut groups,
                world,
                Key::Dots,
                bead,
                Transform::from_translation(*d).with_scale(Vec3::splat(0.035)),
            );
        }
        let mut receptors = Vec::new();
        for _ in &c.receptors {
            let parts = ReceptorParts {
                outside: still(&mut groups, world, Key::ReceptorOut, ball, hidden),
                stalk: still(&mut groups, world, Key::ReceptorIn, rod, hidden),
                inside: still(&mut groups, world, Key::ReceptorIn, ball, hidden),
                tail: still(&mut groups, world, Key::Tail, rod, hidden),
                phospho: [
                    still(&mut groups, world, Key::Phospho, bead, hidden),
                    still(&mut groups, world, Key::Phospho, bead, hidden),
                    still(&mut groups, world, Key::Phospho, bead, hidden),
                ],
                egf: still(&mut groups, world, Key::Egf, ball, hidden),
                grb2: still(&mut groups, world, Key::Grb2, ball, hidden),
            };
            receptors.push(parts);
        }
        let ras = c
            .ras
            .iter()
            .map(|_| {
                (
                    still(&mut groups, world, Key::RasOff, ball, hidden),
                    still(&mut groups, world, Key::RasOn, ball, hidden),
                    still(&mut groups, world, Key::Raf, ball, hidden),
                )
            })
            .collect();
        let erks = c
            .erks
            .iter()
            .map(|_| {
                (
                    still(&mut groups, world, Key::ErkOff, ball, hidden),
                    still(&mut groups, world, Key::ErkOn, ball, hidden),
                )
            })
            .collect();
        let mut pulses = Vec::new();
        for (n, _) in c.pulses.iter().enumerate() {
            for k in 0..5 {
                pulses.push((
                    still(&mut groups, world, Key::Pulse, ball, hidden),
                    n,
                    k as f32 / 5.0,
                ));
            }
        }
        let dna = {
            let mut b = Builder::default();
            b.tube(&c.dna.points, |_| 0.0012, 8);
            world.resource_mut::<Assets<Mesh>>().add(b.mesh())
        };
        still(&mut groups, world, Key::Dna, &dna, Transform::default());
        let sites = c
            .sites
            .iter()
            .map(|_| {
                (
                    still(&mut groups, world, Key::Rb, ball, hidden),
                    still(&mut groups, world, Key::E2f, ball, hidden),
                    still(&mut groups, world, Key::Cdk4, ball, hidden),
                    still(&mut groups, world, Key::P16, ball, hidden),
                    still(&mut groups, world, Key::Phospho, bead, hidden),
                )
            })
            .collect();
        let cyclin_rna = (0..6)
            .map(|_| still(&mut groups, world, Key::Rna, rod, hidden))
            .collect();
        let mut s_rna = Vec::new();
        for (n, _) in c.sites.iter().enumerate() {
            for _ in 0..4 {
                s_rna.push((n, still(&mut groups, world, Key::Rna, rod, hidden)));
            }
        }
        let tgf = c
            .tgf
            .iter()
            .map(|_| {
                (
                    still(&mut groups, world, Key::Tgf, rod, hidden),
                    still(&mut groups, world, Key::Tgf, ball, hidden),
                )
            })
            .collect();
        Ok(Self {
            groups,
            receptors,
            ras,
            erks,
            pulses,
            sites,
            cyclin_rna,
            s_rna,
            tgf,
            places: places(),
            variant,
            placed: None,
        })
    }

    pub fn places(&self) -> &[Place] {
        &self.places
    }

    fn stuck(&self) -> bool {
        VARIANTS[self.variant] == "ras_stuck"
    }

    fn stopped(&self) -> bool {
        matches!(VARIANTS[self.variant], "stop_signal" | "p16_lost")
    }

    fn p16(&self) -> bool {
        VARIANTS[self.variant] == "stop_signal"
    }

    /// How far the downstream events have got at minute `t`: with RAS
    /// stuck on, they run whatever the growth factor does.
    fn downstream(&self, t: f32) -> f32 {
        if self.stuck() { t.max(0.0) + 600.0 } else { t }
    }

    /// How far RB has been let go of at `t`, 0 to 1.
    fn rb_off(&self, t: f32) -> f32 {
        if self.p16() {
            0.0
        } else {
            ease((self.downstream(t) - RB_OFF.0) / (RB_OFF.1 - RB_OFF.0))
        }
    }

    pub fn locate(&self, name: &str, t: f32, focus: Vec3) -> Option<Vec3> {
        let c = cell();
        let nearest = |points: &mut dyn Iterator<Item = Vec3>| {
            points.min_by(|a, b| a.distance(focus).total_cmp(&b.distance(focus)))
        };
        Some(match name {
            "egf" => Vec3::new(0.0, 0.012, 0.0),
            "egfr" => Vec3::new(0.0, 0.006, 0.0),
            "phosphate" => Vec3::new(0.0, -0.01, 0.0),
            "grb2_sos" => Vec3::new(0.0, -0.014, 0.0),
            "ras" => nearest(&mut c.ras.iter().map(|r| r.0))?,
            "raf" => nearest(&mut c.ras.iter().map(|r| r.0 - Vec3::Y * 0.006))?,
            "erk" => nearest(&mut c.erks.iter().map(|e| e.1.at(self.erk_way(e.2, t)).0))?,
            "nuclear_pore" => nearest(&mut c.pores.iter().map(|p| p.translation))?,
            "rb" | "e2f" => c.sites.first().copied()?,
            "cyclin_d_cdk4" => c.sites.first().copied()? + Vec3::new(0.0, 0.012, 0.0),
            "p16" => c.sites.first().copied()? + Vec3::new(0.008, 0.016, 0.0),
            "cyclin_d_gene" => c.cyclin_gene,
            "s_phase_genes" => c.sites.get(1).copied()?,
            _ => return None,
        })
    }

    /// How far along its way an ERK is at minute `t`.
    fn erk_way(&self, goes: f32, t: f32) -> f32 {
        let t = self.downstream(t);
        ease((t - goes) / 8.0)
    }

    pub fn pick(&self, origin: Vec3, direction: Vec3, t: f32, d: f32) -> Option<String> {
        let c = cell();
        let mut best: Option<(f32, &'static str)> = None;
        let mut consider = |centre: Vec3, r: f32, what: &'static str| {
            let o = centre - origin;
            let along = o.dot(direction);
            let miss = o.length_squared() - along * along;
            if along > 0.0 && miss < r * r {
                let hit = along - (r * r - miss).sqrt();
                if best.is_none_or(|(b, _)| hit < b) {
                    best = Some((hit, what));
                }
            }
        };
        if self.groups.seen(Key::ReceptorOut, d) {
            for r in &c.receptors {
                consider(
                    r.at + Vec3::Y * 0.006,
                    0.007,
                    "egfr: the EGF receptor, a tyrosine kinase standing through the membrane; growth factor makes two pair up and phosphorylate each other's tails",
                );
            }
            for (at, _) in &c.ras {
                consider(
                    *at,
                    0.004,
                    if self.stuck() {
                        "ras: a small switch on the membrane's inner face, here stuck on by a mutation (as G12V does), sending the signal on with no growth factor at all"
                    } else {
                        "ras: a small switch on the membrane's inner face, off with GDP, on with GTP once SOS reaches it; on, it calls RAF"
                    },
                );
            }
        }
        if self.groups.seen(Key::ErkOn, d) {
            for (_, path, goes) in &c.erks {
                consider(
                    path.at(self.erk_way(*goes, t)).0,
                    0.006,
                    "erk: the last kinase of the cascade; switched on by MEK, it goes into the nucleus and turns on genes such as cyclin D's",
                );
            }
        }
        if self.groups.seen(Key::Rb, d) {
            for s in &c.sites {
                consider(
                    *s,
                    0.008,
                    "rb: the brake on division; while unphosphorylated it holds E2F on the genes for copying DNA; cyclin D with CDK4 phosphorylates it and it lets go",
                );
            }
        }
        if self.groups.seen(Key::Nucleus, d) {
            consider(
                NUCLEUS_AT,
                NUCLEUS,
                "nucleus: where the genes are; the signal from the surface ends here",
            );
        }
        if self.groups.seen(Key::Cell, d) {
            consider(
                Vec3::new(0.0, -CELL, 0.0),
                CELL,
                "the cell, reading the growth factor at its surface",
            );
        }
        best.map(|(_, w)| w.to_owned())
    }

    pub fn update(&mut self, tick: &Tick, things: &mut Things, glows: &mut Assets<Glow>) {
        let c = cell();
        let t = tick.clock;
        self.groups.update(tick, picks, |_, s| s, things, glows);
        // The cytoplasm glows as ERK fills it.
        let lit = ease((self.downstream(t) - ERK_ON.0) / (ERK_ON.1 - ERK_ON.0));
        self.groups.restyle(Key::Cytoplasm, lit, glows, |m| {
            m.body = LinearRgba::rgb(0.25, 0.6 + 0.4 * lit, 1.0 - 0.4 * lit)
                .with_alpha(0.02 + 0.05 * lit);
        });

        // Ligands drift until they bind; ERK and the signal's pulses run
        // by the world's life as well as its clock, so they always move.
        let life = tick.life;
        let placed_again = self.placed != Some(t);
        self.placed = Some(t);
        let mut dice = Dice::new(5);
        let molecules = !self.groups.hidden(Key::ReceptorOut);
        for (i, (r, parts)) in c.receptors.iter().zip(&self.receptors).enumerate() {
            let partner = r.partner.map(|j| c.receptors[j].at).unwrap_or(r.at);
            let bound = !self.stuck() && t >= r.binds;
            let pairing = if bound {
                ease((t - r.binds) / 0.6)
            } else {
                0.0
            };
            let mid = (r.at + partner) * 0.5;
            let towards = (partner - r.at).normalize_or(Vec3::X);
            let at = r.at.lerp(mid - towards * 0.0045, pairing);
            let (_, up) = surface(at.x, at.z);
            let rot = Quat::from_rotation_arc(Vec3::Y, up);
            let place = |things: &mut Things, e: Entity, t: Transform| {
                set_shown(things, e, molecules);
                if let Ok((mut tr, _)) = things.get_mut(e) {
                    *tr = t;
                }
            };
            // Receptors move only with the clock.
            if placed_again {
                place(
                    things,
                    parts.outside,
                    Transform::from_translation(at + up * (LEAFLET + 0.0055))
                        .with_rotation(rot)
                        .with_scale(Vec3::new(0.0042, 0.0058, 0.0036)),
                );
                place(
                    things,
                    parts.stalk,
                    Transform::from_translation(at)
                        .with_rotation(rot)
                        .with_scale(Vec3::new(0.0007, 0.003, 0.0007)),
                );
                place(
                    things,
                    parts.inside,
                    Transform::from_translation(at - up * (LEAFLET + 0.0035))
                        .with_rotation(rot)
                        .with_scale(Vec3::new(0.0033, 0.0028, 0.003)),
                );
                place(
                    things,
                    parts.tail,
                    Transform::from_translation(at - up * (LEAFLET + 0.0078) + towards * 0.0012)
                        .with_rotation(rot)
                        .with_scale(Vec3::new(0.0005, 0.0018, 0.0005)),
                );
                let phosphorylated = bound && t >= r.binds + 0.5;
                for (k, e) in parts.phospho.iter().enumerate() {
                    set_shown(things, *e, molecules && phosphorylated);
                    if let Ok((mut tr, _)) = things.get_mut(*e) {
                        tr.translation =
                            at - up * (LEAFLET + 0.0065 + k as f32 * 0.0011) + towards * 0.002;
                        tr.scale = Vec3::splat(0.0007);
                    }
                }
                let recruited = phosphorylated && t >= r.binds + 1.2;
                set_shown(things, parts.grb2, molecules && recruited);
                if let Ok((mut tr, _)) = things.get_mut(parts.grb2) {
                    tr.translation = at - up * (LEAFLET + 0.0095) + towards * 0.0035;
                    tr.scale = Vec3::new(0.0028, 0.0022, 0.0028);
                }
            }
            // The growth factor: drifting above until it binds, then held
            // in the receptor's arms.
            let drift = Vec3::new(
                (life * 0.7 + i as f32).sin(),
                (life * 0.5 + i as f32 * 2.0).cos() * 0.5,
                (life * 0.6 + i as f32 * 3.0).cos(),
            ) * 0.01;
            let free = Vec3::new(
                dice.range(-0.3, 0.3),
                dice.range(0.03, 0.09),
                dice.range(-0.3, 0.3),
            ) + drift;
            let seat = at + up * (LEAFLET + 0.0105);
            let arriving = if bound {
                ease((t - r.binds) / 0.5)
            } else {
                0.0
            };
            let present = t >= BIND.0 - 1.0 || self.stuck();
            set_shown(things, parts.egf, molecules && present && !self.stuck());
            if let Ok((mut tr, _)) = things.get_mut(parts.egf) {
                tr.translation = (r.at + free).lerp(seat, arriving);
                tr.scale = Vec3::splat(0.0022);
            }
        }
        // RAS switching on, RAF arriving.
        for ((at, on_at), (off, on, raf)) in c.ras.iter().zip(&self.ras) {
            let on_now = self.stuck() || t >= *on_at;
            set_shown(things, *off, molecules && !on_now);
            set_shown(things, *on, molecules && on_now);
            let raf_now = on_now && (self.stuck() || t >= on_at + 1.0);
            set_shown(things, *raf, molecules && raf_now);
            for (e, offset, size) in [
                (*off, Vec3::ZERO, 0.0022),
                (*on, Vec3::ZERO, 0.0024),
                (*raf, -Vec3::Y * 0.0055, 0.0028),
            ] {
                if let Ok((mut tr, _)) = things.get_mut(e) {
                    tr.translation = *at + offset;
                    tr.scale = Vec3::splat(size);
                }
            }
        }
        // ERK: waiting, then switched on, then on its way to the nucleus.
        let cytoplasm = !self.groups.hidden(Key::ErkOn);
        for (n, ((rest, path, goes), (off, on))) in c.erks.iter().zip(&self.erks).enumerate() {
            let down = self.downstream(t);
            let on_now = down >= ERK_ON.0 + (n % 10) as f32 * 0.5;
            let way = if on_now { self.erk_way(*goes, t) } else { 0.0 };
            // Once the first are there, ERK keeps coming and going while the
            // signal lasts.
            let way = if self.stuck() || down > ERK_IN.1 {
                (life * 0.05 + n as f32 * 0.137).fract()
            } else {
                way
            };
            let jiggle = Vec3::new(
                (life * 1.3 + n as f32).sin(),
                (life * 1.1 + n as f32 * 1.7).cos(),
                (life * 0.9 + n as f32 * 2.3).sin(),
            ) * 0.004;
            let at = if on_now { path.at(way).0 } else { *rest } + jiggle;
            set_shown(things, *off, cytoplasm && !on_now);
            set_shown(things, *on, cytoplasm && on_now);
            for e in [*off, *on] {
                if let Ok((mut tr, _)) = things.get_mut(e) {
                    tr.translation = at;
                    tr.scale = Vec3::splat(0.0035);
                }
            }
        }
        // The signal seen from afar: light running in to the nucleus while
        // ERK is on.
        let signal = ease((self.downstream(t) - ERK_ON.0) / 4.0);
        let afar = !self.groups.hidden(Key::Pulse);
        for (e, n, phase) in &self.pulses {
            let f = (phase + life * 0.12).fract();
            let shown = afar && signal > 0.05;
            set_shown(things, *e, shown);
            if shown && let Ok((mut tr, _)) = things.get_mut(*e) {
                tr.translation = c.pulses[*n].at(f).0;
                tr.scale =
                    Vec3::splat(0.09 * signal * (f * 8.0).min(1.0) * ((1.0 - f) * 8.0).min(1.0));
            }
        }
        // The genes: RB on E2F, cyclin D with CDK4 arriving, p16 holding
        // CDK4 back, RB phosphorylated and letting go, the genes read.
        let genes = !self.groups.hidden(Key::Rb);
        let down = self.downstream(t);
        let cyclin = ease((down - CYCLIN_D.0) / (CYCLIN_D.1 - CYCLIN_D.0));
        let held = if self.p16() {
            ease((t - P16.0) / (P16.1 - P16.0))
        } else {
            0.0
        };
        let off = self.rb_off(t);
        let reading = if self.p16() {
            0.0
        } else {
            ease((down - S_GENES.0) / (S_GENES.1 - S_GENES.0))
        };
        for (k, (site, (rb, e2f, cdk4, p16, phos))) in c.sites.iter().zip(&self.sites).enumerate() {
            let up = Vec3::Y;
            let away = Vec3::new(0.012 * (k as f32 - 1.0), 0.03, 0.01);
            for (e, at, scale, shown) in [
                (
                    *e2f,
                    *site + up * 0.0025,
                    Vec3::new(0.0032, 0.0022, 0.0028),
                    true,
                ),
                (
                    *rb,
                    *site + up * 0.0062 + away * off,
                    Vec3::new(0.0045, 0.0035, 0.004),
                    true,
                ),
                (
                    *cdk4,
                    *site + up * (0.03 - 0.019 * cyclin * (1.0 - held)) + Vec3::X * 0.006,
                    Vec3::splat(0.0036),
                    cyclin > 0.05,
                ),
                (
                    *p16,
                    *site
                        + up * (0.03 - 0.019 * cyclin * (1.0 - held))
                        + Vec3::new(0.0105, 0.002, 0.0),
                    Vec3::splat(0.0018),
                    held > 0.05,
                ),
                (
                    *phos,
                    *site + up * 0.0095 + away * off,
                    Vec3::splat(0.0011),
                    off > 0.02 || (cyclin > 0.9 && !self.p16()),
                ),
            ] {
                set_shown(things, e, genes && shown);
                if let Ok((mut tr, _)) = things.get_mut(e) {
                    tr.translation = at;
                    tr.scale = scale;
                }
            }
        }
        // RNA coming off the genes being read: cyclin D's, then the S phase
        // genes'.
        let dna_dir = (c.dna.at(0.2).0 - c.dna.at(0.0).0).normalize_or(Vec3::X);
        for (k, e) in self.cyclin_rna.iter().enumerate() {
            let grown = cyclin * (1.0 - k as f32 / 8.0);
            set_shown(things, *e, genes && grown > 0.05);
            if let Ok((mut tr, _)) = things.get_mut(*e) {
                let base = c.cyclin_gene + dna_dir * (k as f32 * 0.005 - 0.012);
                let length = 0.012 * grown;
                tr.translation = base + Vec3::new(0.0, length, 0.002);
                tr.rotation = Quat::from_rotation_z(0.25 * (k as f32 - 2.5) / 2.5);
                tr.scale = Vec3::new(0.0006, length, 0.0006);
            }
        }
        for (k, (site, e)) in self.s_rna.iter().enumerate() {
            let grown = reading * (1.0 - (k % 4) as f32 / 6.0);
            set_shown(things, *e, genes && grown > 0.05);
            if let Ok((mut tr, _)) = things.get_mut(*e) {
                let base = c.sites[*site] + dna_dir * ((k % 4) as f32 * 0.005 - 0.006);
                let length = 0.01 * grown;
                tr.translation = base + Vec3::new(0.0, -length, 0.002);
                tr.rotation = Quat::from_rotation_z(0.3 * ((k % 4) as f32 - 1.5) / 1.5);
                tr.scale = Vec3::new(0.0006, length, 0.0006);
            }
        }
        // The stop signal's receptors, with their ligand bound.
        for (at, (stalk, ligand)) in c.tgf.iter().zip(&self.tgf) {
            let shown = molecules && self.stopped();
            let (_, up) = surface(at.x, at.z);
            set_shown(things, *stalk, shown);
            set_shown(things, *ligand, shown && t >= 0.0);
            if let Ok((mut tr, _)) = things.get_mut(*stalk) {
                tr.translation = *at + up * 0.004;
                tr.rotation = Quat::from_rotation_arc(Vec3::Y, up);
                tr.scale = Vec3::new(0.0012, 0.008, 0.0012);
            }
            if let Ok((mut tr, _)) = things.get_mut(*ligand) {
                tr.translation = *at + up * 0.0125;
                tr.scale = Vec3::splat(0.0026);
            }
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
    fn the_cell_has_its_receptors_in_pairs_and_its_events_in_order() {
        let c = cell();
        assert_eq!(c.receptors.len(), RECEPTORS);
        assert!(c.receptors.iter().filter(|r| r.partner.is_some()).count() >= RECEPTORS - 1);
        assert!(c.receptors[0].at.length() < 1e-6);
        // The events follow one another.
        let steps = [BIND, RAS_ON, ERK_ON, ERK_IN, CYCLIN_D, RB_OFF, S_GENES];
        for w in steps.windows(2) {
            assert!(w[0].0 <= w[1].0, "{:?} before {:?}", w[0], w[1]);
        }
        // ERK's way ends inside the nucleus.
        for (_, path, _) in &c.erks {
            assert!(path.at(1.0).0.distance(NUCLEUS_AT) < NUCLEUS);
        }
        let places: Vec<&str> = places().iter().map(|p| p.name).collect();
        assert_eq!(places, PLACES.to_vec());
    }
}
