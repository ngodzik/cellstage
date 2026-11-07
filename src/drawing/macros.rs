//! Shorthands the agent may write in a drawing, expanded into plain SVG
//! before the drawing is read: a template inserted whole, its parts named;
//! shapes scattered over a region, set in a ring, a row, a grid or along a
//! path; a lipid bilayer or a double helix along a path. Each becomes a
//! group, which keeps the shorthand's id, class and style.

use std::fmt::Write;

use resvg::usvg;

use super::{SHAPES, templates};

/// The shorthands, by the element that writes them.
pub const MACROS: [&str; 8] = [
    "insert", "scatter", "ring", "row", "grid", "along", "bilayer", "helix",
];

/// The attributes a shorthand hands on to the group it makes.
const HANDED: [&str; 10] = [
    "id",
    "class",
    "style",
    "opacity",
    "filter",
    "fill",
    "stroke",
    "stroke-width",
    "data-what",
    "transform",
];

/// Expand every shorthand in a drawing. `size` is the drawing's, for a
/// template placed without saying where.
pub fn expand(text: &str, size: (f32, f32)) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((at, name)) = next_macro(rest) {
        out.push_str(&rest[..at]);
        let end = at
            + rest[at..]
                .find('>')
                .ok_or_else(|| format!("<{name} does not close"))?;
        let closed = rest.as_bytes()[end - 1] == b'/';
        let inner = &rest[at + 1 + name.len()..if closed { end - 1 } else { end }];
        let attributes = Attributes::parse(inner).map_err(|e| format!("<{name}>: {e}"))?;
        let mut after = end + 1;
        if !closed {
            let close = format!("</{name}>");
            after = rest[after..]
                .find(&close)
                .map(|i| after + i + close.len())
                .ok_or_else(|| format!("<{name}> has no {close}"))?;
        }
        let written = write(name, &attributes, size, text).map_err(|e| format!("<{name}>: {e}"))?;
        out.push_str(&written);
        rest = &rest[after..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Where the next shorthand begins, and which it is.
fn next_macro(text: &str) -> Option<(usize, &'static str)> {
    let mut from = 0;
    while let Some(i) = text[from..].find('<') {
        let at = from + i;
        let after = &text[at + 1..];
        for name in MACROS {
            if after.starts_with(name)
                && after[name.len()..]
                    .starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>')
            {
                return Some((at, name));
            }
        }
        from = at + 1;
    }
    None
}

fn write(name: &str, a: &Attributes, size: (f32, f32), whole: &str) -> Result<String, String> {
    match name {
        "insert" => insert(a, size),
        "scatter" => scatter(a, whole),
        "ring" => ring(a, whole),
        "row" => row(a, whole),
        "grid" => grid(a, whole),
        "along" => along(a, whole),
        "bilayer" => bilayer(a),
        "helix" => helix(a),
        _ => Err("not a shorthand".to_owned()),
    }
}

/// An element's attributes, in the order written.
#[derive(Debug, Default)]
pub struct Attributes(Vec<(String, String)>);

impl Attributes {
    /// Read `name="value"` pairs, quoted with either quote.
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut pairs = Vec::new();
        let mut rest = s.trim_start();
        while !rest.is_empty() {
            let eq = rest
                .find('=')
                .ok_or_else(|| format!("{} has no value", rest.trim()))?;
            let name = rest[..eq].trim().to_owned();
            let after = rest[eq + 1..].trim_start();
            let quote = after
                .chars()
                .next()
                .filter(|c| *c == '"' || *c == '\'')
                .ok_or_else(|| format!("the value of {name} is not quoted"))?;
            let close = after[1..]
                .find(quote)
                .ok_or_else(|| format!("the value of {name} is not closed"))?;
            pairs.push((name, after[1..1 + close].to_owned()));
            rest = after[close + 2..].trim_start();
        }
        Ok(Self(pairs))
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn number(&self, name: &str) -> Result<Option<f32>, String> {
        self.get(name)
            .map(|v| {
                v.trim()
                    .parse::<f32>()
                    .map_err(|_| format!("{name}=\"{v}\" is not a number"))
            })
            .transpose()
    }

    fn number_or(&self, name: &str, default: f32) -> Result<f32, String> {
        Ok(self.number(name)?.unwrap_or(default))
    }

    /// A value or a range, "1.2" or "0.8 1.2".
    fn range(&self, name: &str, default: (f32, f32)) -> Result<(f32, f32), String> {
        let Some(v) = self.get(name) else {
            return Ok(default);
        };
        let numbers: Vec<f32> = v
            .split([' ', ','])
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<f32>()
                    .map_err(|_| format!("{name}=\"{v}\" is not a number"))
            })
            .collect::<Result<_, _>>()?;
        match numbers[..] {
            [x] => Ok((x, x)),
            [x, y] => Ok((x, y)),
            _ => Err(format!("{name}=\"{v}\": give one number or two")),
        }
    }

    /// The attributes handed on to the group, as written in a tag.
    fn handed(&self, extra_transform: Option<&str>) -> String {
        let mut out = String::new();
        for (k, v) in &self.0 {
            if HANDED.contains(&k.as_str()) && k != "transform" {
                let _ = write!(out, " {k}=\"{v}\"");
            }
        }
        let transform = [self.get("transform"), extra_transform]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        if !transform.is_empty() {
            let _ = write!(out, " transform=\"{transform}\"");
        }
        out
    }
}

