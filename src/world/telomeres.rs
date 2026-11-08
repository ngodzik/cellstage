//! Chromosomes and their ends: the fourth of Hanahan and Weinberg's
//! hallmarks, the endless dividing a cancer cell wins.
//!
//! Each chromosome ends in a telomere, thousands of repeats of TTAGGG
//! wrapped by the shelterin proteins and tucked into a loop, the T-loop,
//! that hides the end from the cell's damage sensors. DNA polymerase
//! cannot copy the very end of a strand, so every division leaves the
//! telomeres a little shorter, some fifty to a hundred bases. After fifty
//! or so divisions (Hayflick's limit) some are too short to loop: the bare
//! ends set off the damage response and the cell stops for good. If p53
//! and RB are gone, the cell goes on dividing into crisis: bare ends are
//! joined to one another, chromosomes fuse end to end and tear at the next
//! division. Most cancer cells come out of it by switching telomerase on,
//! TERT with its RNA template, which writes the repeats back on: they
//! divide without end.
//!
//! The chromosomes are drawn as in a metaphase spread, a few micrometres
//! long; the telomeres are tiny at that scale, so their caps are drawn
//! large there and true to size up close, where the repeats, 6 bases or
//! 2 nm each, are shown a few dozen at a time, as many as the telomere's
//! length allows. The clock counts divisions.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::engine::{Band, Groups, NEVER, NONE, Solids, Things, Tick, ease, set_shown};
use super::film::{Eye, Place};
use super::glow::Glow;
use super::shapes::{Builder, Path, smooth};
use super::skin::Dice;

/// The stories the chromosomes can tell.
pub const VARIANTS: [&str; 3] = ["normal", "telomerase", "crisis"];

/// How many repeats are drawn up close for a full telomere, and how many
/// a division takes away.
const FULL: usize = 64;
const PER_DIVISION: f32 = 0.6;
/// Below this many, the end can no longer loop.
const CRITICAL: usize = 30;
/// When chromosomes begin to fuse in crisis, in divisions.
const FUSING: (f32, f32) = (58.0, 64.0);
/// The pairs of chromosomes that fuse, by index.
const FUSED: [(usize, usize); 4] = [(2, 9), (5, 14), (11, 20), (17, 6)];
const PAIRS: usize = 23;

const SPREAD: Band = Band::new((0.25, 0.8), NEVER);
const CAPS: Band = Band::new((0.25, 0.8), NEVER);
const REPEATS: Band = Band::new(NONE, (0.4, 1.1));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Chromatid,
    Centromere,
    Cap,
    BareCap,
    RepeatT,
    RepeatG,
    Overhang,
    Shelterin,
    Telomerase,
    TemplateRna,
    Damage,
}

fn look(key: Key) -> (Glow, Band) {
    let g = Glow::new;
    match key {
        Key::Chromatid => (
            g([0.35, 0.55, 1.0], 0.55, 0.0)
                .faces([0.3, 0.45, 1.0], 0.05)
                .edges(2.0)
                .varied(1.5),
            SPREAD,
        ),
        Key::Centromere => (
            g([0.75, 0.5, 1.0], 0.8, 0.0)
                .faces([0.7, 0.45, 1.0], 0.2)
                .edges(1.6),
            SPREAD,
        ),
        Key::Cap => (
            g([0.4, 1.0, 0.6], 0.9, 0.0)
                .faces([0.35, 1.0, 0.55], 0.45)
                .edges(1.5),
            CAPS,
        ),
        Key::BareCap => (
            g([1.0, 0.25, 0.25], 1.0, 0.0)
                .faces([1.0, 0.2, 0.2], 0.55)
                .edges(1.4),
            CAPS,
        ),
        Key::RepeatT => (
            g([0.35, 1.0, 0.6], 0.8, 0.0)
                .faces([0.3, 1.0, 0.55], 0.3)
                .edges(1.6),
            REPEATS,
        ),
        Key::RepeatG => (
            g([0.25, 0.75, 1.0], 0.8, 0.0)
                .faces([0.2, 0.7, 1.0], 0.3)
                .edges(1.6),
            REPEATS,
        ),
        Key::Overhang => (
            g([0.6, 1.0, 0.8], 0.8, 0.0)
                .faces([0.5, 1.0, 0.7], 0.2)
                .edges(1.6),
            REPEATS,
        ),
        Key::Shelterin => (
            g([1.0, 0.75, 0.3], 0.8, 0.0)
                .faces([1.0, 0.65, 0.25], 0.25)
                .edges(1.6),
            REPEATS,
        ),
        Key::Telomerase => (
            g([1.0, 0.45, 0.85], 0.9, 0.0)
                .faces([1.0, 0.4, 0.8], 0.3)
                .edges(1.5),
            Band::new(NONE, (3.0, 8.0)),
        ),
        Key::TemplateRna => (
            g([1.0, 0.9, 0.5], 0.8, 0.0)
                .faces([1.0, 0.85, 0.45], 0.3)
                .edges(1.5),
            REPEATS,
        ),
        Key::Damage => (
            g([1.0, 0.3, 0.2], 1.0, 0.0)
                .faces([1.0, 0.25, 0.15], 0.7)
                .edges(1.3),
            Band::new(NONE, NEVER),
        ),
    }
}

