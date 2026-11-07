//! The skin's surface, seen from outside: a stretch of forearm, curving
//! away, its relief and the mole drawn by a shader of their own.

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Registers the material and its shader.
pub struct SurfacePlugin;

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "surface.wesl");
        app.add_plugins(MaterialPlugin::<Surface>::default());
    }
}

/// How the surface looks; the colours are linear.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Surface {
    #[uniform(0)]
    pub tone: LinearRgba,
    /// xy: the mole's middle, z: its radius, w: its darkness.
    #[uniform(0)]
    pub mole: Vec4,
    /// x: how high the mole stands, y: how much is shown, z: how pale the
    /// mole has grown.
    #[uniform(0)]
    pub shape: Vec4,
    /// xyz: towards the light, w: its strength.
    #[uniform(0)]
    pub light: Vec4,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            // A light skin's tone.
            tone: LinearRgba::rgb(0.72, 0.42, 0.3),
            mole: Vec4::ZERO,
            shape: Vec4::new(0.0, 1.0, 0.0, 0.0),
            light: Vec4::new(-0.45, 0.8, 0.4, 1.15),
        }
    }
}

impl Material for Surface {
    fn fragment_shader() -> ShaderRef {
        "embedded://cellstage/world/surface.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

/// The radius of the forearm the surface curves round, in micrometres.
const ARM: f32 = 42_000.0;

/// A stretch of skin `half` across either way about the origin, at height
/// `top` in the middle, curving down to either side as a forearm does,
/// finer near the middle where the camera comes close.
pub fn mesh(top: f32, half: f32) -> Mesh {
    // Spacing that grows away from the middle.
    let mut ticks = vec![0.0f32];
    let mut x = 0.0;
    let mut step = 40.0;
    while x < half {
        x += step;
        ticks.push(x.min(half));
        step *= 1.12;
    }
    let mut line: Vec<f32> = ticks.iter().rev().map(|t| -t).collect();
    line.extend(ticks.iter().skip(1));
    let n = line.len();
    let mut positions = Vec::with_capacity(n * n);
    let mut normals = Vec::with_capacity(n * n);
    for z in &line {
        for x in &line {
            let a = x / ARM;
            positions.push([ARM * a.sin(), top - ARM * (1.0 - a.cos()), *z]);
            normals.push([a.sin(), a.cos(), 0.0]);
        }
    }
    let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = (j * n + i) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            indices.extend([a, c, b, b, c, d]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}
