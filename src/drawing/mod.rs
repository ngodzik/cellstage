//! Drawings the agent writes in SVG, for whatever the other panels cannot
//! show: an embryo, an organ, blood, a pathway, a timeline. Each is
//! wrapped in the house style, so that every drawing of a story speaks the
//! same colours and can use the same shapes; rendered with resvg; moved
//! smoothly from one frame to the next; and its named parts are found again
//! for notes and clicks.
//!
//! Two frames move into each other number by number when they are the same
//! drawing with numbers changed, positions, sizes, colours, opacities; when
//! they are not, one fades into the other.

use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};

/// What a class looks like.
const HOUSE_CSS: &str = include_str!("house.css");
/// Gradients, patterns, filters, markers and shapes.
const HOUSE_SVG: &str = include_str!("house.svg");

/// The size a drawing is written in when it does not say.
pub const DEFAULT_SIZE: (f32, f32) = (1600.0, 1000.0);

/// The ready-made shapes: id, size, what it is, and how much larger the
/// kit's picture draws it.
pub const SHAPES: [(&str, &str, &str, f32); 26] = [
    (
        "cell",
        "100 across",
        "a generic cell: membrane, cytoplasm, nucleus, nucleolus, mitochondria, ER, Golgi",
        1.3,
    ),
    (
        "keratinocyte",
        "70 by 58",
        "a skin cell, with its cap of melanin over the nucleus",
        1.6,
    ),
    (
        "melanocyte",
        "145 by 95",
        "a pigment cell, its dendrites carrying melanosomes",
        1.0,
    ),
    (
        "melanoblast",
        "125 by 40, moving right",
        "a migrating neural crest cell on its way to the skin",
        1.2,
    ),
    (
        "neuron",
        "350 by 140, axon to the right",
        "a neuron: dendrites, body, myelinated axon, terminals",
        0.6,
    ),
    (
        "fibroblast",
        "116 by 30",
        "a spindle cell of the dermis or any connective tissue",
        1.4,
    ),
    ("rbc", "24 across", "a red blood cell, seen face on", 4.0),
    (
        "lymphocyte",
        "28 across",
        "a small lymphocyte: a round nucleus filling it",
        3.6,
    ),
    (
        "neutrophil",
        "36 across",
        "a neutrophil: lobed nucleus, granules",
        3.0,
    ),
    (
        "monocyte",
        "44 across",
        "a monocyte: kidney-shaped nucleus",
        2.6,
    ),
    ("platelet", "10 by 6", "a platelet", 7.0),
    (
        "blast",
        "40 across",
        "a blast: a large immature cell, fine chromatin, nucleoli; leukaemias",
        2.8,
    ),
    (
        "cancer-cell",
        "115 across",
        "a tumour cell: irregular outline, large irregular nucleus, nucleoli",
        1.1,
    ),
    ("dividing-cell", "124 by 76", "a cell dividing in two", 1.1),
    (
        "apoptotic-cell",
        "80 across",
        "a cell in apoptosis: shrunk, blebbing, its nucleus in pieces",
        1.6,
    ),
    (
        "mitochondrion",
        "62 by 28",
        "a mitochondrion with its cristae",
        2.2,
    ),
    (
        "nucleus",
        "60 across",
        "a nucleus: envelope, chromatin, nucleolus",
        2.2,
    ),
    (
        "dna",
        "240 by 36, horizontal",
        "a stretch of double helix",
        0.8,
    ),
    (
        "receptor",
        "68 by 80, membrane at y 0",
        "a receptor dimer across a membrane, outside up",
        1.8,
    ),
    (
        "vessel",
        "80 across",
        "a capillary in cross-section, two red cells inside",
        1.8,
    ),
    (
        "epithelial-cell",
        "34 by 62",
        "a columnar epithelial cell with microvilli; tile them side by side",
        2.2,
    ),
    ("virus", "56 across", "a virus with its spikes", 2.4),
    (
        "bacterium",
        "126 by 26",
        "a rod bacterium with a flagellum",
        1.2,
    ),
    ("antibody", "42 by 50", "an antibody, a Y", 2.4),
    (
        "macrophage",
        "125 by 105",
        "a macrophage, with what it swallowed",
        1.1,
    ),
    (
        "adipocyte",
        "80 across",
        "a fat cell: one big droplet, its nucleus pressed to the side",
        1.6,
    ),
];

