//! The stage: the story being told, the step on screen, and the window
//! split into that step's panels, each with a title, a label saying what
//! kind of thing it shows, a caption, the agent's notes, and what the user
//! clicks on it.

use bevy::prelude::*;
use bevy_egui::egui;

use crate::story::{Call, Story};

/// The most panels a step shows at once.
pub const MAX_PANELS: usize = 6;

/// How the panels share the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Arrangement {
    #[default]
    Columns,
    Rows,
    Grid,
}

impl Arrangement {
    pub fn parse(name: &str) -> Self {
        match name {
            "rows" => Arrangement::Rows,
            "grid" => Arrangement::Grid,
            _ => Arrangement::Columns,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Arrangement::Columns => "columns",
            Arrangement::Rows => "rows",
            Arrangement::Grid => "grid",
        }
    }
}

/// What kind of thing a panel shows, which its label says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Nothing yet.
    Empty,
    /// A structure measured and deposited in the Protein Data Bank.
    Protein,
    /// A cell whose protein levels and fate cellweave computes.
    Cell,
    /// A tissue drawn from what histology describes.
    Skin,
    /// A drawing the agent wrote.
    Drawing,
}

impl Kind {
    /// The label every panel carries, so that a drawing is never taken for
    /// a measurement or a result.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Empty => "",
            Kind::Protein => "MEASURED STRUCTURE",
            Kind::Cell => "SIMULATION · cellweave's network; the cell's look is drawn",
            Kind::Skin => "ILLUSTRATION · drawn from histology, not simulated",
            Kind::Drawing => "ILLUSTRATION · a schematic drawn by the agent",
        }
    }

    fn colour(self) -> egui::Color32 {
        match self {
            Kind::Protein => egui::Color32::from_rgb(120, 190, 255),
            Kind::Cell => egui::Color32::from_rgb(120, 225, 160),
            Kind::Skin => egui::Color32::from_rgb(245, 180, 110),
            Kind::Drawing => egui::Color32::from_rgb(240, 200, 120),
            Kind::Empty => egui::Color32::GRAY,
        }
    }
}

/// What a note points at.
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    /// Fractions of the panel's width and height from its top left.
    Fraction(f32, f32),
    /// A residue of a protein, followed as the protein turns.
    Residue(char, i32),
    /// A named part: of a drawing, a section, a cell.
    Part(String),
}

/// A note the agent wrote on a panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub text: String,
    /// What it points at, with an arrow; nothing for a note that only sits.
    pub target: Option<Anchor>,
    /// Where its words sit, as fractions of the panel; chosen by the window
    /// when not given.
    pub label: Option<(f32, f32)>,
}

/// One panel.
pub struct Panel {
    pub id: String,
    pub title: String,
    pub caption: String,
    /// A line under the label: which entry, which stage, which setting.
    pub source: String,
    pub kind: Kind,
    pub notes: Vec<Note>,
    /// The call whose result it shows, to know whether a step needs it
    /// made again.
    pub shown: Option<Call>,
    /// Where it is on screen, in logical points, as last laid out.
    pub rect: egui::Rect,
    /// The image a 2D panel shows, and its egui texture.
    pub image: Option<Handle<Image>>,
    pub texture: Option<egui::TextureId>,
    /// How far in, and where the middle of the view is, as fractions of
    /// the image.
    pub zoom: f32,
    pub pan: egui::Vec2,
}

impl Panel {
    pub fn new(id: &str, title: &str) -> Self {
        Self {
            id: id.to_owned(),
            title: title.to_owned(),
            caption: String::new(),
            source: String::new(),
            kind: Kind::Empty,
            notes: Vec::new(),
            shown: None,
            rect: egui::Rect::NOTHING,
            image: None,
            texture: None,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
        }
    }

    /// The area inside the panel where its content goes, below the title
    /// and above the caption.
    pub fn content(&self) -> egui::Rect {
        let top = self.rect.top() + TITLE;
        let bottom = self.rect.bottom()
            - if self.caption.is_empty() {
                22.0
            } else {
                CAPTION
            };
        egui::Rect::from_min_max(
            egui::pos2(self.rect.left() + 2.0, top),
            egui::pos2(self.rect.right() - 2.0, bottom.max(top + 10.0)),
        )
    }
}

