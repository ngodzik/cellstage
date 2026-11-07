// Things that glow on the dark: their edges bright and their faces faint,
// added to what is behind them, so that a tissue is seen through and its
// cells read as outlines, the way a fluorescence microscope shows them.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::{view, globals};

// The camera's near plane is kept at this fraction of its distance to what
// it looks at, so that the distance can be read from the projection here
// instead of being written into every material every frame, which would
// make the renderer prepare every cell again.
const NEAR: f32 = 0.004;

struct Glow {
    // The colour of the edges, linear; its alpha is how strong.
    rim: vec4<f32>,
    // The colour of the parts seen face on; its alpha is how strong.
    body: vec4<f32>,
    // x: how narrow the edges are, y: how much is shown (0 to 1),
    // z: the size of the slow variation in colour, 0 for none, w: unused.
    shape: vec4<f32>,
    // xyz: the middle of what is built, w: the radius past which things
    // fade into the dark, 0 for no fading.
    spot: vec4<f32>,
    // The camera distances it is seen at: gone below x, whole from y to z,
    // gone again from w; y at 0 for no near limit, z past 1e29 for no far
    // one.
    band: vec4<f32>,
    // How much it stands out: from x to y, starting at the time z, over w
    // seconds.
    emphasis: vec4<f32>,
}

// How far `d` is from `lo` to `hi`, evenly in its logarithm, smoothed.
fn ramp(d: f32, lo: f32, hi: f32) -> f32 {
    let t = clamp((log(d) - log(lo)) / (log(hi) - log(lo)), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> glow: Glow;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xyz;
    let n = normalize(in.world_normal);
    let to_eye = view.world_position - p;
    let facing = clamp(abs(dot(n, normalize(to_eye))), 0.0, 1.0);
    let edge = pow(1.0 - facing, glow.shape.x);
    var c = glow.rim.rgb * glow.rim.a * edge + glow.body.rgb * glow.body.a * facing * facing;

    let s = glow.shape.z;
    if s > 0.0 {
        let q = p / s;
        let w = sin(q.x * 1.3 + sin(q.z * 0.9)) * sin(q.z * 1.1 + q.y * 0.7 + sin(q.x * 0.6));
        c = c * (1.0 + 0.3 * w);
    }

    let f = view.clip_from_view[3][2] / NEAR;
    var shown = glow.shape.y;
    if glow.band.y > 0.0 {
        shown = shown * ramp(f, glow.band.x, glow.band.y);
    }
    if glow.band.z < 1e29 {
        shown = shown * (1.0 - ramp(f, glow.band.z, glow.band.w));
    }
    let e = glow.emphasis;
    let since = clamp((globals.time - e.z) / max(e.w, 0.001), 0.0, 1.0);
    shown = shown * mix(e.x, e.y, since * since * (3.0 - 2.0 * since));
    let r = glow.spot.w;
    if r > 0.0 {
        // A core of tissue with a clean wall, as if cut out with a punch.
        let d = length((p - glow.spot.xyz).xz);
        shown = shown * (1.0 - smoothstep(r * 0.86, r, d));
    }
    // Like a confocal microscope's optical section: what lies at the
    // distance the camera looks at is bright, what is nearer or farther
    // dims, and what is right in front of the lens is not drawn at all.
    if f > 0.0 {
        let dist = length(to_eye);
        let off = (dist - f) / (0.55 * f);
        let section = 0.16 + 0.84 * exp(-off * off);
        let near = smoothstep(0.12 * f, 0.5 * f, dist);
        shown = shown * section * near;
    }
    // Added to what is behind: premultiplied, with nothing taken away.
    return vec4<f32>(c * shown, 0.0);
}