/// The house classes, by what they are for.
pub const CLASSES: [(&str, &str); 10] = [
    (
        "text",
        "label (24px), title (34px bold), small (18px), light (white), muted (grey)",
    ),
    (
        "lines",
        "arrow, arrow-red, arrow-white, inhibit (a flat bar at the end), line, dashed, path-route (a dotted green route)",
    ),
    (
        "effects",
        "shadow, glow, blur, faint (35% opacity), halo (a soft yellow light behind something), outline (yellow ring)",
    ),
    (
        "a cell",
        "cell, membrane, membrane-band, cytoplasm, nucleus, nucleus-dark, nucleolus, mito, er, golgi, vesicle, lysosome, melanin, melanosome, dying, tumour-cell, lipid",
    ),
    (
        "cells",
        "melanocyte-body, dendrite, crest-cell, neuron-body, neurite, myelin, rbc, wbc, platelet, blast, macrophage",
    ),
    (
        "embryo, in the usual colours of the germ layers",
        "ectoderm (blue), neural (indigo: neural plate and tube), neural-crest (green), mesoderm (red), somite, notochord (pink), endoderm (yellow), yolk, amnion",
    ),
    (
        "tissues",
        "epidermis, dermis (with collagen), dermis-plain, fat, muscle (striated), bone, cartilage, basement, artery, vein, vessel-wall, blood, lymph, nerve",
    ),
    (
        "organs",
        "grey-matter, white-matter, csf, liver, lung, heart, kidney, gut, marrow, skin-surface",
    ),
    (
        "molecules",
        "protein, receptor, ligand, dna-strand, dna-strand-2, dna-rung, antibody",
    ),
    ("microbes", "virus, bacterium"),
];

/// The kit in words, for the agent.
pub fn kit() -> String {
    let mut out = String::from(
        "Write SVG elements; the window wraps them in an <svg> of 1600 by 1000 units (or the view_box you give), on a light page, with the house style. Text in the drawing is fine; notes from annotate are better for pointing during the story.\n\nClasses (class=\"...\", several at once are fine):\n",
    );
    for (what, list) in CLASSES {
        out.push_str(&format!("- {what}: {list}\n"));
    }
    out.push_str("\nShapes, each drawn around 0,0: <use href=\"#cell\" x=\"400\" y=\"300\"/>, or with transform=\"translate(400 300) scale(1.5) rotate(20)\" to size and turn it.\n");
    for (id, size, what, _) in SHAPES {
        out.push_str(&format!("- #{id} ({size}): {what}\n"));
    }
    out.push_str(
        "\nMarkers for your own paths: marker-end=\"url(#h-arrow)\", url(#h-arrow-red), url(#h-bar). Gradients you may fill with: url(#h-cyto), url(#h-nucleus), url(#h-crest), url(#h-ectoderm), url(#h-mesoderm), url(#h-endoderm) and the others named h-<class>.\n\nTwo rules of SVG that surprise: a class's style wins over attributes such as fill=\"...\" or stroke=\"...\" on the same element, so to change a class's colour write style=\"fill:...\"; and a gradient or a filter (the house classes use them) on a perfectly straight horizontal or vertical line draws nothing, so bend such a line a little or give it a plain colour.\n\nParts: give an id to everything you will point at or the user may click, such as id=\"neural_crest\"; annotate with on=\"neural_crest\" puts a note on it, and a click on it tells you its id. Describe ids in parts to say more than the id.\n\nMoving: the quickest is changes, each further frame given as what changes from the one before: [{\"crest_1\": {\"transform\": \"translate(420 380)\"}, \"tube\": {\"d\": \"M ...\"}}, ...]; change attributes the first frame already has. Or give frames, the same drawing with numbers changed (positions, sizes, rotations, colours, opacities), the same elements in the same order, the same path commands; the window moves smoothly from one to the next. Something that appears is in every frame, with opacity 0 before. Frames that differ otherwise fade into each other.",
    );
    out
}

/// The system's fonts, loaded once, on a thread of their own at start.
static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();

