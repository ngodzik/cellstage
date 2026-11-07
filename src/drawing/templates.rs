//! Ready-made drawings of whole things, an organ, a section, a stage of
//! the embryo, each with its parts named, for the agent to set in a drawing
//! with `<insert name="..."/>`, point at, recolour, and move.

use std::sync::OnceLock;

use super::macros::{Attributes, descriptions};

/// Every template: its name and its SVG.
const SOURCES: &[(&str, &str)] = &[
    ("alveolus", include_str!("templates/alveolus.svg")),
    ("body", include_str!("templates/body.svg")),
    (
        "bone-marrow-niche",
        include_str!("templates/bone-marrow-niche.svg"),
    ),
    ("brain-coronal", include_str!("templates/brain-coronal.svg")),
    ("brain-midline", include_str!("templates/brain-midline.svg")),
    ("brain-side", include_str!("templates/brain-side.svg")),
    ("cell-interior", include_str!("templates/cell-interior.svg")),
    (
        "digestive-tract",
        include_str!("templates/digestive-tract.svg"),
    ),
    ("dna-packaging", include_str!("templates/dna-packaging.svg")),
    (
        "embryo-4-weeks",
        include_str!("templates/embryo-4-weeks.svg"),
    ),
    ("eye", include_str!("templates/eye.svg")),
    ("gastrulation", include_str!("templates/gastrulation.svg")),
    ("gut-wall", include_str!("templates/gut-wall.svg")),
    ("heart", include_str!("templates/heart.svg")),
    ("kidney", include_str!("templates/kidney.svg")),
    ("liver-lobule", include_str!("templates/liver-lobule.svg")),
    ("long-bone", include_str!("templates/long-bone.svg")),
    ("lungs", include_str!("templates/lungs.svg")),
    ("lymph-node", include_str!("templates/lymph-node.svg")),
    (
        "membrane-section",
        include_str!("templates/membrane-section.svg"),
    ),
    ("nephron", include_str!("templates/nephron.svg")),
    ("neurulation", include_str!("templates/neurulation.svg")),
    (
        "pancreatic-islet",
        include_str!("templates/pancreatic-islet.svg"),
    ),
    ("sarcomere", include_str!("templates/sarcomere.svg")),
    ("skin-block", include_str!("templates/skin-block.svg")),
    ("spinal-cord", include_str!("templates/spinal-cord.svg")),
    ("synapse", include_str!("templates/synapse.svg")),
    (
        "thyroid-follicle",
        include_str!("templates/thyroid-follicle.svg"),
    ),
    (
        "tumour-microenvironment",
        include_str!("templates/tumour-microenvironment.svg"),
    ),
    ("vessel-wall", include_str!("templates/vessel-wall.svg")),
    (
        "zygote-to-blastocyst",
        include_str!("templates/zygote-to-blastocyst.svg"),
    ),
];

/// A template, read.
#[derive(Debug, Clone)]
pub struct Template {
    pub name: &'static str,
    /// What it shows, in a line.
    pub what: String,
    /// Its viewBox: x, y, width, height.
    pub view: [f32; 4],
    /// Its elements, without the `<svg>` around them.
    pub body: String,
    /// Its parts: id, and what each is.
    pub parts: Vec<(String, String)>,
}

impl Template {
    /// Read a template's SVG.
    pub fn read(name: &'static str, svg: &str) -> Result<Self, String> {
        let open = svg.find("<svg").ok_or("no <svg>")?;
        let end = open + svg[open..].find('>').ok_or("<svg does not close")?;
        let a = Attributes::parse(svg[open + 4..end].trim_end_matches('/'))?;
        let view: Vec<f32> = a
            .get("viewBox")
            .ok_or("no viewBox")?
            .split([' ', ','])
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        let [x, y, w, h] = view[..] else {
            return Err("the viewBox is not four numbers".to_owned());
        };
        let close = svg.rfind("</svg>").ok_or("no </svg>")?;
        let body = svg[end + 1..close].trim().to_owned();
        Ok(Self {
            name,
            what: a.get("data-what").unwrap_or("").to_owned(),
            view: [x, y, w, h],
            parts: descriptions(&body),
            body,
        })
    }
}

/// Every template, read once.
pub fn all() -> &'static [Template] {
    static ALL: OnceLock<Vec<Template>> = OnceLock::new();
    ALL.get_or_init(|| {
        SOURCES
            .iter()
            .filter_map(|(name, svg)| Template::read(name, svg).ok())
            .collect()
    })
}

pub fn find(name: &str) -> Option<&'static Template> {
    all().iter().find(|t| t.name == name)
}

pub fn names() -> Vec<&'static str> {
    all().iter().map(|t| t.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_reads() {
        for (name, svg) in SOURCES {
            let t = Template::read(name, svg).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!t.what.is_empty(), "{name} says what it shows");
            assert!(!t.parts.is_empty(), "{name} has named parts");
            for (id, _) in &t.parts {
                assert!(id.contains('_'), "{name}: part {id} has no underscore");
                assert!(
                    super::super::SHAPES.iter().all(|s| s.0 != id),
                    "{name}: part {id} is a shape's name"
                );
            }
        }
        assert_eq!(all().len(), SOURCES.len());
    }

    #[test]
    fn a_template_reads() {
        let t = Template::read(
            "test",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 200 100\" data-what=\"a test\"><rect id=\"left_box\" data-what=\"a box\" width=\"10\" height=\"10\"/></svg>",
        )
        .unwrap();
        assert_eq!(t.view, [0.0, 0.0, 200.0, 100.0]);
        assert_eq!(t.parts, vec![("left_box".to_owned(), "a box".to_owned())]);
    }
}
