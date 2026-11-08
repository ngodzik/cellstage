//! A section of skin, stained with haematoxylin and eosin as a pathologist
//! sees it, with a melanocytic lesion at one of its stages.
//!
//! An illustration, not a simulation: each stage is drawn from the
//! criteria histology uses to tell it apart. A junctional nevus sits in
//! nests at the tips of the rete ridges; a compound one has nests in the
//! dermis too, smaller and plainer the deeper they lie; a dysplastic one
//! bridges the ridges with elongated nests and carries single cells along
//! the basal layer beyond its dermal part. A melanoma in situ has large
//! atypical cells crowding the junction and climbing through the epidermis,
//! one by one; an invasive one has them in the dermis, as a broad flat
//! lesion with a small dermal part when it spreads superficially, as a
//! nodule when it grows down. Lentigo maligna lies in sun-damaged skin, its
//! epidermis thin and its dermis grey with elastosis; acral melanoma under
//! the thick horn of a palm or a sole.

use bevy::math::Vec2;

use super::canvas::{Canvas, Random, Stain, fbm, noise};

/// The section's size in pixels, and how much tissue a pixel is.
pub const WIDTH: usize = 1280;
pub const HEIGHT: usize = 800;
pub const MICROMETRES_PER_PIXEL: f32 = 0.6;

/// The stages the section can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lesion {
    Normal,
    JunctionalNevus,
    CompoundNevus,
    IntradermalNevus,
    DysplasticNevus,
    MelanomaInSitu,
    SuperficialSpreading,
    Nodular,
    LentigoMaligna,
    Acral,
}

impl Lesion {
    /// Every stage, by the name a tool call gives it.
    pub const ALL: [(&'static str, Lesion); 10] = [
        ("normal", Lesion::Normal),
        ("junctional_nevus", Lesion::JunctionalNevus),
        ("compound_nevus", Lesion::CompoundNevus),
        ("intradermal_nevus", Lesion::IntradermalNevus),
        ("dysplastic_nevus", Lesion::DysplasticNevus),
        ("melanoma_in_situ", Lesion::MelanomaInSitu),
        (
            "superficial_spreading_melanoma",
            Lesion::SuperficialSpreading,
        ),
        ("nodular_melanoma", Lesion::Nodular),
        ("lentigo_maligna_melanoma", Lesion::LentigoMaligna),
        ("acral_lentiginous_melanoma", Lesion::Acral),
    ];

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
    }

    /// What it is, in a sentence, for the caption.
    pub fn what(self) -> &'static str {
        match self {
            Lesion::Normal => {
                "Normal skin: epidermis over dermis, melanocytes one every ten or so basal cells."
            }
            Lesion::JunctionalNevus => {
                "Junctional nevus: nests of nevus cells at the tips of the rete ridges, at the junction."
            }
            Lesion::CompoundNevus => {
                "Compound nevus: nests at the junction and in the dermis, smaller and plainer with depth."
            }
            Lesion::IntradermalNevus => {
                "Intradermal nevus: nests and cords only in the dermis, maturing with depth."
            }
            Lesion::DysplasticNevus => {
                "Dysplastic nevus: nests bridging the rete ridges, single cells along the basal layer beyond the dermal part, mild atypia."
            }
            Lesion::MelanomaInSitu => {
                "Melanoma in situ: atypical melanocytes crowding the junction and climbing through the epidermis; none in the dermis."
            }
            Lesion::SuperficialSpreading => {
                "Superficial spreading melanoma: a broad intraepidermal spread, with a small invasive part in the papillary dermis."
            }
            Lesion::Nodular => {
                "Nodular melanoma: an expansile nodule of atypical cells growing down into the dermis, little spread to the sides."
            }
            Lesion::LentigoMaligna => {
                "Lentigo maligna melanoma: atypical single cells along the basal layer of thin, sun-damaged skin, down the hair follicles."
            }
            Lesion::Acral => {
                "Acral lentiginous melanoma: atypical single cells along the basal layer under the thick horn of a palm or sole."
            }
        }
    }

    fn atypical(self) -> bool {
        matches!(
            self,
            Lesion::MelanomaInSitu
                | Lesion::SuperficialSpreading
                | Lesion::Nodular
                | Lesion::LentigoMaligna
                | Lesion::Acral
        )
    }
}

/// What a point of the section is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Slide,
    Corneum,
    Granular,
    Spinous,
    Basal,
    Melanocyte,
    NevusCell,
    AtypicalMelanocyte,
    Papillary,
    Reticular,
    Elastosis,
    Vessel,
    Lymphocyte,
    Melanophage,
    Follicle,
}

impl Region {
    pub fn name(self) -> &'static str {
        match self {
            Region::Slide => "the glass of the slide",
            Region::Corneum => "the stratum corneum",
            Region::Granular => "the granular layer",
            Region::Spinous => "the spinous layer",
            Region::Basal => "the basal layer",
            Region::Melanocyte => "a melanocyte",
            Region::NevusCell => "a nevus cell",
            Region::AtypicalMelanocyte => "an atypical melanocyte",
            Region::Papillary => "the papillary dermis",
            Region::Reticular => "the reticular dermis",
            Region::Elastosis => "solar elastosis",
            Region::Vessel => "a blood vessel",
            Region::Lymphocyte => "a lymphocyte",
            Region::Melanophage => "a melanophage",
            Region::Follicle => "a hair follicle",
        }
    }

    pub fn what(self) -> &'static str {
        match self {
            Region::Slide => "Nothing: the glass the section lies on.",
            Region::Corneum => {
                "Dead, flattened keratinocytes, the skin's horny surface, in the basket-weave a section shows."
            }
            Region::Granular => "Keratinocytes about to die, full of dark keratohyalin granules.",
            Region::Spinous => "Living keratinocytes, joined by desmosomes, most of the epidermis.",
            Region::Basal => {
                "The dividing keratinocytes on the basement membrane, from which the epidermis renews."
            }
            Region::Melanocyte => {
                "The pigment cell of the basal layer, with a clear halo, about one in ten basal cells: the cell melanoma comes from."
            }
            Region::NevusCell => {
                "A melanocyte of a nevus: round, bland, in nests; arrested, as most nevi stop growing."
            }
            Region::AtypicalMelanocyte => {
                "A malignant melanocyte: large, its nucleus big, dark and irregular, with a prominent nucleolus."
            }
            Region::Papillary => {
                "The thin, loose dermis under the epidermis, rising into the papillae between the rete ridges."
            }
            Region::Reticular => "The thick dermis of coarse collagen bundles.",
            Region::Elastosis => {
                "Dermis damaged by years of sun: elastic fibres degraded into grey-blue clumps."
            }
            Region::Vessel => "A capillary or venule, red blood cells inside.",
            Region::Lymphocyte => {
                "An immune cell; a band of them under a melanoma is the body reacting to it."
            }
            Region::Melanophage => {
                "A macrophage that has swallowed melanin, coarse brown granules."
            }
            Region::Follicle => "A hair follicle: epidermis going down into the dermis.",
        }
    }
}