fn fonts() -> Arc<usvg::fontdb::Database> {
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            db.set_sans_serif_family("Noto Sans");
            Arc::new(db)
        })
        .clone()
}

/// Begin loading the fonts, which takes most of a second, before the first
/// drawing needs them.
pub fn warm() {
    std::thread::spawn(|| {
        fonts();
    });
}

/// What the agent wrote, made ready: its size, and its elements with their
/// colours written as numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub size: (f32, f32),
    pub body: String,
}

impl Sheet {
    /// Read the agent's SVG: a whole `<svg>` document, whose viewBox gives
    /// the size, or bare elements, drawn in `size`.
    pub fn read(input: &str, size: Option<(f32, f32)>) -> Self {
        let mut text = input.trim();
        if text.starts_with("<?xml")
            && let Some(end) = text.find("?>")
        {
            text = text[end + 2..].trim_start();
        }
        let mut size = size.unwrap_or(DEFAULT_SIZE);
        let mut body = text.to_owned();
        if text.starts_with("<svg")
            && let Some(open) = text.find('>')
        {
            let tag = &text[..open];
            let close = text.rfind("</svg>").unwrap_or(text.len());
            body = text[open + 1..close.max(open + 1)].to_owned();
            if let Some(at) = tag.find("viewBox=") {
                let rest = &tag[at + 9..];
                let numbers: Vec<f32> = rest
                    .split(['"', '\''])
                    .next()
                    .unwrap_or("")
                    .split([' ', ','])
                    .filter_map(|n| n.trim().parse().ok())
                    .collect();
                if let [x, y, w, h] = numbers[..] {
                    size = (w, h);
                    if x != 0.0 || y != 0.0 {
                        body = format!("<g transform=\"translate({} {})\">{body}</g>", -x, -y);
                    }
                }
            }
        }
        Self {
            size,
            body: hex_to_rgb(&body),
        }
    }

    /// The whole document: the house style, then the drawing.
    pub fn document(&self) -> String {
        let (w, h) = self.size;
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><style>{HOUSE_CSS}</style><defs>{HOUSE_SVG}</defs>{}</svg>",
            self.body
        )
    }
}

/// Write `#rrggbb` and `#rgb` colours as `rgb(r,g,b)`, so that a colour
/// that changes between frames is numbers that can move. References,
/// `href="#id"` and `url(#id)`, are left alone.
fn hex_to_rgb(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('#') {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let reference = out.ends_with("href=\"")
            || out.ends_with("href='")
            || out.ends_with("url(")
            || out.ends_with("url('")
            || out.ends_with("url(\"");
        let digits = after[1..].bytes().take_while(u8::is_ascii_hexdigit).count();
        let next = after[1 + digits..].chars().next();
        let ends = next.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
        if !reference && ends && (digits == 6 || digits == 3) {
            let hex = &after[1..1 + digits];
            let channel = |i: usize| -> u8 {
                if digits == 6 {
                    u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0)
                } else {
                    u8::from_str_radix(&hex[i..i + 1], 16).unwrap_or(0) * 17
                }
            };
            out.push_str(&format!(
                "rgb({},{},{})",
                channel(0),
                channel(1),
                channel(2)
            ));
            rest = &after[1 + digits..];
        } else {
            out.push('#');
            rest = &after[1..];
        }
    }
    out.push_str(rest);
    out
}

/// A document cut into the text between its numbers, and its numbers.
#[derive(Debug, Clone, PartialEq)]
struct Tokens {
    /// One more piece of text than there are numbers.
    text: Vec<String>,
    numbers: Vec<String>,
}

impl Tokens {
    fn cut(s: &str) -> Self {
        let b = s.as_bytes();
        let mut text = Vec::new();
        let mut numbers = Vec::new();
        let (mut start, mut i) = (0, 0);
        while i < b.len() {
            match number_at(b, i) {
                Some(end) => {
                    text.push(s[start..i].to_owned());
                    numbers.push(s[i..end].to_owned());
                    start = end;
                    i = end;
                }
                None => i += 1,
            }
        }
        text.push(s[start..].to_owned());
        Self { text, numbers }
    }

