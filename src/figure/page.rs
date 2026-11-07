//! A page of words: headings, paragraphs, lists, a definition set apart,
//! for what a course says beside its pictures. Written in a little
//! Markdown: `# heading`, `## subheading`, `- item`, `1. item`, `> a
//! definition`, `**bold**`, a blank line between paragraphs.

use std::fmt::Write;

use serde_json::Value;

use super::Figure;
use crate::drawing::text::{self, Run};

const WIDTH: f32 = 1200.0;
const PAD: f32 = 64.0;

enum Block {
    Heading(String, u8),
    Paragraph(String),
    Item(String, Option<String>),
    Definition(String),
    Rule,
}

fn blocks(source: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut paragraph = String::new();
    let flush = |paragraph: &mut String, out: &mut Vec<Block>| {
        if !paragraph.trim().is_empty() {
            out.push(Block::Paragraph(paragraph.trim().to_owned()));
        }
        paragraph.clear();
    };
    for line in source.lines() {
        let t = line.trim();
        let numbered = t
            .split_once(". ")
            .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if t.is_empty() {
            flush(&mut paragraph, &mut out);
        } else if let Some(h) = t.strip_prefix("## ") {
            flush(&mut paragraph, &mut out);
            out.push(Block::Heading(h.to_owned(), 2));
        } else if let Some(h) = t.strip_prefix("# ") {
            flush(&mut paragraph, &mut out);
            out.push(Block::Heading(h.to_owned(), 1));
        } else if let Some(i) = t.strip_prefix("- ").or(t.strip_prefix("* ")) {
            flush(&mut paragraph, &mut out);
            out.push(Block::Item(i.to_owned(), None));
        } else if let Some((n, i)) = numbered {
            flush(&mut paragraph, &mut out);
            out.push(Block::Item(i.to_owned(), Some(format!("{n}."))));
        } else if let Some(d) = t.strip_prefix("> ") {
            flush(&mut paragraph, &mut out);
            match out.last_mut() {
                Some(Block::Definition(text)) => {
                    text.push(' ');
                    text.push_str(d);
                }
                _ => out.push(Block::Definition(d.to_owned())),
            }
        } else if t == "---" {
            flush(&mut paragraph, &mut out);
            out.push(Block::Rule);
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(t);
        }
    }
    flush(&mut paragraph, &mut out);
    out
}

/// Lines of runs, written as `<text>` with `<tspan>`s for the bold.
fn lines(
    out: &mut String,
    lines: &[Vec<Run>],
    x: f32,
    mut y: f32,
    size: f32,
    colour: &str,
    step: f32,
) -> f32 {
    for line in lines {
        let _ = write!(
            out,
            "<text x=\"{x:.1}\" y=\"{y:.1}\" font-size=\"{size}\" fill=\"{colour}\">"
        );
        for (k, (words, bold)) in line.iter().enumerate() {
            let space = if k > 0 { " " } else { "" };
            if *bold {
                let _ = write!(
                    out,
                    "<tspan font-weight=\"bold\">{space}{}</tspan>",
                    text::escape(words)
                );
            } else {
                let _ = write!(out, "{space}{}", text::escape(words));
            }
        }
        out.push_str("</text>");
        y += step;
    }
    y
}