/// Height of a panel's title bar, and of its caption, in points.
const TITLE: f32 = 50.0;
const CAPTION: f32 = 58.0;
/// The gap between panels.
const GAP: f32 = 6.0;

/// The whole stage.
#[derive(Resource)]
pub struct Stage {
    pub panels: Vec<Panel>,
    pub arrangement: Arrangement,
    /// The story being told, and the step on screen.
    pub story: Option<Story>,
    pub step: usize,
    /// A step the user asked for, and a story the user opened, for the
    /// stage to show at its next turn.
    pub goto: Option<usize>,
    pub open: Option<String>,
    /// Whether the story has changed since it was last written to disk.
    pub dirty: bool,
    /// Whether the library is open in the window.
    pub library: bool,
    /// What the user did, in words, since the agent last asked.
    pub events: Vec<String>,
    /// Whether the window has been shown yet.
    pub shown: bool,
    /// Physical pixels to a logical point, as egui last said.
    pub pixels_per_point: f32,
}

impl Default for Stage {
    fn default() -> Self {
        Self {
            panels: Vec::new(),
            arrangement: Arrangement::Columns,
            story: None,
            step: 0,
            goto: None,
            open: None,
            dirty: false,
            library: false,
            events: Vec::new(),
            shown: false,
            pixels_per_point: 1.0,
        }
    }
}

impl Stage {
    /// The panel with this id, made if there is none and there is room.
    pub fn panel(&mut self, id: &str) -> Option<&mut Panel> {
        if !self.panels.iter().any(|p| p.id == id) {
            if self.panels.len() >= MAX_PANELS {
                return None;
            }
            self.panels.push(Panel::new(id, id));
        }
        self.panels.iter_mut().find(|p| p.id == id)
    }

    /// Where the story is, in words: "step 2 of 5".
    pub fn whereabouts(&self) -> String {
        match &self.story {
            Some(s) => format!(
                "[story {}, step {} of {}]",
                s.id,
                self.step + 1,
                s.steps.len()
            ),
            None => String::new(),
        }
    }

    /// Lay the panels out in `area`.
    pub fn lay_out(&mut self, area: egui::Rect) {
        let n = self.panels.len();
        if n == 0 {
            return;
        }
        let (columns, rows) = match (self.arrangement, n) {
            (Arrangement::Rows, n) => (1, n),
            (Arrangement::Grid, n) if n > 4 => (3, n.div_ceil(3)),
            (Arrangement::Grid, n) if n > 2 => (2, n.div_ceil(2)),
            (_, n) => (n, 1),
        };
        let w = (area.width() - GAP * (columns as f32 - 1.0)) / columns as f32;
        let h = (area.height() - GAP * (rows as f32 - 1.0)) / rows as f32;
        for (i, panel) in self.panels.iter_mut().enumerate() {
            let (c, r) = (i % columns, i / columns);
            let min = area.min + egui::vec2(c as f32 * (w + GAP), r as f32 * (h + GAP));
            panel.rect = egui::Rect::from_min_size(min, egui::vec2(w, h));
        }
    }
}

/// Draw a panel's frame: its title, its label, its caption. The content is
/// drawn by whoever owns it, before or after.
pub fn frame(painter: &egui::Painter, panel: &Panel, solid: bool) {
    let r = panel.rect;
    let back = egui::Color32::from_rgb(14, 16, 22);
    // A protein panel's middle belongs to its camera: only the bars are
    // painted over it.
    if solid {
        painter.rect_filled(r, 4.0, back);
    } else {
        painter.rect_filled(
            egui::Rect::from_min_max(r.min, egui::pos2(r.right(), r.top() + TITLE)),
            4.0,
            back,
        );
        let caption_top = panel.content().bottom();
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(r.left(), caption_top), r.max),
            4.0,
            back,
        );
    }
    painter.rect_stroke(
        r,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(48)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        r.left_top() + egui::vec2(10.0, 7.0),
        egui::Align2::LEFT_TOP,
        &panel.title,
        egui::FontId::proportional(17.0),
        egui::Color32::from_rgb(232, 234, 240),
    );
    let label = if panel.source.is_empty() {
        panel.kind.label().to_owned()
    } else {
        format!("{} · {}", panel.kind.label(), panel.source)
    };
    painter.text(
        r.left_top() + egui::vec2(10.0, 30.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(11.5),
        panel.kind.colour(),
    );
    if !panel.caption.is_empty() {
        let galley = painter.layout(
            panel.caption.clone(),
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(205, 208, 216),
            r.width() - 20.0,
        );
        painter.galley(
            egui::pos2(r.left() + 10.0, panel.content().bottom() + 6.0),
            galley,
            egui::Color32::WHITE,
        );
    }
}

