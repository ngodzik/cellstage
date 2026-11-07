//! The stage: the window split into panels, each with a title, a label
//! saying what kind of thing it shows, a caption, the agent's notes, and
//! what the user clicks on it.

use bevy::prelude::*;
use bevy_egui::egui;

/// How the panels share the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Arrangement {
    #[default]
    Columns,
    Rows,
    Grid,
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
        }
    }
}

/// Where a note is pinned.
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    /// Fractions of the panel's width and height from its top left.
    Fraction(f32, f32),
    /// A residue of a protein, followed as the protein turns.
    Residue(char, i32),
}

/// A note the agent wrote on a panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub text: String,
    pub at: Anchor,
    /// A point the note points to, as fractions of the panel.
    pub arrow_to: Option<(f32, f32)>,
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
#[derive(Resource, Default)]
pub struct Stage {
    pub panels: Vec<Panel>,
    pub arrangement: Arrangement,
    /// What the user clicked, in words, since the agent last asked.
    pub events: Vec<String>,
    /// Whether the window has been shown yet.
    pub shown: bool,
}

impl Stage {
    /// The panel with this id, made if there is none and there is room.
    pub fn panel(&mut self, id: &str) -> Option<&mut Panel> {
        if !self.panels.iter().any(|p| p.id == id) {
            if self.panels.len() >= 4 {
                return None;
            }
            self.panels.push(Panel::new(id, id));
        }
        self.panels.iter_mut().find(|p| p.id == id)
    }

    /// Lay the panels out in `area`.
    pub fn lay_out(&mut self, area: egui::Rect) {
        let n = self.panels.len();
        if n == 0 {
            return;
        }
        let (columns, rows) = match (self.arrangement, n) {
            (Arrangement::Rows, n) => (1, n),
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
    let colour = match panel.kind {
        Kind::Protein => egui::Color32::from_rgb(120, 190, 255),
        Kind::Cell => egui::Color32::from_rgb(120, 225, 160),
        Kind::Skin => egui::Color32::from_rgb(245, 180, 110),
        Kind::Empty => egui::Color32::GRAY,
    };
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
        colour,
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

/// Draw a note, a dark label with an optional arrow.
pub fn note(painter: &egui::Painter, at: egui::Pos2, text: &str, arrow_to: Option<egui::Pos2>) {
    let ink = egui::Color32::from_rgb(255, 236, 160);
    if let Some(to) = arrow_to {
        painter.arrow(at, to - at, egui::Stroke::new(2.0, ink));
    }
    let galley = painter.layout(
        text.to_owned(),
        egui::FontId::proportional(14.0),
        ink,
        260.0,
    );
    let rect =
        egui::Rect::from_min_size(at + egui::vec2(6.0, -galley.size().y - 4.0), galley.size())
            .expand(4.0);
    painter.rect_filled(
        rect,
        3.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 190),
    );
    painter.galley(rect.min + egui::vec2(4.0, 4.0), galley, ink);
}

/// Where an image of `size` pixels sits in `area`, kept in proportion, at
/// a zoom and a pan.
pub fn image_rect(
    area: egui::Rect,
    size: (usize, usize),
    zoom: f32,
    pan: egui::Vec2,
) -> egui::Rect {
    let (w, h) = (size.0 as f32, size.1 as f32);
    let fit = (area.width() / w).min(area.height() / h) * zoom;
    let size = egui::vec2(w, h) * fit;
    let centre = area.center() - egui::vec2(pan.x * size.x, pan.y * size.y);
    egui::Rect::from_center_size(centre, size)
}