/// A die that always rolls the same for the same seed, so that a drawing
/// comes out the same each time it is shown.
struct Dice(u64);

impl Dice {
    fn new(seed: f32) -> Self {
        Self((seed.to_bits() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// A number in [0, 1).
    fn roll(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (x >> 40) as f32 / (1u64 << 24) as f32
    }

    fn between(&mut self, (a, b): (f32, f32)) -> f32 {
        a + (b - a) * self.roll()
    }
}

/// The size of a house shape, from what the kit says of it: "24 across",
/// "145 by 95".
pub fn shape_size(id: &str) -> Option<(f32, f32)> {
    let (_, size, _, _) = SHAPES.iter().find(|s| s.0 == id)?;
    let words: Vec<&str> = size.split([' ', ',']).collect();
    let first: f32 = words.first()?.parse().ok()?;
    match words.get(1) {
        Some(&"by") => Some((first, words.get(2)?.parse().ok()?)),
        _ => Some((first, first)),
    }
}

/// A shape a shorthand sets, and its size.
type Shape<'a> = (&'a str, (f32, f32));

/// The shapes a shorthand sets, from shape="rbc" or shapes="rbc rbc
/// neutrophil": a house shape, or a symbol the drawing defines.
fn shapes<'a>(a: &'a Attributes, whole: &str) -> Result<Vec<Shape<'a>>, String> {
    let list = a
        .get("shape")
        .or(a.get("shapes"))
        .ok_or("which shape? shape=\"rbc\", or shapes=\"rbc rbc neutrophil\" to mix them")?;
    let fallback = a.number("size")?;
    list.split_whitespace()
        .map(|s| {
            let s = s.trim_start_matches('#');
            if let Some(size) = shape_size(s) {
                return Ok((s, fallback.map_or(size, |f| (f, f))));
            }
            if whole.contains(&format!("id=\"{s}\"")) || whole.contains(&format!("id='{s}'")) {
                let f = fallback.unwrap_or(40.0);
                return Ok((s, (f, f)));
            }
            Err(format!(
                "no shape {s}; the house shapes are listed by drawing_kit"
            ))
        })
        .collect()
}

/// One shape, placed.
fn place(out: &mut String, shape: &str, (x, y): (f32, f32), rotate: f32, scale: f32) {
    // Rounded first, so that a hair below nought is not written -0.0.
    let round = |v: f32| (v * 10.0).round() / 10.0 + 0.0;
    let (x, y, rotate) = (round(x), round(y), round(rotate));
    let _ = write!(
        out,
        "<use href=\"#{shape}\" transform=\"translate({x:.1} {y:.1}) rotate({rotate:.1}) scale({scale:.3})\"/>"
    );
}

fn group(a: &Attributes, body: &str) -> String {
    format!("<g{}>{body}</g>", a.handed(None))
}

/// A region to scatter over: a rectangle, an ellipse, or a closed path.
enum Region {
    Rect(f32, f32, f32, f32),
    Ellipse(f32, f32, f32, f32),
    Polygon(Vec<(f32, f32)>),
}

