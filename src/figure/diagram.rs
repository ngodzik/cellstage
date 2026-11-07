//! A diagram laid out by the window: what activates, inhibits, becomes or
//! binds what, a pathway, a lineage, a sequence of events. The agent says
//! the nodes and the edges; the window sets them in layers along the flow,
//! orders each layer to cross as few edges as it can, and routes the edges
//! round the nodes.

use std::collections::HashMap;
use std::fmt::Write;

use serde_json::Value;

use super::Figure;
use crate::drawing::{macros::shape_size, text};

const FONT: f32 = 22.0;
/// The widest a node's words run before they wrap.
const WRAP: f32 = 210.0;
/// Room between layers, and between nodes of a layer.
const LAYER_GAP: f32 = 110.0;
const NODE_GAP: f32 = 34.0;
const MARGIN: f32 = 50.0;

/// How a node of each kind looks: fill, outline, and whether it is dashed.
fn look(kind: &str) -> (&'static str, &'static str, bool) {
    match kind {
        "receptor" => ("#d4f2e6", "#2b9a73", false),
        "ligand" | "molecule" | "ion" | "hormone" => ("#ffe8cc", "#ec8420", false),
        "gene" | "dna" => ("#e3f4d9", "#4f9a3a", false),
        "rna" => ("#fbe0ea", "#c9507f", false),
        "complex" => ("#e6defa", "#7a5cc4", false),
        "cell" => ("#fff3e8", "#c98b74", false),
        "tissue" | "organ" => ("#fde2e0", "#c9534b", false),
        "process" => ("#eceef2", "#6b7280", false),
        "event" | "state" => ("#ffffff", "#8a93a6", true),
        "outcome" => ("#fbe3e3", "#c62828", false),
        "microbe" | "virus" | "bacterium" => ("#e6f3d0", "#6d9a2c", false),
        _ => ("#dbe8fc", "#4479d3", false),
    }
}

