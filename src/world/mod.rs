//! Worlds: places in three dimensions, built cell by cell ahead of time,
//! that the agent films rather than draws. It names a place to go to, what
//! to say there and how far the world's clock runs, and the window flies
//! the camera from one scale to the next, with what lives in the world
//! moving all the while: red cells running through the capillaries,
//! pigment travelling up a melanocyte's arms, a mole growing over the
//! years. The user can take the camera at any moment, turn it, come
//! closer, click on a cell.
//!
//! The skin is the first world: from a forearm seen at a hand's length,
//! through its surface into the epidermis cell by cell, down into one
//! melanocyte's nucleus and its DNA, nine orders of magnitude in one move.
//! Each part of it is drawn at the scales it is seen at and fades out of
//! the others.

pub mod film;
pub mod glow;
pub mod nevus;
pub mod nucleus;
pub mod shapes;
pub mod skin;
pub mod surface;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::stage::Stage;
use film::{Eye, Film, Place};
use glow::Glow;
use nevus::Mole;
use nucleus::{Base, Nucleus};
use skin::{Layer, Skin, Solid};
use surface::Surface;

/// What a world's camera clears to: the dark things glow on.
pub const BACKGROUND: Color = Color::srgb(0.005, 0.007, 0.016);

/// The seed the skin is built from.
const SEED: u64 = 7;

/// The scales a part of the world is seen at: gone below `near.0`, whole
/// from `near.1` up to `far.0`, gone again from `far.1`, in micrometres of
/// the camera's distance to what it looks at, faded between.
#[derive(Debug, Clone, Copy)]
pub struct Band {
    pub near: (f32, f32),
    pub far: (f32, f32),
}

impl Band {
    const fn new(near: (f32, f32), far: (f32, f32)) -> Self {
        Self { near, far }
    }

    /// How much is shown when the camera is `d` away, 0 to 1. The fades
    /// run evenly in the logarithm of the distance, as zooming does.
    pub fn at(&self, d: f32) -> f32 {
        let ramp = |lo: f32, hi: f32| {
            let t = ((d.ln() - lo.ln()) / (hi.ln() - lo.ln())).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let rise = if self.near.1 <= 0.0 {
            1.0
        } else {
            ramp(self.near.0, self.near.1)
        };
        let fall = if self.far.0 >= f32::MAX {
            1.0
        } else {
            1.0 - ramp(self.far.0, self.far.1)
        };
        rise * fall
    }
}

const NONE: (f32, f32) = (0.0, 0.0);
const NEVER: (f32, f32) = (f32::MAX, f32::MAX);
/// The tissue is seen from a few millimetres, when the skin's surface
/// fades, down to the inside of one cell.
const TISSUE: (f32, f32) = (1400.0, 3200.0);
/// The surface, from far away down to two millimetres.
const OUTSIDE: Band = Band::new((1000.0, 2400.0), NEVER);
const NUCLEUS: Band = Band::new((0.25, 0.8), (14.0, 30.0));
const CHROMATIN: Band = Band::new((0.06, 0.25), (16.0, 35.0));
const NUCLEOSOMES: Band = Band::new((0.012, 0.03), (0.3, 0.8));
const HELIX: Band = Band::new(NONE, (0.025, 0.07));

/// A part of the world that has a look of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Tissue(Layer),
    Nevus,
    NevusNucleus,
    Envelope,
    Pore,
    Chromatin,
    Nucleolus,
    Histone,
    Linker,
    Backbone,
    Phosphate,
    Base(Base),
    /// The two halves of the pair that mutates.
    Mutant(usize),
    Flash,
}