impl Region {
    fn read(a: &Attributes) -> Result<Self, String> {
        if let Some(d) = a.get("d") {
            let mut points: Vec<(f32, f32)> = trace(d)?.into_iter().flatten().collect();
            points.dedup();
            if points.len() < 3 {
                return Err("the path d encloses nothing".to_owned());
            }
            return Ok(Region::Polygon(points));
        }
        if let (Some(cx), Some(cy)) = (a.number("cx")?, a.number("cy")?) {
            let r = a.number("r")?;
            let rx = a.number("rx")?.or(r).ok_or("give r, or rx and ry")?;
            let ry = a.number("ry")?.or(r).unwrap_or(rx);
            return Ok(Region::Ellipse(cx, cy, rx, ry));
        }
        Ok(Region::Rect(
            a.number_or("x", 0.0)?,
            a.number_or("y", 0.0)?,
            a.number("width")?
                .ok_or("give width and height, cx cy and r, or a closed path d")?,
            a.number("height")?.ok_or("give height")?,
        ))
    }

    fn bounds(&self) -> (f32, f32, f32, f32) {
        match self {
            Region::Rect(x, y, w, h) => (*x, *y, *w, *h),
            Region::Ellipse(cx, cy, rx, ry) => (cx - rx, cy - ry, 2.0 * rx, 2.0 * ry),
            Region::Polygon(p) => {
                let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for (x, y) in p {
                    (x0, y0, x1, y1) = (x0.min(*x), y0.min(*y), x1.max(*x), y1.max(*y));
                }
                (x0, y0, x1 - x0, y1 - y0)
            }
        }
    }

    fn holds(&self, (x, y): (f32, f32)) -> bool {
        match self {
            Region::Rect(..) => true,
            Region::Ellipse(cx, cy, rx, ry) => {
                ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
            }
            Region::Polygon(p) => {
                let mut inside = false;
                let mut j = p.len() - 1;
                for i in 0..p.len() {
                    let ((xi, yi), (xj, yj)) = (p[i], p[j]);
                    if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                        inside = !inside;
                    }
                    j = i;
                }
                inside
            }
        }
    }
}

/// Shapes strewn over a region without overlapping, as cells in a tissue
/// or a smear.
fn scatter(a: &Attributes, whole: &str) -> Result<String, String> {
    let shapes = shapes(a, whole)?;
    let region = Region::read(a)?;
    let n = a.number("n")?.map_or(300, |n| n.max(0.0) as usize);
    let scale = a.range("scale", (1.0, 1.0))?;
    let rotate = a.range("rotate", (0.0, 360.0))?;
    let gap = a.number_or("gap", 1.0)?;
    let mut dice = Dice::new(a.number_or("seed", 1.0)?);
    let (bx, by, bw, bh) = region.bounds();
    let mut placed: Vec<((f32, f32), f32)> = Vec::new();
    let mut out = String::new();
    for _ in 0..n * 60 + 200 {
        if placed.len() >= n {
            break;
        }
        let p = (bx + dice.roll() * bw, by + dice.roll() * bh);
        let (shape, (w, h)) = shapes[(dice.roll() * shapes.len() as f32) as usize % shapes.len()];
        let s = dice.between(scale);
        let r = (w + h) / 4.0 * s * gap;
        if !region.holds(p)
            || placed
                .iter()
                .any(|(q, rq)| (q.0 - p.0).hypot(q.1 - p.1) < r + rq)
        {
            continue;
        }
        placed.push((p, r));
        place(&mut out, shape, p, dice.between(rotate), s);
    }
    Ok(group(a, &out))
}

/// Shapes set around a circle or an ellipse, as cells around a lumen.
/// face="in" turns each shape's top towards the centre, face="out" away.
fn ring(a: &Attributes, whole: &str) -> Result<String, String> {
    let shapes = shapes(a, whole)?;
    let (cx, cy) = (a.number_or("cx", 0.0)?, a.number_or("cy", 0.0)?);
    let rx = a.number("rx")?.or(a.number("r")?).ok_or("give r")?;
    let ry = a.number("ry")?.unwrap_or(rx);
    let s = a.number_or("scale", 1.0)?;
    let width = shapes[0].1.0 * s;
    let around =
        std::f32::consts::PI * (3.0 * (rx + ry) - ((3.0 * rx + ry) * (rx + 3.0 * ry)).sqrt());
    let n = a
        .number("n")?
        .map_or((around / width.max(1.0)).floor().max(1.0) as usize, |n| {
            n.max(1.0) as usize
        });
    let start = a.number_or("start", -90.0)?;
    let face = a.get("face").unwrap_or("none");
    let mut out = String::new();
    for k in 0..n {
        let phi = (start + k as f32 * 360.0 / n as f32).to_radians();
        let p = (cx + rx * phi.cos(), cy + ry * phi.sin());
        let turn = match face {
            "in" => phi.to_degrees() - 90.0,
            "out" | "along" => phi.to_degrees() + 90.0,
            _ => a.number_or("rotate", 0.0)?,
        };
        place(&mut out, shapes[k % shapes.len()].0, p, turn, s);
    }
    Ok(group(a, &out))
}

