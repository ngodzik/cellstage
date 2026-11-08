//! The skin as a world to film: from a forearm seen at a hand's length,
//! through its surface into the epidermis cell by cell, down into one
//! melanocyte's nucleus and its DNA, nine orders of magnitude in one move;
//! and two stories over its clock, in years since BRAF mutated in that
//! melanocyte: a common mole, which stops, and a melanoma, which does not.
//! Each part is drawn at the scales it is seen at and fades out of the
//! others.

use std::sync::OnceLock;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::engine::{Band, Groups, NEVER, NONE, Solids, Things, Tick, ease, set_shown};
use super::film::{Eye, Place};
use super::glow::Glow;
use super::nevus::{self, Mole};
use super::nucleus::{Base, Nucleus};
use super::skin::{self, Layer, Skin, Solid};
use super::surface::{self, Surface};
use super::tumour::{self, Tumour};

/// The seed the skin is built from.
const SEED: u64 = 7;

/// The tissue is seen from a few millimetres, when the skin's surface
/// fades, down to the inside of one cell.
const TISSUE: (f32, f32) = (1400.0, 3200.0);
/// The surface, from far away down to two millimetres.
const OUTSIDE: Band = Band::new((1000.0, 2400.0), NEVER);
const NUCLEUS: Band = Band::new((0.25, 0.8), (14.0, 30.0));
const CHROMATIN: Band = Band::new((0.06, 0.25), (16.0, 35.0));
const NUCLEOSOMES: Band = Band::new((0.012, 0.03), (0.3, 0.8));
const HELIX: Band = Band::new(NONE, (0.025, 0.07));
/// How many pieces a sprout is drawn in, to grow it piece by piece.
const PIECES: usize = 28;

/// The stories the skin's clock can tell.
pub const VARIANTS: [&str; 2] = ["mole", "melanoma"];

/// A part of the skin that has a look of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Tissue(Layer),
    Nevus,
    NevusNucleus,
    Tumour,
    TumourCore,
    TumourNucleus,
    Sprout,
    Tip,
    Envelope,
    Pore,
    Chromatin,
    Nucleolus,
    Histone,
    Linker,
    Backbone,
    Phosphate,
    Base(Base),
    /// The two halves of BRAF's pair that mutates.
    Mutant(usize),
    /// The halves of the pairs that mutate later, in a melanoma.
    Scar(usize),
    Flash,
}

/// How a part looks, at which scales it is seen, and how far from the
/// origin it reaches before fading. Strengths are low because a tissue is
/// many cells deep and their light adds up.
fn look(key: Key) -> (Glow, Band, f32) {
    let g = Glow::new;
    let tissue = |glow: Glow| (glow, Band::new((5.0, 13.0), TISSUE), skin::RADIUS);
    let near = |glow: Glow, n: (f32, f32)| (glow, Band::new(n, TISSUE), skin::RADIUS);
    let helix = |glow: Glow| (glow, HELIX, 0.016);
    match key {
        Key::Tissue(layer) => match layer {
            Layer::Horny => tissue(g([0.55, 0.5, 0.85], 0.07, 0.0).edges(4.0).varied(40.0)),
            Layer::Granular => tissue(g([0.5, 0.3, 0.95], 0.12, 0.002).edges(3.2).varied(30.0)),
            Layer::Spiny => tissue(g([0.12, 0.38, 1.0], 0.13, 0.002).edges(3.0).varied(25.0)),
            Layer::Basal => tissue(g([0.08, 0.75, 0.9], 0.24, 0.004).edges(2.6).varied(20.0)),
            Layer::Nucleus => tissue(
                g([0.35, 0.55, 1.0], 0.22, 0.0)
                    .faces([0.2, 0.35, 1.0], 0.07)
                    .edges(1.8),
            ),
            Layer::FadingNucleus => tissue(g([0.4, 0.35, 0.9], 0.08, 0.0).edges(2.0)),
            Layer::Melanin => tissue(
                g([1.0, 0.42, 0.08], 0.45, 0.0)
                    .faces([0.9, 0.3, 0.05], 0.16)
                    .edges(1.5),
            ),
            Layer::Melanocyte => tissue(
                g([1.0, 0.68, 0.18], 0.42, 0.0)
                    .faces([1.0, 0.55, 0.12], 0.03)
                    .edges(2.0),
            ),
            // Gone before the camera reaches the nucleus inside it.
            Layer::First => near(
                g([1.0, 0.7, 0.2], 0.9, 0.0)
                    .faces([1.0, 0.55, 0.12], 0.06)
                    .edges(2.0),
                (8.0, 20.0),
            ),
            // The envelope takes over as the camera comes near.
            Layer::FirstNucleus => near(
                g([0.4, 0.6, 1.0], 0.5, 0.0)
                    .faces([0.25, 0.4, 1.0], 0.12)
                    .edges(1.8),
                (10.0, 25.0),
            ),
            Layer::Membrane => tissue(g([1.0, 0.18, 0.62], 0.55, 0.0).edges(6.0)),
            Layer::Collagen => tissue(g([0.95, 0.32, 0.36], 0.13, 0.002).edges(2.4).varied(60.0)),
            Layer::Elastic => tissue(g([1.0, 0.9, 0.6], 0.2, 0.0).edges(1.8)),
            Layer::Fibroblast => tissue(g([0.3, 1.0, 0.45], 0.35, 0.01).edges(2.2)),
            Layer::Vessel => tissue(g([1.0, 0.12, 0.1], 0.25, 0.004).edges(2.8)),
            Layer::RedCell => tissue(
                g([1.0, 0.16, 0.08], 0.6, 0.0)
                    .faces([0.8, 0.05, 0.03], 0.22)
                    .edges(1.6),
            ),
            Layer::Immune => tissue(g([0.85, 0.9, 1.0], 0.35, 0.01).edges(2.2)),
        },
        Key::Nevus => near(
            g([1.0, 0.55, 0.12], 0.5, 0.0)
                .faces([1.0, 0.5, 0.1], 0.03)
                .edges(2.0),
            (1.5, 5.0),
        ),
        Key::NevusNucleus => near(
            g([0.4, 0.55, 1.0], 0.25, 0.0)
                .faces([0.3, 0.4, 1.0], 0.05)
                .edges(1.8),
            (1.5, 5.0),
        ),
        // A melanoma's cells: hot, uneven, unlike anything else in the skin.
        Key::Tumour => near(
            g([1.0, 0.16, 0.42], 0.42, 0.0)
                .faces([1.0, 0.1, 0.35], 0.03)
                .edges(1.8),
            (1.5, 5.0),
        ),
        Key::TumourCore => near(
            g([1.0, 0.16, 0.42], 0.42, 0.0)
                .faces([1.0, 0.1, 0.35], 0.03)
                .edges(1.8),
            (1.5, 5.0),
        ),
        Key::TumourNucleus => near(
            g([1.0, 0.75, 0.85], 0.28, 0.0)
                .faces([1.0, 0.6, 0.8], 0.06)
                .edges(1.8),
            (1.5, 5.0),
        ),
        Key::Sprout => tissue(g([1.0, 0.3, 0.15], 0.45, 0.01).edges(2.4)),
        Key::Tip => tissue(
            g([1.0, 0.8, 0.35], 0.6, 0.0)
                .faces([1.0, 0.7, 0.3], 0.06)
                .edges(1.8),
        ),
        Key::Envelope => (g([0.45, 0.62, 1.0], 0.5, 0.0).edges(3.0), NUCLEUS, 0.0),
        Key::Pore => (
            g([0.85, 0.92, 1.0], 0.6, 0.0)
                .faces([0.7, 0.8, 1.0], 0.15)
                .edges(1.5),
            NUCLEUS,
            0.0,
        ),
        Key::Chromatin => (
            g([0.35, 0.45, 1.0], 0.35, 0.0)
                .faces([0.3, 0.35, 1.0], 0.04)
                .edges(1.6)
                .varied(0.8),
            CHROMATIN,
            0.0,
        ),
        Key::Nucleolus => (
            g([0.75, 0.5, 1.0], 0.5, 0.0)
                .faces([0.6, 0.35, 1.0], 0.12)
                .edges(1.8),
            CHROMATIN,
            0.0,
        ),
        Key::Histone => (
            g([1.0, 0.72, 0.32], 0.65, 0.0)
                .faces([1.0, 0.6, 0.25], 0.18)
                .edges(1.7),
            NUCLEOSOMES,
            0.42,
        ),
        Key::Linker => (
            g([0.45, 0.8, 1.0], 0.7, 0.0)
                .faces([0.35, 0.7, 1.0], 0.15)
                .edges(1.6),
            NUCLEOSOMES,
            0.42,
        ),
        Key::Backbone => helix(
            g([0.7, 0.8, 1.0], 0.28, 0.0)
                .faces([0.6, 0.7, 1.0], 0.03)
                .edges(2.2),
        ),
        Key::Phosphate => helix(
            g([1.0, 0.82, 0.5], 0.1, 0.0)
                .faces([1.0, 0.7, 0.35], 0.015)
                .edges(2.6),
        ),
        Key::Base(b) => helix(g(b.colour(), 0.7, 0.0).faces(b.colour(), 0.35).edges(1.5)),
        // Coloured by their base when made, and by the clock after.
        Key::Mutant(_) | Key::Scar(_) => helix(g([1.0, 1.0, 1.0], 0.7, 0.0).edges(1.5)),
        Key::Flash => (
            g([1.0, 1.0, 1.0], 0.0, 0.0).faces([1.0, 0.95, 0.85], 0.0),
            HELIX,
            0.0,
        ),
    }
}

