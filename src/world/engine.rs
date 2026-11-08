//! What every world is drawn with: groups of parts that share a look and
//! fade in and out of the scales they are seen at, the camera that films
//! them, and what a world is given each frame.

use std::collections::HashMap;
use std::hash::Hash;

use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use super::film::Eye;
use super::glow::{self, Glow};

/// What a world's camera clears to: the dark things glow on.
pub const BACKGROUND: Color = Color::srgb(0.005, 0.007, 0.016);

/// How much what is not set apart is still shown.
const DIM: f32 = 0.26;

/// The scales a part of the world is seen at: gone below `near.0`, whole
/// from `near.1` up to `far.0`, gone again from `far.1`, in micrometres of
/// the camera's distance to what it looks at, faded between.
#[derive(Debug, Clone, Copy)]
pub struct Band {
    pub near: (f32, f32),
    pub far: (f32, f32),
}

/// No limit on that side.
pub const NONE: (f32, f32) = (0.0, 0.0);
pub const NEVER: (f32, f32) = (f32::MAX, f32::MAX);

impl Band {
    pub const fn new(near: (f32, f32), far: (f32, f32)) -> Self {
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

/// Smoothly from 0 to 1 as `t` goes from 0 to 1.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn shown_or_hidden(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// The parts of the world the frame moves, shows and hides.
pub type Things<'w, 's> =
    Query<'w, 's, (&'static mut Transform, &'static mut Visibility), Without<Camera>>;

/// Show or hide a part, touching it only if that changes: every change is
/// work for the renderer.
pub fn set_shown(things: &mut Things, e: Entity, shown: bool) {
    if let Ok((_, mut v)) = things.get_mut(e) {
        let wanted = shown_or_hidden(shown);
        if *v != wanted {
            *v = wanted;
        }
    }
}

/// What a world is told each frame.
pub struct Tick<'a> {
    pub eye: Eye,
    /// The world's clock, and what it was the frame before.
    pub clock: f32,
    pub before: f32,
    /// Seconds since the world opened.
    pub life: f32,
    /// The renderer's clock, which the shaders read too.
    pub now: f32,
    /// What is set apart.
    pub highlight: &'a [String],
}

/// One group of parts that share a look and fade together.
struct Group {
    material: Handle<Glow>,
    band: Band,
    /// How strong it is when wholly shown.
    strength: f32,
    entities: Vec<Entity>,
    hidden: bool,
    /// Whether its parts are shown and hidden one by one by the world,
    /// rather than all together.
    each: bool,
    /// What its material was last given: its strength, how much it stood
    /// out, and the shade the world's clock gave it.
    written: (f32, f32, f32),
}

/// A world's groups of parts, by its own keys.
pub struct Groups<K> {
    map: HashMap<K, Group>,
    pub layer: RenderLayers,
}

impl<K: Copy + Eq + Hash> Groups<K> {
    pub fn new(layer: RenderLayers) -> Self {
        Self {
            map: HashMap::new(),
            layer,
        }
    }

    /// A group that looks like `glow`, seen at the scales of `band`,
    /// fading past the radius `spot.w` across from its middle `spot.xyz`
    /// (no fading for 0); `each` when the world shows and hides its parts
    /// itself.
    pub fn add(
        &mut self,
        materials: &mut Assets<Glow>,
        key: K,
        mut glow: Glow,
        band: Band,
        spot: Vec4,
        each: bool,
    ) {
        glow.spot = spot;
        glow.band = Vec4::new(
            band.near.0,
            band.near.1,
            band.far.0.min(1e30),
            band.far.1.min(1e30),
        );
        let strength = glow.shape.y;
        self.map.insert(
            key,
            Group {
                material: materials.add(glow),
                band,
                strength,
                entities: Vec::new(),
                hidden: false,
                each,
                written: (strength, 1.0, f32::NAN),
            },
        );
    }