const INK: egui::Color32 = egui::Color32::from_rgb(255, 236, 160);

/// The size of a note's words as drawn.
fn note_galley(painter: &egui::Painter, text: &str) -> std::sync::Arc<egui::Galley> {
    painter.layout(
        text.to_owned(),
        egui::FontId::proportional(14.0),
        INK,
        240.0,
    )
}

/// Where a note's words go when the agent did not say: away from the
/// middle of the panel, beside what it points at, clear of the notes
/// already placed.
pub fn place_label(
    painter: &egui::Painter,
    text: &str,
    target: egui::Pos2,
    area: egui::Rect,
    taken: &[egui::Rect],
) -> egui::Rect {
    let size = note_galley(painter, text).size() + egui::vec2(10.0, 8.0);
    let away = target - area.center();
    let base = if away.length() < 20.0 {
        -std::f32::consts::FRAC_PI_4 * 3.0
    } else {
        away.y.atan2(away.x)
    };
    let mut first = None;
    for (k, turn) in [
        0.0f32,
        0.6,
        -0.6,
        1.2,
        -1.2,
        1.9,
        -1.9,
        2.6,
        -2.6,
        std::f32::consts::PI,
    ]
    .iter()
    .enumerate()
    {
        let reach = 70.0 + 18.0 * (k / 2) as f32;
        let a = base + turn;
        let centre =
            target + egui::vec2(a.cos(), a.sin()) * (reach + size.x * 0.35 * a.cos().abs());
        let mut rect = egui::Rect::from_center_size(centre, size);
        // Kept inside the panel.
        let dx = (area.left() - rect.left()).max(0.0) + (area.right() - rect.right()).min(0.0);
        let dy = (area.top() - rect.top()).max(0.0) + (area.bottom() - rect.bottom()).min(0.0);
        rect = rect.translate(egui::vec2(dx, dy));
        if rect.contains(target) {
            continue;
        }
        if !taken.iter().any(|t| t.expand(4.0).intersects(rect)) {
            return rect;
        }
        first.get_or_insert(rect);
    }
    first.unwrap_or_else(|| egui::Rect::from_min_size(area.min + egui::vec2(8.0, 8.0), size))
}

/// The box a note's words take when they sit at a point.
pub fn label_at(painter: &egui::Painter, text: &str, at: egui::Pos2) -> egui::Rect {
    let size = note_galley(painter, text).size() + egui::vec2(10.0, 8.0);
    egui::Rect::from_min_size(at, size)
}

/// Draw a note: its words in a dark box, and an arrow from the box to what
/// it points at.
pub fn note(painter: &egui::Painter, rect: egui::Rect, text: &str, target: Option<egui::Pos2>) {
    if let Some(to) = target {
        let from = egui::pos2(
            to.x.clamp(rect.left(), rect.right()),
            to.y.clamp(rect.top(), rect.bottom()),
        );
        if from.distance(to) > 6.0 {
            painter.arrow(from, to - from, egui::Stroke::new(2.0, INK));
        }
        painter.circle_filled(to, 3.0, INK);
    }
    painter.rect_filled(
        rect,
        3.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
    );
    let galley = note_galley(painter, text);
    painter.galley(rect.min + egui::vec2(5.0, 4.0), galley, INK);
}

/// Where an image of `size` sits in `area`, kept in proportion, at a zoom
/// and a pan.
pub fn image_rect(area: egui::Rect, size: (f32, f32), zoom: f32, pan: egui::Vec2) -> egui::Rect {
    let (w, h) = size;
    let fit = (area.width() / w).min(area.height() / h) * zoom;
    let size = egui::vec2(w, h) * fit;
    let centre = area.center() - egui::vec2(pan.x * size.x, pan.y * size.y);
    egui::Rect::from_center_size(centre, size)
}