fn picks(name: &str, key: Key) -> bool {
    match name {
        "chromosomes" => matches!(key, Key::Chromatid | Key::Centromere),
        "telomeres" => matches!(
            key,
            Key::Cap | Key::BareCap | Key::RepeatT | Key::RepeatG | Key::Overhang | Key::Shelterin
        ),
        "shelterin" => key == Key::Shelterin,
        "telomerase" => matches!(key, Key::Telomerase | Key::TemplateRna),
        "damage" => matches!(key, Key::BareCap | Key::Damage),
        _ => false,
    }
}

const HIGHLIGHTS: [&str; 5] = [
    "chromosomes",
    "telomeres",
    "shelterin",
    "telomerase",
    "damage",
];

pub fn highlights() -> Vec<&'static str> {
    HIGHLIGHTS.to_vec()
}

pub const PLACES: [&str; 6] = [
    "spread",
    "chromosome",
    "end",
    "telomere",
    "t_loop",
    "fusion",
];

const THINGS: [&str; 7] = [
    "chromosome_1",
    "centromere",
    "telomere",
    "t_loop",
    "shelterin",
    "telomerase",
    "fusion",
];

pub fn things() -> Vec<&'static str> {
    THINGS.to_vec()
}

pub fn clock_label(t: f32) -> Option<String> {
    (t >= 0.0).then(|| format!("division {t:.0}"))
}

/// One chromosome as laid in the spread: where its middle is, how it
/// lies, how long it is, where its centromere falls.
#[derive(Debug, Clone, Copy)]
struct Chromosome {
    at: Vec3,
    turn: Quat,
    length: f32,
    centromere: f32,
}

/// The spread, built once.
struct Spread {
    chromosomes: Vec<Chromosome>,
    /// The repeats up close, from the end inwards, each as a stretch of
    /// path, and the T-loop at the end.
    repeats: Vec<Path>,
    loop_path: Path,
}

/// Where the telomere looked at up close is: the top end of chromosome
/// 1's first chromatid, which is set at the origin.
const END: Vec3 = Vec3::ZERO;

impl Spread {
    fn build(seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let mut chromosomes = Vec::new();
        for n in 0..PAIRS * 2 {
            let pair = n / 2;
            // Chromosome 1 the longest, about 9 µm in metaphase, down to
            // about 2 µm for the smallest.
            let length = 9.0 - 6.8 * pair as f32 / (PAIRS - 1) as f32 + dice.range(-0.3, 0.3);
            let ring = 6.0 + 9.0 * dice.unit().sqrt();
            let a = n as f32 * 2.399_963;
            let at = Vec3::new(a.cos() * ring, dice.range(-1.5, 1.5), a.sin() * ring);
            let turn = Quat::from_rotation_y(dice.range(0.0, 6.3))
                * Quat::from_rotation_z(dice.range(-1.3, 1.3));
            chromosomes.push(Chromosome {
                at,
                turn,
                length,
                centromere: dice.range(0.3, 0.5),
            });
        }
        // Chromosome 1 set so that its first chromatid's top end is at the
        // origin.
        let first = chromosomes[0];
        let top = first.turn * Vec3::new(-0.4, first.length * 0.5, 0.0);
        chromosomes[0].at = END - top;
        // The repeats: a straight run of DNA ending at the origin, going
        // into the chromosome.
        let inwards = (first.turn * Vec3::NEG_Y).normalize();
        let side = first.turn * Vec3::Z;
        let step = 0.002;
        let mut repeats = Vec::new();
        for k in 0..FULL {
            let a = END + inwards * (0.012 + k as f32 * step);
            let b = a + inwards * step * 0.9;
            repeats.push(Path::new(vec![a, b]));
        }
        // The T-loop: the end folding back and tucking into the repeats.
        let loop_points: Vec<Vec3> = (0..=24)
            .map(|i| {
                let f = i as f32 / 24.0;
                let a = f * std::f32::consts::PI * 1.6;
                END + inwards * (0.012 - 0.014 * a.sin()) + side * (0.014 * (1.0 - a.cos()))
            })
            .collect();
        Self {
            chromosomes,
            repeats,
            loop_path: Path::new(smooth(&loop_points, 2)),
        }
    }
}