/// Shapes in a line, each `dx`, `dy` from the one before.
fn row(a: &Attributes, whole: &str) -> Result<String, String> {
    let shapes = shapes(a, whole)?;
    let s = a.number_or("scale", 1.0)?;
    let n = a.number_or("n", 5.0)?.max(1.0) as usize;
    let (x, y) = (a.number_or("x", 0.0)?, a.number_or("y", 0.0)?);
    let dx = a.number("dx")?.unwrap_or(shapes[0].1.0 * s * 1.05);
    let dy = a.number_or("dy", 0.0)?;
    let rotate = a.number_or("rotate", 0.0)?;
    let mut out = String::new();
    for k in 0..n {
        let p = (x + dx * k as f32, y + dy * k as f32);
        place(&mut out, shapes[k % shapes.len()].0, p, rotate, s);
    }
    Ok(group(a, &out))
}

/// Shapes in rows and columns, as a sheet of cells; stagger="true" sets
/// every other row half a step over, jitter="0.2" shakes them.
fn grid(a: &Attributes, whole: &str) -> Result<String, String> {
    let shapes = shapes(a, whole)?;
    let s = a.number_or("scale", 1.0)?;
    let cols = a.number_or("cols", 6.0)?.max(1.0) as usize;
    let rows = a.number_or("rows", 4.0)?.max(1.0) as usize;
    let (x, y) = (a.number_or("x", 0.0)?, a.number_or("y", 0.0)?);
    let dx = a.number("dx")?.unwrap_or(shapes[0].1.0 * s * 1.05);
    let dy = a.number("dy")?.unwrap_or(shapes[0].1.1 * s * 1.05);
    let stagger = a.get("stagger") == Some("true");
    let jitter = a.number_or("jitter", 0.0)?;
    let rotate = a.range("rotate", (0.0, 0.0))?;
    let mut dice = Dice::new(a.number_or("seed", 1.0)?);
    let mut out = String::new();
    let mut k = 0;
    for r in 0..rows {
        for c in 0..cols {
            let shift = if stagger && r % 2 == 1 { dx / 2.0 } else { 0.0 };
            let p = (
                x + c as f32 * dx + shift + (dice.roll() - 0.5) * jitter * dx,
                y + r as f32 * dy + (dice.roll() - 0.5) * jitter * dy,
            );
            place(
                &mut out,
                shapes[k % shapes.len()].0,
                p,
                dice.between(rotate),
                s,
            );
            k += 1;
        }
    }
    Ok(group(a, &out))
}

/// Shapes along a path, each turned with it (face="none" to keep them
/// upright), as receptors along a membrane or cells along a route.
fn along(a: &Attributes, whole: &str) -> Result<String, String> {
    let shapes = shapes(a, whole)?;
    let track = Track::new(a.get("d").ok_or("give the path d")?)?;
    let s = a.number_or("scale", 1.0)?;
    let offset = a.number_or("offset", 0.0)?;
    let length = track.length();
    let n = match (a.number("n")?, a.number("spacing")?) {
        (Some(n), _) => n.max(1.0) as usize,
        (None, Some(spacing)) => (length / spacing.max(1.0)).floor() as usize + 1,
        (None, None) => (length / (shapes[0].1.0 * s * 1.1).max(1.0)).floor() as usize + 1,
    };
    let upright = a.get("face") == Some("none");
    let mut out = String::new();
    for k in 0..n {
        let at = if n == 1 {
            length / 2.0
        } else {
            length * k as f32 / (n - 1) as f32
        };
        let ((x, y), angle) = track.at(at);
        let (nx, ny) = (angle.sin(), -angle.cos());
        let p = (x + nx * offset, y + ny * offset);
        let turn = if upright { 0.0 } else { angle.to_degrees() };
        place(&mut out, shapes[k % shapes.len()].0, p, turn, s);
    }
    Ok(group(a, &out))
}

