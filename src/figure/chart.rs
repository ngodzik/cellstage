//! A chart of numbers the agent gives, with where they come from: lines,
//! areas, bars, or points, on axes with round ticks. The same chart with
//! other values moves into it, so a step can make bars grow.

use std::fmt::Write;

use serde_json::Value;

use super::Figure;
use crate::drawing::text;

const WIDTH: f32 = 1400.0;
const HEIGHT: f32 = 820.0;
/// Room round the plot: left, right, top, bottom.
const LEFT: f32 = 120.0;
const RIGHT: f32 = 50.0;
const TOP: f32 = 80.0;
const BOTTOM: f32 = 110.0;
/// Colours told apart by most eyes.
const PALETTE: [&str; 8] = [
    "#3d73cf", "#e05a5a", "#3faa5a", "#ec8420", "#7a5cc4", "#2b9a73", "#c9507f", "#6b7280",
];

struct Series {
    id: String,
    name: String,
    colour: String,
    points: Vec<(f32, f32)>,
}

fn numbers(v: &Value) -> Vec<f32> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|x| x.as_f64().unwrap_or(f64::NAN) as f32)
                .collect()
        })
        .unwrap_or_default()
}

fn slug(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let s = s.trim_matches('_').to_owned();
    if s.is_empty() { "series".to_owned() } else { s }
}

/// Round ticks covering `lo..hi`: 1, 2 or 5 times a power of ten apart.
fn ticks(lo: f32, hi: f32, most: usize) -> Vec<f32> {
    let span = (hi - lo).max(1e-9);
    let raw = span / most as f32;
    let power = 10f32.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * power)
        .find(|s| span / s <= most as f32)
        .unwrap_or(10.0 * power);
    let first = (lo / step).ceil() as i64;
    let last = (hi / step).floor() as i64;
    (first..=last).map(|k| k as f32 * step).collect()
}

fn format_tick(v: f32) -> String {
    if v.abs() >= 10000.0 || (v != 0.0 && v.abs() < 0.01) {
        format!("{v:.0e}")
    } else {
        let mut s = format!("{v:.2}");
        while s.contains('.') && (s.ends_with('0') || s.ends_with('.')) {
            s.pop();
        }
        if s == "-0" { "0".to_owned() } else { s }
    }
}