    /// Put a part of the group in the world.
    pub fn spawn(
        &mut self,
        world: &mut World,
        key: K,
        mesh: &Handle<Mesh>,
        t: Transform,
    ) -> Entity {
        let Some(group) = self.map.get_mut(&key) else {
            return Entity::PLACEHOLDER;
        };
        let e = world
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(group.material.clone()),
                t,
                self.layer.clone(),
            ))
            .id();
        group.entities.push(e);
        e
    }

    /// Put many parts in the world at once.
    pub fn spawn_many(
        &mut self,
        world: &mut World,
        parts: impl IntoIterator<Item = (K, Handle<Mesh>, Transform)>,
    ) -> Vec<Entity> {
        let parts: Vec<(K, Handle<Mesh>, Transform)> = parts
            .into_iter()
            .filter(|(k, _, _)| self.map.contains_key(k))
            .collect();
        let batch: Vec<_> = parts
            .iter()
            .map(|(k, mesh, t)| {
                (
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(self.map[k].material.clone()),
                    *t,
                    self.layer.clone(),
                )
            })
            .collect();
        let spawned: Vec<Entity> = world.spawn_batch(batch).collect();
        for ((k, _, _), e) in parts.iter().zip(&spawned) {
            if let Some(g) = self.map.get_mut(k) {
                g.entities.push(*e);
            }
        }
        spawned
    }

    /// Whether a group is hidden this frame.
    pub fn hidden(&self, key: K) -> bool {
        self.map.get(&key).is_none_or(|g| g.hidden)
    }

    /// Whether a group is well enough seen at `d` to be clicked.
    pub fn seen(&self, key: K, d: f32) -> bool {
        self.map
            .get(&key)
            .is_some_and(|g| !g.hidden && g.band.at(d) > 0.2)
    }

    /// Each frame: hide what the scale does not show, and give the
    /// materials their strength and how much they stand out, writing to a
    /// material only when that changes. The scale's own fades are the
    /// shader's, from the camera's distance: a material written to makes
    /// the renderer prepare again every part that wears it.
    pub fn update(
        &mut self,
        tick: &Tick,
        picks: impl Fn(&str, K) -> bool,
        strength: impl Fn(K, f32) -> f32,
        things: &mut Things,
        glows: &mut Assets<Glow>,
    ) {
        let d = tick.eye.distance;
        for (key, group) in self.map.iter_mut() {
            let s = strength(*key, group.strength);
            let hide = group.band.at(d) < 0.003 || s <= 0.001;
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
            let names = tick.highlight;
            let stands_out = if names.is_empty() || names.iter().any(|n| picks(n, *key)) {
                1.0
            } else {
                DIM
            };
            if (s - group.written.0).abs() <= 1e-3 && stands_out == group.written.1 {
                continue;
            }
            let Some(mut m) = glows.get_mut(&group.material) else {
                continue;
            };
            m.shape.y = s;
            if stands_out != group.written.1 {
                // From where the last change has got to, to the new target,
                // over a second of the renderer's clock.
                let e = m.emphasis;
                let t = ((tick.now - e.z) / e.w.max(0.001)).clamp(0.0, 1.0);
                let from = e.x + (e.y - e.x) * t * t * (3.0 - 2.0 * t);
                m.emphasis = Vec4::new(from, stands_out, tick.now, 1.0);
            }
            group.written.0 = s;
            group.written.1 = stands_out;
        }
    }

    /// Change a group's look by the world's clock: `shade` stands for the
    /// look, rounded so that the material is written only when the look
    /// changes enough to be seen.
    pub fn restyle(
        &mut self,
        key: K,
        shade: f32,
        glows: &mut Assets<Glow>,
        style: impl FnOnce(&mut Glow),
    ) {
        let Some(group) = self.map.get_mut(&key) else {
            return;
        };
        let shade = (shade * 60.0).round();
        if group.hidden || shade == group.written.2 {
            return;
        }
        group.written.2 = shade;
        if let Some(mut m) = glows.get_mut(&group.material) {
            style(&mut m);
        }
    }

    /// Every part, to take the world away.
    pub fn entities(self) -> Vec<Entity> {
        self.map.into_values().flat_map(|g| g.entities).collect()
    }
}

/// A camera for a world, on its own layer, its picture finished as a film
/// would be: in high dynamic range, its brights blooming.
pub fn camera(world: &mut World, order: isize, layer: RenderLayers, eye: &Eye) -> Entity {
    world
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
                near: eye.distance * glow::NEAR,
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
            layer,
        ))
        .id()
}

/// The meshes of simple solids, made once.
#[derive(Clone)]
pub struct Solids {
    pub ball: Handle<Mesh>,
    /// A sphere fine enough to fill the view: a whole cell, a nucleus.
    pub globe: Handle<Mesh>,
    pub bead: Handle<Mesh>,
    pub rod: Handle<Mesh>,
    pub ring: Handle<Mesh>,
    pub plate: Handle<Mesh>,
    pub red_cell: Handle<Mesh>,
}

impl Solids {
    pub fn make(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            ball: meshes.add(super::shapes::ball()),
            globe: meshes.add(
                Sphere::new(1.0)
                    .mesh()
                    .ico(5)
                    .unwrap_or_else(|_| Sphere::new(1.0).mesh().uv(96, 48)),
            ),
            bead: meshes.add(super::shapes::bead()),
            rod: meshes.add(Cylinder::new(1.0, 2.0).mesh().resolution(12).build()),
            ring: meshes.add(
                Torus {
                    minor_radius: 0.35,
                    major_radius: 1.0,
                }
                .mesh()
                .build(),
            ),
            plate: meshes.add(super::shapes::plate()),
            red_cell: meshes.add(super::shapes::red_cell()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_band_fades_in_and_out_across_scales() {
        let band = Band::new((1.0, 10.0), (100.0, 1000.0));
        assert_eq!(band.at(0.5), 0.0);
        assert!((band.at(3.16) - 0.5).abs() < 0.01);
        assert_eq!(band.at(50.0), 1.0);
        assert_eq!(band.at(2000.0), 0.0);
        assert_eq!(Band::new(NONE, NEVER).at(1e-6), 1.0);
    }
}
