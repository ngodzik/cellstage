//! Protein panels: each a 3D scene on a render layer of its own, its own
//! camera looking through the panel's part of the window, lit by lamps that
//! ride with that camera.

use std::collections::HashMap;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::pbr::{
    DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::protein::build::{CAMERA_YAW, Model, Shown};
use crate::protein::pdb::Structure;
use crate::protein::{Look, bound, residue_name};
use crate::stage::Stage;

/// What a protein panel's camera clears to.
pub const BACKGROUND: Color = Color::srgb(0.012, 0.016, 0.028);

/// Lamps, in lux, and how strongly marks glow.
const KEY_LAMP: f32 = 3_800.0;
const FILL_LAMP: f32 = 900.0;
const RIM_LAMP: f32 = 2_600.0;
const GLOW: f32 = 1.4;

/// Seconds to move between two states when played.
const PASSAGE: f32 = 4.0;

/// How a protein is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Atoms,
    Fold,
}

/// Where a panel's camera is, about the point it looks at.
#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub focus: Vec3,
    pub goal: Vec3,
    pub reach: f32,
}

impl Orbit {
    fn around(reach: f32) -> Self {
        Self {
            yaw: CAMERA_YAW,
            pitch: 0.25,
            distance: reach * 3.0,
            focus: Vec3::ZERO,
            goal: Vec3::ZERO,
            reach,
        }
    }

    pub fn transform(&self) -> Transform {
        let around = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        Transform::from_translation(self.focus + around * self.distance)
            .looking_at(self.focus, Vec3::Y)
    }

    /// Turn by a drag of `delta` points.
    pub fn turn(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.006;
        self.pitch = (self.pitch + delta.y * 0.006).clamp(-1.5, 1.5);
    }

    /// Come closer by `lines` of the wheel.
    pub fn zoom(&mut self, lines: f32) {
        self.distance =
            (self.distance * (1.0 - 0.1 * lines)).clamp(self.reach * 0.25, self.reach * 8.0);
    }
}

/// One protein panel.
pub struct ProteinScene {
    pub camera: Entity,
    atoms: Vec<Entity>,
    ribbon: Entity,
    ribbon_mesh: Handle<Mesh>,
    pub model: Model,
    pub look: Look,
    pub style: Style,
    /// Between the first state and the second, and whether it plays.
    pub t: f32,
    pub playing: bool,
    heading: f32,
    pub orbit: Orbit,
    /// What was last placed, to place again only on a change.
    placed: Option<(f32, Style)>,
}

impl ProteinScene {
    /// The atom of a residue to pin a note to or turn to: its alpha carbon,
    /// or its first atom.
    pub fn residue(&self, chain: Option<char>, number: i32) -> Option<&Shown> {
        let matches =
            |a: &&Shown| a.number == number && chain.is_none_or(|c| a.chain == c) && !a.hetero;
        let atoms = &self.model.atoms;
        atoms
            .iter()
            .filter(matches)
            .find(|a| a.name == "CA")
            .or_else(|| atoms.iter().find(matches))
    }

    /// The atom under a ray, among those drawn.
    pub fn pick(&self, origin: Vec3, direction: Vec3) -> Option<&Shown> {
        let t = self.t;
        self.model
            .atoms
            .iter()
            .filter_map(|a| {
                if self.look.hidden.contains(&a.chain) {
                    return None;
                }
                let r = match (self.style, visible(a, self.style)) {
                    (_, true) => a.radius_at(t),
                    (Style::Fold, false) if a.name == "CA" => 1.8,
                    _ => return None,
                };
                let c = a.at(t) - origin;
                let along = c.dot(direction);
                let miss = c.length_squared() - along * along;
                (along > 0.0 && miss < r * r).then(|| (along - (r * r - miss).sqrt(), a))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, a)| a)
    }

    /// Point the camera at a residue, closer.
    pub fn focus(&mut self, chain: Option<char>, number: i32) -> bool {
        let Some(p) = self.residue(chain, number).map(|a| a.at(self.t)) else {
            return false;
        };
        self.orbit.goal = p;
        self.orbit.distance = (self.orbit.reach * 1.1).min(self.orbit.distance);
        true
    }
}