/// How a part looks, at which scales it is seen, and how far from the
/// origin it reaches before fading. Strengths are low because a tissue is
/// many cells deep and their light adds up.
fn look(key: Key) -> (Glow, Band, f32) {
    let g = Glow::new;
    let tissue = |glow: Glow| (glow, Band::new((5.0, 13.0), TISSUE), skin::RADIUS);
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
            Layer::First => (
                g([1.0, 0.7, 0.2], 0.9, 0.0)
                    .faces([1.0, 0.55, 0.12], 0.06)
                    .edges(2.0),
                // Gone before the camera reaches the nucleus inside it.
                Band::new((8.0, 20.0), TISSUE),
                skin::RADIUS,
            ),
            Layer::FirstNucleus => (
                g([0.4, 0.6, 1.0], 0.5, 0.0)
                    .faces([0.25, 0.4, 1.0], 0.12)
                    .edges(1.8),
                // The envelope takes over as the camera comes near.
                Band::new((10.0, 25.0), TISSUE),
                skin::RADIUS,
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
        Key::Nevus => (
            g([1.0, 0.55, 0.12], 0.5, 0.0)
                .faces([1.0, 0.5, 0.1], 0.03)
                .edges(2.0),
            Band::new((1.5, 5.0), TISSUE),
            skin::RADIUS,
        ),
        Key::NevusNucleus => (
            g([0.4, 0.55, 1.0], 0.25, 0.0)
                .faces([0.3, 0.4, 1.0], 0.05)
                .edges(1.8),
            Band::new((1.5, 5.0), TISSUE),
            skin::RADIUS,
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
        // Coloured by its base when made, and by the clock after.
        Key::Mutant(_) => helix(g([1.0, 1.0, 1.0], 0.9, 0.0).edges(1.5)),
        Key::Flash => (
            g([1.0, 1.0, 1.0], 0.0, 0.0).faces([1.0, 0.95, 0.85], 0.0),
            HELIX,
            0.0,
        ),
    }
}

/// One group of the world's parts that share a look and fade together.
struct Group {
    material: Handle<Glow>,
    band: Band,
    /// How strong it is when wholly shown.
    strength: f32,
    entities: Vec<Entity>,
    hidden: bool,
    /// Whether its parts are shown and hidden one by one, by the clock,
    /// rather than all together.
    each: bool,
    /// What its material was last given: its strength, how much it stood
    /// out, and the shade the clock gave it.
    written: (f32, f32, f32),
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
}

/// One world, as shown in a panel.
pub struct WorldScene {
    pub camera: Entity,
    groups: HashMap<Key, Group>,
    runners: Vec<Runner>,
    /// The mole's cells, body and nucleus, by the clone's index.
    nevus: Vec<(Entity, Entity)>,
    /// Basal cells and their parts where the nests sit, crowded out as
    /// the nests fill.
    crowded: Vec<Entity>,
    surface: (Entity, Handle<Surface>),
    skin: Arc<Skin>,
    mole: Arc<Mole>,
    inside: Arc<Nucleus>,
    places: Vec<Place>,
    /// Where the camera is now.
    pub eye: Eye,
    /// The film, how far into it, whether it plays, and where the camera
    /// was when it began.
    pub film: Film,
    pub clock: f32,
    pub playing: bool,
    film_from: Eye,
    /// Whether the user has the camera; when the film plays again the
    /// camera rejoins it over a moment.
    pub manual: bool,
    rejoin: f32,
    /// Seconds since the world opened, which what moves by itself moves by.
    life: f32,
    /// The world's clock, in years since the first melanocyte's mutation.
    pub years: f32,
    /// When, in `life`, the mutation happened on screen.
    mutated: Option<f32>,
    /// The clock and whether the mole was shown when its cells were last
    /// placed.
    mole_placed: Option<(f32, bool)>,
    /// What is being said, and how visible the words are.
    pub say: Option<(String, f32)>,
    /// What is set apart from the rest when no film says otherwise.
    pub highlight: Vec<String>,
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
                    "blood" => matches!(layer, Layer::Vessel | Layer::RedCell),
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
        // The pair that mutates stands out from the rest of the helix when
        // asked to.
        Key::Base(_) | Key::Backbone | Key::Phosphate => {
            !matches!(name, "braf_codon_600" | "mutation")
        }
        // What is inside the nucleus is never dimmed otherwise: it is alone
        // at its scale.
        _ => true,
    }
}

/// How much a part is shown under what is set apart: whole if it is one
/// of them or nothing is, dimmed if not.
fn emphasis(names: &[String], key: Key) -> f32 {
    if names.is_empty() || names.iter().any(|n| picks(n, key)) {
        1.0
    } else {
        0.26
    }
}

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

/// The places the camera can be sent to, as `skin_places` makes them.
pub const PLACES: [&str; 14] = [
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
];

/// Groups of layers that can be set apart together.
const HIGHLIGHTS: [&str; 10] = [
    "mole",
    "braf_codon_600",
    "pigment",
    "keratinocytes",
    "blood",
    "dermis",
    "junction",
    "first_melanocyte",
    "basal_layer",
    "basement_membrane",
];

impl WorldScene {
    pub fn places(&self) -> &[Place] {
        &self.places
    }

    pub fn place(&self, name: &str) -> Option<Eye> {
        self.places.iter().find(|p| p.name == name).map(|p| p.eye)
    }

    /// Play `film` from where the camera is.
    pub fn play(&mut self, film: Film) {
        self.film_from = self.eye;
        self.film = film;
        self.clock = 0.0;
        self.playing = true;
        self.manual = false;
        self.rejoin = 0.0;
    }

    /// Play the film again from its start.
    pub fn replay(&mut self) {
        self.clock = 0.0;
        self.playing = true;
        self.manual = false;
        self.rejoin = 0.0;
        self.years = self.film.start_years;
    }