/// A chart from the agent's series.
pub fn chart(args: &Value) -> Result<Figure, String> {
    let kind = args.get("kind").and_then(Value::as_str).unwrap_or("line");
    if !matches!(kind, "line" | "area" | "bar" | "scatter") {
        return Err(format!("kind {kind}: line, area, bar or scatter"));
    }
    args.get("source")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or("say where the numbers come from, in source: a paper, a database, or that they are made up to show a shape")?;
    let categories: Vec<String> = args
        .get("categories")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|c| c.as_str().map_or(c.to_string(), str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let xs = numbers(args.get("x").unwrap_or(&Value::Null));
    let mut series: Vec<Series> = Vec::new();
    for (k, s) in args
        .get("series")
        .and_then(Value::as_array)
        .ok_or("give series: [{name, values}]")?
        .iter()
        .enumerate()
    {
        let name = s
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let points: Vec<(f32, f32)> = if let Some(p) = s.get("points").and_then(Value::as_array) {
            p.iter()
                .filter_map(|q| {
                    let q = numbers(q);
                    Some((*q.first()?, *q.get(1)?))
                })
                .collect()
        } else {
            numbers(s.get("values").unwrap_or(&Value::Null))
                .into_iter()
                .enumerate()
                .map(|(i, y)| (xs.get(i).copied().unwrap_or(i as f32), y))
                .collect()
        };
        let mut id = format!("series_{}", slug(&name));
        if series.iter().any(|o| o.id == id) {
            id = format!("{id}_{}", k + 1);
        }
        series.push(Series {
            id,
            name,
            colour: s
                .get("colour")
                .and_then(Value::as_str)
                .unwrap_or(PALETTE[k % PALETTE.len()])
                .to_owned(),
            points: points
                .into_iter()
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect(),
        });
    }
    if series.iter().all(|s| s.points.is_empty()) {
        return Err("the series hold no numbers".to_owned());
    }
    let bars = kind == "bar";
    let stacked = args.get("stacked").and_then(Value::as_bool) == Some(true) && bars;
    let log = args.get("log_y").and_then(Value::as_bool) == Some(true);
    // The ranges.
    let all_y: Vec<f32> = if stacked {
        let n = series.iter().map(|s| s.points.len()).max().unwrap_or(0);
        (0..n)
            .map(|i| {
                series
                    .iter()
                    .filter_map(|s| s.points.get(i))
                    .map(|p| p.1.max(0.0))
                    .sum()
            })
            .collect()
    } else {
        series
            .iter()
            .flat_map(|s| s.points.iter().map(|p| p.1))
            .collect()
    };
    let mut y_lo = all_y.iter().copied().fold(f32::MAX, f32::min);
    let mut y_hi = all_y.iter().copied().fold(f32::MIN, f32::max);
    if bars || kind == "area" {
        y_lo = y_lo.min(0.0);
    }
    if log {
        if y_lo <= 0.0 {
            y_lo = all_y
                .iter()
                .copied()
                .filter(|v| *v > 0.0)
                .fold(f32::MAX, f32::min);
        }
        (y_lo, y_hi) = (y_lo.log10().floor(), y_hi.log10().ceil());
    } else {
        let pad = (y_hi - y_lo).abs().max(1e-6) * 0.06;
        y_hi += pad;
        if y_lo < 0.0 || !(bars || kind == "area") {
            y_lo -= pad;
        }
    }
    if let Some(v) = args.get("y_min").and_then(Value::as_f64) {
        y_lo = if log { (v as f32).log10() } else { v as f32 };
    }
    if let Some(v) = args.get("y_max").and_then(Value::as_f64) {
        y_hi = if log { (v as f32).log10() } else { v as f32 };
    }
    if y_hi <= y_lo {
        y_hi = y_lo + 1.0;
    }
    let use_categories = bars || !categories.is_empty();
    let count = series
        .iter()
        .map(|s| s.points.len())
        .max()
        .unwrap_or(0)
        .max(categories.len());
    let all_x: Vec<f32> = series
        .iter()
        .flat_map(|s| s.points.iter().map(|p| p.0))
        .collect();
    let (x_lo, x_hi) = if use_categories {
        (-0.5, count as f32 - 0.5)
    } else {
        let lo = all_x.iter().copied().fold(f32::MAX, f32::min);
        let hi = all_x.iter().copied().fold(f32::MIN, f32::max);
        let lo = args
            .get("x_min")
            .and_then(Value::as_f64)
            .map_or(lo, |v| v as f32);
        let hi = args
            .get("x_max")
            .and_then(Value::as_f64)
            .map_or(hi, |v| v as f32);
        if hi > lo {
            (lo, hi)
        } else {
            (lo - 1.0, lo + 1.0)
        }
    };
    let legend = series.len() > 1 || series.iter().any(|s| !s.name.is_empty());
    let top = TOP + if legend { 30.0 } else { 0.0 };
    let (pw, ph) = (WIDTH - LEFT - RIGHT, HEIGHT - top - BOTTOM);
    let px = |x: f32| LEFT + (x - x_lo) / (x_hi - x_lo) * pw;
    let py = |y: f32| {
        let y = if log { y.max(1e-30).log10() } else { y };
        top + ph - (y - y_lo) / (y_hi - y_lo) * ph
    };
    let mut out = String::new();
    // The grid and the axes.
    let y_ticks: Vec<f32> = if log {
        (y_lo as i32..=y_hi as i32).map(|e| 10f32.powi(e)).collect()
    } else {
        ticks(y_lo, y_hi, 6)
    };
    for t in &y_ticks {
        let y = py(*t);
        let _ = write!(
            out,
            "<path d=\"M{LEFT} {y:.1} L{:.1} {:.1}\" stroke=\"#e2e4ea\" stroke-width=\"1.5\"/><text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-size=\"20\" fill=\"#6b7280\">{}</text>",
            LEFT + pw,
            y + 0.01,
            LEFT - 12.0,
            y + 7.0,
            format_tick(*t)
        );
    }
    if use_categories {
        let slot = pw / count.max(1) as f32;
        let longest = categories
            .iter()
            .map(|c| text::width(c, 20.0, false))
            .fold(0.0, f32::max);
        let turn = longest > slot * 0.95;
        for (i, c) in categories.iter().enumerate() {
            let x = px(i as f32);
            let y = top + ph + 30.0;
            if turn {
                let _ = write!(
                    out,
                    "<text x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"end\" font-size=\"20\" fill=\"#3a3f4b\" transform=\"rotate(-30 {x:.1} {y:.1})\">{}</text>",
                    text::escape(c)
                );
            } else {
                let _ = write!(
                    out,
                    "<text x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" font-size=\"20\" fill=\"#3a3f4b\">{}</text>",
                    text::escape(c)
                );
            }
        }
    } else {
        for t in ticks(x_lo, x_hi, 8) {
            let x = px(t);
            let _ = write!(
                out,
                "<path d=\"M{x:.1} {top} L{:.1} {:.1}\" stroke=\"#eef0f4\" stroke-width=\"1.5\"/><text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"20\" fill=\"#6b7280\">{}</text>",
                x + 0.01,
                top + ph,
                top + ph + 30.0,
                format_tick(t)
            );
        }
    }
    let _ = write!(
        out,
        "<path d=\"M{LEFT} {top} L{LEFT} {:.1} L{:.1} {:.1}\" fill=\"none\" stroke=\"#3a3f4b\" stroke-width=\"2.5\"/>",
        top + ph,
        LEFT + pw,
        top + ph + 0.01
    );
    // The series.
    let slot = pw / count.max(1) as f32;
    let groups = if stacked { 1 } else { series.len().max(1) };
    let bar_w = slot * 0.75 / groups as f32;
    let mut stack = vec![0.0f32; count];
    for (k, s) in series.iter().enumerate() {
        let _ = write!(
            out,
            "<g id=\"{}\" data-what=\"{}\">",
            s.id,
            text::escape(&s.name)
        );
        match kind {
            "bar" => {
                for (i, (_, y)) in s.points.iter().enumerate() {
                    let base = if stacked { stack[i] } else { 0.0 };
                    let x =
                        px(i as f32) - slot * 0.375 + bar_w * if stacked { 0.0 } else { k as f32 };
                    // On a log axis the bars stand on the lowest tick.
                    let floor = if log { 10f32.powf(y_lo) } else { y_lo };
                    let (y0, y1) = (py(base.max(floor)), py(base + y));
                    let _ = write!(
                        out,
                        "<rect id=\"{}_{}\" x=\"{x:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"{}\"/>",
                        s.id,
                        i + 1,
                        y1.min(y0),
                        bar_w * 0.92,
                        (y0 - y1).abs(),
                        s.colour
                    );
                    if stacked {
                        stack[i] += y.max(0.0);
                    }
                }
            }
            _ => {
                let at: Vec<(f32, f32)> = s
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, (x, y))| (px(if use_categories { i as f32 } else { *x }), py(*y)))
                    .collect();
                let line: String = at
                    .iter()
                    .enumerate()
                    .map(|(i, (x, y))| format!("{}{x:.1} {y:.1}", if i == 0 { "M" } else { " L" }))
                    .collect();
                if kind == "area" && at.len() > 1 {
                    let floor = py(if log { 10f32.powf(y_lo) } else { y_lo.max(0.0) });
                    let _ = write!(
                        out,
                        "<path d=\"{line} L{:.1} {floor:.1} L{:.1} {floor:.1} Z\" fill=\"{}\" fill-opacity=\"0.22\"/>",
                        at[at.len() - 1].0,
                        at[0].0,
                        s.colour
                    );
                }
                if kind != "scatter" && at.len() > 1 {
                    let _ = write!(
                        out,
                        "<path d=\"{line}\" fill=\"none\" stroke=\"{}\" stroke-width=\"4\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
                        s.colour
                    );
                }
                let r = if kind == "scatter" { 7.0 } else { 5.0 };
                for (i, (x, y)) in at.iter().enumerate() {
                    let _ = write!(
                        out,
                        "<circle id=\"{}_{}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{r}\" fill=\"{}\" stroke=\"#ffffff\" stroke-width=\"1.5\"/>",
                        s.id,
                        i + 1,
                        s.colour
                    );
                }
            }
        }
        out.push_str("</g>");
    }
    // Words: the axes' names, the legend, the heading.
    if let Some(label) = args.get("x_label").and_then(Value::as_str) {
        let _ = write!(
            out,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"22\" fill=\"#3a3f4b\">{}</text>",
            LEFT + pw / 2.0,
            HEIGHT - 22.0,
            text::escape(label)
        );
    }
    if let Some(label) = args.get("y_label").and_then(Value::as_str) {
        let (x, y) = (32.0, top + ph / 2.0);
        let _ = write!(
            out,
            "<text x=\"{x}\" y=\"{y:.1}\" text-anchor=\"middle\" font-size=\"22\" fill=\"#3a3f4b\" transform=\"rotate(-90 {x} {y:.1})\">{}</text>",
            text::escape(label)
        );
    }
    if legend {
        let mut x = LEFT;
        for s in &series {
            let _ = write!(
                out,
                "<rect x=\"{x:.1}\" y=\"{:.1}\" width=\"22\" height=\"14\" rx=\"3\" fill=\"{}\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"20\" fill=\"#3a3f4b\">{}</text>",
                TOP - 8.0,
                s.colour,
                x + 30.0,
                TOP + 5.0,
                text::escape(&s.name)
            );
            x += 30.0 + text::width(&s.name, 20.0, false) + 34.0;
        }
    }
    if let Some(title) = args.get("heading").and_then(Value::as_str) {
        let _ = write!(
            out,
            "<text x=\"{:.1}\" y=\"44\" text-anchor=\"middle\" class=\"title\">{}</text>",
            WIDTH / 2.0,
            text::escape(title)
        );
    }
    let parts = series
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();
    Ok(Figure {
        svg: out,
        size: (WIDTH, HEIGHT),
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ticks_are_round() {
        assert_eq!(ticks(0.0, 10.0, 6), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(ticks(0.0, 1.0, 5), vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
        assert_eq!(format_tick(0.1 + 0.2), "0.3");
    }

    #[test]
    fn charts_draw_and_name_their_series() {
        let line = chart(&json!({
            "kind": "line", "source": "made up to show the shape",
            "x": [0, 1, 2, 3], "series": [{"name": "p53", "values": [1, 3, 2, 4]}, {"name": "MDM2", "values": [0, 1, 3, 2]}]
        }))
        .unwrap();
        assert!(line.svg.contains("<g id=\"series_p53\""));
        assert!(line.svg.contains("<g id=\"series_mdm2\""));
        assert!(
            line.svg.contains("id=\"series_mdm2_2\""),
            "the second point of MDM2"
        );
        let bars = chart(&json!({
            "kind": "bar", "source": "x", "categories": ["a", "b"], "stacked": true,
            "series": [{"name": "one", "values": [1, 2]}, {"name": "two", "values": [3, 1]}]
        }))
        .unwrap();
        assert_eq!(bars.svg.matches("<rect id=").count(), 4);
        assert!(
            chart(&json!({"series": [{"values": [1]}]})).is_err(),
            "a source is needed"
        );
    }

    #[test]
    fn the_same_chart_with_other_values_can_move() {
        let a = chart(&json!({"kind": "bar", "source": "x", "categories": ["a"], "y_max": 10, "series": [{"name": "n", "values": [2]}]})).unwrap();
        let b = chart(&json!({"kind": "bar", "source": "x", "categories": ["a"], "y_max": 10, "series": [{"name": "n", "values": [8]}]})).unwrap();
        let strip = |s: &str| {
            s.chars()
                .filter(|c| !c.is_ascii_digit() && *c != '.')
                .collect::<String>()
        };
        assert_eq!(strip(&a.svg), strip(&b.svg));
    }
}