/// An atom in words: its amino acid or what it is, with any highlight's
/// label.
pub fn describe(atom: &Shown, look: &Look, t: f32) -> String {
    let residue = if t < 0.5 {
        &atom.residue.0
    } else {
        &atom.residue.1
    };
    let mut words = if atom.hetero {
        bound(residue).map_or_else(
            || format!("{residue}, bound to the protein"),
            |(w, _)| w.to_owned(),
        )
    } else {
        format!(
            "{} {} of chain {}",
            residue_name(residue),
            atom.number,
            atom.chain
        )
    };
    if let Some(label) = atom
        .mark
        .and_then(|m| look.highlights.get(m))
        .map(|h| &h.label)
        && !label.is_empty()
    {
        words.push_str(&format!(" ({label})"));
    }
    words
}

/// Whether an atom is drawn as a sphere in a style.
fn visible(atom: &Shown, style: Style) -> bool {
    match style {
        Style::Atoms => true,
        // What glows stays as atoms on the ribbon; the rest of what is
        // highlighted colours the ribbon itself.
        Style::Fold => !atom.in_ribbon || atom.glows,
    }
}

/// Every protein panel, and what they are drawn with.
#[derive(Resource, Default)]
pub struct Scenes {
    pub map: HashMap<String, ProteinScene>,
    next_layer: usize,
    sphere: Option<Handle<Mesh>>,
    materials: HashMap<([u32; 3], bool), Handle<StandardMaterial>>,
    lit: Option<Handle<StandardMaterial>>,
}

impl Scenes {
    fn material(
        &mut self,
        colour: [f32; 3],
        glows: bool,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        let key = (colour.map(f32::to_bits), glows);
        self.materials
            .entry(key)
            .or_insert_with(|| {
                let base = Color::srgb(colour[0], colour[1], colour[2]);
                materials.add(StandardMaterial {
                    base_color: base,
                    emissive: if glows {
                        base.to_linear() * GLOW
                    } else {
                        LinearRgba::BLACK
                    },
                    perceptual_roughness: 0.72,
                    reflectance: 0.22,
                    ..default()
                })
            })
            .clone()
    }
}

/// Put a protein in a panel, replacing what was there.
pub fn show(
    world: &mut World,
    panel: &str,
    structures: &[Structure],
    look: Look,
    style: Style,
) -> Result<(), String> {
    remove(world, panel);
    let model = Model::new(&look, structures).ok_or("the structure has no atoms this reads")?;
    let mut scenes = world.remove_resource::<Scenes>().unwrap_or_default();
    scenes.next_layer += 1;
    let layer = RenderLayers::layer(scenes.next_layer % 30 + 1);
    let order = 1 + scenes.next_layer as isize % 30;
    let sphere = scenes
        .sphere
        .get_or_insert_with(|| {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            meshes.add(
                Sphere::new(1.0)
                    .mesh()
                    .ico(4)
                    .unwrap_or_else(|_| Sphere::new(1.0).mesh().uv(32, 18)),
            )
        })
        .clone();
    let reach = model.reach;
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
            bevy::camera::Hdr,
            Tonemapping::AcesFitted,
            Bloom {
                intensity: 0.12,
                ..Bloom::NATURAL
            },
            Msaa::Off,
            Fxaa::default(),
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
                radius: 3.0,
                ..default()
            },
            DistanceFog {
                color: BACKGROUND,
                directional_light_color: Color::NONE,
                directional_light_exponent: 1.0,
                falloff: FogFalloff::Linear {
                    start: 50.0,
                    end: 150.0,
                },
            },
            Orbit::around(reach).transform(),
            layer.clone(),
        ))
        .id();
    let lamp = |illuminance: f32, from: Vec3, shadows: bool| {
        (
            DirectionalLight {
                illuminance,
                shadow_maps_enabled: shadows,
                color: Color::srgb(1.0, 0.97, 0.92),
                ..default()
            },
            CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 400.0,
                ..default()
            }
            .build(),
            Transform::from_translation(from).looking_at(Vec3::new(0.0, 0.0, -100.0), Vec3::Y),
            layer.clone(),
        )
    };
    world.entity_mut(camera).with_children(|c| {
        c.spawn(lamp(KEY_LAMP, Vec3::new(-60.0, 70.0, 40.0), true));
        c.spawn(lamp(FILL_LAMP, Vec3::new(80.0, -10.0, 30.0), false));
        c.spawn(lamp(RIM_LAMP, Vec3::new(0.0, 40.0, -260.0), false));
    });
    let mut atoms = Vec::with_capacity(model.atoms.len());
    for atom in &model.atoms {
        let material = {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            scenes.material(atom.colour, atom.glows, &mut materials)
        };
        atoms.push(
            world
                .spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(atom.from)
                        .with_scale(Vec3::splat(atom.radius_at(0.0))),
                    layer.clone(),
                ))
                .id(),
        );
    }
    let lit = scenes
        .lit
        .get_or_insert_with(|| {
            world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.45,
                    reflectance: 0.4,
                    ..default()
                })
        })
        .clone();
    let ribbon_mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(model.ribbon(0.0, &look.hidden));
    let ribbon = world
        .spawn((
            Mesh3d(ribbon_mesh.clone()),
            MeshMaterial3d(lit),
            Transform::default(),
            Visibility::Hidden,
            layer,
        ))
        .id();
    scenes.map.insert(
        panel.to_owned(),
        ProteinScene {
            camera,
            atoms,
            ribbon,
            ribbon_mesh,
            model,
            look,
            style,
            t: 0.0,
            playing: false,
            heading: 1.0,
            orbit: Orbit::around(reach),
            placed: None,
        },
    );
    world.insert_resource(scenes);
    Ok(())
}

