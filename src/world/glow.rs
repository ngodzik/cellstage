//! The material most of a world is drawn with: edges that glow on the dark
//! and faces seen through, added to what lies behind, so that a tissue
//! reads as a crowd of cells one sees into rather than as a wall.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// Registers the material and its shader.
pub struct GlowPlugin;

impl Plugin for GlowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "glow.wesl");
        app.add_plugins(MaterialPlugin::<Glow>::default());
    }
}

/// How a glowing thing looks. The colours are linear.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Glow {
    /// The edges' colour; alpha is their strength.
    #[uniform(0)]
    pub rim: LinearRgba,
    /// The faces' colour; alpha is their strength.
    #[uniform(0)]
    pub body: LinearRgba,
    /// x: the edges' narrowness, y: how much is shown, z: the size of a
    /// slow variation in colour.
    #[uniform(0)]
    pub shape: Vec4,
    /// xyz: the middle of what is built, w: the radius past which things
    /// fade.
    #[uniform(0)]
    pub spot: Vec4,
    /// The camera distances it is seen at, as a world's band.
    #[uniform(0)]
    pub band: Vec4,
    /// How much it stands out: from x to y, from the time z, over w
    /// seconds of the renderer's clock.
    #[uniform(0)]
    pub emphasis: Vec4,
}

/// The camera's near plane as a fraction of its distance to its focus,
/// which the shader reads that distance back from.
pub const NEAR: f32 = 0.004;

impl Glow {
    /// Edges of `rim` at `strength`, faces of `body` at `faces`.
    pub fn new(rim: [f32; 3], strength: f32, faces: f32) -> Self {
        let colour = LinearRgba::rgb(rim[0], rim[1], rim[2]);
        Self {
            rim: colour.with_alpha(strength),
            body: colour.with_alpha(faces),
            shape: Vec4::new(2.2, 1.0, 0.0, 0.0),
            spot: Vec4::ZERO,
            band: Vec4::new(0.0, 0.0, 1e30, 1e30),
            emphasis: Vec4::new(1.0, 1.0, 0.0, 0.0),
        }
    }

    /// Faces of another colour than the edges.
    pub fn faces(mut self, body: [f32; 3], strength: f32) -> Self {
        self.body = LinearRgba::rgb(body[0], body[1], body[2]).with_alpha(strength);
        self
    }

    /// Narrower or wider edges.
    pub fn edges(mut self, narrowness: f32) -> Self {
        self.shape.x = narrowness;
        self
    }

    /// A slow variation in colour over `size` micrometres.
    pub fn varied(mut self, size: f32) -> Self {
        self.shape.z = size;
        self
    }
}

impl Material for Glow {
    fn fragment_shader() -> ShaderRef {
        "embedded://cellstage/world/glow.wesl".into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    // Light that adds up does not care in which order it is drawn, so the
    // material goes with the opaque things, which are drawn many at once,
    // rather than with the transparent ones, which are sorted and drawn one
    // by one: tens of thousands of cells would take a fifth of a second a
    // frame. It adds to what is there instead of covering it, and leaves
    // the depth for what is solid. Both sides of a cell add to its glow,
    // as both sides of a real one would be lit.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}