    /// Take the camera from the film.
    pub fn take(&mut self) {
        self.manual = true;
    }

    /// Give it back, to rejoin the film.
    pub fn give_back(&mut self) {
        if self.manual {
            self.manual = false;
            self.rejoin = 1.0;
        }
    }

    /// The names notes can point at, besides the layers.
    pub fn things() -> &'static [&'static str] {
        &[
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
        ]
    }

    /// Where a named thing is: one of the world's own, or the part of a
    /// layer nearest what the camera looks at.
    pub fn locate(&self, name: &str) -> Option<Vec3> {
        let focus = self.eye.focus;
        let nearest = |points: &mut dyn Iterator<Item = Vec3>| {
            points.min_by(|a, b| a.distance(focus).total_cmp(&b.distance(focus)))
        };
        match name {
            "first_melanocyte" | "dna" | "braf_codon_600" => return Some(Vec3::ZERO),
            "nucleus" => return Some(self.inside.envelope.translation),
            "surface" | "mole" => return Some(Vec3::new(0.0, self.skin.surface, 0.0)),
            "dermal_papilla" => return nearest(&mut self.skin.papillae.iter().copied()),
            "nuclear_pore" => {
                return nearest(&mut self.inside.pores.iter().map(|t| t.translation));
            }
            "nucleosome" => {
                return nearest(&mut self.inside.cores.iter().map(|t| t.translation));
            }
            "nucleolus" => return Some(self.inside.nucleolus.translation),
            "chromatin" => {
                return Some(self.inside.envelope.translation + Vec3::new(-0.8, 0.4, 0.5));
            }
            _ => {}
        }
        if let Some(n) = name
            .strip_prefix("nest_")
            .and_then(|n| n.parse::<usize>().ok())
        {
            return self.mole.nest_at(n.checked_sub(1)?, self.years);
        }
        let layer = LAYERS.iter().find(|l| l.name() == name)?;
        nearest(
            &mut self
                .skin
                .instances
                .iter()
                .filter(|i| i.layer == *layer)
                .map(|i| i.transform.translation),
        )
    }

    /// What a ray from the camera meets first, among what is shown.
    pub fn pick(&self, origin: Vec3, direction: Vec3) -> Option<String> {
        let d = self.eye.distance;
        let shown = |key: Key| {
            self.groups
                .get(&key)
                .is_some_and(|g| !g.hidden && g.band.at(d) > 0.2)
        };
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
        for i in &self.skin.instances {
            // Nuclei and caps are inside cells: the cell is what is
            // clicked.
            if matches!(
                i.layer,
                Layer::Nucleus | Layer::FadingNucleus | Layer::Melanin
            ) || !shown(Key::Tissue(i.layer))
            {
                continue;
            }
            consider(i.transform.translation, i.reach() * 0.8, &|| {
                format!("{}: {}", i.layer.name(), i.layer.what())
            });
        }
        if shown(Key::Nevus) {
            for k in 0..self.mole.cells.len() {
                if let Some(seen) = self.mole.seen(k, self.years, &self.skin) {
                    consider(seen.at, seen.size, &|| {
                        let state = if self.years < nevus::BRAKE.0 {
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
        if shown(Key::Histone) {
            for c in &self.inside.cores {
                consider(c.translation, 0.005, &|| {
                    "nucleosome: DNA wound 1.65 times round a core of eight histones, the first step of packing two metres of DNA into the nucleus".to_owned()
                });
            }
        }
        if shown(Key::Backbone) {
            for h in &self.inside.halves {
                consider(h.transform.translation, 0.0005, &|| {
                    if h.mutates {
                        format!(
                            "the base pair that mutates: codon 600 of BRAF, {} (valine) becoming {} (glutamate), BRAF V600E",
                            self.inside.codon.0, self.inside.codon.1
                        )
                    } else {
                        format!("a base of BRAF's DNA: {:?}", h.base)
                    }
                });
            }
        }
        best.map(|(_, what)| what)
    }
}

/// Every layer, to find one by its name.
pub(crate) const LAYERS: [Layer; 17] = [
    Layer::Horny,
    Layer::Granular,
    Layer::Spiny,
    Layer::Basal,
    Layer::Nucleus,
    Layer::FadingNucleus,
    Layer::Melanin,
    Layer::Melanocyte,
    Layer::First,
    Layer::FirstNucleus,
    Layer::Membrane,
    Layer::Collagen,
    Layer::Elastic,
    Layer::Fibroblast,
    Layer::Vessel,
    Layer::RedCell,
    Layer::Immune,
];

/// What the skin world is made of, built once.
struct Built {
    skin: Arc<Skin>,
    mole: Arc<Mole>,
    inside: Arc<Nucleus>,
}

fn built() -> &'static Built {
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
        Built {
            skin: Arc::new(skin),
            mole: Arc::new(mole),
            inside: Arc::new(inside),
        }
    })
}

/// Build what takes time before it is asked for.
pub fn warm() {
    std::thread::spawn(|| {
        built();
    });
}

/// The places the camera can be sent to in the skin.
fn skin_places(skin: &Skin, mole: &Mole) -> Vec<Place> {
    let s = skin.surface;
    let capillary = skin
        .papillae
        .iter()
        .min_by(|a, b| a.length().total_cmp(&b.length()))
        .copied()
        .unwrap_or(Vec3::new(40.0, 0.0, 0.0));
    let nests = mole.nests.iter().sum::<Vec3>() / mole.nests.len().max(1) as f32;
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
            "the skin's surface close up, a few millimetres across: its lines, pores, and the mole",
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
    ]
}

/// Every world shown, by panel.
#[derive(Resource, Default)]
pub struct Worlds {
    pub map: HashMap<String, WorldScene>,
    next: usize,
    meshes: Option<Meshes>,
}

/// The meshes of the skin, made once and shared by every panel.
#[derive(Clone)]
struct Meshes {
    solids: HashMap<Solid, Handle<Mesh>>,
    tubes: Vec<(Layer, Handle<Mesh>)>,
    rod: Handle<Mesh>,
    ring: Handle<Mesh>,
    chromatin: Handle<Mesh>,
    string: Handle<Mesh>,
    backbones: Handle<Mesh>,
    surface: Handle<Mesh>,
}

/// The names of the worlds there are.
pub const NAMES: [&str; 1] = ["skin"];

/// Put a world in a panel, or keep the one it has.
pub fn show(world: &mut World, panel: &str, name: &str) -> Result<(), String> {
    if name != "skin" {
        return Err(format!("no world {name}; there is {}", NAMES.join(", ")));
    }
    if world
        .get_resource::<Worlds>()
        .is_some_and(|w| w.map.contains_key(panel))
    {
        return Ok(());
    }
    let b = built();
    let (skin, mole, inside) = (b.skin.clone(), b.mole.clone(), b.inside.clone());
    let mut worlds = world.remove_resource::<Worlds>().unwrap_or_default();
    worlds.next += 1;
    let layer = RenderLayers::layer(40 + worlds.next % 20);
    let order = 60 + (worlds.next % 20) as isize;
    let meshes = worlds
        .meshes
        .get_or_insert_with(|| {
            let mut assets = world.resource_mut::<Assets<Mesh>>();
            let solids = [
                (Solid::Ball, shapes::ball()),
                (Solid::Bead, shapes::bead()),
                (Solid::Plate, shapes::plate()),
                (Solid::RedCell, shapes::red_cell()),
            ]
            .into_iter()
            .map(|(s, m)| (s, assets.add(m)))
            .collect::<HashMap<_, _>>();
            let tubes = skin
                .tubes
                .iter()
                .filter(|(_, b)| !b.is_empty())
                .map(|(l, b)| (*l, assets.add(b.mesh())))
                .collect();
            Meshes {
                solids,
                tubes,
                rod: assets.add(Cylinder::new(1.0, 2.0).mesh().resolution(12).build()),
                ring: assets.add(
                    Torus {
                        minor_radius: 0.35,
                        major_radius: 1.0,
                    }
                    .mesh()
                    .build(),
                ),
                chromatin: assets.add(inside.chromatin.mesh()),
                string: assets.add(inside.string.mesh()),
                backbones: assets.add(inside.backbones.mesh()),
                surface: assets.add(surface::mesh(skin.surface, 40_000.0)),
            }
        })
        .clone();
    let places = skin_places(&skin, &mole);
    let eye = places
        .iter()
        .find(|p| p.name == "skin")
        .map_or(places[0].eye, |p| p.eye);
    let camera = world
        .spawn((
            Camera3d::default(),
            Camera {
                order,
                clear_color: ClearColorConfig::Custom(BACKGROUND),
                viewport: Some(Viewport {
                    physical_position: UVec2::ZERO,
                    physical_size: UVec2::new(16, 16),
                    ..default()
                }),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 0.75,
                near: 0.5,
                ..default()
            }),
            bevy::camera::Hdr,
            Tonemapping::AcesFitted,
            Bloom {
                intensity: 0.3,
                low_frequency_boost: 0.75,
                ..Bloom::NATURAL
            },
            // Light added layer on layer would be blended four times over
            // with multisampling; the edges are smoothed after instead.
            Msaa::Off,
            bevy::anti_alias::fxaa::Fxaa::default(),
            eye.transform(),
            layer.clone(),
        ))
        .id();

    // Every look the world needs, one material each.
    let mut keys: Vec<Key> = LAYERS.iter().map(|l| Key::Tissue(*l)).collect();
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
    let mut groups: HashMap<Key, Group> = HashMap::new();
    {
        let mut materials = world.resource_mut::<Assets<Glow>>();
        for key in keys {
            let (mut glow, band, reach) = look(key);
            glow.spot = Vec4::new(0.0, 0.0, 0.0, reach);
            glow.band = Vec4::new(
                band.near.0,
                band.near.1,
                band.far.0.min(1e30),
                band.far.1.min(1e30),
            );
            let strength = glow.shape.y;
            groups.insert(
                key,
                Group {
                    material: materials.add(glow),
                    band,
                    strength,
                    entities: Vec::new(),
                    hidden: false,
                    each: matches!(key, Key::Nevus | Key::NevusNucleus),
                    written: (strength, 1.0, f32::NAN),
                },
            );
        }
    }
    // The tissue, in one batch.
    let ball = meshes.solids[&Solid::Ball].clone();
    let bead = meshes.solids[&Solid::Bead].clone();
    let batch: Vec<_> = skin
        .instances
        .iter()
        .map(|i| {
            (
                Mesh3d(meshes.solids[&i.solid].clone()),
                MeshMaterial3d(groups[&Key::Tissue(i.layer)].material.clone()),
                i.transform,
                layer.clone(),
            )
        })
        .collect();
    let spawned: Vec<Entity> = world.spawn_batch(batch).collect();
    let mut crowded = Vec::new();
    for (i, e) in skin.instances.iter().zip(&spawned) {
        if let Some(g) = groups.get_mut(&Key::Tissue(i.layer)) {
            g.entities.push(*e);
        }
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
    let mut add = |world: &mut World, key: Key, mesh: &Handle<Mesh>, t: Transform| {
        let material = groups[&key].material.clone();
        let e = world
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                t,
                layer.clone(),
            ))
            .id();
        if let Some(g) = groups.get_mut(&key) {
            g.entities.push(e);
        }
        e
    };

    for (l, mesh) in &meshes.tubes {
        add(world, Key::Tissue(*l), mesh, Transform::default());
    }

    // The mole's cells, all made now and shown as the clock reaches them.
    let mut nevus = Vec::with_capacity(mole.cells.len());
    for _ in &mole.cells {
        let body = add(world, Key::Nevus, &ball, Transform::from_scale(Vec3::ZERO));
        let core = add(
            world,
            Key::NevusNucleus,
            &ball,
            Transform::from_scale(Vec3::ZERO),
        );
        nevus.push((body, core));
    }

    // Inside the nucleus.
    add(world, Key::Envelope, &ball, inside.envelope);
    for pore in &inside.pores {
        add(world, Key::Pore, &meshes.ring, *pore);
    }
    add(
        world,
        Key::Chromatin,
        &meshes.chromatin,
        Transform::default(),
    );
    add(world, Key::Nucleolus, &ball, inside.nucleolus);
    for core in &inside.cores {
        add(world, Key::Histone, &meshes.rod, *core);
    }
    add(world, Key::Linker, &meshes.string, Transform::default());
    add(
        world,
        Key::Backbone,
        &meshes.backbones,
        Transform::default(),
    );
    for p in &inside.phosphates {
        add(world, Key::Phosphate, &bead, *p);
    }
    let mut mutant = 0;
    for h in &inside.halves {
        let key = if h.mutates {
            mutant += 1;
            Key::Mutant(mutant - 1)
        } else {
            Key::Base(h.base)
        };
        add(world, key, &meshes.rod, h.transform);
    }
    add(
        world,
        Key::Flash,
        &ball,
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
            layer.clone(),
        ))
        .id();

    // What moves by itself: red cells round every loop, about one every
    // 13 µm, and pigment up the arms of the melanocytes near the middle.
    let mut runners = Vec::new();
    let mut dice = skin::Dice::new(SEED + 1);
    for (n, path) in skin.loops.iter().enumerate() {
        let count = (path.length() / 13.0).round() as usize;
        let rate = 20.0 / path.length().max(1.0);
        for _ in 0..count {
            let e = add(
                world,
                Key::Tissue(Layer::RedCell),
                &meshes.solids[&Solid::RedCell],
                Transform::from_scale(Vec3::splat(3.6)),
            );
            runners.push(Runner {
                entity: e,
                track: Track::Blood(n),
                phase: dice.unit(),
                rate: rate * dice.range(0.92, 1.08),
                size: dice.range(3.4, 3.8),
                spin: dice.unit() * 6.3,
            });
        }
    }
    for (m, cell) in skin.melanocytes.iter().enumerate() {
        if cell.at.length() > 90.0 {
            continue;
        }
        for (d, arm) in cell.dendrites.iter().enumerate() {
            let count = (arm.length() / 4.0).round().max(2.0) as usize;
            for k in 0..count {
                let e = add(
                    world,
                    Key::Tissue(Layer::Melanin),
                    &bead,
                    Transform::from_scale(Vec3::splat(0.5)),
                );
                runners.push(Runner {
                    entity: e,
                    track: Track::Pigment(m, d),
                    phase: (k as f32 + dice.unit()) / count as f32,
                    rate: 2.2 / arm.length().max(1.0),
                    size: dice.range(0.4, 0.6),
                    spin: 0.0,
                });
            }
        }
    }

    worlds.map.insert(
        panel.to_owned(),
        WorldScene {
            camera,
            groups,
            runners,
            nevus,
            crowded,
            surface: (surface_entity, surface_material),
            skin,
            mole,
            inside,
            places,
            eye,
            film: Film::default(),
            clock: 0.0,
            playing: false,
            film_from: eye,
            manual: false,
            rejoin: 0.0,
            life: 0.0,
            years: -1.0,
            mutated: None,
            mole_placed: None,
            say: None,
            highlight: Vec::new(),
        },
    );
    world.insert_resource(worlds);
    Ok(())
}