/// Whether a name the agent gave sets this part apart: a layer's name, or
/// one of the groups of them.
fn picks(name: &str, key: Key) -> bool {
    match key {
        Key::Tissue(layer) => {
            name == layer.name()
                || match name {
                    "keratinocytes" | "epidermis" => matches!(
                        layer,
                        Layer::Horny
                            | Layer::Granular
                            | Layer::Spiny
                            | Layer::Basal
                            | Layer::Nucleus
                            | Layer::FadingNucleus
                    ),
                    "pigment" | "melanocytes" => matches!(
                        layer,
                        Layer::Melanocyte | Layer::First | Layer::FirstNucleus | Layer::Melanin
                    ),
                    "blood" | "vessels" => matches!(layer, Layer::Vessel | Layer::RedCell),
                    "dermis" => matches!(
                        layer,
                        Layer::Collagen
                            | Layer::Elastic
                            | Layer::Fibroblast
                            | Layer::Vessel
                            | Layer::RedCell
                            | Layer::Immune
                    ),
                    "junction" => matches!(layer, Layer::Membrane | Layer::Basal),
                    "first_melanocyte" => matches!(layer, Layer::First | Layer::FirstNucleus),
                    _ => false,
                }
        }
        Key::Nevus | Key::NevusNucleus => matches!(name, "mole" | "nests" | "naevus"),
        Key::Tumour | Key::TumourCore | Key::TumourNucleus => {
            matches!(name, "tumour" | "melanoma")
        }
        Key::Sprout | Key::Tip => matches!(name, "vessels" | "blood" | "sprouts"),
        // The pairs that mutate stand out from the rest of the helix when
        // asked to.
        Key::Base(_) | Key::Backbone | Key::Phosphate => {
            !matches!(name, "braf_codon_600" | "mutation" | "mutations")
        }
        Key::Scar(_) => name != "braf_codon_600",
        // What is inside the nucleus is never dimmed otherwise: it is alone
        // at its scale.
        _ => true,
    }
}

/// Groups of parts that can be set apart together.
const HIGHLIGHTS: [&str; 14] = [
    "mole",
    "melanoma",
    "sprouts",
    "vessels",
    "braf_codon_600",
    "mutations",
    "pigment",
    "keratinocytes",
    "blood",
    "dermis",
    "junction",
    "first_melanocyte",
    "basal_layer",
    "basement_membrane",
];

/// The names that can be set apart: groups of layers, and every layer.
pub fn highlights() -> Vec<&'static str> {
    let mut all = HIGHLIGHTS.to_vec();
    for name in Layer::names() {
        if !all.contains(&name) {
            all.push(name);
        }
    }
    all
}