/// A lipid bilayer along a path, its heads out and its tails meeting in
/// the middle; the outside is on the left of the path's direction, up for
/// a path drawn left to right.
fn bilayer(a: &Attributes) -> Result<String, String> {
    let track = Track::new(a.get("d").ok_or("give the path d")?)?;
    let thick = a.number_or("thickness", 22.0)?;
    let spacing = a.number_or("spacing", 7.0)?.max(2.0);
    let head = (spacing * 0.42).min(thick * 0.16);
    let mut heads = String::new();
    let mut tails = String::new();
    let n = (track.length() / spacing).floor() as usize + 1;
    for k in 0..n {
        let ((x, y), angle) = track.at(k as f32 * spacing);
        let (nx, ny) = (angle.sin(), -angle.cos());
        let (tx, ty) = (angle.cos(), angle.sin());
        for side in [1.0f32, -1.0] {
            let h = thick / 2.0 - head;
            let (hx, hy) = (x + nx * h * side, y + ny * h * side);
            let _ = write!(
                heads,
                "<circle cx=\"{hx:.1}\" cy=\"{hy:.1}\" r=\"{head:.1}\"/>"
            );
            for splay in [-0.22f32, 0.22] {
                let reach = h - head * 0.2;
                let (ex, ey) = (
                    hx - nx * reach * side + tx * splay * spacing,
                    hy - ny * reach * side + ty * splay * spacing,
                );
                let _ = write!(tails, "M{hx:.1} {hy:.1}L{ex:.1} {ey:.1}");
            }
        }
    }
    Ok(group(
        a,
        &format!(
            "<path d=\"{tails}\" fill=\"none\" stroke=\"#d7a86e\" stroke-width=\"{:.2}\" stroke-linecap=\"round\"/><g fill=\"#f3b65f\" stroke=\"#b8742c\" stroke-width=\"0.6\">{heads}</g>",
            (head * 0.45).max(0.6)
        ),
    ))
}

/// A double helix along a path: two strands turning round it, and the
/// base pairs between them.
fn helix(a: &Attributes) -> Result<String, String> {
    let track = Track::new(a.get("d").ok_or("give the path d")?)?;
    let width = a.number_or("width", 36.0)?;
    let turn = a.number_or("turn", 68.0)?.max(8.0);
    let length = track.length();
    let strand = |phase: f32| -> String {
        let mut d = String::new();
        let steps = (length / 2.0).ceil().max(2.0) as usize;
        for k in 0..=steps {
            let s = length * k as f32 / steps as f32;
            let ((x, y), angle) = track.at(s);
            let off = width / 2.0 * (std::f32::consts::TAU * s / turn + phase).sin();
            let (px, py) = (x + angle.sin() * off, y - angle.cos() * off);
            let _ = write!(d, "{}{px:.1} {py:.1}", if k == 0 { "M" } else { "L" });
        }
        d
    };
    let mut rungs = String::new();
    let n = (length / (turn / 10.0)).floor() as usize;
    for k in 0..=n {
        let s = k as f32 * turn / 10.0;
        let ((x, y), angle) = track.at(s);
        let phase = std::f32::consts::TAU * s / turn;
        let (o1, o2) = (
            width / 2.0 * phase.sin(),
            width / 2.0 * (phase + std::f32::consts::PI).sin(),
        );
        if (o1 - o2).abs() < width * 0.15 {
            continue;
        }
        let (nx, ny) = (angle.sin(), -angle.cos());
        // Bent a hair, so that the house's stroke shows on a straight rung.
        let _ = write!(
            rungs,
            "M{:.1} {:.1}Q{:.1} {:.1} {:.1} {:.1}",
            x + nx * o1,
            y + ny * o1,
            x + nx * (o1 + o2) / 2.0 + angle.cos() * 0.6,
            y + ny * (o1 + o2) / 2.0 + angle.sin() * 0.6,
            x + nx * o2,
            y + ny * o2
        );
    }
    Ok(group(
        a,
        &format!(
            "<path d=\"{}\" class=\"dna-strand-2\"/><path d=\"{rungs}\" class=\"dna-rung\"/><path d=\"{}\" class=\"dna-strand\"/>",
            strand(std::f32::consts::PI),
            strand(0.0)
        ),
    ))
}