/// A stained section, ready to show.
pub struct Section {
    /// sRGB RGBA, WIDTH by HEIGHT.
    pub image: Vec<u8>,
    regions: Vec<Region>,
    /// The Breslow thickness of an invasive melanoma: from the top of the
    /// granular layer to the deepest invasive cell, in micrometres.
    pub breslow: Option<f32>,
}

/// The region map is kept at half the resolution.
const GRAIN: usize = 2;

impl Section {
    /// What is at pixel `(x, y)`.
    pub fn region(&self, x: f32, y: f32) -> Region {
        if x < 0.0 || y < 0.0 {
            return Region::Slide;
        }
        let (gx, gy) = (x as usize / GRAIN, y as usize / GRAIN);
        let gw = WIDTH / GRAIN;
        if gx >= gw || gy >= HEIGHT / GRAIN {
            return Region::Slide;
        }
        self.regions[gy * gw + gx]
    }
}

/// The epidermis's shape across the section.
struct Profile {
    surface: Vec<f32>,
    corneum: Vec<f32>,
    junction: Vec<f32>,
    papillary: Vec<f32>,
    ridges: Vec<f32>,
}

impl Profile {
    fn at(v: &[f32], x: f32) -> f32 {
        let i = (x.max(0.0) as usize).min(v.len() - 1);
        v[i]
    }
    fn surface(&self, x: f32) -> f32 {
        Self::at(&self.surface, x)
    }
    /// The bottom of the stratum corneum, the top of the living epidermis.
    fn living(&self, x: f32) -> f32 {
        Self::at(&self.corneum, x)
    }
    fn junction(&self, x: f32) -> f32 {
        Self::at(&self.junction, x)
    }
    fn papillary(&self, x: f32) -> f32 {
        Self::at(&self.papillary, x)
    }
    /// The point on the junction at `x`, and the way up into the
    /// epidermis.
    fn junction_point(&self, x: f32) -> (Vec2, Vec2) {
        let (a, b) = (self.junction(x - 2.0), self.junction(x + 2.0));
        let tangent = Vec2::new(4.0, b - a).normalize_or(Vec2::X);
        (
            Vec2::new(x, self.junction(x)),
            Vec2::new(tangent.y, -tangent.x),
        )
    }
}

/// Cut and stain a section through `lesion`. The same seed gives the same
/// skin, whatever the stage, so stages can be shown one after the other.
pub fn cut(lesion: Lesion, seed: u32) -> Section {
    let mut slide = Slide::new(seed);
    let profile = slide.profile(lesion);
    slide.ground(&profile, lesion);
    slide.dermis(&profile, lesion);
    slide.epidermis(&profile, lesion);
    slide.lesion(&profile, lesion);
    Section {
        image: slide.canvas.develop(),
        regions: slide.regions,
        breslow: slide
            .deepest
            .map(|(x, y)| (y - profile.living(x)) * MICROMETRES_PER_PIXEL),
    }
}

struct Slide {
    canvas: Canvas,
    regions: Vec<Region>,
    random: Random,
    seed: u32,
    /// The deepest invasive cell, for the Breslow thickness.
    deepest: Option<(f32, f32)>,
    centre: f32,
}

impl Slide {
    fn new(seed: u32) -> Self {
        Self {
            canvas: Canvas::new(WIDTH, HEIGHT),
            regions: vec![Region::Slide; (WIDTH / GRAIN) * (HEIGHT / GRAIN)],
            random: Random::new(seed),
            seed,
            deepest: None,
            centre: WIDTH as f32 / 2.0,
        }
    }

    fn mark(&mut self, region: Region, centre: Vec2, radius: f32) {
        let gw = WIDTH / GRAIN;
        let (cx, cy, r) = (
            centre.x / GRAIN as f32,
            centre.y / GRAIN as f32,
            radius / GRAIN as f32 + 0.5,
        );
        for gy in (cy - r).max(0.0) as usize..=((cy + r) as usize).min(HEIGHT / GRAIN - 1) {
            for gx in (cx - r).max(0.0) as usize..=((cx + r) as usize).min(gw - 1) {
                if Vec2::new(gx as f32 - cx, gy as f32 - cy).length() <= r {
                    self.regions[gy * gw + gx] = region;
                }
            }
        }
    }

