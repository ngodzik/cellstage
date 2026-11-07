//! What the agent's calls do: lay the stage out, fill its panels, write on
//! them, and tell a story step by step. Every call that changes what a step
//! shows is kept in the story, so that the step can be shown again when the
//! user comes back to it, today or another day.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_egui::egui;
use serde_json::{Map, Value};

use crate::cell::dish::{Dish, Fate, GENES, Switch, label};
use crate::cell::specimen::{self, Compartment, Specimen};
use crate::drawing::{self, Canvas, Film, Part, Picture, Sheet};
use crate::figure::{self, Figure};
use crate::protein::{self, Highlight, Look};
use crate::scene::{self, Scenes, Style};
use crate::sources;
use crate::stage::{Anchor, Arrangement, Kind, MAX_PANELS, Note, Panel, Stage};
use crate::story::{self, Call, Drawing, Found, PanelSpec, Step, Story};
use crate::tissue::skin::{self, Lesion, Section};

/// A cell panel: the dish, the microscope, the picture.
pub struct CellView {
    pub dish: Dish,
    pub specimen: Specimen,
    pub stains: [Option<usize>; 3],
    pub running: bool,
    pub owed: f32,
    pub since: f32,
    pub frame: u32,
    pub image: Handle<Image>,
    /// A point in each compartment, in pixels, for notes to point at.
    pub places: Vec<(String, (f32, f32))>,
}

/// A skin panel: the section, and a point in each region.
pub struct SkinView {
    pub section: Section,
    pub places: Vec<(String, (f32, f32))>,
}

/// A drawing panel: its frames, where it is in them, its picture.
pub struct DrawView {
    pub film: Film,
    /// Its size in drawing units.
    pub size: (f32, f32),
    pub clock: f32,
    pub playing: bool,
    pub background: [u8; 3],
    pub image: Handle<Image>,
    /// Its named parts as last rendered, in drawing units.
    pub parts: Vec<Part>,
    /// What the agent said its parts are.
    pub described: Vec<(String, String)>,
    /// What was last rendered, so that nothing is rendered twice.
    pub last: Option<(f32, Canvas)>,
}

#[derive(Resource, Default)]
pub struct Views {
    pub cells: HashMap<String, CellView>,
    pub skins: HashMap<String, SkinView>,
    pub drawings: HashMap<String, DrawView>,
}

/// Whether a call is the agent's, to be kept in the story, or the stage
/// showing a step again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Record,
    Replay,
}

pub fn text(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn number(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(Value::as_f64)
}

/// A colour as #rrggbb, in sRGB.
fn colour(hex: &str) -> Option<[f32; 3]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let byte = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16)
            .ok()
            .map(|b| f32::from(b) / 255.0)
    };
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// Do one call, and say what was done.
pub fn handle(world: &mut World, tool: &str, args: &Value, mode: Mode) -> Result<String, String> {
    match tool {
        "story_new" => story_new(world, args),
        "story_step" => story_step(world, args),
        "story_edit" => story_edit(world, args),
        "story_goto" => {
            let n = step_number(world, args)?;
            let forward = n == world.resource::<Stage>().step + 1;
            show_step(world, n, forward);
            Ok(format!("Showing step {}. {}", n + 1, step_summary(world)))
        }
        "story_remove_step" => story_remove_step(world, args),
        "story_open" => story_open(world, args),
        "library_search" => Ok(library_search(args)),
        "image_search" => image_search(args),
        "protein_search" => sources::proteins(&text(args, "query").ok_or("search for what?")?),
        "pathway_lookup" => sources::reactome(
            &text(args, "query")
                .or(text(args, "id"))
                .ok_or("give a subject, or a Reactome id such as R-HSA-109581")?,
        ),
        "get_user_events" => {
            let events: Vec<String> = std::mem::take(&mut world.resource_mut::<Stage>().events);
            Ok(if events.is_empty() {
                "The user has done nothing in the window since you last asked.".to_owned()
            } else {
                events.join("\n")
            })
        }
        _ => {
            let args = if tool == "draw" && mode == Mode::Record {
                resolve_drawing(args)?
            } else {
                args.clone()
            };
            if mode == Mode::Record {
                ensure_story(world);
            }
            let answer = match tool {
                "stage_layout" => layout(world, &args),
                "show_protein" => show_protein(world, &args),
                "show_cell" => show_cell(world, &args),
                "show_skin" => show_skin(world, &args),
                "draw" => draw(world, &args),
                "show_diagram" => show_diagram(world, &args),
                "show_chart" => show_chart(world, &args),
                "show_text" => show_text(world, &args),
                "show_image" => show_image(world, &args),
                "show_world" => show_world(world, &args),
                "annotate" => annotate(world, &args),
                "clear_annotations" => {
                    let only = text(&args, "panel");
                    for panel in world.resource_mut::<Stage>().panels.iter_mut() {
                        if only.as_ref().is_none_or(|p| *p == panel.id) {
                            panel.notes.clear();
                        }
                    }
                    Ok("cleared".to_owned())
                }
                "focus_protein" => focus_protein(world, &args),
                other => Err(format!("no tool {other}")),
            }?;
            if mode == Mode::Replay {
                return Ok(answer);
            }
            record(world, tool, &args);
            let mut answer = answer;
            if tool == "draw"
                && let Some(name) = text(&args, "name")
            {
                match Drawing::keep(&name, &args) {
                    Ok(name) => {
                        answer.push_str(&format!(" Kept in the library as the drawing {name}."))
                    }
                    Err(e) => answer.push_str(&format!(" Could not keep it: {e}.")),
                }
            }
            let stage = world.resource::<Stage>();
            Ok(format!("{answer} {}", stage.whereabouts()))
        }
    }
}

/// A `draw` call that names a kept drawing and gives no SVG: the kept call,
/// with what this one says over it.
fn resolve_drawing(args: &Value) -> Result<Value, String> {
    let has_svg = args.get("svg").is_some() || args.get("frames").is_some();
    let Some(name) = text(args, "name").filter(|_| !has_svg) else {
        return Ok(args.clone());
    };
    let kept = Drawing::load(&name).map_err(|e| e.to_string())?;
    let mut merged: Map<String, Value> = kept.args.as_object().cloned().unwrap_or_default();
    if let Some(map) = args.as_object() {
        for (k, v) in map {
            if k != "name" {
                merged.insert(k.clone(), v.clone());
            }
        }
    }
    Ok(Value::Object(merged))
}

/// Take away whatever a panel showed.
fn forget(world: &mut World, id: &str) {
    scene::remove(world, id);
    crate::world::remove(world, id);
    let mut views = world.resource_mut::<Views>();
    views.cells.remove(id);
    views.skins.remove(id);
    views.drawings.remove(id);
}