/// The places the camera can be sent to, as `places` makes them.
pub const PLACES: [&str; 19] = [
    "arm",
    "mole",
    "skin",
    "epidermis",
    "basal_layer",
    "melanocyte",
    "nucleus",
    "chromatin",
    "nucleosomes",
    "dna",
    "nests",
    "underneath",
    "dermis",
    "capillary",
    "spread",
    "breach",
    "nodule",
    "sprout",
    "leaving",
];

/// What notes can point at, besides the layers.
const THINGS: [&str; 24] = [
    "first_melanocyte",
    "surface",
    "mole",
    "dermal_papilla",
    "nest_1",
    "nest_2",
    "nest_3",
    "nest_4",
    "nest_5",
    "nest_6",
    "nest_7",
    "nest_8",
    "nucleus",
    "nuclear_pore",
    "chromatin",
    "nucleolus",
    "nucleosome",
    "dna",
    "braf_codon_600",
    "melanoma",
    "breach",
    "nodule",
    "sprout_tip",
    "new_vessel",
];

/// What the skin world is made of, built once.
pub struct Built {
    skin: Skin,
    mole: Mole,
    inside: Nucleus,
    tumour: Tumour,
}

pub fn built() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let skin = Skin::build(SEED);
        let mole = Mole::build(&skin, SEED + 2);
        let envelope = skin
            .instances
            .iter()
            .find(|i| i.layer == Layer::FirstNucleus)
            .map(|i| i.transform)
            .unwrap_or_default();
        let inside = Nucleus::build(envelope, SEED + 3);
        let pairs = inside.halves.len() / 2;
        let tumour = Tumour::build(&skin, &mole, pairs, SEED + 4);
        Built {
            skin,
            mole,
            inside,
            tumour,
        }
    })
}

/// The places the camera can be sent to in the skin.
fn places(b: &Built) -> Vec<Place> {
    let (skin, mole, t) = (&b.skin, &b.mole, &b.tumour);
    let s = skin.surface;
    let capillary = skin
        .papillae
        .iter()
        .min_by(|a, b| a.length().total_cmp(&b.length()))
        .copied()
        .unwrap_or(Vec3::new(40.0, 0.0, 0.0));
    let nests = mole.nests.iter().sum::<Vec3>() / mole.nests.len().max(1) as f32;
    let sprout = t.sprouts.first().map_or(t.nodule, |sp| sp.path.at(0.3).0);
    let leaving = t.sprouts.first().map_or(t.nodule, |sp| sp.path.at(0.86).0);
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
    vec![
        place(
            "arm",
            "a forearm's skin from a hand's length, the mole (once it has grown) a small brown spot",
            Vec3::new(0.0, s, 0.0),
            30_000.0,
            0.5,
            0.95,
        ),
        place(
            "mole",
            "the skin's surface close up, a few millimetres across: its lines, pores, and the mole or the melanoma",
            Vec3::new(0.0, s, 0.0),
            7_000.0,
            0.6,
            1.1,
        ),
        place(
            "skin",
            "a core of skin half a millimetre across, every layer at once, its cut side showing them in section",
            Vec3::new(0.0, s - 85.0, 0.0),
            520.0,
            0.55,
            0.36,
        ),
        place(
            "epidermis",
            "the epidermis from the side, its layers from the horny one down to the basal one",
            Vec3::new(0.0, s - 45.0, 0.0),
            190.0,
            0.7,
            0.18,
        ),
        place(
            "basal_layer",
            "the basal layer at the bottom of a rete ridge, the pigment cells among the basal cells",
            Vec3::new(0.0, 6.0, 0.0),
            105.0,
            0.85,
            0.38,
        ),
        place(
            "melanocyte",
            "the first melanocyte close up, its arms reaching up between keratinocytes, pigment travelling up them",
            Vec3::new(0.0, 9.0, 0.0),
            48.0,
            0.95,
            0.28,
        ),
        place(
            "nucleus",
            "the first melanocyte's nucleus, its envelope pierced by pores",
            Vec3::ZERO,
            9.0,
            1.0,
            0.25,
        ),
        place(
            "chromatin",
            "inside the nucleus: chromatin, the DNA packed with its proteins, and the nucleolus",
            Vec3::ZERO,
            1.3,
            1.1,
            0.2,
        ),
        place(
            "nucleosomes",
            "chromatin up close: DNA wound round cores of histones, a string of beads",
            Vec3::ZERO,
            0.13,
            0.4,
            0.3,
        ),
        place(
            "dna",
            "the double helix at codon 600 of BRAF, base pair by base pair, the T that mutates in the middle",
            Vec3::ZERO,
            0.011,
            0.0,
            0.12,
        ),
        place(
            "nests",
            "the junction where the mole's nests gather, at the bottoms of the rete ridges",
            nests,
            240.0,
            0.7,
            0.35,
        ),
        place(
            "underneath",
            "the epidermis seen from below, through the dermis: the net of rete ridges and the pegs where they meet",
            Vec3::new(0.0, -10.0, 0.0),
            260.0,
            0.4,
            -0.75,
        ),
        place(
            "dermis",
            "the dermis: collagen, fibroblasts, the capillary loops rising into the papillae",
            Vec3::new(0.0, s - 170.0, 0.0),
            300.0,
            1.2,
            0.12,
        ),
        place(
            "capillary",
            "a capillary loop inside a dermal papilla, red cells running up and down it",
            capillary - Vec3::Y * 22.0,
            70.0,
            0.2,
            0.15,
        ),
        place(
            "spread",
            "a melanoma's radial growth: its cells along the junction and climbing into the epidermis, seen from above at a slant",
            t.start + Vec3::new(20.0, 25.0, 0.0),
            330.0,
            0.5,
            0.55,
        ),
        place(
            "breach",
            "where a melanoma breaks through the basement membrane into the dermis, seen from below",
            t.breach,
            170.0,
            0.6,
            -0.45,
        ),
        place(
            "nodule",
            "a melanoma's nodule in the dermis, its vertical growth, the new vessels that feed it",
            t.nodule,
            330.0,
            0.75,
            0.08,
        ),
        place(
            "sprout",
            "a new capillary sprouting towards the nodule, its tip cell leading",
            sprout,
            90.0,
            1.1,
            0.2,
        ),
        place(
            "leaving",
            "a new vessel leaving the nodule, melanoma cells carried off in the blood",
            leaving,
            110.0,
            0.3,
            0.25,
        ),
    ]
}