/// How an edge of each kind is drawn: colour, width, dash, and its end.
fn edge_look(kind: &str) -> (&'static str, f32, &'static str, End) {
    match kind {
        "inhibits" | "blocks" => ("#c62828", 3.0, "", End::Bar),
        "becomes" | "differentiates" | "divides" => ("#5b6b8c", 4.5, "", End::Arrow),
        "binds" => ("#7a5cc4", 3.0, "7 6", End::None),
        "makes" | "produces" | "transcribes" | "secretes" => ("#4f9a3a", 3.0, "10 6", End::Arrow),
        "moves" | "travels" => ("#3faa5a", 3.5, "2 7", End::Arrow),
        "line" => ("#9aa3b5", 2.5, "", End::None),
        _ => ("#2b2d33", 3.0, "", End::Arrow),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum End {
    Arrow,
    Bar,
    None,
}

struct Node {
    id: String,
    lines: Vec<String>,
    kind: String,
    shape: Option<(String, f32)>,
    fill: Option<String>,
    what: String,
    /// Its size along and across the flow, and its place.
    w: f32,
    h: f32,
    x: f32,
    y: f32,
    layer: usize,
    pinned: Option<usize>,
    /// A point an edge passes through on its way across layers.
    dummy: bool,
}

struct Edge {
    from: usize,
    to: usize,
    kind: String,
    label: Option<String>,
    /// The nodes it runs through, dummies included, from its start.
    chain: Vec<usize>,
    /// Laid out against its direction, to break a cycle.
    reversed: bool,
}

fn safe_id(id: &str) -> String {
    id.trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// A diagram from the agent's nodes and edges.
pub fn diagram(args: &Value) -> Result<Figure, String> {
    let down = matches!(
        args.get("direction").and_then(Value::as_str),
        Some("down" | "top-down" | "vertical")
    );
    let mut nodes: Vec<Node> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let list = args
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("give nodes: [{id, label, kind}]")?;
    for n in list {
        let id = n
            .get("id")
            .and_then(Value::as_str)
            .ok_or("every node needs an id")?;
        let label = n.get("label").and_then(Value::as_str).unwrap_or(id);
        add_node(&mut nodes, &mut index, id, label, n)?;
    }
    let mut edges: Vec<Edge> = Vec::new();
    for e in args
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let from = e
            .get("from")
            .and_then(Value::as_str)
            .ok_or("an edge needs from")?;
        let to = e
            .get("to")
            .and_then(Value::as_str)
            .ok_or("an edge needs to")?;
        for end in [from, to] {
            if !index.contains_key(&safe_id(end)) {
                add_node(&mut nodes, &mut index, end, end, &Value::Null)?;
            }
        }
        edges.push(Edge {
            from: index[&safe_id(from)],
            to: index[&safe_id(to)],
            kind: e
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("activates")
                .to_owned(),
            label: e.get("label").and_then(Value::as_str).map(str::to_owned),
            chain: Vec::new(),
            reversed: false,
        });
    }
    if nodes.is_empty() {
        return Err("a diagram needs at least one node".to_owned());
    }
    // Sizes, along the flow first.
    for n in nodes.iter_mut() {
        let wide = n
            .lines
            .iter()
            .map(|l| text::width(l, FONT, false))
            .fold(0.0, f32::max);
        let words_h = n.lines.len() as f32 * FONT * 1.25;
        let (bw, bh) = match &n.shape {
            Some(_) => (wide.max(90.0) + 10.0, 84.0 + words_h),
            None => (wide + 36.0, words_h + 22.0),
        };
        (n.w, n.h) = if down { (bh, bw) } else { (bw, bh) };
    }
    layer(&mut nodes, &mut edges);
    let layers = order(&mut nodes, &mut edges);
    position(&mut nodes, &edges, &layers);
    // From flow coordinates to the page: along is x when the flow runs
    // right, y when it runs down.
    if down {
        for n in nodes.iter_mut() {
            (n.x, n.y) = (n.y, n.x);
            (n.w, n.h) = (n.h, n.w);
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for n in &nodes {
        x0 = x0.min(n.x - n.w / 2.0);
        y0 = y0.min(n.y - n.h / 2.0);
        x1 = x1.max(n.x + n.w / 2.0);
        y1 = y1.max(n.y + n.h / 2.0);
    }
    let shift = (MARGIN - x0, MARGIN + 30.0 - y0);
    for n in nodes.iter_mut() {
        n.x += shift.0;
        n.y += shift.1;
    }
    let size = ((x1 - x0) + 2.0 * MARGIN, (y1 - y0) + 2.0 * MARGIN + 30.0);
    let svg = draw(args, &nodes, &edges, down, size)?;
    let mut parts: Vec<(String, String)> = nodes
        .iter()
        .filter(|n| !n.dummy)
        .map(|n| (n.id.clone(), n.what.clone()))
        .collect();
    for e in &edges {
        parts.push((
            edge_id(&nodes, e),
            format!(
                "{} {} {}",
                nodes[e.from].lines.join(" "),
                e.kind,
                nodes[e.to].lines.join(" ")
            ),
        ));
    }
    Ok(Figure { svg, size, parts })
}

fn add_node(
    nodes: &mut Vec<Node>,
    index: &mut HashMap<String, usize>,
    id: &str,
    label: &str,
    n: &Value,
) -> Result<(), String> {
    let id = safe_id(id);
    if index.contains_key(&id) {
        return Err(format!("two nodes have the id {id}"));
    }
    let shape = match n.get("shape").and_then(Value::as_str) {
        Some(s) => {
            let s = s.trim_start_matches('#');
            let (w, h) = shape_size(s)
                .ok_or_else(|| format!("node {id}: no house shape {s}; drawing_kit lists them"))?;
            Some((s.to_owned(), (70.0 / w).min(64.0 / h)))
        }
        None => None,
    };
    let lines = text::wrap(&[(label.to_owned(), false)], FONT, WRAP)
        .into_iter()
        .map(|l| l.into_iter().map(|(t, _)| t).collect::<Vec<_>>().join(" "))
        .collect();
    index.insert(id.clone(), nodes.len());
    nodes.push(Node {
        id,
        lines,
        kind: n
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("protein")
            .to_owned(),
        shape,
        fill: n.get("colour").and_then(Value::as_str).map(str::to_owned),
        what: n
            .get("what")
            .and_then(Value::as_str)
            .unwrap_or(label)
            .to_owned(),
        w: 0.0,
        h: 0.0,
        x: 0.0,
        y: 0.0,
        layer: 0,
        pinned: n.get("layer").and_then(Value::as_u64).map(|l| l as usize),
        dummy: false,
    });
    Ok(())
}

/// Every node in a layer: edges run from lower layers to higher, those
/// that close a cycle laid out backwards; a layer the agent gives is kept.
fn layer(nodes: &mut [Node], edges: &mut [Edge]) {
    // Break cycles: an edge that reaches back to a node on the path being
    // walked is laid out backwards.
    let n = nodes.len();
    let mut state = vec![0u8; n];
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (k, e) in edges.iter().enumerate() {
        out[e.from].push(k);
    }
    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        let mut stack = vec![(start, 0usize)];
        state[start] = 1;
        while let Some((v, i)) = stack.pop() {
            if i < out[v].len() {
                stack.push((v, i + 1));
                let k = out[v][i];
                let w = edges[k].to;
                match state[w] {
                    0 => {
                        state[w] = 1;
                        stack.push((w, 0));
                    }
                    1 => edges[k].reversed = true,
                    _ => {}
                }
            } else {
                state[v] = 2;
            }
        }
    }
    // Longest path from the sources.
    for _ in 0..n + 1 {
        let mut moved = false;
        for e in edges.iter() {
            let (a, b) = if e.reversed {
                (e.to, e.from)
            } else {
                (e.from, e.to)
            };
            if a == b {
                continue;
            }
            if nodes[b].pinned.is_none() && nodes[b].layer < nodes[a].layer + 1 {
                nodes[b].layer = nodes[a].layer + 1;
                moved = true;
            }
        }
        for node in nodes.iter_mut() {
            if let Some(p) = node.pinned {
                node.layer = p;
            }
        }
        if !moved {
            break;
        }
    }
}

/// The layers, each in an order that crosses few edges, with a dummy node
/// wherever an edge crosses a layer.
fn order(nodes: &mut Vec<Node>, edges: &mut [Edge]) -> Vec<Vec<usize>> {
    for e in edges.iter_mut() {
        let (a, b) = if e.reversed {
            (e.to, e.from)
        } else {
            (e.from, e.to)
        };
        e.chain = vec![a];
        let (la, lb) = (nodes[a].layer, nodes[b].layer);
        if lb > la + 1 {
            for l in la + 1..lb {
                nodes.push(Node {
                    id: String::new(),
                    lines: Vec::new(),
                    kind: String::new(),
                    shape: None,
                    fill: None,
                    what: String::new(),
                    w: 8.0,
                    h: 8.0,
                    x: 0.0,
                    y: 0.0,
                    layer: l,
                    pinned: None,
                    dummy: true,
                });
                e.chain.push(nodes.len() - 1);
            }
        }
        e.chain.push(b);
    }
    let deepest = nodes.iter().map(|n| n.layer).max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); deepest + 1];
    for (k, n) in nodes.iter().enumerate() {
        layers[n.layer].push(k);
    }
    // Who each node meets in the layer before and after.
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut down: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for e in edges.iter() {
        for w in e.chain.windows(2) {
            if nodes[w[0]].layer < nodes[w[1]].layer {
                down[w[0]].push(w[1]);
                up[w[1]].push(w[0]);
            } else if nodes[w[0]].layer > nodes[w[1]].layer {
                down[w[1]].push(w[0]);
                up[w[0]].push(w[1]);
            }
        }
    }
    let mut rank = vec![0.0f32; nodes.len()];
    let set_ranks = |layers: &Vec<Vec<usize>>, rank: &mut Vec<f32>| {
        for l in layers {
            for (i, &v) in l.iter().enumerate() {
                rank[v] = i as f32;
            }
        }
    };
    set_ranks(&layers, &mut rank);
    for sweep in 0..12 {
        let downward = sweep % 2 == 0;
        let range: Vec<usize> = if downward {
            (1..layers.len()).collect()
        } else {
            (0..layers.len().saturating_sub(1)).rev().collect()
        };
        for l in range {
            let neighbours = if downward { &up } else { &down };
            let mut keyed: Vec<(f32, usize)> = layers[l]
                .iter()
                .map(|&v| {
                    let ns = &neighbours[v];
                    let key = if ns.is_empty() {
                        rank[v]
                    } else {
                        ns.iter().map(|&u| rank[u]).sum::<f32>() / ns.len() as f32
                    };
                    (key, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
            layers[l] = keyed.into_iter().map(|(_, v)| v).collect();
            for (i, &v) in layers[l].iter().enumerate() {
                rank[v] = i as f32;
            }
        }
    }
    layers
}

/// Places: each layer a column (in flow coordinates, x along the flow),
/// each node drawn towards the nodes it meets, without overlapping.
fn position(nodes: &mut [Node], edges: &[Edge], layers: &[Vec<usize>]) {
    let mut along = 0.0;
    for l in layers {
        let thick = l.iter().map(|&v| nodes[v].w).fold(0.0, f32::max);
        for &v in l {
            nodes[v].x = along + thick / 2.0;
        }
        along += thick + LAYER_GAP;
    }
    for l in layers {
        let mut y = 0.0;
        for &v in l {
            nodes[v].y = y + nodes[v].h / 2.0;
            y += nodes[v].h + NODE_GAP;
        }
    }
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for e in edges {
        for w in e.chain.windows(2) {
            neighbours[w[0]].push(w[1]);
            neighbours[w[1]].push(w[0]);
        }
    }
    for _ in 0..40 {
        for l in layers {
            // Where each would like to be.
            let wanted: Vec<f32> = l
                .iter()
                .map(|&v| {
                    let ns = &neighbours[v];
                    if ns.is_empty() {
                        nodes[v].y
                    } else {
                        ns.iter().map(|&u| nodes[u].y).sum::<f32>() / ns.len() as f32
                    }
                })
                .collect();
            for (i, &v) in l.iter().enumerate() {
                nodes[v].y += (wanted[i] - nodes[v].y) * 0.5;
            }
            // Pushed apart, in order, from the top and then from the
            // bottom.
            for i in 1..l.len() {
                let (a, b) = (l[i - 1], l[i]);
                let least = nodes[a].y + (nodes[a].h + nodes[b].h) / 2.0 + NODE_GAP;
                if nodes[b].y < least {
                    nodes[b].y = least;
                }
            }
            for i in (1..l.len()).rev() {
                let (a, b) = (l[i - 1], l[i]);
                let most = nodes[b].y - (nodes[a].h + nodes[b].h) / 2.0 - NODE_GAP;
                if nodes[a].y > most {
                    nodes[a].y = most;
                }
            }
        }
    }
}

fn edge_id(nodes: &[Node], e: &Edge) -> String {
    format!("{}_to_{}", nodes[e.from].id, nodes[e.to].id)
}

fn draw(
    args: &Value,
    nodes: &[Node],
    edges: &[Edge],
    down: bool,
    size: (f32, f32),
) -> Result<String, String> {
    let list = |key: &str| -> Vec<String> {
        args.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(safe_id).collect())
            .unwrap_or_default()
    };
    let (lit, dim) = (list("highlight"), list("dim"));
    let mut out = String::new();
    // Markers in each edge colour.
    out.push_str("<defs>");
    for colour in ["#2b2d33", "#5b6b8c", "#4f9a3a", "#3faa5a", "#c62828"] {
        let id = &colour[1..];
        let _ = write!(
            out,
            "<marker id=\"d-arrow-{id}\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" markerWidth=\"5\" markerHeight=\"5\" orient=\"auto-start-reverse\"><path d=\"M0 0 L10 5 L0 10 L2.5 5 Z\" fill=\"{colour}\"/></marker><marker id=\"d-bar-{id}\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\"><path d=\"M5 0 L5 10\" stroke=\"{colour}\" stroke-width=\"3\"/></marker>"
        );
    }
    out.push_str("</defs>");
    // Groups behind the nodes.
    for g in args
        .get("groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let members: Vec<&Node> = g
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|id| nodes.iter().find(|n| !n.dummy && n.id == safe_id(id)))
            .collect();
        if members.is_empty() {
            continue;
        }
        let pad = 18.0;
        let x0 = members
            .iter()
            .map(|n| n.x - n.w / 2.0)
            .fold(f32::MAX, f32::min)
            - pad;
        let y0 = members
            .iter()
            .map(|n| n.y - n.h / 2.0)
            .fold(f32::MAX, f32::min)
            - pad
            - 26.0;
        let x1 = members
            .iter()
            .map(|n| n.x + n.w / 2.0)
            .fold(f32::MIN, f32::max)
            + pad;
        let y1 = members
            .iter()
            .map(|n| n.y + n.h / 2.0)
            .fold(f32::MIN, f32::max)
            + pad;
        let label = g.get("label").and_then(Value::as_str).unwrap_or("");
        let id = g
            .get("id")
            .and_then(Value::as_str)
            .map(safe_id)
            .unwrap_or_else(|| format!("group_{}", safe_id(label)));
        let _ = write!(
            out,
            "<g id=\"{id}\" data-what=\"{}\"><rect x=\"{x0:.1}\" y=\"{y0:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"22\" fill=\"{}\" fill-opacity=\"0.55\" stroke=\"#b9bfcc\" stroke-width=\"2\" stroke-dasharray=\"8 6\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"20\" font-weight=\"bold\" fill=\"#6b7280\">{}</text></g>",
            text::escape(label),
            x1 - x0,
            y1 - y0,
            g.get("colour").and_then(Value::as_str).unwrap_or("#eef1f6"),
            x0 + 14.0,
            y0 + 24.0,
            text::escape(label)
        );
    }
    // Edges under the nodes.
    for e in edges {
        let mut points: Vec<(f32, f32)> =
            e.chain.iter().map(|&k| (nodes[k].x, nodes[k].y)).collect();
        if e.reversed {
            points.reverse();
        }
        let (a, b) = (&nodes[e.from], &nodes[e.to]);
        let (colour, width, dash, end) = edge_look(&e.kind);
        let faded = dim.contains(&a.id) || dim.contains(&b.id);
        let d = if e.from == e.to {
            let (x, y) = (a.x, a.y - a.h / 2.0);
            format!(
                "M{:.1} {y:.1} C{:.1} {:.1} {:.1} {:.1} {:.1} {y:.1}",
                x - 20.0,
                x - 40.0,
                y - 60.0,
                x + 40.0,
                y - 60.0,
                x + 20.0
            )
        } else {
            let n = points.len();
            points[0] = border(a, points[1], down);
            points[n - 1] = border(b, points[n - 2], down);
            smooth(&points, down)
        };
        let marker = match end {
            End::Arrow => format!(" marker-end=\"url(#d-arrow-{})\"", &colour[1..]),
            End::Bar => format!(" marker-end=\"url(#d-bar-{})\"", &colour[1..]),
            End::None => String::new(),
        };
        let dash = if dash.is_empty() {
            String::new()
        } else {
            format!(" stroke-dasharray=\"{dash}\"")
        };
        let _ = write!(
            out,
            "<g id=\"{}\" opacity=\"{}\"><path d=\"{d}\" fill=\"none\" stroke=\"{colour}\" stroke-width=\"{width}\" stroke-linecap=\"round\"{dash}{marker}/>",
            edge_id(nodes, e),
            if faded { 0.2 } else { 1.0 }
        );
        if let Some(label) = &e.label {
            let mid = e.chain.len() / 2;
            let (mx, my) = if e.chain.len() % 2 == 1 {
                (nodes[e.chain[mid]].x, nodes[e.chain[mid]].y)
            } else {
                let (p, q) = (&nodes[e.chain[mid - 1]], &nodes[e.chain[mid]]);
                ((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)
            };
            let _ = write!(
                out,
                "<text x=\"{mx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"18\" font-style=\"italic\" fill=\"{colour}\" stroke=\"#f7f5f0\" stroke-width=\"6\" stroke-linejoin=\"round\" paint-order=\"stroke\">{}</text>",
                my - 8.0,
                text::escape(label)
            );
        }
        out.push_str("</g>");
    }
    for n in nodes.iter().filter(|n| !n.dummy) {
        let (fill, line, dashed) = look(&n.kind);
        let fill = n.fill.as_deref().unwrap_or(fill);
        let opacity = if dim.contains(&n.id) { 0.25 } else { 1.0 };
        let ring = if lit.contains(&n.id) { 7.0 } else { 0.0 };
        let (x0, y0) = (n.x - n.w / 2.0, n.y - n.h / 2.0);
        let _ = write!(
            out,
            "<g id=\"{}\" data-what=\"{}\" opacity=\"{opacity}\"><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" fill=\"none\" stroke=\"rgb(255,179,0)\" stroke-opacity=\"0.8\" stroke-width=\"{ring}\"/>",
            n.id,
            text::escape(&n.what),
            x0 - 6.0,
            y0 - 6.0,
            n.w + 12.0,
            n.h + 12.0,
            if n.shape.is_some() {
                20.0
            } else {
                (n.h / 2.0 + 6.0).min(26.0)
            }
        );
        let words_top = match &n.shape {
            Some((shape, scale)) => {
                let _ = write!(
                    out,
                    "<use href=\"#{shape}\" transform=\"translate({:.1} {:.1}) scale({scale:.3})\"/>",
                    n.x,
                    y0 + 40.0
                );
                y0 + 84.0
            }
            None => {
                let dash = if dashed {
                    " stroke-dasharray=\"6 5\""
                } else {
                    ""
                };
                let _ = write!(
                    out,
                    "<rect x=\"{x0:.1}\" y=\"{y0:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" fill=\"{fill}\" stroke=\"{line}\" stroke-width=\"2.5\"{dash}/>",
                    n.w,
                    n.h,
                    (n.h / 2.0).min(20.0)
                );
                y0 + 11.0
            }
        };
        for (i, l) in n.lines.iter().enumerate() {
            let _ = write!(
                out,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"{FONT}\" fill=\"#23262d\">{}</text>",
                n.x,
                words_top + FONT * (1.25 * i as f32 + 0.95),
                text::escape(l)
            );
        }
        out.push_str("</g>");
    }
    if let Some(title) = args.get("heading").and_then(Value::as_str) {
        let _ = write!(
            out,
            "<text x=\"{:.1}\" y=\"40\" text-anchor=\"middle\" class=\"title\">{}</text>",
            size.0 / 2.0,
            text::escape(title)
        );
    }
    Ok(out)
}

/// Where the line from a node's centre towards `to` leaves its box.
fn border(n: &Node, to: (f32, f32), down: bool) -> (f32, f32) {
    let (dx, dy) = (to.0 - n.x, to.1 - n.y);
    let (hw, hh) = (n.w / 2.0 + 4.0, n.h / 2.0 + 4.0);
    // Leave from the side facing the flow when the target lies that way.
    if down && dy.abs() > 1.0 {
        return (
            n.x + dx.clamp(-hw * 0.5, hw * 0.5) * 0.3,
            n.y + hh * dy.signum(),
        );
    }
    if !down && dx.abs() > 1.0 {
        return (
            n.x + hw * dx.signum(),
            n.y + dy.clamp(-hh * 0.5, hh * 0.5) * 0.3,
        );
    }
    let t = (hw / dx.abs().max(1e-3)).min(hh / dy.abs().max(1e-3));
    (n.x + dx * t, n.y + dy * t)
}

/// A smooth path through points, leaving and arriving along the flow.
fn smooth(points: &[(f32, f32)], down: bool) -> String {
    let mut d = format!("M{:.1} {:.1}", points[0].0, points[0].1);
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (c1, c2) = if down {
            let m = (b.1 - a.1) * 0.5;
            ((a.0, a.1 + m), (b.0, b.1 - m))
        } else {
            let m = (b.0 - a.0) * 0.5;
            ((a.0 + m, a.1), (b.0 - m, b.1))
        };
        let _ = write!(
            d,
            " C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
            c1.0, c1.1, c2.0, c2.1, b.0, b.1
        );
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_pathway_lays_out_in_layers() {
        let f = diagram(&json!({
            "nodes": [
                {"id": "egf", "label": "EGF", "kind": "ligand"},
                {"id": "egfr", "label": "EGFR", "kind": "receptor"},
                {"id": "ras", "label": "RAS"},
                {"id": "erk", "label": "ERK"},
                {"id": "growth", "label": "the cell grows and divides", "kind": "outcome"}
            ],
            "edges": [
                {"from": "egf", "to": "egfr", "kind": "binds"},
                {"from": "egfr", "to": "ras"},
                {"from": "ras", "to": "erk"},
                {"from": "erk", "to": "growth"},
                {"from": "egf", "to": "growth", "kind": "line"}
            ]
        }))
        .unwrap();
        assert!(
            f.size.0 > f.size.1,
            "a flow to the right is wide: {:?}",
            f.size
        );
        for id in ["egf", "egfr", "ras", "erk", "growth", "ras_to_erk"] {
            assert!(f.svg.contains(&format!("id=\"{id}\"")), "{id}");
        }
    }

    #[test]
    fn cycles_and_unknown_ends_do_not_break_it() {
        let f = diagram(&json!({
            "direction": "down",
            "nodes": [{"id": "p53"}, {"id": "mdm2"}],
            "edges": [
                {"from": "p53", "to": "mdm2", "kind": "makes"},
                {"from": "mdm2", "to": "p53", "kind": "inhibits"},
                {"from": "p53", "to": "arrest", "kind": "activates"}
            ]
        }))
        .unwrap();
        assert!(f.svg.contains("id=\"arrest\""));
        assert!(f.svg.contains("d-bar-c62828"));
    }

    #[test]
    fn a_lineage_uses_shapes() {
        let f = diagram(&json!({
            "direction": "down",
            "nodes": [
                {"id": "hsc", "label": "stem cell", "shape": "blast"},
                {"id": "red", "label": "red cell", "shape": "rbc"}
            ],
            "edges": [{"from": "hsc", "to": "red", "kind": "becomes"}]
        }))
        .unwrap();
        assert!(f.svg.contains("href=\"#rbc\""));
        assert!(diagram(&json!({"nodes": [{"id": "x", "shape": "unicorn"}]})).is_err());
    }
}
