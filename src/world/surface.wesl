// The skin's surface as the eye sees it from a few centimetres: its tone
// and mottling, the fine lines that cross it in a net of small fields,
// the openings of follicles and sweat ducts, and the mole, its size,
// darkness and height given by the world's clock. Everything is computed
// where it is seen, so it stays sharp from an arm's length to a
// millimetre.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;

struct Surface {
    // The skin's colour, linear.
    tone: vec4<f32>,
    // xy: the mole's middle (x, z), z: its radius, w: how dark it is.
    mole: vec4<f32>,
    // x: how high the mole stands, y: how much is shown, z: how pale the
    // mole has grown, w: how far a melanoma has grown out of it, 0 to 1.
    shape: vec4<f32>,
    // xyz: towards the light, w: its strength.
    light: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> surface: Surface;

fn hash1(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash1(i);
    let b = hash1(i + vec2<f32>(1.0, 0.0));
    let c = hash1(i + vec2<f32>(0.0, 1.0));
    let d = hash1(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var k = 0; k < 5; k = k + 1) {
        v = v + a * noise(q);
        q = q * 2.03 + vec2<f32>(1.7, 9.2);
        a = a * 0.5;
    }
    return v;
}

// The distance to the nearest edge of a Voronoi cell, the distance to its
// seed, and a number that tells the cell apart.
fn voronoi(x: vec2<f32>) -> vec3<f32> {
    let n = floor(x);
    let f = fract(x);
    var mg = vec2<f32>(0.0);
    var mr = vec2<f32>(0.0);
    var md = 8.0;
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let g = vec2<f32>(f32(i), f32(j));
            let o = hash2(n + g);
            let r = g + o - f;
            let d = dot(r, r);
            if d < md {
                md = d;
                mr = r;
                mg = g;
            }
        }
    }
    var edge = 8.0;
    for (var j = -2; j <= 2; j = j + 1) {
        for (var i = -2; i <= 2; i = i + 1) {
            let g = mg + vec2<f32>(f32(i), f32(j));
            let o = hash2(n + g);
            let r = g + o - f;
            if dot(mr - r, mr - r) > 0.00001 {
                edge = min(edge, dot(0.5 * (mr + r), normalize(r - mr)));
            }
        }
    }
    return vec3<f32>(edge, sqrt(md), hash1(n + mg));
}

// The skin's relief at a point, in micrometres: the lines sunk, the fields
// between them gently domed.
fn relief(p: vec2<f32>) -> f32 {
    // The lines run in two main directions, as on a forearm.
    let q = vec2<f32>(p.x * 0.8 + p.y * 0.45, p.y * 1.1 - p.x * 0.3);
    let primary = voronoi(q / 420.0);
    let secondary = voronoi(q / 140.0 + vec2<f32>(5.3, 1.1));
    let line1 = 1.0 - smoothstep(0.0, 0.07, primary.x);
    let line2 = 1.0 - smoothstep(0.0, 0.09, secondary.x);
    let dome = smoothstep(0.0, 0.45, primary.x);
    return -26.0 * line1 - 9.0 * line2 + 6.0 * dome + 3.0 * fbm(p / 60.0);
}

// How far into the mole a point is, 0 outside, with an uneven edge.
fn mole_at(p: vec2<f32>) -> vec2<f32> {
    let r = surface.mole.z;
    if r <= 1.0 {
        return vec2<f32>(0.0, 2.0);
    }
    let g = surface.shape.w;
    // A melanoma grows unevenly: its middle drifts, its edge notches.
    let d = p - surface.mole.xy - vec2<f32>(0.3, -0.15) * r * g;
    let a = atan2(d.y, d.x);
    let dir = vec2<f32>(cos(a), sin(a));
    let lobes = 0.7 * noise(dir * 2.2 + vec2<f32>(3.0, 7.0)) + 0.3 * noise(dir * 4.5 + vec2<f32>(1.0, 2.0));
    let wobble = 1.0 + (0.1 + 0.35 * g) * (lobes - 0.5) * 2.0;
    let rr = length(d) / (r * wobble);
    return vec2<f32>(1.0 - smoothstep(0.9, 1.01, rr), rr);
}