/// Something that moves along a path by itself.
struct Runner {
    entity: Entity,
    track: Track,
    /// Where along its path it starts, 0 to 1, and how many times a
    /// second it goes round.
    phase: f32,
    rate: f32,
    size: f32,
    spin: f32,
}

#[derive(Clone, Copy)]
enum Track {
    /// Round a capillary loop, the way blood goes.
    Blood(usize),
    /// Up one of a melanocyte's arms, to be handed over at its tip.
    Pigment(usize, usize),
    /// Through a new vessel, once it carries blood.
    Sprout(usize),
    /// A melanoma cell leaving through a new vessel.
    Leaving(usize),
}

/// The meshes of the skin, made once and shared by every panel.
#[derive(Clone)]
pub struct Meshes {
    solids: Solids,
    tubes: Vec<(Layer, Handle<Mesh>)>,
    chromatin: Handle<Mesh>,
    string: Handle<Mesh>,
    backbones: Handle<Mesh>,
    surface: Handle<Mesh>,
    /// Each sprout's pieces, with the stretch of it each covers.
    sprouts: Vec<Vec<(f32, f32, Handle<Mesh>)>>,
}

impl Meshes {
    pub fn make(meshes: &mut Assets<Mesh>) -> Self {
        let b = built();
        let tubes = b
            .skin
            .tubes
            .iter()
            .filter(|(_, t)| !t.is_empty())
            .map(|(l, t)| (*l, meshes.add(t.mesh())))
            .collect();
        let sprouts = b
            .tumour
            .sprouts
            .iter()
            .map(|sp| {
                (0..PIECES)
                    .map(|k| {
                        let (a, z) = (k as f32 / PIECES as f32, (k + 1) as f32 / PIECES as f32);
                        let points: Vec<Vec3> = (0..=6)
                            .map(|s| sp.path.at(a + (z - a) * s as f32 / 6.0).0)
                            .collect();
                        let mut tube = super::shapes::Builder::default();
                        tube.tube(&points, |_| 2.6, 8);
                        (a, z, meshes.add(tube.mesh()))
                    })
                    .collect()
            })
            .collect();
        Self {
            solids: Solids::make(meshes),
            tubes,
            chromatin: meshes.add(b.inside.chromatin.mesh()),
            string: meshes.add(b.inside.string.mesh()),
            backbones: meshes.add(b.inside.backbones.mesh()),
            surface: meshes.add(surface::mesh(b.skin.surface, 40_000.0)),
            sprouts,
        }
    }
}

/// The skin, as shown in a panel.
pub struct SkinWorld {
    groups: Groups<Key>,
    runners: Vec<Runner>,
    /// The mole's cells, body and nucleus, by the clone's index.
    nevus: Vec<(Entity, Entity)>,
    /// The melanoma's cells, body and nucleus, by its index.
    cancer: Vec<(Entity, Entity)>,
    /// The sprouts' pieces: which sprout, the stretch, the part.
    pieces: Vec<(usize, f32, f32, Entity)>,
    /// The tip cells: which sprout, from which end.
    tips: Vec<(usize, bool, Entity)>,
    /// Basal cells and their parts where the nests sit, crowded out as
    /// the nests fill.
    crowded: Vec<Entity>,
    /// The helix's halves that later mutate in a melanoma, their base and
    /// when.
    scars: Vec<(usize, Base, f32)>,
    surface: (Entity, Handle<Surface>),
    places: Vec<Place>,
    melanoma: bool,
    /// When, in the world's life, BRAF mutated on screen.
    mutated: Option<f32>,
    /// The clock and whether the clones were shown when last placed.
    placed: Option<(f32, bool, bool)>,
}