/// A page from the agent's text.
pub fn page(args: &Value) -> Result<Figure, String> {
    let source = args
        .get("text")
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .ok_or("give the text")?;
    let width = args
        .get("width")
        .and_then(Value::as_f64)
        .map_or(WIDTH, |w| (w as f32).clamp(500.0, 2400.0));
    let body = args
        .get("size")
        .and_then(Value::as_f64)
        .map_or(30.0, |s| (s as f32).clamp(14.0, 60.0));
    let inner = width - 2.0 * PAD;
    let mut out = String::new();
    let mut parts = Vec::new();
    let mut y = PAD;
    for (k, block) in blocks(source).iter().enumerate() {
        let id = format!("block_{}", k + 1);
        let mut what = String::new();
        let _ = write!(out, "<g id=\"{id}\">");
        match block {
            Block::Heading(h, level) => {
                let size = if *level == 1 { body * 1.6 } else { body * 1.22 };
                y += size * 0.35;
                let wrapped = text::wrap(&[(h.clone(), true)], size, inner);
                let colour = if *level == 1 { "#1f2a44" } else { "#2f4a7a" };
                y = lines(&mut out, &wrapped, PAD, y + size, size, colour, size * 1.25);
                y += size * 0.25;
                what = h.clone();
            }
            Block::Paragraph(p) => {
                let wrapped = text::wrap(&text::runs(p), body, inner);
                y = lines(
                    &mut out,
                    &wrapped,
                    PAD,
                    y + body,
                    body,
                    "#2b2d33",
                    body * 1.42,
                );
                y += body * 0.45;
                what = p.replace("**", "");
            }
            Block::Item(i, number) => {
                let indent = body * 1.4;
                let wrapped = text::wrap(&text::runs(i), body, inner - indent);
                match number {
                    Some(n) => {
                        let _ = write!(
                            out,
                            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{body}\" font-weight=\"bold\" fill=\"#3d73cf\">{n}</text>",
                            PAD,
                            y + body
                        );
                    }
                    None => {
                        let _ = write!(
                            out,
                            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#3d73cf\"/>",
                            PAD + body * 0.35,
                            y + body * 0.66,
                            body * 0.17
                        );
                    }
                }
                y = lines(
                    &mut out,
                    &wrapped,
                    PAD + indent,
                    y + body,
                    body,
                    "#2b2d33",
                    body * 1.38,
                );
                y += body * 0.2;
                what = i.replace("**", "");
            }
            Block::Definition(d) => {
                let pad = body * 0.7;
                let wrapped = text::wrap(&text::runs(d), body, inner - 2.0 * pad - 8.0);
                let h = wrapped.len() as f32 * body * 1.38 + 2.0 * pad - body * 0.38;
                let _ = write!(
                    out,
                    "<rect x=\"{PAD}\" y=\"{y:.1}\" width=\"{inner:.1}\" height=\"{h:.1}\" rx=\"14\" fill=\"#eef4fd\"/><rect x=\"{PAD}\" y=\"{y:.1}\" width=\"8\" height=\"{h:.1}\" rx=\"4\" fill=\"#3d73cf\"/>"
                );
                lines(
                    &mut out,
                    &wrapped,
                    PAD + pad + 8.0,
                    y + pad + body * 0.8,
                    body,
                    "#1f2a44",
                    body * 1.38,
                );
                y += h + body * 0.6;
                what = d.replace("**", "");
            }
            Block::Rule => {
                let _ = write!(
                    out,
                    "<path d=\"M{PAD} {:.1} L{:.1} {:.1}\" stroke=\"#d5d9e2\" stroke-width=\"2\"/>",
                    y + body * 0.5,
                    PAD + inner,
                    y + body * 0.5 + 0.01
                );
                y += body;
            }
        }
        out.push_str("</g>");
        if !what.is_empty() {
            let short: String = what.chars().take(60).collect();
            parts.push((id, short));
        }
    }
    let height = (y + PAD * 0.6).max(200.0);
    Ok(Figure {
        svg: out,
        size: (width, height),
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn markdown_becomes_blocks() {
        let b = blocks(
            "# Title\n\nA paragraph\nthat goes on.\n\n- one\n- two\n1. first\n> a word: what it means\n---",
        );
        assert!(matches!(&b[0], Block::Heading(t, 1) if t == "Title"));
        assert!(matches!(&b[1], Block::Paragraph(t) if t == "A paragraph that goes on."));
        assert!(matches!(&b[2], Block::Item(_, None)));
        assert!(matches!(&b[4], Block::Item(_, Some(n)) if n == "1."));
        assert!(matches!(&b[5], Block::Definition(_)));
        assert!(matches!(&b[6], Block::Rule));
    }

    #[test]
    fn a_page_grows_with_its_text() {
        let short = page(&json!({"text": "# Melanocytes\n\nThey make **melanin**."})).unwrap();
        let long = page(
            &json!({"text": "# Melanocytes\n\n".to_owned() + &"They make melanin. ".repeat(60)}),
        )
        .unwrap();
        assert!(long.size.1 > short.size.1 + 100.0);
        assert!(short.svg.contains("font-weight=\"bold\">"));
        assert_eq!(short.parts[0].0, "block_1");
    }
}