/// Take a panel's protein away, if it has one.
pub fn remove(world: &mut World, panel: &str) {
    let Some(scene) = world
        .get_resource_mut::<Scenes>()
        .and_then(|mut s| s.map.remove(panel))
    else {
        return;
    };
    for e in scene.atoms.into_iter().chain([scene.ribbon, scene.camera]) {
        if let Ok(entity) = world.get_entity_mut(e) {
            entity.despawn();
        }
    }
}

/// Each frame: the cameras where their panels are and where their orbits
/// put them, and the atoms where the state between two structures puts them.
pub fn place(
    time: Res<Time>,
    stage: Res<Stage>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut scenes: ResMut<Scenes>,
    mut cameras: Query<(&mut Camera, &mut Transform, &mut DistanceFog)>,
    mut atoms: Query<(&mut Transform, &mut Visibility), Without<Camera>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let scale = windows.single().map_or(1.0, |w| w.scale_factor());
    let size = windows.single().map_or(UVec2::ONE, |w| {
        UVec2::new(w.physical_width(), w.physical_height())
    });
    let dt = time.delta_secs();
    for (id, scene) in scenes.map.iter_mut() {
        let Some(panel) = stage.panels.iter().find(|p| &p.id == id) else {
            continue;
        };
        if scene.playing && scene.model.moves {
            scene.t = (scene.t + dt / PASSAGE * scene.heading).clamp(0.0, 1.0);
            if scene.t <= 0.0 || scene.t >= 1.0 {
                scene.heading = -scene.heading;
            }
        }
        let k = 1.0 - (-5.0 * dt).exp();
        scene.orbit.focus = scene.orbit.focus.lerp(scene.orbit.goal, k);
        if let Ok((mut camera, mut transform, mut fog)) = cameras.get_mut(scene.camera) {
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
            *transform = scene.orbit.transform();
            let d = scene.orbit.distance;
            fog.falloff = FogFalloff::Linear {
                start: (d - scene.orbit.reach * 0.2).max(0.0),
                end: d + scene.orbit.reach * 1.6,
            };
        }
        let state = (scene.t, scene.style);
        if scene.placed == Some(state) {
            continue;
        }
        scene.placed = Some(state);
        for (atom, entity) in scene.model.atoms.iter().zip(&scene.atoms) {
            if let Ok((mut transform, mut visibility)) = atoms.get_mut(*entity) {
                transform.translation = atom.at(scene.t);
                transform.scale = Vec3::splat(atom.radius_at(scene.t));
                let shown = !scene.look.hidden.contains(&atom.chain) && visible(atom, scene.style);
                *visibility = if shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
        if let Ok((_, mut visibility)) = atoms.get_mut(scene.ribbon) {
            *visibility = if scene.style == Style::Fold {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if scene.style == Style::Fold
            && let Some(mut mesh) = meshes.get_mut(&scene.ribbon_mesh)
        {
            *mesh = scene.model.ribbon(scene.t, &scene.look.hidden);
        }
    }
}