impl SkinWorld {
    pub fn spawn(
        world: &mut World,
        layer: RenderLayers,
        variant: &str,
        meshes: &Meshes,
    ) -> Result<Self, String> {
        let melanoma = match variant {
            "" | "mole" => false,
            "melanoma" => true,
            other => {
                return Err(format!(
                    "no story {other} in the skin; there are {}",
                    VARIANTS.join(", ")
                ));
            }
        };
        let b = built();
        let (skin, mole, inside, tumour) = (&b.skin, &b.mole, &b.inside, &b.tumour);
        let solids = &meshes.solids;

        // Every look the world needs, one material each.
        let mut keys: Vec<Key> = Layer::ALL.into_iter().map(Key::Tissue).collect();
        keys.extend([
            Key::Nevus,
            Key::NevusNucleus,
            Key::Envelope,
            Key::Pore,
            Key::Chromatin,
            Key::Nucleolus,
            Key::Histone,
            Key::Linker,
            Key::Backbone,
            Key::Phosphate,
            Key::Base(Base::A),
            Key::Base(Base::C),
            Key::Base(Base::G),
            Key::Base(Base::T),
            Key::Mutant(0),
            Key::Mutant(1),
            Key::Flash,
        ]);
        let scar_pairs: Vec<(usize, f32)> = if melanoma {
            tumour.scars.clone()
        } else {
            Vec::new()
        };
        if melanoma {
            keys.extend([
                Key::Tumour,
                Key::TumourCore,
                Key::TumourNucleus,
                Key::Sprout,
                Key::Tip,
            ]);
            keys.extend((0..scar_pairs.len() * 2).map(Key::Scar));
        }
        let mut groups = Groups::new(layer);
        {
            let mut materials = world.resource_mut::<Assets<Glow>>();
            for key in keys {
                let (glow, band, reach) = look(key);
                let each = matches!(
                    key,
                    Key::Nevus
                        | Key::NevusNucleus
                        | Key::Tumour
                        | Key::TumourCore
                        | Key::TumourNucleus
                        | Key::Sprout
                        | Key::Tip
                );
                groups.add(
                    &mut materials,
                    key,
                    glow,
                    band,
                    Vec4::new(0.0, 0.0, 0.0, reach),
                    each,
                );
            }
        }

        // The tissue, in one batch.
        let solid = |s: Solid| match s {
            Solid::Ball => solids.ball.clone(),
            Solid::Plate => solids.plate.clone(),
        };
        let spawned = groups.spawn_many(
            world,
            skin.instances
                .iter()
                .map(|i| (Key::Tissue(i.layer), solid(i.solid), i.transform)),
        );
        let mut crowded = Vec::new();
        for (i, e) in skin.instances.iter().zip(&spawned) {
            let in_a_nest = mole
                .nests
                .iter()
                .any(|n| n.distance(i.transform.translation) < 15.0);
            if in_a_nest
                && matches!(
                    i.layer,
                    Layer::Basal | Layer::Nucleus | Layer::Melanin | Layer::Melanocyte
                )
            {
                crowded.push(*e);
            }
        }
        for (l, mesh) in &meshes.tubes {
            groups.spawn(world, Key::Tissue(*l), mesh, Transform::default());
        }

        // The clones' cells, all made now and shown as the clock reaches
        // them.
        let hidden = Transform::from_scale(Vec3::ZERO);
        let nevus = mole
            .cells
            .iter()
            .map(|_| {
                (
                    groups.spawn(world, Key::Nevus, &solids.ball, hidden),
                    groups.spawn(world, Key::NevusNucleus, &solids.ball, hidden),
                )
            })
            .collect();
        let mut cancer = Vec::new();
        let mut pieces = Vec::new();
        let mut tips = Vec::new();
        if melanoma {
            for c in &tumour.cells {
                let body = if c.core { Key::TumourCore } else { Key::Tumour };
                cancer.push((
                    groups.spawn(world, body, &solids.ball, hidden),
                    groups.spawn(world, Key::TumourNucleus, &solids.ball, hidden),
                ));
            }
            for (n, sprout) in meshes.sprouts.iter().enumerate() {
                for (a, z, mesh) in sprout {
                    let e = groups.spawn(world, Key::Sprout, mesh, Transform::default());
                    pieces.push((n, *a, *z, e));
                }
                for from_start in [true, false] {
                    tips.push((
                        n,
                        from_start,
                        groups.spawn(world, Key::Tip, &solids.ball, hidden),
                    ));
                }
            }
        }

        // Inside the nucleus.
        groups.spawn(world, Key::Envelope, &solids.ball, inside.envelope);
        for pore in &inside.pores {
            groups.spawn(world, Key::Pore, &solids.ring, *pore);
        }
        groups.spawn(
            world,
            Key::Chromatin,
            &meshes.chromatin,
            Transform::default(),
        );
        groups.spawn(world, Key::Nucleolus, &solids.ball, inside.nucleolus);
        for core in &inside.cores {
            groups.spawn(world, Key::Histone, &solids.rod, *core);
        }
        groups.spawn(world, Key::Linker, &meshes.string, Transform::default());
        groups.spawn(
            world,
            Key::Backbone,
            &meshes.backbones,
            Transform::default(),
        );
        for p in &inside.phosphates {
            groups.spawn(world, Key::Phosphate, &solids.bead, *p);
        }
        let mut scars = Vec::new();
        let mut mutant = 0;
        for (i, h) in inside.halves.iter().enumerate() {
            let pair = i / 2;
            let key = if h.mutates {
                mutant += 1;
                Key::Mutant(mutant - 1)
            } else if let Some(n) = scar_pairs.iter().position(|(p, _)| *p == pair) {
                scars.push((pair, h.base, scar_pairs[n].1));
                Key::Scar(scars.len() - 1)
            } else {
                Key::Base(h.base)
            };
            groups.spawn(world, key, &solids.rod, h.transform);
        }
        groups.spawn(
            world,
            Key::Flash,
            &solids.ball,
            Transform::from_scale(Vec3::splat(0.0008)),
        );

        // The surface.
        let surface_material = world
            .resource_mut::<Assets<Surface>>()
            .add(Surface::default());
        let surface_entity = world
            .spawn((
                Mesh3d(meshes.surface.clone()),
                MeshMaterial3d(surface_material.clone()),
                Transform::default(),
                groups.layer.clone(),
            ))
            .id();

        // What moves by itself: red cells round every loop, about one every
        // 13 µm, pigment up the arms of the melanocytes near the middle,
        // and in a melanoma, blood through the new vessels and cells
        // leaving by them.
        let mut runners = Vec::new();
        let mut dice = skin::Dice::new(SEED + 1);
        let mut red = |world: &mut World, groups: &mut Groups<Key>, track, length: f32| {
            let count = (length / 13.0).round() as usize;
            let rate = 20.0 / length.max(1.0);
            (0..count)
                .map(|_| Runner {
                    entity: groups.spawn(
                        world,
                        Key::Tissue(Layer::RedCell),
                        &solids.red_cell,
                        Transform::from_scale(Vec3::splat(3.6)),
                    ),
                    track,
                    phase: dice.unit(),
                    rate: rate * dice.range(0.92, 1.08),
                    size: dice.range(3.4, 3.8),
                    spin: dice.unit() * 6.3,
                })
                .collect::<Vec<_>>()
        };
        for (n, path) in skin.loops.iter().enumerate() {
            runners.extend(red(world, &mut groups, Track::Blood(n), path.length()));
        }
        if melanoma {
            for (n, sp) in tumour.sprouts.iter().enumerate() {
                runners.extend(red(world, &mut groups, Track::Sprout(n), sp.path.length()));
                for k in 0..3 {
                    runners.push(Runner {
                        entity: groups.spawn(world, Key::Tumour, &solids.ball, hidden),
                        track: Track::Leaving(n),
                        phase: k as f32 / 3.0,
                        rate: 0.09,
                        size: 4.6,
                        spin: 0.0,
                    });
                }
            }
        }
        let mut dice = skin::Dice::new(SEED + 5);
        for (m, cell) in skin.melanocytes.iter().enumerate() {
            if cell.at.length() > 90.0 {
                continue;
            }
            for (d, arm) in cell.dendrites.iter().enumerate() {
                let count = (arm.length() / 4.0).round().max(2.0) as usize;
                for k in 0..count {
                    runners.push(Runner {
                        entity: groups.spawn(
                            world,
                            Key::Tissue(Layer::Melanin),
                            &solids.bead,
                            Transform::from_scale(Vec3::splat(0.5)),
                        ),
                        track: Track::Pigment(m, d),
                        phase: (k as f32 + dice.unit()) / count as f32,
                        rate: 2.2 / arm.length().max(1.0),
                        size: dice.range(0.4, 0.6),
                        spin: 0.0,
                    });
                }
            }
        }

        Ok(Self {
            groups,
            runners,
            nevus,
            cancer,
            pieces,
            tips,
            crowded,
            scars,
            surface: (surface_entity, surface_material),
            places: places(b),
            melanoma,
            mutated: None,
            placed: None,
        })
    }