/// Take a panel's world away, if it has one.
pub fn remove(world: &mut World, panel: &str) {
    let Some(scene) = world
        .get_resource_mut::<Worlds>()
        .and_then(|mut w| w.map.remove(panel))
    else {
        return;
    };
    let mut gone: Vec<Entity> = scene
        .groups
        .into_values()
        .flat_map(|g| g.entities)
        .collect();
    gone.push(scene.camera);
    gone.push(scene.surface.0);
    for e in gone {
        if let Ok(entity) = world.get_entity_mut(e) {
            entity.despawn();
        }
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn shown_or_hidden(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// Each frame: every world's camera where its panel is and where the film
/// or the user puts it, each part as visible as the scale wants it, the
/// mole as the clock has it, and what moves by itself, moved.
#[allow(clippy::too_many_arguments)]
pub fn run(
    time: Res<Time>,
    stage: Res<Stage>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut worlds: ResMut<Worlds>,
    mut cameras: Query<(&mut Camera, &mut Transform, &mut Projection)>,
    mut things: Query<(&mut Transform, &mut Visibility), Without<Camera>>,
    mut glows: ResMut<Assets<Glow>>,
    mut surfaces: ResMut<Assets<Surface>>,
) {
    let scale = windows.single().map_or(1.0, |w| w.scale_factor());
    let size = windows.single().map_or(UVec2::ONE, |w| {
        UVec2::new(w.physical_width(), w.physical_height())
    });
    let dt = time.delta_secs().min(0.1);
    for (id, scene) in worlds.map.iter_mut() {
        let Some(panel) = stage.panels.iter().find(|p| &p.id == id) else {
            continue;
        };
        scene.life += dt;
        if scene.playing && !scene.manual {
            scene.clock += dt;
            if scene.clock >= scene.film.length() {
                scene.clock = scene.film.length();
                scene.playing = false;
            }
        }
        let before = scene.years;
        let mut highlight = scene.highlight.clone();
        if !scene.film.shots.is_empty() {
            let frame = scene.film.frame(scene.clock, &scene.film_from);
            if !scene.manual {
                if scene.rejoin > 0.0 {
                    scene.rejoin = (scene.rejoin - dt).max(0.0);
                    scene.eye = film::between(&scene.eye, &frame.eye, 1.0 - scene.rejoin);
                } else {
                    scene.eye = frame.eye;
                }
            }
            scene.years = frame.years;
            scene.say = frame.say;
            if !scene.manual {
                highlight = frame.highlight;
            }
        }
        let years = scene.years;
        if before < 0.0 && years >= 0.0 {
            scene.mutated = Some(scene.life);
        }
        if years < 0.0 {
            scene.mutated = None;
        }
        let flash = scene
            .mutated
            .map_or(0.0, |t| (-(scene.life - t) * 2.0).exp());
        let eye = scene.eye;
        let d = eye.distance;
        if let Ok((mut camera, mut transform, mut projection)) = cameras.get_mut(scene.camera) {
            let r = panel.content();
            let min = (Vec2::new(r.min.x, r.min.y) * scale)
                .max(Vec2::ZERO)
                .as_uvec2();
            let extent = (Vec2::new(r.width(), r.height()) * scale)
                .max(Vec2::splat(8.0))
                .as_uvec2();
            let extent = extent.min(size.saturating_sub(min)).max(UVec2::splat(1));
            camera.viewport = Some(Viewport {
                physical_position: min,
                physical_size: extent,
                ..default()
            });
            camera.is_active = panel.rect.width() > 10.0;
            *transform = eye.transform();
            if let Projection::Perspective(p) = projection.as_mut() {
                p.near = d * glow::NEAR;
            }
        }

        // The first melanocyte rounds up and loses its arms once it has
        // mutated; the clone takes over from it. A material is written to
        // only when what it shows changes: the scale's fades are the
        // shader's, from the camera's distance, and every write makes the
        // renderer prepare again every part that wears it.
        let rounding = ease((years - 0.05) / 0.6);
        let now = time.elapsed_secs_wrapped();
        for (key, group) in scene.groups.iter_mut() {
            let mut strength = group.strength;
            match key {
                Key::Tissue(Layer::First) => strength *= (1.0 - rounding) * (1.0 + 2.0 * flash),
                Key::Tissue(Layer::FirstNucleus) => strength *= 1.0 - rounding,
                Key::Flash => strength = 3.0 * flash,
                _ => {}
            }
            let hide = group.band.at(d) < 0.003 || strength <= 0.001;
            if !group.each && hide != group.hidden {
                for e in &group.entities {
                    if let Ok((_, mut v)) = things.get_mut(*e) {
                        *v = shown_or_hidden(!hide);
                    }
                }
            }
            group.hidden = hide;
            if hide {
                continue;
            }
            let stands_out = emphasis(&highlight, *key);
            // The clock's colours, rounded so that the material is written
            // only when the colour changes enough to be seen.
            let shade = match key {
                Key::Nevus => {
                    let (c, k) = nevus::colour(years);
                    (c[0] * 7.0 + c[1] * 3.0 + c[2] + k * 5.0) * 0.1
                }
                Key::NevusNucleus => {
                    ease((years - nevus::BRAKE.0) / (nevus::BRAKE.1 - nevus::BRAKE.0))
                }
                Key::Mutant(_) => f32::from(years >= 0.0) + flash,
                _ => 0.0,
            };
            let shade = (shade * 60.0).round();
            let stale = (strength - group.written.0).abs() > 1e-3
                || stands_out != group.written.1
                || shade != group.written.2;
            if !stale {
                continue;
            }
            let Some(mut m) = glows.get_mut(&group.material) else {
                continue;
            };
            m.shape.y = strength;
            if stands_out != group.written.1 {
                // From where the last change has got to, to the new target,
                // over a second, by the renderer's clock.
                let e = m.emphasis;
                let t = ((now - e.z) / e.w.max(0.001)).clamp(0.0, 1.0);
                let from = e.x + (e.y - e.x) * t * t * (3.0 - 2.0 * t);
                m.emphasis = Vec4::new(from, stands_out, now, 1.0);
            }
            group.written = (strength, stands_out, shade);
            match key {
                Key::Nevus => {
                    let (c, k) = nevus::colour(years);
                    m.rim = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.45 * k);
                    m.body = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.025 * k);
                }
                Key::NevusNucleus => {
                    // The brake: the nuclei change colour as the clone stops.
                    let brake = ease((years - nevus::BRAKE.0) / (nevus::BRAKE.1 - nevus::BRAKE.0));
                    let c = Vec3::new(0.4, 0.55, 1.0).lerp(Vec3::new(1.0, 0.3, 0.55), brake);
                    m.rim = LinearRgba::rgb(c.x, c.y, c.z).with_alpha(0.22 + 0.06 * brake);
                }
                Key::Mutant(k) => {
                    // T and A trade places on the two strands.
                    let half = scene.inside.halves.iter().filter(|h| h.mutates).nth(*k);
                    if let Some(h) = half {
                        let base = if years >= 0.0 { h.base.pair() } else { h.base };
                        let c = base.colour();
                        // Brighter than its neighbours once it has changed,
                        // and very bright as it changes.
                        let boost = if years >= 0.0 { 1.6 } else { 1.0 } + 4.0 * flash;
                        m.rim = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.7 * boost);
                        m.body = LinearRgba::rgb(c[0], c[1], c[2]).with_alpha(0.35 * boost);
                    }
                }
                Key::Flash => {
                    m.body = LinearRgba::rgb(1.0, 0.95, 0.85).with_alpha(1.0);
                }
                _ => {}
            }
        }

        // The mole's cells where the clock puts them, when it has moved.
        let nevus_shown = !scene.groups[&Key::Nevus].hidden;
        let placed = (years, nevus_shown);
        let mole_moved = scene.mole_placed != Some(placed);
        scene.mole_placed = Some(placed);
        for (k, (body, core)) in scene.nevus.iter().enumerate() {
            if !mole_moved {
                break;
            }
            let seen = if nevus_shown {
                scene.mole.seen(k, years, &scene.skin)
            } else {
                None
            };
            for (e, part) in [(*body, 1.0), (*core, 0.5)] {
                if let Ok((mut t, mut v)) = things.get_mut(e) {
                    let wanted = shown_or_hidden(seen.is_some());
                    if *v != wanted {
                        *v = wanted;
                    }
                    if let Some(s) = seen {
                        t.translation = s.at;
                        t.scale = Vec3::splat(s.size * part);
                    }
                }
            }
        }
        // The basal cells where the nests sit, crowded out while the nests
        // are at the junction, back once the nests have sunk below it.
        let crowding = scene.mole.crowding(years) * (1.0 - ease((years - 18.0) / 12.0));
        let basal_shown = !scene.groups[&Key::Tissue(Layer::Basal)].hidden;
        let wanted = shown_or_hidden(basal_shown && crowding < 0.5);
        for e in &scene.crowded {
            if let Ok((_, mut v)) = things.get_mut(*e)
                && *v != wanted
            {
                *v = wanted;
            }
        }

        // The surface, and the mole on it: growing over its first years,
        // darkest in its first decade, raised and paler after decades.
        let outside = OUTSIDE.at(d);
        if let Ok((_, mut v)) = things.get_mut(scene.surface.0) {
            *v = shown_or_hidden(outside > 0.003);
        }
        if outside > 0.003
            && let Some(mut m) = surfaces.get_mut(&scene.surface.1)
        {
            let sink = ease((years - 15.0) / 35.0);
            m.shape.y = outside;
            let radius = if years <= 0.3 {
                0.0
            } else {
                1_800.0 * ease(years / 8.0).max(0.04)
            };
            m.mole = Vec4::new(0.0, 0.0, radius, ease(years / 3.0) * (1.0 - 0.45 * sink));
            m.shape.x = 320.0 * sink;
            m.shape.z = sink;
        }

        let skin = scene.skin.clone();
        for runner in &scene.runners {
            let f = (runner.phase + scene.life * runner.rate).fract();
            let Ok((mut t, _)) = things.get_mut(runner.entity) else {
                continue;
            };
            match runner.track {
                Track::Blood(n) => {
                    let (p, heading) = skin.loops[n].at(f);
                    t.translation = p;
                    t.rotation = Quat::from_rotation_arc(Vec3::Y, heading)
                        * Quat::from_rotation_x(0.5)
                        * Quat::from_rotation_y(runner.spin);
                    t.scale = Vec3::splat(runner.size);
                }
                Track::Pigment(m, d) => {
                    let (p, _) = skin.melanocytes[m].dendrites[d].at(f);
                    t.translation = p;
                    // Handed over at the tip: it shrinks into the
                    // keratinocyte that takes it.
                    let mut fade = ((1.0 - f) / 0.12).clamp(0.0, 1.0) * (f / 0.05).clamp(0.0, 1.0);
                    if m == 0 {
                        fade *= 1.0 - rounding;
                    }
                    t.scale = Vec3::new(1.0, 0.7, 0.8) * runner.size * fade.max(0.001);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_offered_are_the_places_made() {
        let b = built();
        let made: Vec<&str> = skin_places(&b.skin, &b.mole)
            .iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(made, PLACES.to_vec());
    }

    #[test]
    fn a_band_fades_in_and_out_across_scales() {
        let band = Band::new((1.0, 10.0), (100.0, 1000.0));
        assert_eq!(band.at(0.5), 0.0);
        assert!((band.at(3.16) - 0.5).abs() < 0.01);
        assert_eq!(band.at(50.0), 1.0);
        assert_eq!(band.at(2000.0), 0.0);
        assert_eq!(Band::new(NONE, NEVER).at(1e-6), 1.0);
    }

    #[test]
    fn setting_apart_dims_the_rest_but_not_what_is_alone_at_its_scale() {
        let mole = vec!["mole".to_owned()];
        assert_eq!(emphasis(&mole, Key::Nevus), 1.0);
        assert!(emphasis(&mole, Key::Tissue(Layer::Spiny)) < 0.5);
        assert_eq!(emphasis(&mole, Key::Chromatin), 1.0);
        let codon = vec!["braf_codon_600".to_owned()];
        assert_eq!(emphasis(&codon, Key::Mutant(0)), 1.0);
        assert!(emphasis(&codon, Key::Base(Base::G)) < 0.5);
        assert_eq!(emphasis(&[], Key::Tissue(Layer::Horny)), 1.0);
    }
}
