//! Figures the window lays out itself from what the agent says: a diagram,
//! a chart, a page of words. Each is written as a drawing, so that it takes
//! notes and clicks, is kept with its step, and moves into the next as
//! drawings do.

pub mod chart;
pub mod diagram;
pub mod page;

/// A figure written as SVG: its elements, its size, and its named parts
/// with what each is.
pub struct Figure {
    pub svg: String,
    pub size: (f32, f32),
    pub parts: Vec<(String, String)>,
}