    /// The epidermis's surface, the bottom of its horn, its junction with
    /// the dermis and the papillary dermis's lower edge, across the section.
    fn profile(&mut self, lesion: Lesion) -> Profile {
        let seed = self.seed;
        // Rete ridges 60 to 100 µm wide, 100 to 150 µm apart; deeper and
        // closer on a sole, gone in sun-damaged skin.
        let (corneum, living, depth, spacing) = match lesion {
            Lesion::Acral => (120.0, 95.0, 95.0, 150.0),
            Lesion::LentigoMaligna => (14.0, 40.0, 4.0, 200.0),
            Lesion::MelanomaInSitu | Lesion::SuperficialSpreading => (24.0, 95.0, 66.0, 190.0),
            _ => (24.0, 85.0, 62.0, 190.0),
        };
        // Ridge centres across the section, jittered.
        let mut centres = Vec::new();
        let mut x = self.random.between(10.0, spacing);
        while x < WIDTH as f32 + spacing {
            centres.push((
                x,
                self.random.between(0.75, 1.15),
                self.random.between(52.0, 72.0),
            ));
            x += spacing * self.random.between(0.8, 1.2);
        }
        let centre = self.centre;
        let mut p = Profile {
            surface: Vec::with_capacity(WIDTH),
            corneum: Vec::with_capacity(WIDTH),
            junction: Vec::with_capacity(WIDTH),
            papillary: Vec::with_capacity(WIDTH),
            ridges: centres.iter().map(|c| c.0).collect(),
        };
        for xi in 0..WIDTH {
            let x = xi as f32;
            let surface = 78.0
                + 7.0 * (x / 170.0 + seed as f32).sin()
                + 6.0 * fbm(Vec2::new(x * 0.01, 0.0), seed, 3);
            // Over a nodule the epidermis is stretched thin and flat.
            let over_nodule = if lesion == Lesion::Nodular {
                (1.0 - ((x - centre) / 260.0).powi(2)).max(0.0)
            } else {
                0.0
            };
            // Each ridge a rounded tongue of epidermis, broad at its base.
            let ridges: f32 = centres
                .iter()
                .map(|(c, scale, half)| {
                    let t = ((x - c) / half).abs();
                    if t < 1.0 {
                        depth * scale * 0.5 * (1.0 + (std::f32::consts::PI * t).cos())
                    } else {
                        0.0
                    }
                })
                .sum();
            let ridges = ridges * (1.0 - over_nodule);
            let living = living * (1.0 - 0.45 * over_nodule);
            let horn = surface + corneum;
            let junction = horn + living + ridges;
            p.surface.push(surface);
            p.corneum.push(horn);
            p.junction.push(junction);
            p.papillary.push(horn + living + depth + 70.0);
        }
        p
    }