    /// The document a fraction `t` of the way from `self` to `other`, if
    /// they differ only by their numbers.
    fn between(&self, other: &Self, t: f32) -> Option<String> {
        if self.text != other.text {
            return None;
        }
        let mut out = String::new();
        for (k, piece) in self.text.iter().enumerate() {
            out.push_str(piece);
            let (Some(a), Some(b)) = (self.numbers.get(k), other.numbers.get(k)) else {
                continue;
            };
            if a == b {
                out.push_str(a);
            } else {
                let (x, y) = (a.parse::<f32>().ok()?, b.parse::<f32>().ok()?);
                let v = x + (y - x) * t;
                let mut n = format!("{v:.3}");
                while n.ends_with('0') {
                    n.pop();
                }
                if n.ends_with('.') {
                    n.pop();
                }
                out.push_str(&n);
            }
        }
        Some(out)
    }
}

/// Where a number that starts at `i` ends, if one does.
fn number_at(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    if matches!(b[j], b'+' | b'-') {
        j += 1;
    }
    let whole = b[j..].iter().take_while(|c| c.is_ascii_digit()).count();
    j += whole;
    let mut fraction = 0;
    if b.get(j) == Some(&b'.') && b.get(j + 1).is_some_and(u8::is_ascii_digit) {
        fraction = b[j + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        j += 1 + fraction;
    }
    if whole + fraction == 0 {
        return None;
    }
    if matches!(b.get(j), Some(b'e' | b'E')) {
        let k = j + 1 + usize::from(matches!(b.get(j + 1), Some(b'+' | b'-')));
        if b.get(k).is_some_and(u8::is_ascii_digit) {
            j = k + b[k..].iter().take_while(|c| c.is_ascii_digit()).count();
        }
    }
    Some(j)
}

/// A drawing's frames and how it moves through them.
#[derive(Debug, Clone)]
pub struct Film {
    frames: Vec<String>,
    tokens: Vec<Tokens>,
    /// Seconds each frame is held before moving on, and seconds the move
    /// takes.
    pub hold: f32,
    pub seconds: f32,
    pub looping: bool,
    /// Seconds the first frame is held: shorter when the film begins by
    /// moving from what the step before showed.
    lead: f32,
}

/// What to render at a moment of a film.
pub enum Picture<'a> {
    Still(&'a str),
    Morph(String),
    /// Two frames that cannot move into each other, the second showing
    /// through by a fraction.
    Blend(&'a str, &'a str, f32),
}

impl Film {
    pub fn new(frames: Vec<String>, hold: f32, seconds: f32, looping: bool) -> Self {
        let tokens = frames.iter().map(|f| Tokens::cut(f)).collect();
        let hold = hold.max(0.0);
        Self {
            frames,
            tokens,
            hold,
            seconds: seconds.max(0.05),
            looping,
            lead: hold,
        }
    }

    pub fn frames(&self) -> &[String] {
        &self.frames
    }

    /// How long frame `k` is held before the move to the next.
    fn held(&self, k: usize) -> f32 {
        if k == 0 { self.lead } else { self.hold }
    }

    /// Seconds from the first frame to the last.
    pub fn length(&self) -> f32 {
        (0..self.frames.len().saturating_sub(1))
            .map(|k| self.held(k) + self.seconds)
            .sum()
    }

    /// Where in the frames the film is after `clock` seconds: 1.5 is half
    /// way from the second frame to the third.
    pub fn position(&self, clock: f32) -> f32 {
        let last = self.frames.len().saturating_sub(1);
        if last == 0 {
            return 0.0;
        }
        let mut clock = if self.looping {
            clock.rem_euclid(self.length() + self.hold)
        } else {
            clock.clamp(0.0, self.length())
        };
        for k in 0..last {
            let segment = self.held(k) + self.seconds;
            if clock < segment {
                if clock <= self.held(k) {
                    return k as f32;
                }
                let t = ((clock - self.held(k)) / self.seconds).clamp(0.0, 1.0);
                return k as f32 + t * t * (3.0 - 2.0 * t);
            }
            clock -= segment;
        }
        last as f32
    }

    /// The clock at which the film reaches a position.
    pub fn clock_at(&self, position: f32) -> f32 {
        let k = position.max(0.0).floor() as usize;
        let t = position - k as f32;
        let before: f32 = (0..k).map(|j| self.held(j) + self.seconds).sum();
        before
            + if t > 0.0 {
                self.held(k) + t * self.seconds
            } else {
                0.0
            }
    }

    /// What the film shows at a position.
    pub fn picture(&self, position: f32) -> Picture<'_> {
        let last = self.frames.len().saturating_sub(1);
        let k = (position.max(0.0).floor() as usize).min(last);
        let t = position - k as f32;
        if k == last || t < 0.002 {
            return Picture::Still(&self.frames[k]);
        }
        match self.tokens[k].between(&self.tokens[k + 1], t) {
            Some(svg) => Picture::Morph(svg),
            None => Picture::Blend(&self.frames[k], &self.frames[k + 1], t),
        }
    }

    /// Put a frame before the first, so that the film begins by moving at
    /// once from what was shown before.
    pub fn begin_from(&mut self, frame: String) {
        self.tokens.insert(0, Tokens::cut(&frame));
        self.frames.insert(0, frame);
        self.lead = 0.15;
    }
}

