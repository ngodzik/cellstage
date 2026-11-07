//! The width of words in the drawings' font, so that the window can wrap
//! the text it lays out itself: a page of text, a diagram's boxes, a
//! chart's legend.

use resvg::usvg::fontdb;

use super::fonts;

/// A run of words in one weight.
pub type Run = (String, bool);

/// How wide a text is, in units, at a font size.
pub fn width(text: &str, size: f32, bold: bool) -> f32 {
    let db = fonts();
    let query = fontdb::Query {
        families: &[fontdb::Family::Name("Noto Sans"), fontdb::Family::SansSerif],
        weight: if bold {
            fontdb::Weight::BOLD
        } else {
            fontdb::Weight::NORMAL
        },
        ..fontdb::Query::default()
    };
    db.query(&query)
        .and_then(|id| {
            db.with_face_data(id, |data, index| {
                let face = ttf_parser::Face::parse(data, index).ok()?;
                let em = f32::from(face.units_per_em());
                let units: f32 = text
                    .chars()
                    .map(|c| {
                        face.glyph_index(c)
                            .and_then(|g| face.glyph_hor_advance(g))
                            .map_or(0.55 * em, f32::from)
                    })
                    .sum();
                Some(units / em * size)
            })
        })
        .flatten()
        .unwrap_or(text.chars().count() as f32 * 0.55 * size)
}

/// Runs of words cut into lines no wider than `max`, each line its runs.
pub fn wrap(runs: &[Run], size: f32, max: f32) -> Vec<Vec<Run>> {
    let mut lines: Vec<Vec<Run>> = Vec::new();
    let mut line: Vec<Run> = Vec::new();
    let mut used = 0.0;
    let space = width(" ", size, false);
    for (words, bold) in runs {
        for word in words.split_whitespace() {
            let w = width(word, size, *bold);
            let gap = if line.is_empty() { 0.0 } else { space };
            if !line.is_empty() && used + gap + w > max {
                lines.push(std::mem::take(&mut line));
                used = 0.0;
            }
            let gap = if line.is_empty() { 0.0 } else { space };
            match line.last_mut() {
                Some((text, b)) if *b == *bold => {
                    text.push(' ');
                    text.push_str(word);
                }
                Some((text, _)) => {
                    text.push(' ');
                    line.push((word.to_owned(), *bold));
                }
                None => line.push((word.to_owned(), *bold)),
            }
            used += gap + w;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Runs from text marked with `**bold**`.
pub fn runs(text: &str) -> Vec<Run> {
    text.split("**")
        .enumerate()
        .filter(|(_, s)| !s.is_empty())
        .map(|(k, s)| (s.to_owned(), k % 2 == 1))
        .collect()
}

/// Text made safe to put between tags.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wider_words_are_wider() {
        let narrow = width("iiii", 20.0, false);
        let wide = width("MMMM", 20.0, false);
        assert!(wide > narrow * 1.5, "{narrow} {wide}");
        assert!(width("MMMM", 20.0, true) >= wide);
    }

    #[test]
    fn lines_fit() {
        let text = runs(
            "The **neural crest** leaves the closing tube and travels through the embryo to the skin.",
        );
        let lines = wrap(&text, 20.0, 260.0);
        assert!(lines.len() >= 3);
        for line in &lines {
            let w: f32 = line.iter().map(|(t, b)| width(t, 20.0, *b)).sum();
            assert!(w <= 270.0, "{line:?} is {w}");
        }
        assert!(
            lines
                .iter()
                .flatten()
                .any(|(t, b)| *b && t.contains("crest"))
        );
    }
}