fn height(p: vec2<f32>) -> f32 {
    let m = mole_at(p);
    var raised = surface.shape.x * max(1.0 - m.y * m.y, 0.0);
    // A melanoma's nodule, standing up out of it once it has grown down.
    let g = surface.shape.w;
    if g > 0.6 {
        let nod = length(p - surface.mole.xy - vec2<f32>(0.25, 0.1) * surface.mole.z) / (0.28 * surface.mole.z);
        raised = raised + 260.0 * (g - 0.6) / 0.4 * max(1.0 - nod * nod, 0.0);
    }
    // A mole's skin is smoother than the skin around it.
    return relief(p) * (1.0 - 0.5 * m.x) + raised;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xz;
    let to_eye = view.world_position - in.world_position.xyz;
    let v = normalize(to_eye);
    // A step for the slope fine enough for what is seen, never finer than
    // the relief.
    let e = clamp(length(to_eye) * 0.0006, 2.0, 40.0);
    let h = height(p);
    let hx = height(p + vec2<f32>(e, 0.0));
    let hz = height(p + vec2<f32>(0.0, e));
    let base = normalize(in.world_normal);
    let n = normalize(base + vec3<f32>(-(hx - h) / e, 0.0, -(hz - h) / e) * 0.9);

    // The skin's colour, mottled, a little redder where it is thin.
    let mottle = fbm(p / 1800.0);
    let freckle = smoothstep(0.72, 0.9, noise(p / 260.0 + vec2<f32>(11.0, 4.0)));
    var albedo = surface.tone.rgb * (0.9 + 0.2 * mottle);
    albedo = mix(albedo, albedo * vec3<f32>(0.82, 0.68, 0.6), freckle * 0.35);
    let line = smoothstep(-4.0, -20.0, h);
    albedo = albedo * (1.0 - 0.18 * line);

    // Pores: openings at some of the corners of the fields.
    let q = vec2<f32>(p.x * 0.8 + p.y * 0.45, p.y * 1.1 - p.x * 0.3);
    let cell = voronoi(q / 420.0);
    let pore = (1.0 - smoothstep(0.035, 0.07, cell.y)) * step(0.72, cell.z);
    albedo = albedo * (1.0 - 0.55 * pore);

    // The mole: brown, darker in its middle and in its nests, paler as it
    // ages.
    let m = mole_at(p);
    if m.x > 0.0 {
        let nests = smoothstep(0.45, 0.8, noise(p / 55.0)) * 0.5 + 0.5 * smoothstep(0.5, 0.9, noise(p / 21.0));
        let deep = vec3<f32>(0.11, 0.045, 0.02);
        let brown = vec3<f32>(0.24, 0.11, 0.05);
        let tan = vec3<f32>(0.42, 0.24, 0.15);
        var mole = mix(brown, deep, (1.0 - m.y) * 0.5 + nests * 0.25);
        // A melanoma's colours: black, blue-grey, red, and white where it
        // regresses, in patches, more as it grows.
        let g = surface.shape.w;
        if g > 0.0 {
            let blot = fbm(p / 650.0 + vec2<f32>(4.0, 1.0));
            let other = fbm(p / 380.0 + vec2<f32>(9.0, 3.0));
            mole = mix(mole, vec3<f32>(0.03, 0.018, 0.016), smoothstep(0.45, 0.7, blot) * g);
            mole = mix(mole, vec3<f32>(0.14, 0.16, 0.22), smoothstep(0.58, 0.8, other) * g * 0.6);
            mole = mix(mole, vec3<f32>(0.32, 0.1, 0.08), smoothstep(0.62, 0.82, 1.0 - blot) * g * 0.5);
            mole = mix(mole, vec3<f32>(0.5, 0.42, 0.4), smoothstep(0.78, 0.9, other * blot * 2.0) * g * 0.5);
        }
        // The pigment network a dermatoscope shows: the rete ridges, full
        // of pigment, a darker honeycomb thinning towards the edge.
        let ridges = voronoi(p / 110.0 + vec2<f32>(2.0, 9.0));
        let network = (1.0 - smoothstep(0.0, 0.12, ridges.x)) * (1.0 - smoothstep(0.6, 1.0, m.y));
        mole = mole * (1.0 - 0.4 * network);
        mole = mix(mole, tan, surface.shape.z);
        albedo = mix(albedo, mole, m.x * surface.mole.w);
    }

    // Light: wrapped, as skin scatters it; a warm glow where it is in
    // shade; an oily sheen, less in the lines.
    let l = normalize(surface.light.xyz);
    let lit = max((dot(n, l) + 0.35) / 1.35, 0.0);
    let half_way = normalize(l + v);
    let sheen = pow(max(dot(n, half_way), 0.0), 70.0) * 0.22 * (1.0 - line) * (1.0 - 0.6 * m.x);
    let rim = pow(1.0 - max(dot(n, v), 0.0), 4.0) * 0.08;
    let scatter = vec3<f32>(0.5, 0.12, 0.06) * (1.0 - lit) * 0.18;
    var colour = albedo * (lit * surface.light.w * vec3<f32>(1.0, 0.95, 0.88) + vec3<f32>(0.16, 0.17, 0.2)) + scatter * albedo * 2.0 + vec3<f32>(sheen + rim);

    // Far away, the arm falls off into the dark, as in a studio.
    let d = length(p - surface.mole.xy);
    let falloff = 1.0 - smoothstep(9000.0, 26000.0, d);
    colour = colour * falloff;
    return vec4<f32>(colour, surface.shape.y);
}