/// A frame made from another by changing some attributes of some elements:
/// each change is an element's id, an attribute, and its new value. An
/// attribute the element does not have is added.
pub fn changed(svg: &str, changes: &[(String, String, String)]) -> Result<String, String> {
    let mut out = svg.to_owned();
    for (id, attribute, value) in changes {
        let at = [format!(" id=\"{id}\""), format!(" id='{id}'")]
            .iter()
            .find_map(|needle| out.find(needle.as_str()))
            .ok_or_else(|| format!("no element with id {id}"))?;
        let start = out[..at]
            .rfind('<')
            .ok_or_else(|| format!("{id} is not in a tag"))?;
        let end = at
            + out[at..]
                .find('>')
                .ok_or_else(|| format!("{id}'s tag does not end"))?;
        let tag = &out[start..end];
        let value = value.replace('"', "'");
        let found = [
            format!(" {attribute}=\""),
            format!("\t{attribute}=\""),
            format!("\n{attribute}=\""),
        ]
        .iter()
        .find_map(|needle| tag.find(needle.as_str()).map(|i| i + needle.len()));
        let new_tag = match found {
            Some(from) => {
                let close = tag[from..]
                    .find('"')
                    .ok_or_else(|| format!("{id}'s {attribute} is not closed"))?;
                format!("{}{value}{}", &tag[..from], &tag[from + close..])
            }
            None => {
                let body = tag.strip_suffix('/').unwrap_or(tag);
                let slash = if tag.ends_with('/') { "/" } else { "" };
                format!("{body} {attribute}=\"{value}\"{slash}")
            }
        };
        out.replace_range(start..end, &new_tag);
    }
    Ok(out)
}

/// A named part of a drawing, and the box it takes, in the drawing's units.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub id: String,
    pub min: (f32, f32),
    pub max: (f32, f32),
}

impl Part {
    pub fn centre(&self) -> (f32, f32) {
        (
            (self.min.0 + self.max.0) / 2.0,
            (self.min.1 + self.max.1) / 2.0,
        )
    }

    fn area(&self) -> f32 {
        (self.max.0 - self.min.0) * (self.max.1 - self.min.1)
    }
}

/// The part under a point: the smallest box that holds it.
pub fn part_at(parts: &[Part], x: f32, y: f32) -> Option<&Part> {
    parts
        .iter()
        .filter(|p| p.min.0 <= x && x <= p.max.0 && p.min.1 <= y && y <= p.max.1)
        .min_by(|a, b| a.area().total_cmp(&b.area()))
}

/// Where a picture goes: the pixels it fills, and how drawing units map to
/// them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub offset: (f32, f32),
    pub background: [u8; 3],
}