    /// The glass, the dermis's ground substance and the regions' map.
    fn ground(&mut self, p: &Profile, lesion: Lesion) {
        let seed = self.seed;
        let elastosis = lesion == Lesion::LentigoMaligna;
        self.canvas.fill(Stain::Eosin, |x, y| {
            let (xf, yf) = (x as f32, y as f32);
            let v = Vec2::new(xf, yf);
            if yf < Profile::at(&p.surface, xf) {
                0.012
            } else if yf < Profile::at(&p.junction, xf) {
                0.0
            } else if yf < Profile::at(&p.papillary, xf) {
                0.24 + 0.10 * fbm(v * 0.05, seed + 1, 3)
            } else {
                0.20 + 0.10 * fbm(v * 0.02, seed + 2, 3)
            }
        });
        if elastosis {
            self.canvas.fill(Stain::Haematoxylin, |x, y| {
                let (xf, yf) = (x as f32, y as f32);
                let j = Profile::at(&p.junction, xf);
                if yf > j + 12.0 && yf < j + 260.0 {
                    let clumps = fbm(Vec2::new(xf, yf) * 0.06, seed + 5, 4);
                    0.5 * ((clumps - 0.32) * 2.2).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            });
        }
        let gw = WIDTH / GRAIN;
        for gy in 0..HEIGHT / GRAIN {
            for gx in 0..gw {
                let (x, y) = ((gx * GRAIN) as f32, (gy * GRAIN) as f32);
                let region = if y < p.surface(x) {
                    Region::Slide
                } else if y < p.living(x) {
                    Region::Corneum
                } else if y < p.living(x) + 10.0 {
                    Region::Granular
                } else if y < p.junction(x) - 9.0 {
                    Region::Spinous
                } else if y < p.junction(x) {
                    Region::Basal
                } else if elastosis && y < p.junction(x) + 260.0 && y > p.junction(x) + 12.0 {
                    Region::Elastosis
                } else if y < p.papillary(x) {
                    Region::Papillary
                } else {
                    Region::Reticular
                };
                self.regions[gy * gw + gx] = region;
            }
        }
    }

    /// Collagen, fibroblasts and vessels.
    fn dermis(&mut self, p: &Profile, lesion: Lesion) {
        // Fine, loose fibres of the papillary dermis.
        for _ in 0..1100 {
            let x = self.random.between(0.0, WIDTH as f32);
            let top = p.junction(x) + 4.0;
            let bottom = p.papillary(x);
            if bottom <= top {
                continue;
            }
            let y = self.random.between(top, bottom);
            let (angle, length) = (
                self.random.between(-1.2, 1.2),
                self.random.between(20.0, 60.0),
            );
            let path = self.wavy(Vec2::new(x, y), angle, length, 0.5);
            self.canvas.stroke(Stain::Eosin, &path, 1.3, 0.20);
        }
        // Coarse bundles of the reticular dermis, mostly lying flat.
        for _ in 0..380 {
            let x = self.random.between(-100.0, WIDTH as f32);
            let y = self
                .random
                .between(p.papillary(x.max(0.0)) + 6.0, HEIGHT as f32 + 20.0);
            let angle = self.random.between(-0.35, 0.35);
            let length = self.random.between(120.0, 420.0);
            let path = self.wavy(Vec2::new(x, y), angle, length, 2.5);
            let width = self.random.between(7.0, 18.0);
            self.canvas.stroke(Stain::Eosin, &path, width, 0.26);
            // The cleft beside a bundle, where the section parted.
            let cleft: Vec<Vec2> = path
                .iter()
                .map(|q| *q + Vec2::new(0.0, width / 2.0 + 1.5))
                .collect();
            self.canvas.stroke(Stain::Eosin, &cleft, 1.6, -0.22);
        }
        // Fibroblasts: thin dark nuclei among the fibres.
        for _ in 0..320 {
            let x = self.random.between(0.0, WIDTH as f32);
            let y = self.random.between(p.junction(x) + 8.0, HEIGHT as f32);
            let angle =
                self.random.between(-0.5, 0.5) + if self.random.chance(0.2) { 1.3 } else { 0.0 };
            let length = self.random.between(4.5, 7.0);
            self.canvas.blob(
                Stain::Haematoxylin,
                Vec2::new(x, y),
                Vec2::new(length, 1.4),
                angle,
                1.0,
                &|_| 1.0,
            );
        }
        // Capillaries in the papillae between the ridges, and venules lower.
        let ridges = p.ridges.clone();
        for pair in ridges.windows(2) {
            let x = (pair[0] + pair[1]) / 2.0 + self.random.between(-8.0, 8.0);
            if !(10.0..WIDTH as f32 - 10.0).contains(&x) {
                continue;
            }
            let y = p.living(x) + 72.0 + self.random.between(0.0, 25.0);
            if y < p.junction(x) + 8.0 {
                continue;
            }
            let r = self.random.between(5.0, 8.5);
            self.vessel(Vec2::new(x, y), r);
        }
        for _ in 0..5 {
            let x = self.random.between(60.0, WIDTH as f32 - 60.0);
            let y = self
                .random
                .between(p.papillary(x) + 40.0, HEIGHT as f32 - 40.0);
            if lesion == Lesion::Nodular && (x - self.centre).abs() < 260.0 {
                continue;
            }
            let r = self.random.between(12.0, 22.0);
            self.vessel(Vec2::new(x, y), r);
        }
    }

    /// A vessel: an empty lumen, flat endothelial nuclei round it, red
    /// cells inside.
    fn vessel(&mut self, centre: Vec2, r: f32) {
        self.canvas.blob(
            Stain::Eosin,
            centre,
            Vec2::splat(r + 2.0),
            0.0,
            -0.25,
            &|_| 1.0,
        );
        self.canvas.blob(
            Stain::Eosin,
            centre,
            Vec2::splat(r + 2.5),
            0.0,
            0.12,
            &|l| {
                if l.length() > r { 1.0 } else { 0.0 }
            },
        );
        let around = (r / 4.0).round().max(3.0) as usize;
        for k in 0..around {
            let a =
                k as f32 / around as f32 * std::f32::consts::TAU + self.random.between(-0.3, 0.3);
            let at = centre + Vec2::new(a.cos(), a.sin()) * (r + 1.5);
            self.canvas.blob(
                Stain::Haematoxylin,
                at,
                Vec2::new(4.0, 1.4),
                a + std::f32::consts::FRAC_PI_2,
                1.0,
                &|_| 1.0,
            );
        }
        let cells = (r * 0.5).round() as usize;
        for _ in 0..cells {
            let a = self.random.between(0.0, std::f32::consts::TAU);
            let d = self.random.between(0.0, (r - 3.5).max(0.0));
            let at = centre + Vec2::new(a.cos(), a.sin()) * d;
            self.canvas
                .blob(Stain::Eosin, at, Vec2::splat(3.4), 0.0, 1.2, &|l| {
                    0.85 + 0.15 * (l.length() / 3.4)
                });
        }
        self.mark(Region::Vessel, centre, r + 2.0);
    }

    /// A gently wavy path from `start` along `angle`.
    fn wavy(&mut self, start: Vec2, angle: f32, length: f32, wobble: f32) -> Vec<Vec2> {
        let steps = (length / 6.0).max(2.0) as usize;
        let phase = self.random.between(0.0, 6.0);
        let along = Vec2::new(angle.cos(), angle.sin());
        let across = along.perp();
        (0..=steps)
            .map(|k| {
                let t = k as f32 * 6.0;
                start + along * t + across * (wobble * (t * 0.05 + phase).sin() * 2.0)
            })
            .collect()
    }

    /// Horn, granular layer, keratinocytes, basal cells and melanocytes.
    fn epidermis(&mut self, p: &Profile, lesion: Lesion) {
        let seed = self.seed;
        let acral = lesion == Lesion::Acral;
        // The horn: basket-weave, or compact and laminated on a sole.
        self.canvas.fill(Stain::Eosin, |x, y| {
            let (xf, yf) = (x as f32, y as f32);
            let (top, bottom) = (Profile::at(&p.surface, xf), Profile::at(&p.corneum, xf));
            if yf < top || yf >= bottom {
                return 0.0;
            }
            let v = Vec2::new(xf * 0.02, yf * 0.28);
            if acral {
                0.42 + 0.12 * fbm(v, seed + 7, 3)
            } else {
                let weave = ((fbm(v, seed + 7, 3) - 0.38) * 4.0).clamp(0.0, 1.0);
                0.40 * weave
            }
        });
        // The living epidermis: pink cytoplasm, a little mottled.
        self.canvas.fill(Stain::Eosin, |x, y| {
            let (xf, yf) = (x as f32, y as f32);
            let (top, bottom) = (Profile::at(&p.corneum, xf), Profile::at(&p.junction, xf));
            if yf < top || yf >= bottom {
                return 0.0;
            }
            let granular = if yf < top + 10.0 { 0.08 } else { 0.0 };
            // Cells about 13 pixels across, their borders paler: the
            // spinous layer's mosaic.
            let border = cell_border(Vec2::new(xf, yf), 13.0, seed + 9);
            0.36 + granular + 0.07 * noise(Vec2::new(xf, yf) * 0.12, seed + 8) - 0.16 * border
        });
        self.canvas.fill(Stain::Haematoxylin, |x, y| {
            let (xf, yf) = (x as f32, y as f32);
            let (top, bottom) = (Profile::at(&p.corneum, xf), Profile::at(&p.junction, xf));
            if yf < top || yf >= bottom { 0.0 } else { 0.05 }
        });
        // Keratohyalin granules.
        for _ in 0..2600 {
            let x = self.random.between(0.0, WIDTH as f32);
            let y = p.living(x) + self.random.between(0.0, 10.0);
            self.canvas.blob(
                Stain::Haematoxylin,
                Vec2::new(x, y),
                Vec2::splat(self.random.between(0.6, 1.2)),
                0.0,
                1.0,
                &|_| 1.0,
            );
        }
        // Keratinocyte nuclei on a jittered grid, flatter as they rise.
        let step = 14.0;
        let mut y = 40.0;
        while y < HEIGHT as f32 {
            let mut x = 0.0;
            while x < WIDTH as f32 {
                let at = Vec2::new(
                    x + self.random.between(-5.5, 5.5),
                    y + self.random.between(-5.5, 5.5),
                );
                if self.random.chance(0.08) {
                    x += step;
                    continue;
                }
                let (top, bottom) = (p.living(at.x) + 8.0, p.junction(at.x) - 11.0);
                if at.y > top && at.y < bottom {
                    let height = ((at.y - top) / (bottom - top).max(1.0)).clamp(0.0, 1.0);
                    let flat = 0.55 + 0.45 * height;
                    let r = Vec2::new(4.8, 4.0 * flat) * self.random.between(0.85, 1.1);
                    let s = self.random.between(0.0, 1000.0) as u32;
                    self.canvas.blob(
                        Stain::Haematoxylin,
                        at,
                        r,
                        self.random.between(-0.3, 0.3),
                        0.62,
                        &move |l| {
                            let rim = ((l / r).length() - 0.55).max(0.0) * 1.6;
                            0.6 + rim + 0.4 * noise(l * 0.9, s)
                        },
                    );
                }
                x += step;
            }
            y += step * 0.86;
        }
        // The basal layer, one cell deep along the junction, with a
        // melanocyte about every tenth cell.
        let replaced = |x: f32| -> bool {
            let d = (x - WIDTH as f32 / 2.0).abs();
            match lesion {
                Lesion::MelanomaInSitu | Lesion::SuperficialSpreading => d < 300.0,
                Lesion::LentigoMaligna => d < 380.0,
                Lesion::Acral => d < 320.0,
                Lesion::Nodular => d < 170.0,
                _ => false,
            }
        };
        let mut x = 2.0;
        let mut count = 0;
        while x < WIDTH as f32 - 2.0 {
            let (at, up) = p.junction_point(x);
            let atypical_here = replaced(x)
                && self.random.chance(match lesion {
                    Lesion::LentigoMaligna => 0.75,
                    Lesion::Acral => 0.55,
                    _ => 0.6,
                });
            if atypical_here {
                let scale = if lesion == Lesion::LentigoMaligna {
                    0.85
                } else {
                    1.0
                };
                self.atypical(at + up * 10.0, scale);
            } else if count % 10 == 7 {
                self.melanocyte(at + up * 5.0);
            } else {
                let centre = at + up * 5.5;
                let angle = up.y.atan2(up.x);
                self.canvas.blob(
                    Stain::Haematoxylin,
                    centre,
                    Vec2::new(5.2, 3.2),
                    angle,
                    1.05,
                    &|_| 1.0,
                );
                // A cap of melanin over the nucleus, shading it from light.
                self.canvas.blob(
                    Stain::Melanin,
                    centre + up * 4.5,
                    Vec2::new(3.0, 4.0),
                    angle,
                    0.12,
                    &|_| 1.0,
                );
            }
            count += 1;
            let (b, _) = p.junction_point(x + 9.0);
            x += 9.0 * 9.0 / at.distance(b).max(1.0);
        }
    }

    /// A melanocyte of the basal layer: small dark nucleus, clear halo.
    fn melanocyte(&mut self, at: Vec2) {
        self.canvas
            .blob(Stain::Eosin, at, Vec2::splat(7.0), 0.0, -0.22, &|_| 1.0);
        self.canvas.blob(
            Stain::Haematoxylin,
            at,
            Vec2::new(3.4, 3.0),
            0.0,
            1.2,
            &|_| 1.0,
        );
        self.mark(Region::Melanocyte, at, 7.0);
    }

    /// A nevus cell, plainer and smaller the deeper it lies: `depth` from
    /// nought at the junction to one at its deepest.
    fn nevus_cell(&mut self, at: Vec2, depth: f32, atypia: f32) {
        let r = (9.0 - 3.0 * depth) * (1.0 + 0.2 * atypia);
        self.canvas
            .blob(Stain::Eosin, at, Vec2::splat(r), 0.0, 0.14, &|_| 1.0);
        self.canvas
            .blob(Stain::Haematoxylin, at, Vec2::splat(r), 0.0, 0.05, &|_| 1.0);
        if depth < 0.4 {
            let s = self.random.between(0.0, 500.0) as u32;
            self.canvas.blob(
                Stain::Melanin,
                at,
                Vec2::splat(r),
                0.0,
                0.10 * (1.0 - depth),
                &move |l| noise(l * 0.5, s),
            );
        }
        let (nucleus, angle) = if depth > 0.75 {
            (Vec2::new(5.5, 2.2), self.random.between(-0.4, 0.4))
        } else {
            (Vec2::splat(4.0 - 0.6 * depth) * (1.0 + 0.25 * atypia), 0.0)
        };
        self.canvas.blob(
            Stain::Haematoxylin,
            at,
            nucleus,
            angle,
            0.95 + 0.15 * depth + 0.2 * atypia,
            &|_| 1.0,
        );
        self.mark(Region::NevusCell, at, r);
    }

    /// A malignant melanocyte: large, its nucleus big, dark and irregular,
    /// a prominent nucleolus; now and then dividing.
    fn atypical(&mut self, at: Vec2, scale: f32) {
        let r = self.random.between(11.0, 14.0) * scale;
        self.canvas
            .blob(Stain::Eosin, at, Vec2::splat(r), 0.0, 0.20, &|_| 1.0);
        self.canvas
            .blob(Stain::Haematoxylin, at, Vec2::splat(r), 0.0, 0.07, &|_| 1.0);
        let pigment = self.random.between(0.0, 0.4);
        let s = self.random.between(0.0, 500.0) as u32;
        self.canvas.blob(
            Stain::Melanin,
            at,
            Vec2::splat(r),
            0.0,
            pigment,
            &move |l| (noise(l * 0.35, s) - 0.3).max(0.0),
        );
        if self.random.chance(0.05) {
            // A mitotic figure: the chromosomes, condensed and clumped.
            for _ in 0..6 {
                let off = Vec2::new(
                    self.random.between(-4.0, 4.0),
                    self.random.between(-2.0, 2.0),
                ) * scale;
                self.canvas.blob(
                    Stain::Haematoxylin,
                    at + off,
                    Vec2::new(2.2, 1.2) * scale,
                    self.random.between(0.0, 3.0),
                    1.9,
                    &|_| 1.0,
                );
            }
        } else {
            let lobes = 3;
            for _ in 0..lobes {
                let off = Vec2::new(
                    self.random.between(-1.6, 1.6),
                    self.random.between(-1.6, 1.6),
                ) * scale;
                let rr =
                    Vec2::new(self.random.between(5.0, 7.0), self.random.between(4.2, 6.0)) * scale;
                let s2 = self.random.between(0.0, 500.0) as u32;
                // Vesicular: the chromatin against the nuclear membrane,
                // the middle paler.
                self.canvas.blob(
                    Stain::Haematoxylin,
                    at + off,
                    rr,
                    self.random.between(0.0, 3.0),
                    0.36,
                    &move |l| {
                        let edge = ((l / rr).length() - 0.6).max(0.0) * 2.5;
                        0.55 + edge + 0.5 * noise(l * 0.7, s2)
                    },
                );
            }
            let off = Vec2::new(
                self.random.between(-1.5, 1.5),
                self.random.between(-1.5, 1.5),
            ) * scale;
            self.canvas.blob(
                Stain::Eosin,
                at + off,
                Vec2::splat(2.0 * scale),
                0.0,
                1.4,
                &|_| 1.0,
            );
            self.canvas.blob(
                Stain::Haematoxylin,
                at + off,
                Vec2::splat(2.0 * scale),
                0.0,
                0.3,
                &|_| 1.0,
            );
        }
        self.mark(Region::AtypicalMelanocyte, at, r);
    }

    fn lymphocyte(&mut self, at: Vec2) {
        self.canvas.blob(
            Stain::Haematoxylin,
            at,
            Vec2::splat(self.random.between(2.9, 3.5)),
            0.0,
            1.45,
            &|_| 1.0,
        );
        self.mark(Region::Lymphocyte, at, 4.0);
    }

    fn melanophage(&mut self, at: Vec2) {
        self.canvas
            .blob(Stain::Eosin, at, Vec2::splat(8.0), 0.0, 0.1, &|_| 1.0);
        for _ in 0..9 {
            let off = Vec2::new(
                self.random.between(-6.0, 6.0),
                self.random.between(-6.0, 6.0),
            );
            self.canvas.blob(
                Stain::Melanin,
                at + off,
                Vec2::splat(self.random.between(1.3, 2.6)),
                0.0,
                1.0,
                &|_| 1.0,
            );
        }
        self.canvas.blob(
            Stain::Haematoxylin,
            at,
            Vec2::new(3.2, 2.4),
            0.0,
            0.9,
            &|_| 1.0,
        );
        self.mark(Region::Melanophage, at, 8.0);
    }

    /// Cells packed into a roundish nest, none closer than a cell across.
    fn nest_points(&mut self, centre: Vec2, radii: Vec2, count: usize, spacing: f32) -> Vec<Vec2> {
        let mut points: Vec<Vec2> = Vec::new();
        for _ in 0..count * 30 {
            if points.len() >= count {
                break;
            }
            let a = self.random.between(0.0, std::f32::consts::TAU);
            let d = self.random.between(0.0, 1.0).sqrt();
            let p = centre + Vec2::new(a.cos() * radii.x, a.sin() * radii.y) * d;
            if points.iter().all(|q| q.distance(p) > spacing) {
                points.push(p);
            }
        }
        points
    }

    /// Note a lesional cell in the dermis, for the Breslow thickness.
    fn invasive(&mut self, at: Vec2) {
        if self.deepest.is_none_or(|(_, y)| at.y > y) {
            self.deepest = Some((at.x, at.y));
        }
    }

    /// The lesion's cells, as its stage has them.
    fn lesion(&mut self, p: &Profile, lesion: Lesion) {
        let c = self.centre;
        let ridges: Vec<f32> = p
            .ridges
            .iter()
            .copied()
            .filter(|x| (x - c).abs() < 330.0)
            .collect();
        let tip = |x: f32| Vec2::new(x, p.junction(x));
        match lesion {
            Lesion::Normal => {}
            Lesion::JunctionalNevus | Lesion::CompoundNevus | Lesion::DysplasticNevus => {
                let reach = if lesion == Lesion::DysplasticNevus {
                    270.0
                } else {
                    220.0
                };
                let atypia = if lesion == Lesion::DysplasticNevus {
                    0.5
                } else {
                    0.0
                };
                for &x in ridges.iter().filter(|x| (*x - c).abs() < reach) {
                    let n = self.random.between(8.0, 14.0) as usize;
                    let at = tip(x) - Vec2::new(0.0, 4.0);
                    for q in self.nest_points(at, Vec2::new(24.0, 18.0), n, 14.0) {
                        self.nevus_cell(q, 0.0, atypia);
                    }
                }
                if lesion == Lesion::DysplasticNevus {
                    // Nests bridging neighbouring ridges, and single cells
                    // along the basal layer between them.
                    let near: Vec<f32> = ridges
                        .iter()
                        .copied()
                        .filter(|x| (*x - c).abs() < 250.0)
                        .collect();
                    for pair in near.windows(2) {
                        if self.random.chance(0.7) {
                            let (a, b) = (tip(pair[0]), tip(pair[1]));
                            let steps = (a.distance(b) / 15.0) as usize;
                            for k in 1..steps {
                                let x = pair[0] + (pair[1] - pair[0]) * k as f32 / steps as f32;
                                let (j, up) = p.junction_point(x);
                                self.nevus_cell(j + up * 9.0, 0.0, atypia);
                            }
                        }
                    }
                    // Lamellar fibroplasia: dense collagen in layers under
                    // the ridges.
                    for &x in &near {
                        for k in 0..3 {
                            let y0 = p.junction(x) + 10.0 + 6.0 * k as f32;
                            let path: Vec<Vec2> = (-6..=6)
                                .map(|i| Vec2::new(x + i as f32 * 5.0, y0 + (i * i) as f32 * 0.25))
                                .collect();
                            self.canvas.stroke(Stain::Eosin, &path, 2.5, 0.3);
                        }
                    }
                    for _ in 0..30 {
                        let x = c + self.random.between(-200.0, 200.0);
                        let y = p.junction(x) + self.random.between(15.0, 90.0);
                        self.lymphocyte(Vec2::new(x, y));
                    }
                }
                if matches!(lesion, Lesion::CompoundNevus | Lesion::DysplasticNevus) {
                    let (width, depth) = if lesion == Lesion::CompoundNevus {
                        (170.0, 230.0)
                    } else {
                        (110.0, 90.0)
                    };
                    self.dermal_nevus(p, width, depth, 30.0, atypia);
                }
            }
            Lesion::IntradermalNevus => {
                self.dermal_nevus(p, 210.0, 300.0, 22.0, 0.0);
            }
            Lesion::MelanomaInSitu
            | Lesion::SuperficialSpreading
            | Lesion::Acral
            | Lesion::LentigoMaligna
            | Lesion::Nodular => {
                let reach = match lesion {
                    Lesion::LentigoMaligna => 380.0,
                    Lesion::Acral => 320.0,
                    Lesion::Nodular => 170.0,
                    Lesion::SuperficialSpreading => 330.0,
                    _ => 300.0,
                };
                // Irregular nests along the junction.
                let nests = match lesion {
                    Lesion::LentigoMaligna | Lesion::Acral => 3,
                    _ => 8,
                };
                for _ in 0..nests {
                    let x = c + self.random.between(-reach * 0.9, reach * 0.9);
                    let (j, up) = p.junction_point(x);
                    let n = self.random.between(4.0, 11.0) as usize;
                    let radii = Vec2::new(
                        self.random.between(18.0, 34.0),
                        self.random.between(14.0, 22.0),
                    );
                    for q in self.nest_points(j + up * 8.0, radii, n, 20.0) {
                        self.atypical(q, 1.0);
                    }
                }
                // Pagetoid spread: single cells climbing through the
                // epidermis, fewer the higher they go.
                let climbing = match lesion {
                    Lesion::LentigoMaligna => 8,
                    Lesion::Acral => 22,
                    Lesion::Nodular => 12,
                    _ => 60,
                };
                for _ in 0..climbing {
                    let x = c + self.random.between(-reach, reach);
                    let (top, bottom) = (p.living(x) + 10.0, p.junction(x) - 14.0);
                    if bottom <= top {
                        continue;
                    }
                    let h = self.random.between(0.0, 1.0).powf(1.6);
                    self.atypical(Vec2::new(x, bottom - h * (bottom - top)), 0.95);
                }
                if lesion == Lesion::LentigoMaligna {
                    self.follicle(p, c + 120.0);
                }
                // Invasion.
                match lesion {
                    Lesion::SuperficialSpreading => {
                        for _ in 0..12 {
                            let x = c + self.random.between(-130.0, 130.0);
                            let y = p.junction(x) + self.random.between(20.0, 130.0);
                            let n = self.random.between(3.0, 9.0) as usize;
                            for q in
                                self.nest_points(Vec2::new(x, y), Vec2::new(22.0, 16.0), n, 20.0)
                            {
                                self.atypical(q, 1.0);
                                self.invasive(q);
                            }
                        }
                    }
                    Lesion::Acral => {
                        for _ in 0..5 {
                            let x = c + self.random.between(-90.0, 90.0);
                            let y = p.junction(x) + self.random.between(20.0, 80.0);
                            for q in
                                self.nest_points(Vec2::new(x, y), Vec2::new(18.0, 14.0), 5, 20.0)
                            {
                                self.atypical(q, 1.0);
                                self.invasive(q);
                            }
                        }
                    }
                    Lesion::Nodular => {
                        let centre = Vec2::new(c, p.junction(c) + 230.0);
                        let radii = Vec2::new(235.0, 195.0);
                        let mut y = centre.y - radii.y;
                        while y < centre.y + radii.y {
                            let mut x = centre.x - radii.x;
                            while x < centre.x + radii.x {
                                let q = Vec2::new(
                                    x + self.random.between(-5.0, 5.0),
                                    y + self.random.between(-5.0, 5.0),
                                );
                                let inside = ((q - centre) / radii).length() < 1.0;
                                if inside && q.y > p.junction(q.x) + 10.0 {
                                    self.atypical(q, 0.95);
                                    self.invasive(q);
                                }
                                x += 19.0;
                            }
                            y += 18.0;
                        }
                        // Lymphocytes at its edge, the body's reaction.
                        for _ in 0..170 {
                            let a = self.random.between(0.0, std::f32::consts::TAU);
                            let d = self.random.between(1.02, 1.18);
                            let q = centre + Vec2::new(a.cos() * radii.x, a.sin() * radii.y) * d;
                            if q.y > p.junction(q.x) + 6.0 && q.y < HEIGHT as f32 {
                                self.lymphocyte(q);
                            }
                        }
                    }
                    _ => {}
                }
                // A band of lymphocytes and melanophages under the lesion.
                if lesion != Lesion::Nodular {
                    let (count, depth) = match lesion {
                        Lesion::SuperficialSpreading => (220, 170.0),
                        Lesion::MelanomaInSitu => (140, 110.0),
                        _ => (80, 110.0),
                    };
                    for _ in 0..count {
                        let x = c + self.random.between(-reach * 0.9, reach * 0.9);
                        let y = p.junction(x) + self.random.between(12.0, depth);
                        self.lymphocyte(Vec2::new(x, y));
                    }
                    for _ in 0..12 {
                        let x = c + self.random.between(-reach * 0.8, reach * 0.8);
                        let y = p.junction(x) + self.random.between(18.0, 70.0);
                        self.melanophage(Vec2::new(x, y));
                    }
                }
            }
        }
        let _ = lesion.atypical();
    }

    /// The dermal part of a nevus: nests and cords, narrower, smaller and
    /// plainer with depth, what pathologists call maturation.
    fn dermal_nevus(&mut self, p: &Profile, width: f32, depth: f32, start: f32, atypia: f32) {
        let c = self.centre;
        let mut y_off = start;
        while y_off < depth {
            let frac = y_off / depth;
            let half = width * (1.0 - 0.45 * frac);
            let nests = (half / 45.0).max(1.0) as usize;
            for _ in 0..nests {
                let x = c + self.random.between(-half, half);
                let y = p.junction(x).max(p.living(x) + 95.0) + y_off;
                let n = (self.random.between(5.0, 11.0) * (1.0 - 0.4 * frac)) as usize;
                let spacing = 15.0 - 5.0 * frac;
                let radii = if frac > 0.7 {
                    Vec2::new(30.0, 8.0)
                } else {
                    Vec2::new(20.0, 16.0)
                };
                for q in self.nest_points(Vec2::new(x, y), radii, n, spacing) {
                    self.nevus_cell(q, frac, atypia);
                }
            }
            y_off += 26.0;
        }
    }

    /// A hair follicle going down from the epidermis, atypical cells
    /// along its basal layer.
    fn follicle(&mut self, p: &Profile, x: f32) {
        let top = p.junction(x);
        let bottom = top + 330.0;
        // Narrow at its neck, wider at its bulb, leaning a little.
        let half = |y: f32| {
            let t = ((y - top) / (bottom - top)).clamp(0.0, 1.0);
            24.0 + 14.0 * t * t
        };
        let middle = |y: f32| x + 22.0 * ((y - top) / (bottom - top)).powi(2);
        for yi in (top as i32..bottom as i32).step_by(2) {
            let y = yi as f32;
            let (m, w) = (middle(y), half(y));
            for xi in ((m - w) as i32..(m + w) as i32).step_by(2) {
                let xf = xi as f32;
                if (xf - m).abs() < w - 7.0 {
                    self.canvas.blob(
                        Stain::Eosin,
                        Vec2::new(xf, y),
                        Vec2::splat(1.6),
                        0.0,
                        0.18,
                        &|_| 1.0,
                    );
                }
            }
        }
        let width = 38.0;
        let mut y = top + 8.0;
        while y < bottom {
            let (m, w) = (middle(y), half(y));
            for side in [-1.0f32, 1.0] {
                let at = Vec2::new(m + side * (w - 5.0), y);
                if self.random.chance(0.45) {
                    self.atypical(at, 0.8);
                } else {
                    self.canvas.blob(
                        Stain::Haematoxylin,
                        at,
                        Vec2::new(3.0, 5.0),
                        0.0,
                        1.0,
                        &|_| 1.0,
                    );
                }
            }
            for _ in 0..2 {
                let at = Vec2::new(m + self.random.between(-w + 12.0, w - 12.0), y);
                self.canvas.blob(
                    Stain::Haematoxylin,
                    at,
                    Vec2::new(4.0, 3.4),
                    0.0,
                    0.8,
                    &|_| 1.0,
                );
            }
            y += 11.0;
        }
        self.mark(Region::Follicle, Vec2::new(x, (top + bottom) / 2.0), 0.0);
        let gw = WIDTH / GRAIN;
        for gy in (top as usize / GRAIN)..(bottom as usize / GRAIN).min(HEIGHT / GRAIN) {
            for gx in ((x - width) as usize / GRAIN)..=((x + width) as usize / GRAIN).min(gw - 1) {
                let r = self.regions[gy * gw + gx];
                if !matches!(r, Region::AtypicalMelanocyte) {
                    self.regions[gy * gw + gx] = Region::Follicle;
                }
            }
        }
    }
}

/// How near a point is to the border between two cells of a mosaic of
/// cells `size` pixels across: one on a border, nought in a cell's middle.
fn cell_border(p: Vec2, size: f32, seed: u32) -> f32 {
    let q = p / size;
    let (cx, cy) = (q.x.floor() as i32, q.y.floor() as i32);
    let (mut d1, mut d2) = (f32::MAX, f32::MAX);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (gx, gy) = (cx + dx, cy + dy);
            let site = Vec2::new(
                gx as f32 + super::canvas::hash(gx, gy, seed),
                gy as f32 + super::canvas::hash(gx, gy, seed ^ 0x9e37),
            );
            let d = q.distance(site);
            if d < d1 {
                d2 = d1;
                d1 = d;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (1.0 - (d2 - d1) * size / 1.6).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stage_is_cut_and_only_invasive_ones_have_a_breslow_thickness() {
        for (name, lesion) in Lesion::ALL {
            let section = cut(lesion, 3);
            assert_eq!(section.image.len(), WIDTH * HEIGHT * 4, "{name}");
            let invasive = matches!(
                lesion,
                Lesion::SuperficialSpreading | Lesion::Nodular | Lesion::Acral
            );
            assert_eq!(section.breslow.is_some(), invasive, "{name}");
        }
        let nodular = cut(Lesion::Nodular, 3).breslow.unwrap();
        let spreading = cut(Lesion::SuperficialSpreading, 3).breslow.unwrap();
        assert!(
            nodular > spreading,
            "a nodule is thicker: {nodular} against {spreading}"
        );
    }

    /// Every stage as a PNG into the directory `CELLSTAGE_OUT` names, to be
    /// looked at: `CELLSTAGE_OUT=dir cargo test -- --ignored draw_every_stage`.
    #[test]
    #[ignore]
    fn draw_every_stage() {
        let Some(dir) = std::env::var_os("CELLSTAGE_OUT") else {
            return;
        };
        for (name, lesion) in Lesion::ALL {
            let section = cut(lesion, 4);
            let file =
                std::fs::File::create(std::path::Path::new(&dir).join(format!("{name}.png")))
                    .unwrap();
            let mut encoder = png::Encoder::new(file, WIDTH as u32, HEIGHT as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&section.image)
                .unwrap();
        }
    }

    #[test]
    fn the_top_is_glass_and_the_bottom_is_dermis() {
        let section = cut(Lesion::Normal, 3);
        assert_eq!(section.region(600.0, 5.0), Region::Slide);
        assert!(matches!(
            section.region(600.0, (HEIGHT - 10) as f32),
            Region::Reticular | Region::Vessel | Region::Papillary
        ));
    }
}