    pub fn places(&self) -> &[Place] {
        &self.places
    }

    pub fn things() -> Vec<&'static str> {
        THINGS.iter().copied().chain(Layer::names()).collect()
    }

    /// The clock in words, for the corner of the panel.
    pub fn clock_label(t: f32) -> Option<String> {
        if t < 0.0 {
            None
        } else if t < 1.0 {
            Some(format!("{:.0} months", t * 12.0))
        } else {
            Some(format!("year {t:.1}"))
        }
    }

    /// Where a named thing is: one of the world's own, or the part of a
    /// layer nearest what the camera looks at.
    pub fn locate(&self, name: &str, years: f32, focus: Vec3) -> Option<Vec3> {
        let b = built();
        let nearest = |points: &mut dyn Iterator<Item = Vec3>| {
            points.min_by(|a, b| a.distance(focus).total_cmp(&b.distance(focus)))
        };
        let t = &b.tumour;
        match name {
            "first_melanocyte" | "dna" | "braf_codon_600" => return Some(Vec3::ZERO),
            "nucleus" => return Some(b.inside.envelope.translation),
            "surface" | "mole" => return Some(Vec3::new(0.0, b.skin.surface, 0.0)),
            "dermal_papilla" => return nearest(&mut b.skin.papillae.iter().copied()),
            "nuclear_pore" => return nearest(&mut b.inside.pores.iter().map(|t| t.translation)),
            "nucleosome" => return nearest(&mut b.inside.cores.iter().map(|t| t.translation)),
            "nucleolus" => return Some(b.inside.nucleolus.translation),
            "chromatin" => {
                return Some(b.inside.envelope.translation + Vec3::new(-0.8, 0.4, 0.5));
            }
            "melanoma" => return Some(t.start),
            "breach" => return Some(t.breach),
            "nodule" => return Some(t.nodule),
            "new_vessel" => return t.sprouts.first().map(|s| s.path.at(0.5).0),
            "sprout_tip" => {
                let g = Tumour::sprouted(years) * 0.5;
                return t.sprouts.first().map(|s| s.path.at(g.max(0.02)).0);
            }
            _ => {}
        }
        if let Some(n) = name
            .strip_prefix("nest_")
            .and_then(|n| n.parse::<usize>().ok())
        {
            return b.mole.nest_at(n.checked_sub(1)?, years);
        }
        let layer = Layer::named(name)?;
        nearest(
            &mut b
                .skin
                .instances
                .iter()
                .filter(|i| i.layer == layer)
                .map(|i| i.transform.translation),
        )
    }

    /// What a ray from the camera meets first, among what is shown.
    pub fn pick(&self, origin: Vec3, direction: Vec3, years: f32, d: f32) -> Option<String> {
        let b = built();
        let mut best: Option<(f32, String)> = None;
        let mut consider = |centre: Vec3, r: f32, what: &dyn Fn() -> String| {
            let c = centre - origin;
            let along = c.dot(direction);
            let miss = c.length_squared() - along * along;
            if along > 0.0 && miss < r * r {
                let t = along - (r * r - miss).sqrt();
                if best.as_ref().is_none_or(|(b, _)| t < *b) {
                    best = Some((t, what()));
                }
            }
        };
        for i in &b.skin.instances {
            // Nuclei and caps are inside cells: the cell is what is
            // clicked.
            if matches!(
                i.layer,
                Layer::Nucleus | Layer::FadingNucleus | Layer::Melanin
            ) || !self.groups.seen(Key::Tissue(i.layer), d)
            {
                continue;
            }
            consider(i.transform.translation, i.reach() * 0.8, &|| {
                format!("{}: {}", i.layer.name(), i.layer.what())
            });
        }
        if self.groups.seen(Key::Nevus, d) {
            for k in 0..b.mole.cells.len() {
                if let Some(seen) = b.mole.seen(k, years, &b.skin) {
                    consider(seen.at, seen.size, &|| {
                        let state = if years < nevus::BRAKE.0 {
                            "dividing"
                        } else {
                            "no longer dividing, held by the senescence BRAF's own signal set off"
                        };
                        format!(
                            "naevus_cell: a cell of the mole, a descendant of the first melanocyte carrying its BRAF mutation, {state}"
                        )
                    });
                }
            }
        }
        if self.melanoma && self.groups.seen(Key::Tumour, d) {
            for (k, c) in b.tumour.cells.iter().enumerate() {
                if let Some((at, size)) = b.tumour.seen(k, years) {
                    consider(at, size, &|| {
                        let place = if c.core && years > tumour::SPROUTS.0 {
                            "in the nodule's middle, short of oxygen, calling for vessels with VEGF"
                        } else if c.at.y < b.skin.junction.height(c.at.x, c.at.z) {
                            "invading the dermis, in the nodule"
                        } else {
                            "spreading in the epidermis"
                        };
                        let endless = if years >= tumour::IMMORTAL {
                            "; with telomerase switched on, it can divide without end"
                        } else {
                            ""
                        };
                        format!(
                            "melanoma_cell: a descendant of the mole's cell that lost p16 (CDKN2A) and kept dividing, {place}{endless}"
                        )
                    });
                }
            }
        }
        if self.groups.seen(Key::Histone, d) {
            for c in &b.inside.cores {
                consider(c.translation, 0.005, &|| {
                    "nucleosome: DNA wound 1.65 times round a core of eight histones, the first step of packing two metres of DNA into the nucleus".to_owned()
                });
            }
        }
        if self.groups.seen(Key::Backbone, d) {
            for (i, h) in b.inside.halves.iter().enumerate() {
                let scar = self.scars.iter().find(|s| s.0 == i / 2);
                consider(h.transform.translation, 0.0005, &|| {
                    if h.mutates {
                        format!(
                            "the base pair that mutates: codon 600 of BRAF, {} (valine) becoming {} (glutamate), BRAF V600E",
                            b.inside.codon.0, b.inside.codon.1
                        )
                    } else if let Some((_, _, when)) = scar.filter(|s| years >= s.2) {
                        format!(
                            "a base pair mutated in year {when:.0}: one of the many changes a melanoma's DNA gathers, most of them left by ultraviolet light"
                        )
                    } else {
                        format!("a base of BRAF's DNA: {:?}", h.base)
                    }
                });
            }
        }
        best.map(|(_, what)| what)
    }

    /// Each frame: what the scale shows, the clones and the vessels as the
    /// clock has them, the surface, and what moves by itself.
    pub fn update(
        &mut self,
        tick: &Tick,
        things: &mut Things,
        glows: &mut Assets<Glow>,
        surfaces: &mut Assets<Surface>,
    ) {
        let b = built();
        let years = tick.clock;
        let d = tick.eye.distance;
        if tick.before < 0.0 && years >= 0.0 {
            self.mutated = Some(tick.life);
        }
        if years < 0.0 {
            self.mutated = None;
        }
        let flash = self.mutated.map_or(0.0, |t| (-(tick.life - t) * 2.0).exp());
        // The first melanocyte rounds up and loses its arms once it has
        // mutated; the clone takes over from it.
        let rounding = ease((years - 0.05) / 0.6);
        self.groups.update(
            tick,
            picks,
            |key, s| match key {
                Key::Tissue(Layer::First) => s * (1.0 - rounding) * (1.0 + 2.0 * flash),
                Key::Tissue(Layer::FirstNucleus) => s * (1.0 - rounding),
                Key::Flash => 3.0 * flash,
                _ => s,
            },
            things,
            glows,
        );

        // Looks that follow the clock.
        let (c, k) = nevus::colour(years);
        self.groups.restyle(
            Key::Nevus,
            c[0] * 7.0 + c[1] * 3.0 + c[2] + k * 5.0,
            glows,
            |m| {
                m.rim = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.45 * k);
                m.body = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.025 * k);
            },
        );
        // The brake: the nuclei change colour as the clone stops; in a
        // melanoma, the clone that escapes it keeps its colour.
        let brake = ease((years - nevus::BRAKE.0) / (nevus::BRAKE.1 - nevus::BRAKE.0));
        self.groups.restyle(Key::NevusNucleus, brake, glows, |m| {
            let c = Vec3::new(0.4, 0.55, 1.0).lerp(Vec3::new(1.0, 0.3, 0.55), brake);
            m.rim = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.22 + 0.06 * brake);
        });
        // Short of oxygen, the nodule's middle turns from crimson to violet.
        let hypoxia =
            ease((years - tumour::SPROUTS.0 + 1.0) / 2.0) * (1.0 - 0.6 * Tumour::sprouted(years));
        self.groups.restyle(Key::TumourCore, hypoxia, glows, |m| {
            let c = Vec3::new(1.0, 0.16, 0.42).lerp(Vec3::new(0.45, 0.2, 1.0), hypoxia);
            m.rim = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.42);
            m.body = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.03);
        });
        // The basement membrane gives way under the melanoma.
        if self.melanoma {
            let width = b.tumour.breach_width(years);
            let breach = b.tumour.breach;
            self.groups
                .restyle(Key::Tissue(Layer::Membrane), width / 5.0, glows, |m| {
                    m.hole = breach.extend(width);
                });
        }
        let inside = &b.inside;
        for (mutant, h) in inside.halves.iter().filter(|h| h.mutates).enumerate() {
            let shade = f32::from(years >= 0.0) + flash;
            let key = Key::Mutant(mutant);
            self.groups.restyle(key, shade, glows, |m| {
                // T and A trade places on the two strands, brighter than
                // their neighbours once changed, very bright as they change.
                let base = if years >= 0.0 { h.base.pair() } else { h.base };
                let c = base.colour();
                let boost = if years >= 0.0 { 1.6 } else { 1.0 } + 4.0 * flash;
                m.rim = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.7 * boost);
                m.body = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.35 * boost);
            });
        }
        for (n, (_, base, when)) in self.scars.iter().enumerate() {
            let changed = years >= *when;
            self.groups
                .restyle(Key::Scar(n), f32::from(changed), glows, |m| {
                    let shown = if changed { base.transition() } else { *base };
                    let c = shown.colour();
                    let boost = if changed { 1.4 } else { 1.0 };
                    m.rim = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.7 * boost);
                    m.body = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.35 * boost);
                });
        }

        // The clones' cells where the clock puts them, when it has moved.
        let nevus_shown = !self.groups.hidden(Key::Nevus);
        let cancer_shown = self.melanoma && !self.groups.hidden(Key::Tumour);
        let placed = (years, nevus_shown, cancer_shown);
        if self.placed != Some(placed) {
            self.placed = Some(placed);
            for (k, (body, core)) in self.nevus.iter().enumerate() {
                let seen = nevus_shown
                    .then(|| b.mole.seen(k, years, &b.skin))
                    .flatten();
                for (e, part) in [(*body, 1.0), (*core, 0.5)] {
                    set_shown(things, e, seen.is_some());
                    if let (Some(s), Ok((mut t, _))) = (seen, things.get_mut(e)) {
                        t.translation = s.at;
                        t.scale = Vec3::splat(s.size * part);
                    }
                }
            }
            for (k, (body, core)) in self.cancer.iter().enumerate() {
                let seen = cancer_shown.then(|| b.tumour.seen(k, years)).flatten();
                for (e, part) in [(*body, 1.0), (*core, 0.55)] {
                    set_shown(things, e, seen.is_some());
                    if let (Some((at, size)), Ok((mut t, _))) = (seen, things.get_mut(e)) {
                        t.translation = at;
                        t.scale = Vec3::splat(size * part);
                    }
                }
            }
            // The sprouts grow from both ends and meet in the middle.
            let grown = Tumour::sprouted(years) * 0.5;
            let vessels_shown = self.melanoma && !self.groups.hidden(Key::Sprout);
            for (_, a, z, e) in &self.pieces {
                let reached = *z <= grown + 0.02 || *a >= 1.0 - grown - 0.02;
                set_shown(things, *e, vessels_shown && grown > 0.0 && reached);
            }
            for (n, from_start, e) in &self.tips {
                let growing = vessels_shown && grown > 0.0 && grown < 0.5;
                set_shown(things, *e, growing);
                if growing && let Ok((mut t, _)) = things.get_mut(*e) {
                    let path = &b.tumour.sprouts[*n].path;
                    let at = if *from_start { grown } else { 1.0 - grown };
                    let (p, heading) = path.at(at);
                    t.translation = p;
                    t.rotation = Quat::from_rotation_arc(Vec3::X, heading);
                    t.scale = Vec3::new(9.0, 2.6, 2.6);
                }
            }
        }
        // The basal cells where the nests sit, crowded out while the nests
        // are at the junction, back once the nests have sunk below it.
        let crowding = b.mole.crowding(years) * (1.0 - ease((years - 18.0) / 12.0));
        let basal_shown = !self.groups.hidden(Key::Tissue(Layer::Basal));
        for e in &self.crowded {
            set_shown(things, *e, basal_shown && crowding < 0.5);
        }

        // The surface: the mole growing over its first years, darkest in
        // its first decade, raised and paler after decades; or the
        // melanoma spreading out of it, uneven in shape and colour.
        let outside = OUTSIDE.at(d);
        set_shown(things, self.surface.0, outside > 0.003);
        if outside > 0.003 {
            let sink = if self.melanoma {
                0.0
            } else {
                ease((years - 15.0) / 35.0)
            };
            let growth = if self.melanoma {
                ease((years - tumour::BRAKE_LOST) / 9.0)
            } else {
                0.0
            };
            let radius = if years <= 0.3 {
                0.0
            } else {
                1_800.0 * ease(years / 8.0).max(0.04) * (1.0 + 0.8 * growth)
            };
            let mole = Vec4::new(0.0, 0.0, radius, ease(years / 3.0) * (1.0 - 0.45 * sink));
            let shape = Vec4::new(320.0 * sink, outside, sink, growth);
            if let Some(m) = surfaces.get(&self.surface.1)
                && (m.mole != mole || m.shape != shape)
                && let Some(mut m) = surfaces.get_mut(&self.surface.1)
            {
                m.mole = mole;
                m.shape = shape;
            }
        }

        let blood = years >= tumour::SPROUTS.1;
        let leaving = years >= tumour::LEAVING;
        for runner in &self.runners {
            let f = (runner.phase + tick.life * runner.rate).fract();
            let Ok((mut t, _)) = things.get_mut(runner.entity) else {
                continue;
            };
            match runner.track {
                Track::Blood(n) | Track::Sprout(n) => {
                    let path = match runner.track {
                        Track::Blood(_) => &b.skin.loops[n],
                        _ => &b.tumour.sprouts[n].path,
                    };
                    let (p, heading) = path.at(f);
                    let flowing = matches!(runner.track, Track::Blood(_)) || blood;
                    t.translation = p;
                    t.rotation = Quat::from_rotation_arc(Vec3::Y, heading)
                        * Quat::from_rotation_x(0.5)
                        * Quat::from_rotation_y(runner.spin);
                    t.scale = Vec3::splat(if flowing { runner.size } else { 0.0 });
                }
                Track::Pigment(m, d) => {
                    let (p, _) = b.skin.melanocytes[m].dendrites[d].at(f);
                    t.translation = p;
                    // Handed over at the tip: it shrinks into the
                    // keratinocyte that takes it.
                    let mut fade = ((1.0 - f) / 0.12).clamp(0.0, 1.0) * (f / 0.05).clamp(0.0, 1.0);
                    if m == 0 {
                        fade *= 1.0 - rounding;
                    }
                    t.scale = Vec3::new(1.0, 0.7, 0.8) * runner.size * fade.max(0.001);
                }
                Track::Leaving(n) => {
                    // From the nodule's middle out along the new vessel,
                    // squeezed thin, and away.
                    let (p, heading) = b.tumour.sprouts[n].path.at(0.5 + 0.5 * f);
                    let fade = (f / 0.1).clamp(0.0, 1.0) * ((1.0 - f) / 0.15).clamp(0.0, 1.0);
                    t.translation = p;
                    t.rotation = Quat::from_rotation_arc(Vec3::X, heading);
                    t.scale = if leaving {
                        Vec3::new(1.6, 0.6, 0.6) * runner.size * fade.max(0.001)
                    } else {
                        Vec3::ZERO
                    };
                }
            }
        }
    }

    /// Every part, to take the world away.
    pub fn entities(self) -> Vec<Entity> {
        let mut all = self.groups.entities();
        all.push(self.surface.0);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_offered_are_the_places_made() {
        let made: Vec<&str> = places(built()).iter().map(|p| p.name).collect();
        assert_eq!(made, PLACES.to_vec());
    }

    #[test]
    fn setting_apart_dims_the_rest_but_not_what_is_alone_at_its_scale() {
        assert!(picks("mole", Key::Nevus));
        assert!(!picks("mole", Key::Tissue(Layer::Spiny)));
        assert!(picks("mole", Key::Chromatin));
        assert!(picks("braf_codon_600", Key::Mutant(0)));
        assert!(!picks("braf_codon_600", Key::Base(Base::G)));
        assert!(picks("melanoma", Key::TumourCore));
        assert!(picks("sprouts", Key::Tip));
    }
}