/// Render a picture: its pixels, RGBA, and its named parts.
pub fn render(picture: &Picture, canvas: &Canvas) -> Result<(Vec<u8>, Vec<Part>), String> {
    match picture {
        Picture::Still(svg) => render_one(svg, canvas),
        Picture::Morph(svg) => render_one(svg, canvas),
        Picture::Blend(a, b, t) => {
            let (mut first, parts_a) = render_one(a, canvas)?;
            let (second, parts_b) = render_one(b, canvas)?;
            for (x, y) in first.iter_mut().zip(&second) {
                *x = (f32::from(*x) * (1.0 - t) + f32::from(*y) * t) as u8;
            }
            Ok((first, if *t < 0.5 { parts_a } else { parts_b }))
        }
    }
}

fn render_one(svg: &str, canvas: &Canvas) -> Result<(Vec<u8>, Vec<Part>), String> {
    let options = usvg::Options {
        fontdb: fonts(),
        font_family: "Noto Sans".to_owned(),
        ..usvg::Options::default()
    };
    let tree =
        usvg::Tree::from_str(svg, &options).map_err(|e| format!("the SVG does not read: {e}"))?;
    let mut pixmap = tiny_skia::Pixmap::new(canvas.width.max(1), canvas.height.max(1))
        .ok_or("a canvas of no size")?;
    let [r, g, b] = canvas.background;
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    let transform = tiny_skia::Transform::from_row(
        canvas.scale,
        0.0,
        0.0,
        canvas.scale,
        canvas.offset.0,
        canvas.offset.1,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut parts = Vec::new();
    collect(tree.root(), tiny_skia::Transform::identity(), &mut parts);
    Ok((pixmap.take(), parts))
}

/// The named parts of a group, their boxes found by composing the
/// transforms from the root down. resvg draws with these relative
/// transforms; its absolute boxes count a `<use>`'s transform twice.
fn collect(group: &usvg::Group, at: tiny_skia::Transform, parts: &mut Vec<Part>) {
    for node in group.children() {
        let id = node.id();
        if !id.is_empty()
            && let Some((min, max)) = extent(node, at)
        {
            parts.push(Part {
                id: id.to_owned(),
                min,
                max,
            });
        }
        if let usvg::Node::Group(g) = node {
            collect(g, at.pre_concat(g.transform()), parts);
        }
    }
}

type Extent = ((f32, f32), (f32, f32));

/// The box a node covers, under the transform of its parent.
fn extent(node: &usvg::Node, at: tiny_skia::Transform) -> Option<Extent> {
    match node {
        usvg::Node::Group(g) => {
            let inner = at.pre_concat(g.transform());
            g.children()
                .iter()
                .filter_map(|c| extent(c, inner))
                .reduce(|a, b| {
                    (
                        (a.0.0.min(b.0.0), a.0.1.min(b.0.1)),
                        (a.1.0.max(b.1.0), a.1.1.max(b.1.1)),
                    )
                })
        }
        leaf => {
            let b = match leaf {
                usvg::Node::Path(p) => p.stroke_bounding_box(),
                other => other.bounding_box(),
            };
            let mut corners = [
                tiny_skia::Point::from_xy(b.left(), b.top()),
                tiny_skia::Point::from_xy(b.right(), b.top()),
                tiny_skia::Point::from_xy(b.left(), b.bottom()),
                tiny_skia::Point::from_xy(b.right(), b.bottom()),
            ];
            at.map_points(&mut corners);
            let (mut min, mut max) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
            for p in corners {
                min = (min.0.min(p.x), min.1.min(p.y));
                max = (max.0.max(p.x), max.1.max(p.y));
            }
            Some((min, max))
        }
    }
}

/// A picture of the kit: every shape with its name, every class as a
/// swatch.
pub fn kit_sheet() -> Sheet {
    let mut body = String::from("<rect width=\"1600\" height=\"1250\" fill=\"#f7f5f0\"/>");
    for (i, (id, _, _, scale)) in SHAPES.iter().enumerate() {
        let (col, row) = (i % 7, i / 7);
        let (mut x, y) = (115.0 + col as f32 * 228.0, 95.0 + row as f32 * 190.0);
        if *id == "neuron" {
            // Drawn around its body, its axon reaching far to the right.
            x -= 60.0;
        }
        body.push_str(&format!(
            "<use href=\"#{id}\" transform=\"translate({x} {y}) scale({scale})\"/><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" class=\"small\">#{id}</text>",
            y + 85.0
        ));
    }
    let swatches = [
        "ectoderm",
        "neural",
        "neural-crest",
        "mesoderm",
        "somite",
        "notochord",
        "endoderm",
        "yolk",
        "amnion",
        "epidermis",
        "dermis",
        "fat",
        "muscle",
        "bone",
        "cartilage",
        "artery",
        "vein",
        "nerve",
        "grey-matter",
        "white-matter",
        "csf",
        "liver",
        "lung",
        "heart",
        "kidney",
        "gut",
        "marrow",
        "protein",
        "receptor",
        "ligand",
        "tumour-cell",
        "lymph",
    ];
    for (i, class) in swatches.iter().enumerate() {
        let (col, row) = (i % 11, i / 11);
        let (x, y) = (30.0 + col as f32 * 143.0, 885.0 + row as f32 * 120.0);
        body.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"120\" height=\"60\" rx=\"8\" class=\"{class}\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"small\">{class}</text>",
            x + 60.0,
            y + 84.0
        ));
    }
    Sheet::read(&body, Some((1600.0, 1250.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_become_numbers_but_references_do_not() {
        let out = hex_to_rgb(
            "<use href=\"#cell\" fill=\"#ff8000\"/><path fill=\"url(#h-crest)\" stroke=\"#fff\"/><g id=\"#bad1\"/>",
        );
        assert!(out.contains("href=\"#cell\""));
        assert!(out.contains("rgb(255,128,0)"));
        assert!(out.contains("url(#h-crest)"));
        assert!(out.contains("rgb(255,255,255)"));
        assert!(out.contains("#bad1"));
    }

    #[test]
    fn frames_that_differ_by_numbers_move_into_each_other() {
        let a = Tokens::cut("<circle id=\"cell2\" cx=\"10\" cy=\"-4.5\" fill=\"rgb(0,0,0)\"/>");
        let b = Tokens::cut("<circle id=\"cell2\" cx=\"30\" cy=\"5.5\" fill=\"rgb(200,100,0)\"/>");
        let half = a.between(&b, 0.5).expect("same drawing");
        assert_eq!(
            half,
            "<circle id=\"cell2\" cx=\"20\" cy=\"0.5\" fill=\"rgb(100,50,0)\"/>"
        );
        let other = Tokens::cut("<rect x=\"1\"/>");
        assert!(a.between(&other, 0.5).is_none());
    }

    #[test]
    fn a_frame_is_made_from_what_changes() {
        let first = "<circle id=\"c\" cx=\"10\" r=\"4\"/><path id='p' d=\"M0 0\"></path>";
        let next = changed(
            first,
            &[
                ("c".into(), "cx".into(), "30".into()),
                ("c".into(), "opacity".into(), "0.5".into()),
                ("p".into(), "d".into(), "M5 5".into()),
            ],
        )
        .expect("changed");
        assert_eq!(
            next,
            "<circle id=\"c\" cx=\"30\" r=\"4\" opacity=\"0.5\"/><path id='p' d=\"M5 5\"></path>"
        );
        assert!(changed(first, &[("x".into(), "cx".into(), "1".into())]).is_err());
    }

    #[test]
    fn a_film_that_begins_from_the_step_before_moves_at_once() {
        let mut film = Film::new(vec!["a".into()], 1.0, 2.0, false);
        film.begin_from("z".into());
        assert!(film.position(0.5) > 0.0);
        assert!((film.length() - 2.15).abs() < 1e-5);
    }

    #[test]
    fn numbers_are_read_as_svg_writes_them() {
        let t = Tokens::cut("M10-5.5.5 1e-3 1em");
        assert_eq!(t.numbers, ["10", "-5.5", ".5", "1e-3", "1"]);
    }

    #[test]
    fn a_film_holds_then_moves() {
        let film = Film::new(vec!["a".into(), "b".into(), "c".into()], 1.0, 2.0, false);
        assert_eq!(film.length(), 6.0);
        assert_eq!(film.position(0.5), 0.0);
        assert!((film.position(2.0) - 0.5).abs() < 1e-6);
        assert_eq!(film.position(3.0), 1.0);
        assert_eq!(film.position(99.0), 2.0);
        assert!((film.position(film.clock_at(1.5)) - 1.5).abs() < 0.2);
    }

    #[test]
    fn a_whole_document_keeps_its_view_box() {
        let s = Sheet::read(
            "<?xml version=\"1.0\"?><svg viewBox=\"0 0 800 400\"><circle r=\"3\"/></svg>",
            None,
        );
        assert_eq!(s.size, (800.0, 400.0));
        assert_eq!(s.body, "<circle r=\"3\"/>");
    }

    #[test]
    fn every_shape_and_class_the_kit_names_exists() {
        for (id, ..) in SHAPES {
            assert!(
                HOUSE_SVG.contains(&format!("<symbol id=\"{id}\"")),
                "no shape {id}"
            );
        }
        for (_, list) in CLASSES {
            for class in list.split(", ") {
                let name = class.split(' ').next().unwrap_or("");
                assert!(HOUSE_CSS.contains(&format!(".{name} ")), "no class {name}");
            }
        }
    }

    #[test]
    fn a_drawing_renders_and_names_its_parts() {
        let sheet = Sheet::read(
            "<g id=\"crest\"><use href=\"#melanoblast\" x=\"200\" y=\"100\"/></g><rect id=\"plate\" class=\"neural\" x=\"10\" y=\"300\" width=\"300\" height=\"40\"/>",
            Some((400.0, 400.0)),
        );
        let canvas = Canvas {
            width: 200,
            height: 200,
            scale: 0.5,
            offset: (0.0, 0.0),
            background: [247, 245, 240],
        };
        let (pixels, parts) = render(&Picture::Still(&sheet.document()), &canvas).expect("renders");
        assert_eq!(pixels.len(), 200 * 200 * 4);
        let plate = parts
            .iter()
            .find(|p| p.id == "plate")
            .expect("the plate is named");
        assert_eq!(plate.centre(), (160.0, 320.0));
        assert_eq!(
            part_at(&parts, 200.0, 100.0).map(|p| p.id.as_str()),
            Some("crest")
        );
        // Shapes drawn from a <use> that moves, scales and turns keep their
        // place, after other elements as before them.
        let turned = Sheet::read(
            "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\"/><use id=\"red\" href=\"#rbc\" transform=\"translate(545 410) rotate(30) scale(0.5)\"/><use id=\"moving\" href=\"#melanoblast\" transform=\"translate(545 410) rotate(0) scale(.5)\"/>",
            None,
        );
        let (_, parts) = render(&Picture::Still(&turned.document()), &canvas).expect("renders");
        for id in ["red", "moving"] {
            let part = parts.iter().find(|p| p.id == id).expect("named");
            let (x, y) = part.centre();
            assert!(
                (x - 545.0).abs() < 6.0 && (y - 410.0).abs() < 6.0,
                "{id} at {x} {y}"
            );
        }
        // The neural plate is drawn in the house's indigo.
        let i = (160 * 200 + 80) * 4;
        assert!(pixels[i + 2] > pixels[i], "{:?}", &pixels[i..i + 4]);
    }

    #[test]
    fn the_kit_sheet_renders() {
        let sheet = kit_sheet();
        let canvas = Canvas {
            width: 400,
            height: 312,
            scale: 0.25,
            offset: (0.0, 0.0),
            background: [247, 245, 240],
        };
        assert!(render(&Picture::Still(&sheet.document()), &canvas).is_ok());
    }

    /// Write the kit's picture to `$CELLSTAGE_OUT/kit.png`, to look at.
    #[test]
    #[ignore = "writes a picture to look at"]
    fn draw_the_kit() {
        let Some(dir) = std::env::var_os("CELLSTAGE_OUT") else {
            return;
        };
        let sheet = kit_sheet();
        let canvas = Canvas {
            width: 1600,
            height: 1250,
            scale: 1.0,
            offset: (0.0, 0.0),
            background: [247, 245, 240],
        };
        let (pixels, _) = render(&Picture::Still(&sheet.document()), &canvas).expect("renders");
        let file = std::fs::File::create(std::path::Path::new(&dir).join("kit.png")).expect("file");
        let mut encoder = png::Encoder::new(file, 1600, 1250);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .expect("header")
            .write_image_data(&pixels)
            .expect("pixels");
    }
}