fn spread() -> &'static Spread {
    static SPREAD_BUILT: std::sync::OnceLock<Spread> = std::sync::OnceLock::new();
    SPREAD_BUILT.get_or_init(|| Spread::build(41))
}

fn places() -> Vec<Place> {
    let s = spread();
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
    let c1 = s.chromosomes[0].at;
    let (a, b) = FUSED[0];
    let fusion = (s.chromosomes[a].at + s.chromosomes[b].at) * 0.5;
    vec![
        place(
            "spread",
            "a cell's 46 chromosomes in metaphase, telomeres glowing at their ends",
            Vec3::ZERO,
            48.0,
            0.4,
            1.0,
        ),
        place(
            "chromosome",
            "chromosome 1, its two sister chromatids joined at the centromere, a telomere at each of its four ends",
            c1,
            14.0,
            0.6,
            0.3,
        ),
        place(
            "end",
            "the end of a chromatid and its telomere cap",
            END,
            2.2,
            0.7,
            0.25,
        ),
        place(
            "telomere",
            "a telomere up close: TTAGGG repeats wrapped by shelterin, ending in the T-loop",
            END,
            0.09,
            0.9,
            0.3,
        ),
        place(
            "t_loop",
            "the T-loop: the strand's end tucked back into the repeats, hidden from the damage sensors",
            END,
            0.045,
            1.2,
            0.5,
        ),
        place(
            "fusion",
            "chromosomes fused end to end where their bare telomeres were joined, in crisis",
            fusion,
            22.0,
            0.5,
            0.6,
        ),
    ]
}

/// The chromosomes, as shown in a panel.
pub struct TelomereWorld {
    groups: Groups<Key>,
    /// Each chromosome's root, which its parts hang from.
    roots: Vec<Entity>,
    /// Each chromosome's four caps, healthy and bare.
    caps: Vec<[(Entity, Entity); 4]>,
    repeats: Vec<(Entity, bool)>,
    overhang: Entity,
    shelterin: Vec<(Entity, usize)>,
    telomerase: (Entity, Entity),
    damage: Entity,
    places: Vec<Place>,
    variant: usize,
}

