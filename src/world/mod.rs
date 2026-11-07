//! Worlds: places in three dimensions, built cell by cell ahead of time,
//! that the agent films rather than draws. It names a place to go to, what
//! to say there and how far the world's clock runs, and the window flies
//! the camera from one scale to the next, with what lives in the world
//! moving all the while. The user can take the camera at any moment, turn
//! it, come closer, click on a cell.
//!
//! Each world is its own module, built on the engine's groups of glowing
//! parts; this one shows them in panels and runs their films.

pub mod engine;
pub mod film;
pub mod glow;
pub mod nevus;
pub mod nucleus;
pub mod shapes;
pub mod skin;
pub mod skin_world;
pub mod surface;
pub mod tumour;

use std::collections::HashMap;

use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::stage::Stage;
use engine::{Things, Tick};
use film::{Eye, Film, Place};
use glow::Glow;
use skin_world::SkinWorld;
use surface::Surface;

/// What a world can be asked for: its name, its stories, its places, what
/// can be set apart in it, and what its clock counts.
pub struct About {
    pub name: &'static str,
    pub variants: &'static [&'static str],
    pub places: &'static [&'static str],
    pub highlights: fn() -> Vec<&'static str>,
    pub things: fn() -> Vec<&'static str>,
    /// Where the clock starts when nothing is said.
    pub start: f32,
    pub clock: fn(f32) -> Option<String>,
}

/// Every world there is.
pub const WORLDS: [About; 1] = [About {
    name: "skin",
    variants: &skin_world::VARIANTS,
    places: &skin_world::PLACES,
    highlights: skin_world::highlights,
    things: SkinWorld::things,
    start: -1.0,
    clock: SkinWorld::clock_label,
}];

/// The world of that name.
pub fn about(name: &str) -> Option<&'static About> {
    WORLDS.iter().find(|w| w.name == name)
}

/// Every world's name.
pub fn names() -> Vec<&'static str> {
    WORLDS.iter().map(|w| w.name).collect()
}

/// Every place in every world, once each.
pub fn all_places() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = Vec::new();
    for w in &WORLDS {
        for p in w.places {
            if !all.contains(p) {
                all.push(p);
            }
        }
    }
    all
}

/// Everything that can be set apart in any world, once each.
pub fn all_highlights() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = Vec::new();
    for w in &WORLDS {
        for h in (w.highlights)() {
            if !all.contains(&h) {
                all.push(h);
            }
        }
    }
    all
}

/// Every variant of every world, once each.
pub fn all_variants() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = Vec::new();
    for w in &WORLDS {
        for v in w.variants {
            if !all.contains(v) {
                all.push(v);
            }
        }
    }
    all
}

/// What a panel's world is.
enum Content {
    Skin(SkinWorld),
}

impl Content {
    fn places(&self) -> &[Place] {
        match self {
            Content::Skin(w) => w.places(),
        }
    }

    fn locate(&self, name: &str, clock: f32, focus: Vec3) -> Option<Vec3> {
        match self {
            Content::Skin(w) => w.locate(name, clock, focus),
        }
    }

    fn pick(&self, origin: Vec3, direction: Vec3, clock: f32, d: f32) -> Option<String> {
        match self {
            Content::Skin(w) => w.pick(origin, direction, clock, d),
        }
    }

    fn update(
        &mut self,
        tick: &Tick,
        things: &mut Things,
        glows: &mut Assets<Glow>,
        surfaces: &mut Assets<Surface>,
    ) {
        match self {
            Content::Skin(w) => w.update(tick, things, glows, surfaces),
        }
    }

    fn entities(self) -> Vec<Entity> {
        match self {
            Content::Skin(w) => w.entities(),
        }
    }
}

/// One world, as shown in a panel.
pub struct WorldScene {
    pub camera: Entity,
    content: Content,
    pub about: &'static About,
    pub variant: String,
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
    /// The world's own clock: years, hours, divisions, as the world counts.
    pub time: f32,
    /// What is being said, and how visible the words are.
    pub say: Option<(String, f32)>,
    /// What is set apart from the rest when no film says otherwise.
    pub highlight: Vec<String>,
}

impl WorldScene {
    pub fn places(&self) -> &[Place] {
        self.content.places()
    }