/// A template set in the drawing: centred at x, y (the drawing's centre
/// when not given), sized by width, height or scale, turned by rotate,
/// mirrored by flip="x"; prefix="a" renames its parts a_..., for two of
/// the same in one drawing; hide="id id" makes parts transparent, to show
/// them later with changes; highlight="id id" rings parts in yellow.
fn insert(a: &Attributes, size: (f32, f32)) -> Result<String, String> {
    let name = a.get("name").ok_or("which template? name=\"heart\"")?;
    let t = templates::find(name).ok_or_else(|| {
        format!(
            "no template {name}; the templates are {}",
            templates::names().join(", ")
        )
    })?;
    let [vx, vy, vw, vh] = t.view;
    let scale = match (a.number("width")?, a.number("height")?, a.number("scale")?) {
        (Some(w), _, _) => w / vw,
        (None, Some(h), _) => h / vh,
        (None, None, Some(s)) => s,
        _ => 1.0,
    };
    let x = a.number_or("x", size.0 / 2.0)?;
    let y = a.number_or("y", size.1 / 2.0)?;
    let rotate = a.number_or("rotate", 0.0)?;
    let flip = if a.get("flip") == Some("x") {
        -1.0
    } else {
        1.0
    };
    let mut body = t.body.clone();
    let prefix = a.get("prefix").filter(|p| !p.is_empty());
    let named = |id: &str| prefix.map_or(id.to_owned(), |p| format!("{p}_{id}"));
    if let Some(p) = prefix {
        body = prefix_ids(&body, p);
    }
    if let Some(hide) = a.get("hide") {
        let changes: Vec<(String, String, String)> = hide
            .split_whitespace()
            .map(|id| (named(id), "opacity".to_owned(), "0".to_owned()))
            .collect();
        body = super::changed(&body, &changes)?;
    }
    if let Some(list) = a.get("highlight") {
        let ids: Vec<String> = list
            .split_whitespace()
            .map(|id| format!("#{}", named(id)))
            .collect();
        if !ids.is_empty() {
            body.push_str(&format!(
                "<style>{} {{ stroke: rgb(255,190,0); stroke-width: 6; }}</style>",
                ids.join(", ")
            ));
        }
    }
    let placed = format!(
        "translate({x} {y}) rotate({rotate}) scale({} {scale}) translate({} {})",
        scale * flip,
        -(vx + vw / 2.0),
        -(vy + vh / 2.0)
    );
    Ok(format!("<g{}>{body}</g>", a.handed(Some(&placed))))
}

/// Every id a fragment defines renamed `prefix_id`, and every reference to
/// one of them with it.
fn prefix_ids(body: &str, prefix: &str) -> String {
    let defined = ids_defined(body);
    let mut out = String::with_capacity(body.len() + defined.len() * (prefix.len() + 1));
    let mut rest = body;
    loop {
        let next_id = rest.find(" id=\"").map(|i| (i, 5));
        let next_ref = rest.find('#').map(|i| (i, 1));
        let Some((at, skip)) = [next_id, next_ref].into_iter().flatten().min() else {
            break;
        };
        out.push_str(&rest[..at + skip]);
        rest = &rest[at + skip..];
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':'))
            .unwrap_or(rest.len());
        if defined.iter().any(|d| d == &rest[..len]) {
            out.push_str(prefix);
            out.push('_');
        }
        out.push_str(&rest[..len]);
        rest = &rest[len..];
    }
    out.push_str(rest);
    out
}

fn ids_defined(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for quote in ['"', '\''] {
        let needle = format!(" id={quote}");
        let mut rest = text;
        while let Some(i) = rest.find(&needle) {
            rest = &rest[i + needle.len()..];
            if let Some(end) = rest.find(quote) {
                ids.push(rest[..end].to_owned());
            }
        }
    }
    ids
}

