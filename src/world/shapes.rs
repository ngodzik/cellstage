//! Meshes a world is built of: tubes along curves, sheets over a height
//! field, the few solids cells are made from.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// A mesh grown from positions, normals and triangles.
#[derive(Default)]
pub struct Builder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl Builder {
    pub fn mesh(&self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone())
        .with_inserted_indices(Indices::U32(self.indices.clone()))
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// A tube through `points`, `radius(s)` thick at `s` from 0 at the
    /// first point to 1 at the last, with `sides` around. Its frames are
    /// carried along the curve so that it does not twist.
    pub fn tube(&mut self, points: &[Vec3], radius: impl Fn(f32) -> f32, sides: usize) {
        if points.len() < 2 {
            return;
        }
        let base = self.positions.len() as u32;
        let n = points.len();
        let tangent = |i: usize| {
            let a = points[i.saturating_sub(1)];
            let b = points[(i + 1).min(n - 1)];
            (b - a).normalize_or(Vec3::Y)
        };
        let t0 = tangent(0);
        let mut normal = t0.any_orthonormal_vector();
        let mut previous = t0;
        for (i, p) in points.iter().enumerate() {
            let t = tangent(i);
            // Turn the frame by the rotation that takes the last tangent to
            // this one.
            let turn = Quat::from_rotation_arc(previous, t);
            normal = (turn * normal).normalize_or(t.any_orthonormal_vector());
            previous = t;
            let binormal = t.cross(normal);
            let r = radius(i as f32 / (n - 1) as f32);
            for k in 0..sides {
                let a = k as f32 / sides as f32 * std::f32::consts::TAU;
                let out = normal * a.cos() + binormal * a.sin();
                self.positions.push((*p + out * r).to_array());
                self.normals.push(out.to_array());
            }
        }
        let s = sides as u32;
        for i in 0..(n as u32 - 1) {
            for k in 0..s {
                let a = base + i * s + k;
                let b = base + i * s + (k + 1) % s;
                let c = a + s;
                let d = b + s;
                self.indices.extend([a, c, b, b, c, d]);
            }
        }
    }

    /// A sheet over `x` and `z` in `[-half, half]`, at the height
    /// `height(x, z)`, kept within `radius` of the middle, `step` apart.
    pub fn sheet(&mut self, half: f32, step: f32, radius: f32, height: impl Fn(f32, f32) -> f32) {
        let n = (2.0 * half / step).ceil() as usize + 1;
        let base = self.positions.len() as u32;
        let at = |i: usize, j: usize| {
            let x = -half + i as f32 * step;
            let z = -half + j as f32 * step;
            Vec3::new(x, height(x, z), z)
        };
        for j in 0..n {
            for i in 0..n {
                let p = at(i, j);
                let dx = at((i + 1).min(n - 1), j) - at(i.saturating_sub(1), j);
                let dz = at(i, (j + 1).min(n - 1)) - at(i, j.saturating_sub(1));
                let normal = dz.cross(dx).normalize_or(Vec3::Y);
                self.positions.push(p.to_array());
                self.normals.push(normal.to_array());
            }
        }
        let inside = |i: usize, j: usize| {
            let p = at(i, j);
            p.x * p.x + p.z * p.z < radius * radius
        };
        for j in 0..n - 1 {
            for i in 0..n - 1 {
                if !inside(i, j) {
                    continue;
                }
                let a = base + (j * n + i) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                self.indices.extend([a, c, b, b, c, d]);
            }
        }
    }
}

/// A smooth curve through `points`, `per` points between each two.
pub fn smooth(points: &[Vec3], per: usize) -> Vec<Vec3> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let n = points.len();
    let mut out = Vec::with_capacity(n * per);
    for i in 0..n - 1 {
        let p0 = points[i.saturating_sub(1)];
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = points[(i + 2).min(n - 1)];
        for k in 0..per {
            let t = k as f32 / per as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            out.push(
                0.5 * ((2.0 * p1)
                    + (-p0 + p2) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3),
            );
        }
    }
    out.push(points[n - 1]);
    out
}

/// A curve's length, and where along it a fraction of that length falls.
#[derive(Debug, Clone, Default)]
pub struct Path {
    pub points: Vec<Vec3>,
    walked: Vec<f32>,
}

impl Path {
    pub fn new(points: Vec<Vec3>) -> Self {
        let mut walked = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                total += p.distance(points[i - 1]);
            }
            walked.push(total);
        }
        Self { points, walked }
    }

    pub fn length(&self) -> f32 {
        self.walked.last().copied().unwrap_or(0.0)
    }

    /// The point a fraction `f` of the way along, and the way it heads.
    pub fn at(&self, f: f32) -> (Vec3, Vec3) {
        let n = self.points.len();
        if n == 0 {
            return (Vec3::ZERO, Vec3::Y);
        }
        if n == 1 {
            return (self.points[0], Vec3::Y);
        }
        let s = f.clamp(0.0, 1.0) * self.length();
        let i = self.walked.partition_point(|&w| w < s).clamp(1, n - 1);
        let (a, b) = (self.points[i - 1], self.points[i]);
        let span = (self.walked[i] - self.walked[i - 1]).max(1e-6);
        let t = ((s - self.walked[i - 1]) / span).clamp(0.0, 1.0);
        (a.lerp(b, t), (b - a).normalize_or(Vec3::Y))
    }
}

/// A sphere of radius one, fine enough for its edge to read as round.
pub fn ball() -> Mesh {
    Sphere::new(1.0)
        .mesh()
        .ico(2)
        .unwrap_or_else(|_| Sphere::new(1.0).mesh().uv(24, 14))
}

/// A sphere of radius one with few faces, for what is only ever small.
pub fn bead() -> Mesh {
    Sphere::new(1.0)
        .mesh()
        .ico(1)
        .unwrap_or_else(|_| Sphere::new(1.0).mesh().uv(12, 8))
}

/// A red blood cell of radius one: a disc, thinner in the middle.
pub fn red_cell() -> Mesh {
    let mut mesh = Sphere::new(1.0).mesh().uv(28, 16);
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(ps)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in ps.iter_mut() {
            let r2 = p[0] * p[0] + p[2] * p[2];
            p[1] *= 0.12 + 0.3 * r2;
        }
    }
    mesh.compute_smooth_normals();
    mesh
}

/// A flat six-sided plate of radius one and thickness one: a cell of the
/// horny layer.
pub fn plate() -> Mesh {
    Cylinder::new(1.0, 1.0).mesh().resolution(6).build()
}