fn layout(world: &mut World, args: &Value) -> Result<String, String> {
    let panels = args
        .get("panels")
        .and_then(Value::as_array)
        .ok_or("panels: a list")?;
    let asked: Vec<(String, String)> = panels
        .iter()
        .take(MAX_PANELS)
        .filter_map(|p| {
            let id = text(p, "id")?;
            let title = text(p, "title").unwrap_or_else(|| id.clone());
            Some((id, title))
        })
        .collect();
    if asked.is_empty() {
        return Err("at least one panel, with an id".to_owned());
    }
    set_panels(
        world,
        &asked,
        Arrangement::parse(&text(args, "arrangement").unwrap_or_default()),
    );
    reveal(world);
    Ok(format!(
        "{} panels: {}",
        asked.len(),
        asked
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Make the stage's panels these, in this order, keeping what those that
/// stay show.
fn set_panels(world: &mut World, asked: &[(String, String)], arrangement: Arrangement) {
    let gone: Vec<String> = world
        .resource::<Stage>()
        .panels
        .iter()
        .filter(|p| !asked.iter().any(|(id, _)| *id == p.id))
        .map(|p| p.id.clone())
        .collect();
    for id in &gone {
        forget(world, id);
    }
    let mut stage = world.resource_mut::<Stage>();
    stage
        .panels
        .retain(|p| asked.iter().any(|(id, _)| *id == p.id));
    let mut ordered = Vec::new();
    for (id, title) in asked {
        let mut panel = stage
            .panels
            .iter()
            .position(|p| p.id == *id)
            .map(|i| stage.panels.remove(i))
            .unwrap_or_else(|| Panel::new(id, title));
        panel.title = title.clone();
        ordered.push(panel);
    }
    stage.panels = ordered;
    stage.arrangement = arrangement;
}

/// Show the window if it is not.
pub fn reveal(world: &mut World) {
    if world.resource::<Stage>().shown {
        return;
    }
    let mut windows = world.query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>();
    if let Ok(mut window) = windows.single_mut(world) {
        window.visible = true;
    }
    world.resource_mut::<Stage>().shown = true;
}

/// Get a panel ready for new content: made if need be, its old content and
/// notes taken away, titled.
fn prepare(
    world: &mut World,
    args: &Value,
    kind: Kind,
    default_title: &str,
) -> Result<String, String> {
    let id = text(args, "panel").ok_or("which panel?")?;
    let keep_cell = kind == Kind::Cell && world.resource::<Views>().cells.contains_key(&id);
    let keep_world = kind == Kind::World
        && world
            .resource::<crate::world::Worlds>()
            .map
            .contains_key(&id);
    if !keep_cell && !keep_world {
        forget(world, &id);
    }
    let mut stage = world.resource_mut::<Stage>();
    let panel = stage
        .panel(&id)
        .ok_or("six panels at most; lay them out again to make room")?;
    if panel.kind != kind {
        panel.notes.clear();
        panel.zoom = 1.0;
        panel.pan = egui::Vec2::ZERO;
    }
    if kind == Kind::Drawing {
        panel.zoom = 1.0;
        panel.pan = egui::Vec2::ZERO;
    }
    panel.kind = kind;
    panel.label = None;
    if let Some(title) = text(args, "title") {
        panel.title = title;
    } else if panel.title == panel.id {
        panel.title = default_title.to_owned();
    }
    if let Some(caption) = text(args, "caption") {
        panel.caption = caption;
    }
    reveal(world);
    Ok(id)
}

/// Make a panel show nothing.
fn clear_panel(world: &mut World, id: &str) {
    forget(world, id);
    if let Some(panel) = world
        .resource_mut::<Stage>()
        .panels
        .iter_mut()
        .find(|p| p.id == id)
    {
        panel.kind = Kind::Empty;
        panel.label = None;
        panel.image = None;
        panel.texture = None;
        panel.source.clear();
        panel.caption.clear();
        panel.notes.clear();
    }
}

fn show_protein(world: &mut World, args: &Value) -> Result<String, String> {
    let predicted = match text(args, "uniprot") {
        Some(accession) => Some((accession.to_uppercase(), sources::predicted(&accession)?)),
        None => None,
    };
    let first = match &predicted {
        Some((accession, _)) => accession.clone(),
        None => text(args, "pdb_id").ok_or("which PDB entry, or which UniProt accession?")?,
    };
    let mut texts = vec![match &predicted {
        Some((_, p)) => p.text.clone(),
        None => protein::fetch(&first).map_err(|e| e.to_string())?,
    }];
    let second = text(args, "second_pdb_id");
    if let Some(id) = &second {
        texts.push(protein::fetch(id).map_err(|e| e.to_string())?);
    }
    let structures: Vec<protein::pdb::Structure> = texts
        .iter()
        .map(|t| protein::pdb::Structure::parse(t))
        .collect();
    let mut look = Look {
        marked: true,
        ..Look::default()
    };
    if let Some(map) = args.get("chain_colours").and_then(Value::as_object) {
        for (chain, hex) in map {
            if let (Some(c), Some(rgb)) = (chain.chars().next(), hex.as_str().and_then(colour)) {
                look.chains.push((c, rgb));
            }
        }
    }
    if let Some(list) = args.get("hide_chains").and_then(Value::as_array) {
        look.hidden = list
            .iter()
            .filter_map(|c| c.as_str()?.chars().next())
            .collect();
    }
    if let Some(list) = args.get("highlights").and_then(Value::as_array) {
        for h in list {
            let Some(from) = h.get("from").and_then(Value::as_i64) else {
                continue;
            };
            let to = h.get("to").and_then(Value::as_i64).unwrap_or(from);
            look.highlights.push(Highlight {
                chains: text(h, "chains").unwrap_or_default(),
                from: from as i32,
                to: to as i32,
                colour: text(h, "colour")
                    .as_deref()
                    .and_then(colour)
                    .unwrap_or([1.0, 0.12, 0.06]),
                glows: h.get("glow").and_then(Value::as_bool).unwrap_or(true),
                label: text(h, "label").unwrap_or_default(),
            });
        }
    }
    let style = match text(args, "style").as_deref() {
        Some("fold") => Style::Fold,
        _ => Style::Atoms,
    };
    let (title, experiment) = protein::describe(&texts[0]);
    let id = prepare(world, args, Kind::Protein, &first.to_uppercase())?;
    scene::show(world, &id, &structures, look, style)?;
    {
        let mut stage = world.resource_mut::<Stage>();
        if let Some(panel) = stage.panels.iter_mut().find(|p| p.id == id) {
            panel.source = match (&predicted, &second) {
                (Some((accession, p)), _) => {
                    panel.label = Some("PREDICTED STRUCTURE · AlphaFold, not measured".to_owned());
                    format!(
                        "AlphaFold DB, UniProt {accession}, mean confidence {:.0} of 100",
                        p.confidence
                    )
                }
                (None, Some(s)) => format!(
                    "PDB {} and {}, {experiment}",
                    first.to_uppercase(),
                    s.to_uppercase()
                ),
                (None, None) => format!("PDB {}, {experiment}", first.to_uppercase()),
            };
        }
    }
    if let Some((accession, p)) = &predicted {
        let loose = if p.loose.is_empty() {
            "No long stretch is predicted with low confidence.".to_owned()
        } else {
            format!(
                "Predicted with low confidence (pLDDT under 50), likely without a fixed shape, so their shape here means little: residues {}.",
                p.loose
                    .iter()
                    .map(|(a, b)| format!("{a}-{b}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        return Ok(format!(
            "Showing the AlphaFold prediction for {accession}, {}, in panel {id}: a predicted structure, not a measured one; mean confidence {:.0} of 100. {loose}\n{}",
            p.name,
            p.confidence,
            contents(&structures[0])
        ));
    }
    Ok(format!(
        "Showing {} in panel {id}: {title} ({experiment}).\n{}",
        first.to_uppercase(),
        contents(&structures[0])
    ))
}

/// What a structure holds: its chains with their residue numbers, and what
/// is bound to it.
fn contents(s: &protein::pdb::Structure) -> String {
    let mut chains: Vec<(char, i32, i32, usize, bool)> = Vec::new();
    let mut bound: Vec<String> = Vec::new();
    for a in &s.atoms {
        if a.hetero {
            let name = format!("{} (chain {})", a.residue, a.chain);
            if !bound.contains(&name) {
                bound.push(name);
            }
            continue;
        }
        let nucleic = matches!(
            a.residue.as_str(),
            "DA" | "DC" | "DG" | "DT" | "A" | "C" | "G" | "U"
        );
        match chains.iter_mut().find(|c| c.0 == a.chain) {
            Some(c) => {
                c.1 = c.1.min(a.number);
                c.2 = c.2.max(a.number);
                if a.name == "CA" || a.name == "P" {
                    c.3 += 1;
                }
            }
            None => chains.push((a.chain, a.number, a.number, 1, nucleic)),
        }
    }
    let mut out: Vec<String> = chains
        .iter()
        .map(|(c, lo, hi, n, nucleic)| {
            format!(
                "chain {c}: {} {lo} to {hi}, {n} seen",
                if *nucleic { "nucleotides" } else { "residues" }
            )
        })
        .collect();
    if !bound.is_empty() {
        out.push(format!("bound: {}", bound.join(", ")));
    }
    out.join("\n")
}

/// A point in each named place of an image, for notes to point at: the
/// point of the place nearest its middle.
fn places(
    width: usize,
    height: usize,
    name_of: impl Fn(f32, f32) -> Option<String>,
) -> Vec<(String, (f32, f32))> {
    const STEP: usize = 6;
    let mut sums: Vec<(String, f64, f64, f64)> = Vec::new();
    let mut points: Vec<(usize, f32, f32)> = Vec::new();
    for y in (STEP / 2..height).step_by(STEP) {
        for x in (STEP / 2..width).step_by(STEP) {
            let (fx, fy) = (x as f32, y as f32);
            let Some(name) = name_of(fx, fy) else {
                continue;
            };
            let k = match sums.iter().position(|s| s.0 == name) {
                Some(k) => k,
                None => {
                    sums.push((name, 0.0, 0.0, 0.0));
                    sums.len() - 1
                }
            };
            sums[k].1 += f64::from(fx);
            sums[k].2 += f64::from(fy);
            sums[k].3 += 1.0;
            points.push((k, fx, fy));
        }
    }
    sums.iter()
        .enumerate()
        .filter_map(|(k, (name, sx, sy, n))| {
            let (cx, cy) = ((sx / n) as f32, (sy / n) as f32);
            let best = points.iter().filter(|p| p.0 == k).min_by(|a, b| {
                let da = (a.1 - cx).powi(2) + (a.2 - cy).powi(2);
                let db = (b.1 - cx).powi(2) + (b.2 - cy).powi(2);
                da.total_cmp(&db)
            })?;
            Some((name.clone(), (best.1, best.2)))
        })
        .collect()
}

/// A name without its article: "the basal layer" is "basal layer".
fn bare(name: &str) -> String {
    name.trim_start_matches("the ")
        .trim_start_matches("an ")
        .trim_start_matches("a ")
        .to_owned()
}

fn show_cell(world: &mut World, args: &Value) -> Result<String, String> {
    let id = prepare(world, args, Kind::Cell, "One cell")?;
    if !world.resource::<Views>().cells.contains_key(&id)
        || args.get("new_cell").and_then(Value::as_bool) == Some(true)
    {
        let dish = Dish::new().map_err(|e| e.to_string())?;
        let image = world
            .resource_mut::<Assets<Image>>()
            .add(crate::blank(specimen::SIZE, specimen::SIZE));
        let stains = [dish.index("erk"), dish.index("bcl2"), dish.index("egfr")];
        let specimen = Specimen::new(11);
        let places = places(specimen::SIZE, specimen::SIZE, |x, y| {
            let c = specimen.compartment(x, y);
            (c != Compartment::Outside).then(|| bare(c.name()))
        });
        world.resource_mut::<Views>().cells.insert(
            id.clone(),
            CellView {
                dish,
                specimen,
                stains,
                running: true,
                owed: 0.0,
                since: 1.0,
                frame: 0,
                image: image.clone(),
                places,
            },
        );
        if let Some(panel) = world
            .resource_mut::<Stage>()
            .panels
            .iter_mut()
            .find(|p| p.id == id)
        {
            panel.image = Some(image);
            panel.texture = None;
        }
    }
    let mut views = world.resource_mut::<Views>();
    let view = views.cells.get_mut(&id).ok_or("no cell")?;
    let dish = &mut view.dish;
    if let Some(v) = number(args, "oxygen") {
        dish.oxygen = v.clamp(0.0, 1.0);
    }
    if let Some(v) = number(args, "egf") {
        dish.egf = v.clamp(0.0, 1.0);
    }
    if let Some(v) = number(args, "stress") {
        dish.stress = v.clamp(0.0, 1.0);
    }
    let names = |key: &str| -> Vec<String> {
        args.get(key)
            .and_then(Value::as_array)
            .map(|l| {
                l.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let (removed, stuck) = (names("removed"), names("stuck_on"));
    if args.get("removed").is_some() || args.get("stuck_on").is_some() {
        for gene in GENES {
            let switch = if removed.iter().any(|g| g == gene) {
                Switch::Removed
            } else if stuck.iter().any(|g| g == gene) {
                Switch::StuckOn
            } else {
                Switch::Normal
            };
            if let Some(i) = dish.index(gene) {
                dish.set_switch(i, switch);
            }
        }
    }
    let stains = names("stains");
    if !stains.is_empty() {
        let picked: Vec<Option<usize>> = stains.iter().map(|s| dish.index(s)).collect();
        for k in 0..3 {
            view.stains[k] = picked.get(k).copied().flatten();
        }
    }
    let dish = &mut view.dish;
    for _ in 0..args
        .get("steps")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(2000)
    {
        dish.step();
    }
    if let Some(running) = args.get("running").and_then(Value::as_bool) {
        view.running = running;
    }
    view.since = 1.0;
    let mut summary = cell_summary(&view.dish);
    let stained: Vec<String> = view
        .stains
        .iter()
        .zip(["green", "red", "magenta"])
        .filter_map(|(s, dye)| {
            let name = &view.dish.names()[(*s)?];
            Some(format!(
                "{} in {dye}: {}",
                label(name),
                specimen::place_of(name)
            ))
        })
        .collect();
    if !stained.is_empty() {
        summary.push_str(&format!(" Stained: {}", stained.join(" ")));
    }
    let where_: Vec<&str> = view.places.iter().map(|(n, _)| n.as_str()).collect();
    summary.push_str(&format!(
        " Places to point at with annotate on=: {}.",
        where_.join(", ")
    ));
    let source = format!(
        "O₂ {:.2} · EGF {:.2} · stress {:.2}{}",
        view.dish.oxygen,
        view.dish.egf,
        view.dish.stress,
        switches(&view.dish)
    );
    if let Some(panel) = world
        .resource_mut::<Stage>()
        .panels
        .iter_mut()
        .find(|p| p.id == id)
    {
        panel.source = source;
    }
    Ok(summary)
}

/// The genes removed or stuck on, as a label adds them.
fn switches(dish: &Dish) -> String {
    let mut out = String::new();
    for (i, name) in dish.names().iter().enumerate() {
        match dish.switch(i) {
            Switch::Removed => out.push_str(&format!(" · no {}", label(name))),
            Switch::StuckOn => out.push_str(&format!(" · {} stuck on", label(name))),
            Switch::Normal => {}
        }
    }
    out
}

/// The cell in words: its fate and every protein's level.
fn cell_summary(dish: &Dish) -> String {
    let fate = match dish.fate {
        Fate::Cycling(t) => format!("cycling, {:.0}% of the way to its next division", t * 100.0),
        Fate::Arrested => "arrested: alive, its cycle stopped (RB at or above 0.45)".to_owned(),
        Fate::Apoptotic(_) => "in apoptosis, taking itself apart".to_owned(),
        Fate::Necrotic(_) => "necrotic: dead of energy failure (BCL2 below 0.25)".to_owned(),
    };
    let levels: Vec<String> = dish
        .names()
        .iter()
        .zip(dish.levels())
        .map(|(n, v)| format!("{} {v:.3}", label(n)))
        .collect();
    format!(
        "The cell is {fate}, after {} steps. Levels: {}.",
        dish.steps,
        levels.join(", ")
    )
}

fn show_skin(world: &mut World, args: &Value) -> Result<String, String> {
    let stage_name = text(args, "stage").ok_or("which stage?")?;
    let lesion = Lesion::parse(&stage_name).ok_or_else(|| {
        format!(
            "no stage {stage_name}; the stages are {}",
            Lesion::ALL
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1) as u32;
    let id = prepare(world, args, Kind::Skin, "A section of skin, H&E")?;
    let section = skin::cut(lesion, seed);
    let mut image = crate::blank(skin::WIDTH, skin::HEIGHT);
    if let Some(data) = image.data.as_mut() {
        data.copy_from_slice(&section.image);
    }
    let handle = world.resource_mut::<Assets<Image>>().add(image);
    let breslow = section.breslow;
    let places = places(skin::WIDTH, skin::HEIGHT, |x, y| {
        let r = section.region(x, y);
        (r != skin::Region::Slide).then(|| bare(r.name()))
    });
    let names: Vec<String> = places.iter().map(|(n, _)| n.clone()).collect();
    world
        .resource_mut::<Views>()
        .skins
        .insert(id.clone(), SkinView { section, places });
    if let Some(panel) = world
        .resource_mut::<Stage>()
        .panels
        .iter_mut()
        .find(|p| p.id == id)
    {
        panel.image = Some(handle);
        panel.texture = None;
        panel.source = stage_name.replace('_', " ");
    }
    let mut answer = format!(
        "Drew {} in panel {id}: {} The section is {:.0} µm across.",
        stage_name.replace('_', " "),
        lesion.what(),
        skin::WIDTH as f32 * skin::MICROMETRES_PER_PIXEL
    );
    if let Some(b) = breslow {
        answer.push_str(&format!(
            " Breslow thickness as drawn: {:.2} mm.",
            b / 1000.0
        ));
    }
    answer.push_str(&format!(
        " Places to point at with annotate on=: {}.",
        names.join(", ")
    ));
    Ok(answer)
}

/// The page a drawing is drawn on.
fn background(args: &Value) -> [u8; 3] {
    match text(args, "background").as_deref() {
        Some("dark") => [16, 19, 24],
        Some(hex) if hex.starts_with('#') => colour(hex)
            .map(|c| c.map(|v| (v * 255.0) as u8))
            .unwrap_or([247, 245, 240]),
        _ => [247, 245, 240],
    }
}

/// What a panel that shows a drawing says of itself, besides being one.
struct Labelled {
    label: Option<String>,
    source: Option<String>,
    title: &'static str,
    /// What the parts are, from whatever made the drawing.
    described: Vec<(String, String)>,
    /// Whether the answer names every part; a chart's points are many.
    name_parts: bool,
}

impl Labelled {
    fn drawing() -> Self {
        Self {
            label: None,
            source: None,
            title: "A drawing",
            described: Vec::new(),
            name_parts: true,
        }
    }
}

fn draw(world: &mut World, args: &Value) -> Result<String, String> {
    let mut inputs: Vec<String> = Vec::new();
    if let Some(svg) = text(args, "svg") {
        inputs.push(svg);
    }
    if let Some(list) = args.get("frames").and_then(Value::as_array) {
        inputs.extend(list.iter().filter_map(|f| f.as_str().map(str::to_owned)));
    }
    if inputs.is_empty() {
        return Err("give svg, or frames, or the name of a drawing kept in the library".to_owned());
    }
    let size = args
        .get("view_box")
        .and_then(Value::as_array)
        .and_then(|a| Some((a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32)));
    let size = Sheet::read(&inputs[0], size).size;
    // Shorthands written out and pictures fetched first, so that changes
    // may name what they made.
    let mut inputs: Vec<String> = inputs
        .iter()
        .enumerate()
        .map(|(k, i)| {
            drawing::macros::expand(i, size)
                .and_then(|s: String| sources::resolve_commons(&s))
                .map_err(|e| format!("frame {}: {e}", k + 1))
        })
        .collect::<Result<_, _>>()?;
    // Frames given by what changes from the frame before.
    if let Some(list) = args.get("changes").and_then(Value::as_array) {
        let mut last = inputs.last().cloned().unwrap_or_default();
        for (k, frame) in list.iter().enumerate() {
            let mut edits = Vec::new();
            for (id, attributes) in frame.as_object().into_iter().flatten() {
                for (attribute, value) in attributes.as_object().into_iter().flatten() {
                    let value = match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    edits.push((id.clone(), attribute.clone(), value));
                }
            }
            // An attribute brought in by a change is given its neutral
            // value in the frames before, so that they move into it rather
            // than fade.
            for (id, attribute, value) in &edits {
                if drawing::attribute(&last, id, attribute).is_none()
                    && let Some(neutral) = drawing::neutral(attribute, value)
                {
                    let edit = [(id.clone(), attribute.clone(), neutral)];
                    for frame in inputs.iter_mut() {
                        if drawing::attribute(frame, id, attribute).is_none() {
                            *frame =
                                drawing::changed(frame, &edit).unwrap_or_else(|_| frame.clone());
                        }
                    }
                    last = drawing::changed(&last, &edit).unwrap_or(last);
                }
            }
            last = drawing::changed(&last, &edits).map_err(|e| format!("change {}: {e}", k + 1))?;
            inputs.push(last.clone());
        }
    }
    stage_drawing(world, args, inputs, size, Labelled::drawing())
}

/// A figure the window laid out, shown as a drawing.
fn show_figure(
    world: &mut World,
    args: &Value,
    figure: Result<Figure, String>,
    mut how: Labelled,
) -> Result<String, String> {
    let figure = figure?;
    how.described = figure.parts;
    let svg = format!(
        "<svg viewBox=\"0 0 {} {}\">{}</svg>",
        figure.size.0, figure.size.1, figure.svg
    );
    stage_drawing(world, args, vec![svg], figure.size, how)
}

/// Put a drawing's frames in a panel.
fn stage_drawing(
    world: &mut World,
    args: &Value,
    inputs: Vec<String>,
    size: (f32, f32),
    how: Labelled,
) -> Result<String, String> {
    let sheets: Vec<Sheet> = inputs.iter().map(|i| Sheet::read(i, Some(size))).collect();
    let documents = drawing::documents(&sheets);
    // Read every frame now, so that a mistake is told while the agent is
    // still drawing.
    let probe = Canvas {
        width: 16,
        height: 10,
        scale: 16.0 / size.0,
        offset: (0.0, 0.0),
        background: [0, 0, 0],
    };
    let mut parts = Vec::new();
    for (k, d) in documents.iter().enumerate() {
        let (_, p) = drawing::render(&Picture::Still(d), &probe)
            .map_err(|e| format!("frame {}: {e}", k + 1))?;
        if k == 0 {
            parts = p;
        }
    }
    let mut described = drawing::macros::descriptions(&inputs[0]);
    for (id, what) in how.described.into_iter().chain(
        args.get("parts")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_owned())),
    ) {
        described.retain(|(i, _)| *i != id);
        described.push((id, what));
    }
    let frames = documents.len();
    let seconds = number(args, "seconds").unwrap_or(2.0) as f32;
    let hold = number(args, "hold").unwrap_or(0.8) as f32;
    let looping = args.get("loop").and_then(Value::as_bool).unwrap_or(false);
    let film = Film::new(documents, hold, seconds, looping);
    let length = film.length();
    let id = prepare(world, args, Kind::Drawing, how.title)?;
    let image = world
        .resource_mut::<Assets<Image>>()
        .add(crate::blank(16, 16));
    world.resource_mut::<Views>().drawings.insert(
        id.clone(),
        DrawView {
            film,
            size,
            clock: 0.0,
            playing: frames > 1,
            background: background(args),
            image: image.clone(),
            parts: parts.clone(),
            described,
            last: None,
        },
    );
    if let Some(panel) = world
        .resource_mut::<Stage>()
        .panels
        .iter_mut()
        .find(|p| p.id == id)
    {
        panel.image = Some(image);
        panel.texture = None;
        panel.label = how.label;
        panel.source = match (how.source, frames > 1) {
            (Some(s), _) => s,
            (None, true) => format!("{frames} frames"),
            (None, false) => String::new(),
        };
    }
    let mut ids: Vec<&str> = parts.iter().map(|p| p.id.as_str()).collect();
    ids.dedup();
    let mut answer = format!("Shown in panel {id}, {} by {} units", size.0, size.1);
    if frames > 1 {
        answer.push_str(&format!(", {frames} frames over {length:.1} s"));
    }
    answer.push('.');
    if how.name_parts {
        if ids.is_empty() {
            answer.push_str(" It has no named parts: give ids to what you will point at.");
        } else {
            answer.push_str(&format!(
                " Parts to point at with annotate on=: {}.",
                ids.join(", ")
            ));
        }
    }
    Ok(answer)
}

fn show_diagram(world: &mut World, args: &Value) -> Result<String, String> {
    show_figure(
        world,
        args,
        figure::diagram::diagram(args),
        Labelled {
            label: Some("SCHEMATIC · laid out from the agent's nodes and edges".to_owned()),
            title: "A diagram",
            ..Labelled::drawing()
        },
    )
}

fn show_chart(world: &mut World, args: &Value) -> Result<String, String> {
    let source = text(args, "source").unwrap_or_default();
    let chart = figure::chart::chart(args)?;
    let series: Vec<String> = chart.parts.iter().map(|(id, _)| id.clone()).collect();
    let answer = show_figure(
        world,
        args,
        Ok(chart),
        Labelled {
            label: Some("CHART · numbers given by the agent".to_owned()),
            source: Some(format!("from {source}")),
            title: "A chart",
            name_parts: false,
            ..Labelled::drawing()
        },
    )?;
    Ok(format!(
        "{answer} Series to point at: {}; each point or bar is the series id with _1, _2... after it.",
        series.join(", ")
    ))
}

fn show_text(world: &mut World, args: &Value) -> Result<String, String> {
    show_figure(
        world,
        args,
        figure::page::page(args),
        Labelled {
            label: Some("TEXT · written by the agent".to_owned()),
            title: "Notes",
            ..Labelled::drawing()
        },
    )
}

fn show_image(world: &mut World, args: &Value) -> Result<String, String> {
    let file = text(args, "file").ok_or("which file? image_search finds them")?;
    let p = sources::commons_picture(&file)?;
    let (w, h) = (p.width as f32, p.height as f32);
    let svg = format!(
        "<svg viewBox=\"0 0 {w} {h}\"><image id=\"picture\" href=\"{}\" x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\"/></svg>",
        drawing::text::escape(&p.path.to_string_lossy())
    );
    let mut how = Labelled {
        label: Some("PICTURE · Wikimedia Commons".to_owned()),
        source: Some(format!("{}, {}", short_credit(&p.author), p.licence)),
        title: "A picture",
        name_parts: false,
        ..Labelled::drawing()
    };
    how.described = vec![("picture".to_owned(), p.description.clone())];
    let answer = stage_drawing(world, args, vec![svg], (w, h), how)?;
    Ok(format!(
        "{answer} {}: {} By {}, {}. Point at things in it with annotate arrow_to=[x, y] as fractions of the picture, or draw over it with draw and <image href=\"commons:{}\" width=\"{w}\" height=\"{h}\"/>.",
        p.title, p.description, p.author, p.licence, p.title
    ))
}

/// An author's credit short enough for a panel's label.
fn short_credit(author: &str) -> String {
    let a = author.trim();
    if a.chars().count() > 60 {
        format!("{}…", a.chars().take(58).collect::<String>())
    } else if a.is_empty() {
        "author unknown".to_owned()
    } else {
        a.to_owned()
    }
}

fn image_search(args: &Value) -> Result<String, String> {
    let query = text(args, "query").ok_or("search for what?")?;
    let limit = number(args, "limit").unwrap_or(8.0) as usize;
    let found = sources::commons_search(&query, limit)?;
    if found.is_empty() {
        return Ok(format!(
            "Nothing on Wikimedia Commons for {query}; try other words, in English."
        ));
    }
    let mut out = String::from(
        "On Wikimedia Commons (show_image file=... shows one; its licence and author are shown with it):\n",
    );
    for p in found {
        out.push_str(&format!(
            "- {} ({} by {} px; {}; by {}): {}\n",
            p.title,
            p.width,
            p.height,
            p.licence,
            short_credit(&p.author),
            p.description
        ));
    }
    Ok(out)
}

/// Open a world in a panel, or keep the one it shows, and play the film
/// the agent directs in it.
fn show_world(world: &mut World, args: &Value) -> Result<String, String> {
    let name = text(args, "world").unwrap_or_else(|| "skin".to_owned());
    let variant = text(args, "variant").unwrap_or_default();
    let about = crate::world::about(&name).ok_or_else(|| {
        format!(
            "no world {name}; there are {}",
            crate::world::names().join(", ")
        )
    })?;
    let variant = if variant.is_empty() {
        about
            .variants
            .first()
            .copied()
            .unwrap_or_default()
            .to_owned()
    } else {
        variant
    };
    let id = prepare(world, args, Kind::World, "A world")?;
    crate::world::show(world, &id, &name, &variant)?;
    let mut worlds = world.resource_mut::<crate::world::Worlds>();
    let scene = worlds
        .map
        .get_mut(&id)
        .ok_or("the world could not be opened")?;
    let mut start = scene.eye;
    if let Some(place) = text(args, "place") {
        start = scene.place(&place).ok_or_else(|| {
            format!(
                "no place {place} in the {name}; there are {}",
                about.places.join(", ")
            )
        })?;
        scene.eye = start;
    }
    let clock = number(args, "clock")
        .or(number(args, "years"))
        .map_or(about.start, |y| y as f32);
    scene.time = clock;
    scene.highlight = crate::world::film::names(args.get("highlight"));
    let shots = args.get("shots").and_then(Value::as_array);
    let film = match shots {
        Some(list) => {
            let shots = crate::world::film::read_shots(list, |n| scene.place(n), start)?;
            crate::world::film::Film::new(shots, clock)
        }
        None => crate::world::film::Film::default(),
    };
    let length = film.length();
    let count = film.shots.len();
    scene.play(film);
    let places = scene
        .places()
        .iter()
        .map(|p| format!("{} ({})", p.name, p.what))
        .collect::<Vec<_>>()
        .join("; ");
    let film = if count > 0 {
        format!("Playing {count} shots, {length:.0} s. ")
    } else {
        String::new()
    };
    Ok(format!(
        "{film}The world {name}, telling {variant}, in panel {id}. Places: {places}. Notes can point at: {}.",
        (about.things)().join(", ")
    ))
}

fn annotate(world: &mut World, args: &Value) -> Result<String, String> {
    let id = text(args, "panel").ok_or("which panel?")?;
    let words = text(args, "text").ok_or("what to write?")?;
    let at = match (number(args, "x"), number(args, "y")) {
        (Some(x), Some(y)) => Some((x as f32, y as f32)),
        _ => None,
    };
    let arrow_to = args
        .get("arrow_to")
        .and_then(Value::as_array)
        .and_then(|a| Some((a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32)));
    let target = if let Some(on) = text(args, "on") {
        Some(Anchor::Part(find_place(world, &id, &on)?))
    } else if let Some(r) = args.get("residue").and_then(Value::as_i64) {
        Some(Anchor::Residue(
            text(args, "chain")
                .and_then(|c| c.chars().next())
                .unwrap_or(' '),
            r as i32,
        ))
    } else {
        arrow_to.map(|(x, y)| Anchor::Fraction(x, y))
    };
    let note = Note {
        text: words,
        target,
        label: at.or(
            if arrow_to.is_none() && args.get("on").is_none() && args.get("residue").is_none() {
                Some((0.04, 0.06))
            } else {
                None
            },
        ),
    };
    let mut stage = world.resource_mut::<Stage>();
    let panel = stage
        .panels
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("no such panel")?;
    panel.notes.push(note);
    Ok("noted".to_owned())
}

/// The name of a place in a panel that `on` asks for, or the places there
/// are.
fn find_place(world: &World, panel: &str, on: &str) -> Result<String, String> {
    let views = world.resource::<Views>();
    let names: Vec<String> = if let Some(d) = views.drawings.get(panel) {
        d.parts.iter().map(|p| p.id.clone()).collect()
    } else if let Some(s) = views.skins.get(panel) {
        s.places.iter().map(|(n, _)| n.clone()).collect()
    } else if let Some(c) = views.cells.get(panel) {
        c.places.iter().map(|(n, _)| n.clone()).collect()
    } else if let Some(scene) = world.resource::<crate::world::Worlds>().map.get(panel) {
        (scene.about.things)()
            .into_iter()
            .map(str::to_owned)
            .collect()
    } else {
        return Err("on= points at a part of a drawing, a section, a cell or a world; on a protein, give chain and residue".to_owned());
    };
    let wanted = story::fold(&bare(on));
    names
        .iter()
        .find(|n| story::fold(n) == wanted)
        .or_else(|| names.iter().find(|n| story::fold(n).contains(&wanted)))
        .cloned()
        .ok_or_else(|| {
            format!(
                "no part {on} in panel {panel}; there are: {}",
                names.join(", ")
            )
        })
}

fn focus_protein(world: &mut World, args: &Value) -> Result<String, String> {
    let panel = text(args, "panel").ok_or("which panel?")?;
    let residue = args
        .get("residue")
        .and_then(Value::as_i64)
        .ok_or("which residue?")? as i32;
    let chain = text(args, "chain").and_then(|c| c.chars().next());
    let mut scenes = world.resource_mut::<Scenes>();
    let scene = scenes
        .map
        .get_mut(&panel)
        .ok_or("no protein in that panel")?;
    if scene.focus(chain, residue) {
        Ok(format!("turned to residue {residue}"))
    } else {
        Err(format!("no residue {residue} in that structure"))
    }
}

/// A story to keep the agent's calls in, begun if there is none.
fn ensure_story(world: &mut World) {
    let mut stage = world.resource_mut::<Stage>();
    if stage.story.is_none() {
        let title = format!("Untitled, {}", story::date(now()));
        stage.story = Some(Story::new(&title, "", Vec::new()));
        stage.step = 0;
        stage.dirty = true;
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The calls that fill a panel.
const SHOWS: [&str; 9] = [
    "show_protein",
    "show_world",
    "show_skin",
    "show_cell",
    "draw",
    "show_diagram",
    "show_chart",
    "show_text",
    "show_image",
];

/// The calls whose panel is a drawing, which can move into one another.
const DRAWN: [&str; 5] = [
    "draw",
    "show_diagram",
    "show_chart",
    "show_text",
    "show_image",
];

/// Keep a call in the step on screen: its panels as the stage now has
/// them, and what the call put in its panel.
fn record(world: &mut World, tool: &str, args: &Value) {
    let shown: Vec<(String, String)> = world
        .resource::<Stage>()
        .panels
        .iter()
        .map(|p| (p.id.clone(), p.title.clone()))
        .collect();
    let mut stage = world.resource_mut::<Stage>();
    let arrangement = stage.arrangement.name().to_owned();
    let n = stage.step;
    let Some(story) = stage.story.as_mut() else {
        return;
    };
    let Some(step) = story.steps.get_mut(n) else {
        return;
    };
    let old = std::mem::take(&mut step.panels);
    step.panels = shown
        .iter()
        .map(|(id, title)| {
            let mut spec = old
                .iter()
                .find(|s| s.id == *id)
                .cloned()
                .unwrap_or(PanelSpec {
                    id: id.clone(),
                    title: title.clone(),
                    show: None,
                    after: Vec::new(),
                });
            spec.title = title.clone();
            spec
        })
        .collect();
    step.arrangement = arrangement;
    let call = Call {
        tool: tool.to_owned(),
        args: args.clone(),
    };
    let panel = text(args, "panel");
    let mut displayed = None;
    match tool {
        tool if SHOWS.contains(&tool) => {
            if let Some(spec) = step
                .panels
                .iter_mut()
                .find(|s| Some(&s.id) == panel.as_ref())
            {
                let before = spec.show.take();
                let same_kind = before.as_ref().is_some_and(|b| b.tool == tool);
                let fresh = args.get("new_cell").and_then(Value::as_bool) == Some(true);
                let merged = match (&before, tool) {
                    (Some(b), "show_cell") if same_kind && !fresh => {
                        let mut map = b.args.as_object().cloned().unwrap_or_default();
                        if let Some(new) = args.as_object() {
                            for (k, v) in new {
                                map.insert(k.clone(), v.clone());
                            }
                        }
                        map.remove("steps");
                        Call {
                            tool: tool.to_owned(),
                            args: Value::Object(map),
                        }
                    }
                    _ => call,
                };
                if !same_kind {
                    spec.after.clear();
                }
                spec.show = Some(merged.clone());
                displayed = Some(merged);
            }
        }
        "annotate" | "focus_protein" => {
            if let Some(spec) = step
                .panels
                .iter_mut()
                .find(|s| Some(&s.id) == panel.as_ref())
            {
                spec.after.push(call);
            }
        }
        "clear_annotations" => {
            for spec in step.panels.iter_mut() {
                if panel.as_ref().is_none_or(|p| *p == spec.id) {
                    spec.after.retain(|c| c.tool != "annotate");
                }
            }
        }
        _ => {}
    }
    stage.dirty = true;
    if let (Some(call), Some(id)) = (displayed, panel)
        && let Some(p) = stage.panels.iter_mut().find(|p| p.id == id)
    {
        p.shown = Some(call);
    }
}

/// Show step `n` of the story: its panels, what fills them, its notes.
/// Panels that already show what the step wants are left as they are; a
/// drawing that changes on the way forward moves from what it was.
pub fn show_step(world: &mut World, n: usize, forward: bool) {
    let step = {
        let stage = world.resource::<Stage>();
        let Some(step) = stage.story.as_ref().and_then(|s| s.steps.get(n)) else {
            return;
        };
        step.clone()
    };
    world.resource_mut::<Stage>().step = n;
    let asked: Vec<(String, String)> = step
        .panels
        .iter()
        .map(|p| (p.id.clone(), p.title.clone()))
        .collect();
    set_panels(world, &asked, Arrangement::parse(&step.arrangement));
    for spec in &step.panels {
        let current = world
            .resource::<Stage>()
            .panels
            .iter()
            .find(|p| p.id == spec.id)
            .and_then(|p| p.shown.clone());
        if spec.show != current {
            // The frame a drawing showed, to move from.
            let from = match (&spec.show, &current) {
                (Some(new), Some(old))
                    if forward
                        && DRAWN.contains(&new.tool.as_str())
                        && DRAWN.contains(&old.tool.as_str()) =>
                {
                    world.resource::<Views>().drawings.get(&spec.id).map(|d| {
                        let last = d.film.frames().len().saturating_sub(1);
                        d.film.frames()[last].clone()
                    })
                }
                _ => None,
            };
            match &spec.show {
                None => clear_panel(world, &spec.id),
                Some(call) => {
                    if let Err(e) = handle(world, &call.tool, &call.args, Mode::Replay) {
                        warn!("step {} panel {}: {e}", n + 1, spec.id);
                    }
                }
            }
            if let Some(frame) = from
                && let Some(d) = world.resource_mut::<Views>().drawings.get_mut(&spec.id)
                && d.film.frames().first() != Some(&frame)
            {
                d.film.begin_from(frame);
                d.clock = 0.0;
                d.playing = true;
            }
            if let Some(p) = world
                .resource_mut::<Stage>()
                .panels
                .iter_mut()
                .find(|p| p.id == spec.id)
            {
                p.shown = spec.show.clone();
            }
        } else if let Some(d) = world.resource_mut::<Views>().drawings.get_mut(&spec.id)
            && d.film.frames().len() > 1
        {
            // The same drawing: played again from its start.
            d.clock = 0.0;
            d.playing = true;
        }
        if let Some(p) = world
            .resource_mut::<Stage>()
            .panels
            .iter_mut()
            .find(|p| p.id == spec.id)
        {
            p.notes.clear();
            p.title = spec.title.clone();
        }
        for call in &spec.after {
            if let Err(e) = handle(world, &call.tool, &call.args, Mode::Replay) {
                warn!("step {} panel {}: {e}", n + 1, spec.id);
            }
        }
    }
    if !step.panels.is_empty() {
        reveal(world);
    }
}

/// The step on screen, in words.
fn step_summary(world: &World) -> String {
    let stage = world.resource::<Stage>();
    let Some(story) = &stage.story else {
        return String::new();
    };
    let Some(step) = story.steps.get(stage.step) else {
        return String::new();
    };
    let panels: Vec<String> = step
        .panels
        .iter()
        .map(|p| {
            format!(
                "{} ({})",
                p.id,
                p.show.as_ref().map(|c| c.tool.as_str()).unwrap_or("empty")
            )
        })
        .collect();
    format!(
        "Step {} of {}, \"{}\": {}. {}",
        stage.step + 1,
        story.steps.len(),
        step.title,
        if panels.is_empty() {
            "no panels yet".to_owned()
        } else {
            format!("panels {}", panels.join(", "))
        },
        if step.narration.is_empty() {
            String::new()
        } else {
            format!("Narration: {}", step.narration)
        }
    )
}

/// A step number from the agent, counted from 1, as an index.
fn step_number(world: &World, args: &Value) -> Result<usize, String> {
    let stage = world.resource::<Stage>();
    let story = stage.story.as_ref().ok_or("no story is open")?;
    let n = args
        .get("step")
        .and_then(Value::as_u64)
        .ok_or("which step? from 1")? as usize;
    if n == 0 || n > story.steps.len() {
        return Err(format!("the story has steps 1 to {}", story.steps.len()));
    }
    Ok(n - 1)
}

fn tags(args: &Value) -> Vec<String> {
    args.get("tags")
        .and_then(Value::as_array)
        .map(|l| {
            l.iter()
                .filter_map(|t| t.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn story_new(world: &mut World, args: &Value) -> Result<String, String> {
    let title = text(args, "title").ok_or("a title for the story")?;
    let mut story = Story::new(
        &title,
        &text(args, "summary").unwrap_or_default(),
        tags(args),
    );
    story.steps[0].title = text(args, "step_title").unwrap_or_else(|| title.clone());
    story.steps[0].narration = text(args, "narration").unwrap_or_default();
    let id = story.id.clone();
    {
        let mut stage = world.resource_mut::<Stage>();
        stage.story = Some(story);
        stage.step = 0;
        stage.dirty = true;
    }
    show_step(world, 0, false);
    reveal(world);
    Ok(format!(
        "Began the story {id}, at its first step, empty: lay it out and fill it, then story_step for the next. It is kept in the library as you go."
    ))
}

fn story_step(world: &mut World, args: &Value) -> Result<String, String> {
    ensure_story(world);
    let keep = args
        .get("keep_panels")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let keep_notes = args
        .get("keep_notes")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let n = {
        let mut stage = world.resource_mut::<Stage>();
        let at = stage.step;
        let story = stage.story.as_mut().ok_or("no story")?;
        let mut step = Step {
            title: text(args, "title").unwrap_or_else(|| format!("Step {}", story.steps.len() + 1)),
            narration: text(args, "narration").unwrap_or_default(),
            ..Step::default()
        };
        if keep && let Some(current) = story.steps.get(at) {
            step.arrangement = current.arrangement.clone();
            step.panels = current.panels.clone();
            if !keep_notes {
                for p in &mut step.panels {
                    p.after.retain(|c| c.tool != "annotate");
                }
            }
        }
        let n = (at + 1).min(story.steps.len());
        story.steps.insert(n, step);
        stage.dirty = true;
        n
    };
    show_step(world, n, true);
    Ok(format!(
        "Now at step {} of {}{}.",
        n + 1,
        world
            .resource::<Stage>()
            .story
            .as_ref()
            .map_or(0, |s| s.steps.len()),
        if keep {
            ", with the panels of the step before; change what needs to change"
        } else {
            ", empty: lay it out and fill it"
        }
    ))
}

fn story_edit(world: &mut World, args: &Value) -> Result<String, String> {
    let n = if args.get("step").is_some() {
        step_number(world, args)?
    } else {
        world.resource::<Stage>().step
    };
    let mut stage = world.resource_mut::<Stage>();
    let story = stage.story.as_mut().ok_or("no story is open")?;
    if let Some(t) = text(args, "story_title") {
        story.title = t;
    }
    if let Some(s) = text(args, "summary") {
        story.summary = s;
    }
    if args.get("tags").is_some() {
        story.tags = tags(args);
    }
    let step = story.steps.get_mut(n).ok_or("no such step")?;
    if let Some(t) = text(args, "title") {
        step.title = t;
    }
    if let Some(t) = text(args, "narration") {
        step.narration = t;
    }
    stage.dirty = true;
    Ok(format!("Changed. {}", stage.whereabouts()))
}

fn story_remove_step(world: &mut World, args: &Value) -> Result<String, String> {
    let n = if args.get("step").is_some() {
        step_number(world, args)?
    } else {
        world.resource::<Stage>().step
    };
    let next = {
        let mut stage = world.resource_mut::<Stage>();
        let story = stage.story.as_mut().ok_or("no story is open")?;
        story.steps.remove(n);
        if story.steps.is_empty() {
            story.steps.push(Step {
                title: story.title.clone(),
                ..Step::default()
            });
        }
        let next = n.min(story.steps.len() - 1);
        stage.dirty = true;
        next
    };
    show_step(world, next, false);
    Ok(format!("Removed step {}. {}", n + 1, step_summary(world)))
}

fn story_open(world: &mut World, args: &Value) -> Result<String, String> {
    let id = text(args, "id").ok_or("which story? its id, from library_search")?;
    let story = Story::load(&id).map_err(|e| e.to_string())?;
    let outline = story.outline();
    let n = args
        .get("step")
        .and_then(Value::as_u64)
        .map_or(0, |s| (s as usize).saturating_sub(1))
        .min(story.steps.len().saturating_sub(1));
    {
        let mut stage = world.resource_mut::<Stage>();
        stage.story = Some(story);
        stage.step = n;
    }
    show_step(world, n, false);
    reveal(world);
    Ok(format!(
        "Opened. {outline}\nShowing step {}; the user moves through the steps with the arrow keys. {}",
        n + 1,
        step_summary(world)
    ))
}

fn library_search(args: &Value) -> String {
    let query = text(args, "query").unwrap_or_default();
    let found = story::search(&query);
    if found.is_empty() {
        return "Nothing in the library speaks of that yet. Begin a story with story_new; it is kept as you tell it.".to_owned();
    }
    let mut out = vec!["In the library:".to_owned()];
    for f in found {
        match f {
            Found::Story(s) => out.push(s.outline()),
            Found::Drawing(d) => out.push(format!(
                "drawing {}: \"{}\"{}",
                d.name,
                d.title,
                if d.caption.is_empty() {
                    String::new()
                } else {
                    format!(", {}", d.caption)
                }
            )),
        }
    }
    out.push(
        "Open a story with story_open {id}; show a kept drawing with draw {panel, name}."
            .to_owned(),
    );
    out.join("\n")
}

/// Write the story to the library if it changed.
pub fn save(world: &mut World) {
    let mut stage = world.resource_mut::<Stage>();
    if !stage.dirty {
        return;
    }
    stage.dirty = false;
    if let Some(story) = stage.story.as_mut()
        && let Err(e) = story.save()
    {
        warn!("could not keep the story: {e}");
    }
}

/// Every story in the library, for the window's list.
pub fn library_list() -> Vec<(String, String, usize, String)> {
    Story::all()
        .into_iter()
        .map(|s| {
            let changed = story::date(s.updated);
            (s.id, s.title, s.steps.len(), changed)
        })
        .collect()
}

/// The kit's picture, for the agent to see the shapes.
pub fn template_parts(t: &drawing::templates::Template) -> String {
    let mut out = format!(
        "Template {}: {} Drawn {} by {}; set it with <insert name=\"{}\" x y width/>. Its parts, to point at with annotate on= or recolour:\n",
        t.name, t.what, t.view[2], t.view[3], t.name
    );
    for (id, what) in &t.parts {
        out.push_str(&format!("- {id}: {what}\n"));
    }
    out
}