/// What a drawing says of its parts: the id and data-what of every element
/// that has both.
pub fn descriptions(svg: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(i) = rest.find("data-what=\"") {
        let start = rest[..i].rfind('<').unwrap_or(0);
        let end = i + rest[i..].find('>').unwrap_or(rest.len() - i);
        let tag = &rest[start..end];
        let what = &rest[i + 11..];
        let what = &what[..what.find('"').unwrap_or(what.len())];
        if let Some(j) = tag.find(" id=\"") {
            let id = &tag[j + 5..];
            if let Some(close) = id.find('"') {
                out.push((id[..close].to_owned(), what.to_owned()));
            }
        }
        rest = &rest[end.max(i + 11)..];
    }
    out
}

/// The lines of a path, each subpath as points: curves are cut into short
/// straight pieces.
pub fn trace(d: &str) -> Result<Vec<Vec<(f32, f32)>>, String> {
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"><path d=\"{d}\" fill=\"none\" stroke=\"black\"/></svg>"
    );
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
        .map_err(|e| format!("the path does not read: {e}"))?;
    let path = tree
        .root()
        .children()
        .iter()
        .find_map(|n| match n {
            usvg::Node::Path(p) => Some(p.data().clone()),
            _ => None,
        })
        .ok_or("the path d draws nothing")?;
    let mut lines: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut at = (0.0f32, 0.0f32);
    use usvg::tiny_skia_path::PathSegment;
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                at = (p.x, p.y);
                lines.push(vec![at]);
            }
            PathSegment::LineTo(p) => {
                at = (p.x, p.y);
                if let Some(l) = lines.last_mut() {
                    l.push(at);
                }
            }
            PathSegment::QuadTo(c, p) => {
                let from = at;
                for k in 1..=16 {
                    let t = k as f32 / 16.0;
                    let u = 1.0 - t;
                    let q = (
                        u * u * from.0 + 2.0 * u * t * c.x + t * t * p.x,
                        u * u * from.1 + 2.0 * u * t * c.y + t * t * p.y,
                    );
                    if let Some(l) = lines.last_mut() {
                        l.push(q);
                    }
                }
                at = (p.x, p.y);
            }
            PathSegment::CubicTo(c1, c2, p) => {
                let from = at;
                for k in 1..=24 {
                    let t = k as f32 / 24.0;
                    let u = 1.0 - t;
                    let q = (
                        u * u * u * from.0
                            + 3.0 * u * u * t * c1.x
                            + 3.0 * u * t * t * c2.x
                            + t * t * t * p.x,
                        u * u * u * from.1
                            + 3.0 * u * u * t * c1.y
                            + 3.0 * u * t * t * c2.y
                            + t * t * t * p.y,
                    );
                    if let Some(l) = lines.last_mut() {
                        l.push(q);
                    }
                }
                at = (p.x, p.y);
            }
            PathSegment::Close => {
                if let Some(l) = lines.last_mut()
                    && let Some(&first) = l.first()
                {
                    l.push(first);
                    at = first;
                }
            }
        }
    }
    lines.retain(|l| l.len() > 1);
    if lines.is_empty() {
        return Err("the path d draws nothing".to_owned());
    }
    Ok(lines)
}

/// A path walked by its length.
struct Track {
    points: Vec<(f32, f32)>,
    /// The length walked to reach each point.
    walked: Vec<f32>,
}