impl TelomereWorld {
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
                    "no story {variant} in the chromosomes; there are {}",
                    VARIANTS.join(", ")
                )
            })?
        };
        let s = spread();
        let mut groups = Groups::new(layer.clone());
        {
            let mut materials = world.resource_mut::<Assets<Glow>>();
            for key in [
                Key::Chromatid,
                Key::Centromere,
                Key::Cap,
                Key::BareCap,
                Key::RepeatT,
                Key::RepeatG,
                Key::Overhang,
                Key::Shelterin,
                Key::Telomerase,
                Key::TemplateRna,
                Key::Damage,
            ] {
                let (glow, band) = look(key);
                let spot = if matches!(key, Key::RepeatT | Key::RepeatG | Key::Shelterin) {
                    Vec4::new(END.x, END.y, END.z, 0.16)
                } else {
                    Vec4::ZERO
                };
                groups.add(&mut materials, key, glow, band, spot, true);
            }
        }
        let hidden = Transform::from_scale(Vec3::ZERO);
        // One chromatid's mesh per length, made in the chromosome's own
        // frame: up along y, the two sisters side by side along x.
        let mut roots = Vec::new();
        let mut caps = Vec::new();
        for c in &s.chromosomes {
            let root = world
                .spawn((
                    Transform::from_translation(c.at).with_rotation(c.turn),
                    Visibility::Inherited,
                ))
                .id();
            roots.push(root);
            let mut sisters = Builder::default();
            for side in [-0.4f32, 0.4] {
                let half = c.length * 0.5;
                let pinch = -half + c.length * c.centromere;
                let points: Vec<Vec3> = (0..=24)
                    .map(|i| {
                        let y = -half + c.length * i as f32 / 24.0;
                        let near = (-((y - pinch) / 0.5).powi(2)).exp();
                        Vec3::new(side * (1.0 - 0.7 * near), y, 0.0)
                    })
                    .collect();
                sisters.tube(
                    &points,
                    |f| {
                        let y = -half + c.length * f;
                        let near = (-((y - pinch) / 0.45).powi(2)).exp();
                        0.38 * (1.0 - 0.45 * near) * (1.0 - (2.0 * f - 1.0).powi(10)).max(0.25)
                    },
                    10,
                );
            }
            let mesh = world.resource_mut::<Assets<Mesh>>().add(sisters.mesh());
            let body = groups.spawn(world, Key::Chromatid, &mesh, Transform::default());
            let pinch = -c.length * 0.5 + c.length * c.centromere;
            let centromere = groups.spawn(
                world,
                Key::Centromere,
                &solids.ball,
                Transform::from_xyz(0.0, pinch, 0.0).with_scale(Vec3::new(0.5, 0.3, 0.35)),
            );
            let mut four = [(Entity::PLACEHOLDER, Entity::PLACEHOLDER); 4];
            for slot in four.iter_mut() {
                let healthy = groups.spawn(world, Key::Cap, &solids.ball, hidden);
                let bare = groups.spawn(world, Key::BareCap, &solids.ball, hidden);
                for e in [healthy, bare] {
                    world.entity_mut(e).insert(ChildOf(root));
                }
                *slot = (healthy, bare);
            }
            for e in [body, centromere] {
                world.entity_mut(e).insert(ChildOf(root));
            }
            caps.push(four);
        }
        // The telomere up close, from the end inwards.
        let mut repeats = Vec::new();
        for (k, r) in s.repeats.iter().enumerate() {
            let mut b = Builder::default();
            b.tube(&r.points, |_| 0.001, 8);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(b.mesh());
            let ttag = k % 2 == 0;
            let key = if ttag { Key::RepeatT } else { Key::RepeatG };
            repeats.push((groups.spawn(world, key, &mesh, Transform::default()), ttag));
        }
        let overhang = {
            let mut b = Builder::default();
            b.tube(&s.loop_path.points, |_| 0.0007, 8);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(b.mesh());
            groups.spawn(world, Key::Overhang, &mesh, Transform::default())
        };
        let shelterin = (0..FULL / 3)
            .map(|k| {
                (
                    groups.spawn(world, Key::Shelterin, &solids.ball, hidden),
                    k * 3 + 1,
                )
            })
            .collect();
        let telomerase = (
            groups.spawn(world, Key::Telomerase, &solids.ball, hidden),
            groups.spawn(world, Key::TemplateRna, &solids.ring, hidden),
        );
        let damage = groups.spawn(world, Key::Damage, &solids.ball, hidden);
        Ok(Self {
            groups,
            roots,
            caps,
            repeats,
            overhang,
            shelterin,
            telomerase,
            damage,
            places: places(),
            variant,
        })
    }

    pub fn places(&self) -> &[Place] {
        &self.places
    }

    fn telomerase_on(&self) -> bool {
        VARIANTS[self.variant] == "telomerase"
    }

    fn crisis(&self) -> bool {
        VARIANTS[self.variant] == "crisis"
    }

    /// How many repeats are left after `divisions`, of the full count.
    fn left(&self, divisions: f32) -> usize {
        Self::left_for(self.variant, divisions)
    }

    fn left_for(variant: usize, divisions: f32) -> usize {
        if VARIANTS[variant] == "telomerase" {
            // Shortened at first, then held where telomerase keeps it.
            let floor = FULL as f32 - 12.0;
            (FULL as f32 - divisions.max(0.0) * PER_DIVISION).max(floor) as usize
        } else {
            let lost = divisions.max(0.0) * PER_DIVISION;
            (FULL as f32 - lost).max(4.0) as usize
        }
    }

    /// How long the caps are drawn on the spread, 0 to 1.
    fn cap(&self, divisions: f32) -> f32 {
        self.left(divisions) as f32 / FULL as f32
    }

    pub fn locate(&self, name: &str, divisions: f32, _focus: Vec3) -> Option<Vec3> {
        let s = spread();
        let toward = |n: usize| s.chromosomes[n].at;
        Some(match name {
            "chromosome_1" => toward(0),
            "centromere" => {
                let c = s.chromosomes[0];
                c.at + c.turn * Vec3::new(0.0, -c.length * 0.5 + c.length * c.centromere, 0.0)
            }
            "telomere" | "t_loop" => END,
            "shelterin" => s.repeats.get(4).map(|r| r.points[0])?,
            "telomerase" => END,
            "fusion" => {
                let (a, b) = FUSED[0];
                let f = self.fusing(divisions);
                (toward(a) + toward(b)) * 0.5 * (1.0 - 0.3 * f)
            }
            _ => return None,
        })
    }

    fn fusing(&self, divisions: f32) -> f32 {
        if self.crisis() {
            ease((divisions - FUSING.0) / (FUSING.1 - FUSING.0))
        } else {
            0.0
        }
    }

    pub fn pick(&self, origin: Vec3, direction: Vec3, divisions: f32, d: f32) -> Option<String> {
        let s = spread();
        let left = self.left(divisions);
        let mut best: Option<(f32, String)> = None;
        let mut consider = |centre: Vec3, r: f32, what: String| {
            let o = centre - origin;
            let along = o.dot(direction);
            let miss = o.length_squared() - along * along;
            if along > 0.0 && miss < r * r {
                let hit = along - (r * r - miss).sqrt();
                if best.as_ref().is_none_or(|(b, _)| hit < *b) {
                    best = Some((hit, what));
                }
            }
        };
        if self.groups.seen(Key::Chromatid, d) {
            for (n, c) in s.chromosomes.iter().enumerate() {
                consider(
                    c.at,
                    c.length * 0.5,
                    format!(
                        "chromosome {}: two sister chromatids joined at the centromere, a telomere at each end",
                        n / 2 + 1
                    ),
                );
            }
        }
        if self.groups.seen(Key::RepeatT, d) {
            let state = if left < CRITICAL && !self.telomerase_on() {
                "too short to loop: its bare end sets off the damage response"
            } else if self.telomerase_on() {
                "telomerase writes the repeats back on as fast as division wears them away"
            } else {
                "a little shorter after every division"
            };
            consider(
                END,
                0.03,
                format!(
                    "telomere: {left} of {FULL} repeats left as drawn here (each drawn repeat stands for some 150 bases); {state}"
                ),
            );
        }
        best.map(|(_, w)| w)
    }

    pub fn update(&mut self, tick: &Tick, things: &mut Things, glows: &mut Assets<Glow>) {
        let s = spread();
        let divisions = tick.clock;
        self.groups.update(tick, picks, |_, st| st, things, glows);
        let left = self.left(divisions);
        let short = left < CRITICAL && !self.telomerase_on();
        let fusing = self.fusing(divisions);
        let spread_shown = !self.groups.hidden(Key::Chromatid);
        // The chromosomes: in crisis, pairs drawn end to end.
        for (n, (root, c)) in self.roots.iter().zip(&s.chromosomes).enumerate() {
            let mut at = c.at;
            let mut turn = c.turn;
            if let Some((a, b)) = FUSED.iter().find(|(a, b)| *a == n || *b == n) {
                let (ca, cb) = (s.chromosomes[*a], s.chromosomes[*b]);
                let mid = (ca.at + cb.at) * 0.5;
                let axis = (cb.at - ca.at).normalize_or(Vec3::X);
                let joined_turn =
                    Quat::from_rotation_arc(Vec3::Y, if n == *a { -axis } else { axis });
                let offset = if n == *a {
                    -ca.length * 0.5
                } else {
                    cb.length * 0.5
                };
                let joined = mid + axis * offset;
                at = c.at.lerp(joined, fusing);
                turn = c.turn.slerp(joined_turn, fusing);
            }
            if let Ok((mut t, _)) = things.get_mut(*root) {
                t.translation = at;
                t.rotation = turn;
            }
            // The four caps, smaller each division, red once bare; in a
            // fused pair, the joined ends lose theirs.
            let length = self.cap(divisions);
            for (k, (healthy, bare)) in self.caps[n].iter().enumerate() {
                let top = k < 2;
                let x = if k % 2 == 0 { -0.4 } else { 0.4 };
                let y = if top { c.length * 0.5 } else { -c.length * 0.5 };
                let joined_end = FUSED
                    .iter()
                    .any(|(a, b)| (n == *a && top) || (n == *b && !top));
                let gone = joined_end && fusing > 0.5;
                // Not every end wears down evenly: some reach the limit first.
                let uneven = ((n * 4 + k) % 7) as f32 / 7.0;
                let this_short =
                    short && uneven < 0.45 + 0.08 * (CRITICAL - left.min(CRITICAL)) as f32;
                let size = 0.22 + 0.32 * length;
                let place =
                    Transform::from_xyz(x, y + if top { size * 0.6 } else { -size * 0.6 }, 0.0)
                        .with_scale(Vec3::new(0.36, size, 0.36));
                set_shown(things, *healthy, spread_shown && !gone && !this_short);
                set_shown(things, *bare, spread_shown && !gone && this_short);
                for e in [*healthy, *bare] {
                    if let Ok((mut t, _)) = things.get_mut(e) {
                        *t = place;
                    }
                }
            }
        }
        // Up close: the end stays where the camera looks and the run of
        // repeats behind it shortens; the loop while the telomere is long
        // enough to make one; shelterin on what is left.
        let close = !self.groups.hidden(Key::RepeatT);
        for (k, (e, _)) in self.repeats.iter().enumerate() {
            set_shown(things, *e, close && k < left);
        }
        set_shown(things, self.overhang, close && !short);
        for (e, k) in &self.shelterin {
            let on = close && *k < left && !(short && *k < 6);
            set_shown(things, *e, on);
            if let Ok((mut t, _)) = things.get_mut(*e) {
                let p = s.repeats[*k].points[0];
                let side = (s.chromosomes[0].turn * Vec3::Z) * 0.0022;
                t.translation = p + side * if k % 2 == 0 { 1.0 } else { -1.0 };
                t.scale = Vec3::new(0.0024, 0.0018, 0.0022);
            }
        }
        // The bare end's alarm.
        let alarm = close && short;
        set_shown(things, self.damage, alarm);
        if let Ok((mut t, _)) = things.get_mut(self.damage) {
            t.translation = s.repeats[0].points[0];
            t.scale = Vec3::splat(0.004 * (1.0 + 0.3 * (tick.life * 4.0).sin()));
        }
        // Telomerase at the end, writing.
        let writing = self.telomerase_on() && divisions >= 0.0;
        let end = s.repeats[0].points[0];
        set_shown(things, self.telomerase.0, writing);
        set_shown(things, self.telomerase.1, writing && close);
        if let Ok((mut t, _)) = things.get_mut(self.telomerase.0) {
            let far = !close;
            t.translation = end;
            t.scale = if far {
                Vec3::splat(0.5)
            } else {
                Vec3::new(0.0075, 0.006, 0.007)
            };
        }
        if let Ok((mut t, _)) = things.get_mut(self.telomerase.1) {
            t.translation = end + Vec3::Y * 0.004;
            t.rotation = Quat::from_rotation_x(tick.life * 0.8);
            t.scale = Vec3::new(0.004, 0.0015, 0.004);
        }
    }

    pub fn entities(self) -> Vec<Entity> {
        let mut all = self.groups.entities();
        all.extend(self.roots);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telomeres_shorten_with_division_unless_telomerase_writes_them_back() {
        let normal = TelomereWorld::left_for(0, 0.0);
        assert_eq!(normal, FULL);
        assert!(TelomereWorld::left_for(0, 60.0) < CRITICAL);
        assert!(TelomereWorld::left_for(1, 200.0) > CRITICAL);
        let names: Vec<&str> = places().iter().map(|p| p.name).collect();
        assert_eq!(names, PLACES.to_vec());
        assert_eq!(spread().chromosomes.len(), 46);
    }
}