    pub fn place(&self, name: &str) -> Option<Eye> {
        self.places().iter().find(|p| p.name == name).map(|p| p.eye)
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
        self.time = self.film.start;
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

    /// The world's clock in words, if the film runs it.
    pub fn clock_label(&self) -> Option<String> {
        if self.film.shots.iter().any(|s| s.clock.is_some()) {
            (self.about.clock)(self.time)
        } else {
            None
        }
    }

    /// Where a named thing is.
    pub fn locate(&self, name: &str) -> Option<Vec3> {
        self.content.locate(name, self.time, self.eye.focus)
    }

    /// What a ray from the camera meets first, among what is shown.
    pub fn pick(&self, origin: Vec3, direction: Vec3) -> Option<String> {
        self.content
            .pick(origin, direction, self.time, self.eye.distance)
    }
}

/// Every world shown, by panel.
#[derive(Resource, Default)]
pub struct Worlds {
    pub map: HashMap<String, WorldScene>,
    next: usize,
    skin_meshes: Option<skin_world::Meshes>,
}

/// Build what takes time before it is asked for.
pub fn warm() {
    std::thread::spawn(|| {
        skin_world::built();
    });
}

/// Put a world in a panel, or keep the one it shows if it is the same
/// world telling the same story.
pub fn show(world: &mut World, panel: &str, name: &str, variant: &str) -> Result<(), String> {
    let about =
        about(name).ok_or_else(|| format!("no world {name}; there are {}", names().join(", ")))?;
    if world
        .get_resource::<Worlds>()
        .and_then(|w| w.map.get(panel))
        .is_some_and(|s| s.about.name == name && s.variant == variant)
    {
        return Ok(());
    }
    remove(world, panel);
    let mut worlds = world.remove_resource::<Worlds>().unwrap_or_default();
    worlds.next += 1;
    let layer = RenderLayers::layer(40 + worlds.next % 20);
    let order = 60 + (worlds.next % 20) as isize;
    let made = match name {
        "skin" => {
            let meshes = worlds
                .skin_meshes
                .get_or_insert_with(|| {
                    skin_world::Meshes::make(&mut world.resource_mut::<Assets<Mesh>>())
                })
                .clone();
            SkinWorld::spawn(world, layer.clone(), variant, &meshes).map(Content::Skin)
        }
        _ => Err(format!("no world {name}")),
    };
    let content = match made {
        Ok(c) => c,
        Err(e) => {
            world.insert_resource(worlds);
            return Err(e);
        }
    };
    let eye = content
        .places()
        .iter()
        .find(|p| p.name == "skin")
        .or(content.places().first())
        .map(|p| p.eye)
        .unwrap_or(Eye {
            focus: Vec3::ZERO,
            distance: 100.0,
            yaw: 0.5,
            pitch: 0.3,
        });
    let camera = engine::camera(world, order, layer, &eye);
    worlds.map.insert(
        panel.to_owned(),
        WorldScene {
            camera,
            content,
            about,
            variant: variant.to_owned(),
            eye,
            film: Film::default(),
            clock: 0.0,
            playing: false,
            film_from: eye,
            manual: false,
            rejoin: 0.0,
            life: 0.0,
            time: about.start,
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
    let mut gone = scene.content.entities();
    gone.push(scene.camera);
    for e in gone {
        if let Ok(entity) = world.get_entity_mut(e) {
            entity.despawn();
        }
    }
}

/// Each frame: every world's camera where its panel is and where the film
/// or the user puts it, and the world brought to the film's moment.
#[allow(clippy::too_many_arguments)]
pub fn run(
    time: Res<Time>,
    stage: Res<Stage>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut worlds: ResMut<Worlds>,
    mut cameras: Query<(&mut Camera, &mut Transform, &mut Projection)>,
    mut things: Things,
    mut glows: ResMut<Assets<Glow>>,
    mut surfaces: ResMut<Assets<Surface>>,
) {
    let scale = windows.single().map_or(1.0, |w| w.scale_factor());
    let size = windows.single().map_or(UVec2::ONE, |w| {
        UVec2::new(w.physical_width(), w.physical_height())
    });
    let dt = time.delta_secs().min(0.1);
    let now = time.elapsed_secs_wrapped();
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
        let before = scene.time;
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
                highlight = frame.highlight;
            }
            scene.time = frame.clock;
            scene.say = frame.say;
        }
        let eye = scene.eye;
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
                p.near = eye.distance * glow::NEAR;
            }
        }
        let tick = Tick {
            eye,
            clock: scene.time,
            before,
            life: scene.life,
            now,
            highlight: &highlight,
        };
        scene
            .content
            .update(&tick, &mut things, &mut glows, &mut surfaces);
    }
}