impl Track {
    fn new(d: &str) -> Result<Self, String> {
        let points: Vec<(f32, f32)> = trace(d)?.into_iter().flatten().collect();
        let mut walked = vec![0.0];
        for w in points.windows(2) {
            let last = *walked.last().unwrap_or(&0.0);
            walked.push(last + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1));
        }
        if walked.last().copied().unwrap_or(0.0) <= 0.0 {
            return Err("the path d has no length".to_owned());
        }
        Ok(Self { points, walked })
    }

    fn length(&self) -> f32 {
        self.walked.last().copied().unwrap_or(0.0)
    }

    /// The point a length `s` along, and the direction there, in radians.
    fn at(&self, s: f32) -> ((f32, f32), f32) {
        let s = s.clamp(0.0, self.length());
        let k = self
            .walked
            .partition_point(|w| *w < s)
            .clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[k - 1], self.points[k]);
        let span = (self.walked[k] - self.walked[k - 1]).max(1e-6);
        let t = ((s - self.walked[k - 1]) / span).clamp(0.0, 1.0);
        (
            (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
            (b.1 - a.1).atan2(b.0 - a.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes() {
        let a = Attributes::parse(" shape=\"rbc\" n='3' scale=\"0.8 1.2\" ").unwrap();
        assert_eq!(a.get("shape"), Some("rbc"));
        assert_eq!(a.number("n").unwrap(), Some(3.0));
        assert_eq!(a.range("scale", (1.0, 1.0)).unwrap(), (0.8, 1.2));
        assert!(Attributes::parse("n=3").is_err());
    }

    #[test]
    fn sizes() {
        assert_eq!(shape_size("rbc"), Some((24.0, 24.0)));
        assert_eq!(shape_size("melanocyte"), Some((145.0, 95.0)));
    }

    #[test]
    fn scatter_stays_inside_and_apart() {
        let out = expand(
            "<scatter id=\"blood\" shape=\"rbc\" n=\"30\" cx=\"100\" cy=\"100\" r=\"80\"/>",
            (200.0, 200.0),
        )
        .unwrap();
        assert!(out.starts_with("<g id=\"blood\">"));
        let n = out.matches("<use").count();
        assert!(n > 10 && n <= 30, "{n} placed");
        // The same each time.
        let again = expand(
            "<scatter id=\"blood\" shape=\"rbc\" n=\"30\" cx=\"100\" cy=\"100\" r=\"80\"/>",
            (200.0, 200.0),
        )
        .unwrap();
        assert_eq!(out, again);
    }

    #[test]
    fn unknown_shapes_are_told() {
        let e = expand("<row shape=\"unicorn\" n=\"3\"/>", (100.0, 100.0)).unwrap_err();
        assert!(e.contains("no shape unicorn"), "{e}");
        let mine = "<defs><symbol id=\"mine\"><circle r=\"5\"/></symbol></defs><row shape=\"mine\" n=\"3\"/>";
        assert_eq!(
            expand(mine, (100.0, 100.0))
                .unwrap()
                .matches("#mine")
                .count(),
            3
        );
    }

    #[test]
    fn ring_faces_the_centre() {
        let out = expand(
            "<ring shape=\"epithelial-cell\" n=\"4\" cx=\"0\" cy=\"0\" r=\"100\" face=\"in\"/>",
            (100.0, 100.0),
        )
        .unwrap();
        // The first at the top, its top turned down to the centre.
        assert!(
            out.contains("translate(0.0 -100.0) rotate(-180.0)"),
            "{out}"
        );
    }

    #[test]
    fn paths_are_walked() {
        let t = Track::new("M0 0 L100 0 L100 100").unwrap();
        assert!((t.length() - 200.0).abs() < 0.01);
        let ((x, y), angle) = t.at(150.0);
        assert!((x - 100.0).abs() < 0.01 && (y - 50.0).abs() < 0.01);
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 0.01);
        let curve = Track::new("M0 0 C 0 100, 100 100, 100 0").unwrap();
        assert!(curve.length() > 150.0);
    }

    #[test]
    fn bilayer_and_helix_draw() {
        let out = expand(
            "<bilayer id=\"m\" d=\"M0 50 L300 50\"/><helix d=\"M0 150 Q150 100 300 150\"/>",
            (300.0, 200.0),
        )
        .unwrap();
        assert!(out.matches("<circle").count() > 60);
        assert!(out.contains("dna-strand-2") && out.contains("dna-rung"));
    }

    #[test]
    fn ids_take_a_prefix() {
        let body = "<defs><linearGradient id=\"t-g\"/></defs><path id=\"left_ventricle\" fill=\"url(#t-g)\"/><use href=\"#rbc\"/>";
        let out = prefix_ids(body, "a");
        assert!(out.contains("id=\"a_t-g\"") && out.contains("url(#a_t-g)"));
        assert!(out.contains("id=\"a_left_ventricle\""));
        assert!(
            out.contains("href=\"#rbc\""),
            "house shapes keep their names: {out}"
        );
    }

    #[test]
    fn parts_say_what_they_are() {
        let d = descriptions(
            "<g id=\"left_ventricle\" class=\"heart\" data-what=\"the left ventricle\"><path/></g><path data-what=\"x\"/>",
        );
        assert_eq!(
            d,
            vec![("left_ventricle".to_owned(), "the left ventricle".to_owned())]
        );
    }
}
