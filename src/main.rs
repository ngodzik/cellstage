//! cellstage: a window an agent draws biology on while it explains it.
//!
//! The program is a Model Context Protocol server over standard input and
//! output, and a window. An agent connected to it, in a terminal or any
//! client, splits the window into panels and shows in them proteins at
//! their measured atomic structure, a cell under a fluorescence microscope
//! whose network cellweave runs live, and stained tissue sections drawn
//! from histology. The user turns and zooms what is shown, clicks on it,
//! and the agent hears what was clicked.
//!
//! Every panel says what it is: a measured structure, a simulation, or an
//! illustration.

#![deny(unsafe_code)]

mod cell;
mod mcp;
mod protein;
mod scene;
mod stage;
mod tissue;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use bevy::asset::RenderAssetUsages;
use bevy::camera::CameraOutputMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{BlendState, Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{ExitCondition, PrimaryWindow, WindowCloseRequested};
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, EguiTextureHandle,
    PrimaryEguiContext, egui,
};
use serde_json::Value;

use crate::cell::dish::{Dish, Fate, GENES, Switch, label};
use crate::cell::specimen::{self, Channel, Compartment, Specimen};
use crate::mcp::{Message, Reply};
use crate::protein::{Highlight, Look};
use crate::scene::{Scenes, Style};
use crate::stage::{Anchor, Arrangement, Kind, Note, Stage};
use crate::tissue::skin::{self, Lesion, Section};

/// Tool calls from the protocol's thread.
#[derive(Resource)]
struct Inbox(Mutex<Receiver<Message>>);

/// A cell panel: the dish, the microscope, the picture.
struct CellView {
    dish: Dish,
    specimen: Specimen,
    stains: [Option<usize>; 3],
    running: bool,
    owed: f32,
    since: f32,
    frame: u32,
    image: Handle<Image>,
}

/// A skin panel: the section and its picture.
struct SkinView {
    section: Section,
}

#[derive(Resource, Default)]
struct Views {
    cells: HashMap<String, CellView>,
    skins: HashMap<String, SkinView>,
}

/// The dyes of the cell's three protein channels.
const DYES: [[f32; 3]; 3] = [specimen::GREEN, specimen::RED, specimen::MAGENTA];

fn main() -> anyhow::Result<()> {
    let (to_window, inbox) = channel();
    std::thread::spawn(move || mcp::serve(to_window));
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "cellstage".to_owned(),
                resolution: (1600, 960).into(),
                // Shown when the agent first draws, not when it connects.
                visible: false,
                ..default()
            }),
            exit_condition: ExitCondition::DontExit,
            close_when_requested: false,
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .insert_resource(ClearColor(scene::BACKGROUND))
        .insert_resource(Inbox(Mutex::new(inbox)))
        .insert_resource(Stage::default())
        .insert_resource(Scenes::default())
        .insert_resource(Views::default())
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.75, 0.8, 1.0),
            brightness: 120.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (bridge, live_cells, scene::place, hide_on_close).chain(),
        )
        .add_systems(EguiPrimaryContextPass, chrome)
        .run();
    Ok(())
}

/// A camera that clears the whole window, and one that draws the panels'
/// frames and 2D content over everything.
fn setup(mut commands: Commands, mut egui: ResMut<EguiGlobalSettings>) {
    egui.auto_create_primary_context = false;
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(scene::BACKGROUND),
            ..default()
        },
        bevy::camera::Hdr,
        bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        Msaa::Off,
        RenderLayers::none(),
    ));
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        // The default needs lookup tables this build leaves out; the panels
        // are drawn in plain sRGB anyway.
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        RenderLayers::none(),
        Msaa::Off,
        Camera {
            order: 100,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
}

/// Hide the window when the user closes it: the agent may draw again.
fn hide_on_close(
    mut closed: MessageReader<WindowCloseRequested>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut stage: ResMut<Stage>,
) {
    if closed.read().count() > 0
        && let Ok(mut window) = windows.single_mut()
    {
        window.visible = false;
        stage.shown = false;
    }
}

/// Do every tool call that has arrived.
fn bridge(world: &mut World) {
    let calls: Vec<Message> = {
        let inbox = world.resource::<Inbox>();
        let Ok(rx) = inbox.0.lock() else { return };
        rx.try_iter().collect()
    };
    for message in calls {
        match message {
            Message::Quit => {
                world.write_message(AppExit::Success);
            }
            Message::Call(call) => {
                if call.tool == "screenshot" {
                    screenshot(world, call.reply);
                    continue;
                }
                let reply = match handle(world, &call.tool, &call.args) {
                    Ok(text) => Reply::text(text),
                    Err(e) => Reply::error(e),
                };
                let _ = call.reply.send(reply);
            }
        }
    }
}

/// Show the window if it is not.
fn reveal(world: &mut World) {
    if world.resource::<Stage>().shown {
        return;
    }
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    if let Ok(mut window) = windows.single_mut(world) {
        window.visible = true;
    }
    world.resource_mut::<Stage>().shown = true;
}

fn text(args: &Value, key: &str) -> Option<String> {
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

/// Do one tool call, and say what was done.
fn handle(world: &mut World, tool: &str, args: &Value) -> Result<String, String> {
    match tool {
        "stage_layout" => layout(world, args),
        "show_protein" => show_protein(world, args),
        "show_cell" => show_cell(world, args),
        "show_skin" => show_skin(world, args),
        "annotate" => annotate(world, args),
        "clear_annotations" => {
            let only = text(args, "panel");
            for panel in world.resource_mut::<Stage>().panels.iter_mut() {
                if only.as_ref().is_none_or(|p| *p == panel.id) {
                    panel.notes.clear();
                }
            }
            Ok("cleared".to_owned())
        }
        "focus_protein" => {
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
        "get_user_events" => {
            let events: Vec<String> = std::mem::take(&mut world.resource_mut::<Stage>().events);
            Ok(if events.is_empty() {
                "The user has clicked nothing since you last asked.".to_owned()
            } else {
                events.join("\n")
            })
        }
        other => Err(format!("no tool {other}")),
    }
}

fn layout(world: &mut World, args: &Value) -> Result<String, String> {
    let panels = args
        .get("panels")
        .and_then(Value::as_array)
        .ok_or("panels: a list")?;
    let asked: Vec<(String, String)> = panels
        .iter()
        .take(4)
        .filter_map(|p| {
            let id = text(p, "id")?;
            let title = text(p, "title").unwrap_or_else(|| id.clone());
            Some((id, title))
        })
        .collect();
    if asked.is_empty() {
        return Err("at least one panel, with an id".to_owned());
    }
    let gone: Vec<String> = world
        .resource::<Stage>()
        .panels
        .iter()
        .filter(|p| !asked.iter().any(|(id, _)| *id == p.id))
        .map(|p| p.id.clone())
        .collect();
    for id in &gone {
        scene::remove(world, id);
        let mut views = world.resource_mut::<Views>();
        views.cells.remove(id);
        views.skins.remove(id);
    }
    let mut stage = world.resource_mut::<Stage>();
    stage
        .panels
        .retain(|p| asked.iter().any(|(id, _)| *id == p.id));
    let mut ordered = Vec::new();
    for (id, title) in &asked {
        let mut panel = stage
            .panels
            .iter()
            .position(|p| p.id == *id)
            .map(|i| stage.panels.remove(i))
            .unwrap_or_else(|| stage::Panel::new(id, title));
        panel.title = title.clone();
        ordered.push(panel);
    }
    stage.panels = ordered;
    stage.arrangement = match text(args, "arrangement").as_deref() {
        Some("rows") => Arrangement::Rows,
        Some("grid") => Arrangement::Grid,
        _ => Arrangement::Columns,
    };
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
    if !keep_cell {
        scene::remove(world, &id);
        let mut views = world.resource_mut::<Views>();
        views.cells.remove(&id);
        views.skins.remove(&id);
    }
    let mut stage = world.resource_mut::<Stage>();
    let panel = stage
        .panel(&id)
        .ok_or("four panels at most; lay them out again to make room")?;
    if panel.kind != kind {
        panel.notes.clear();
        panel.zoom = 1.0;
        panel.pan = egui::Vec2::ZERO;
    }
    panel.kind = kind;
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

fn show_protein(world: &mut World, args: &Value) -> Result<String, String> {
    let first = text(args, "pdb_id").ok_or("which PDB entry?")?;
    let mut texts = vec![protein::fetch(&first).map_err(|e| e.to_string())?];
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
            panel.source = match &second {
                Some(s) => format!(
                    "PDB {} and {}, {experiment}",
                    first.to_uppercase(),
                    s.to_uppercase()
                ),
                None => format!("PDB {}, {experiment}", first.to_uppercase()),
            };
        }
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

fn show_cell(world: &mut World, args: &Value) -> Result<String, String> {
    let id = prepare(world, args, Kind::Cell, "One cell")?;
    if !world.resource::<Views>().cells.contains_key(&id)
        || args.get("new_cell").and_then(Value::as_bool) == Some(true)
    {
        let dish = Dish::new().map_err(|e| e.to_string())?;
        let image = world
            .resource_mut::<Assets<Image>>()
            .add(blank(specimen::SIZE, specimen::SIZE));
        let stains = [dish.index("erk"), dish.index("bcl2"), dish.index("egfr")];
        world.resource_mut::<Views>().cells.insert(
            id.clone(),
            CellView {
                dish,
                specimen: Specimen::new(11),
                stains,
                running: true,
                owed: 0.0,
                since: 1.0,
                frame: 0,
                image: image.clone(),
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
        for (node, gene) in GENES.iter().enumerate() {
            let switch = if removed.iter().any(|g| g == gene) {
                Switch::Removed
            } else if stuck.iter().any(|g| g == gene) {
                Switch::StuckOn
            } else {
                Switch::Normal
            };
            if let Some(i) = dish.index(gene) {
                let _ = node;
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
    let mut image = blank(skin::WIDTH, skin::HEIGHT);
    if let Some(data) = image.data.as_mut() {
        data.copy_from_slice(&section.image);
    }
    let handle = world.resource_mut::<Assets<Image>>().add(image);
    let breslow = section.breslow;
    world
        .resource_mut::<Views>()
        .skins
        .insert(id.clone(), SkinView { section });
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
    Ok(answer)
}

fn annotate(world: &mut World, args: &Value) -> Result<String, String> {
    let id = text(args, "panel").ok_or("which panel?")?;
    let note = text(args, "text").ok_or("what to write?")?;
    let at = match (
        args.get("residue").and_then(Value::as_i64),
        number(args, "x"),
        number(args, "y"),
    ) {
        (Some(r), _, _) => Anchor::Residue(
            text(args, "chain")
                .and_then(|c| c.chars().next())
                .unwrap_or(' '),
            r as i32,
        ),
        (None, Some(x), Some(y)) => Anchor::Fraction(x as f32, y as f32),
        _ => Anchor::Fraction(0.05, 0.12),
    };
    let arrow_to = args
        .get("arrow_to")
        .and_then(Value::as_array)
        .and_then(|a| Some((a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32)));
    let mut stage = world.resource_mut::<Stage>();
    let panel = stage
        .panels
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("no such panel")?;
    panel.notes.push(Note {
        text: note,
        at,
        arrow_to,
    });
    Ok("noted".to_owned())
}

/// A picture of the window, sent back as a PNG once the renderer has
/// taken it.
fn screenshot(world: &mut World, reply: Sender<Reply>) {
    if !world.resource::<Stage>().shown {
        let _ = reply.send(Reply::error(
            "the window is not shown yet: draw something first",
        ));
        return;
    }
    let reply = Mutex::new(Some(reply));
    world
        .spawn(Screenshot::primary_window())
        .observe(move |captured: On<ScreenshotCaptured>| {
            let Some(reply) = reply.lock().ok().and_then(|mut r| r.take()) else {
                return;
            };
            let answer = match png(&captured.image) {
                Ok(png) => Reply {
                    text: "The window as it is now.".to_owned(),
                    image: Some(png),
                    error: false,
                },
                Err(e) => Reply::error(format!("could not encode the picture: {e}")),
            };
            let _ = reply.send(answer);
        });
}

/// An image the renderer captured, as a PNG, halved if it is large.
fn png(image: &Image) -> anyhow::Result<Vec<u8>> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let data = image
        .data
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no pixels"))?;
    let bgra = matches!(
        image.texture_descriptor.format,
        TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm
    );
    let halve = w > 1400;
    let (ow, oh) = if halve { (w / 2, h / 2) } else { (w, h) };
    let mut rgb = Vec::with_capacity(ow * oh * 3);
    for y in 0..oh {
        for x in 0..ow {
            let mut sum = [0u32; 3];
            let samples: &[(usize, usize)] = if halve {
                &[(0, 0), (1, 0), (0, 1), (1, 1)]
            } else {
                &[(0, 0)]
            };
            for (dx, dy) in samples {
                let (sx, sy) = if halve {
                    (x * 2 + dx, y * 2 + dy)
                } else {
                    (x, y)
                };
                let i = (sy * w + sx) * 4;
                let (r, g, b) = if bgra {
                    (data[i + 2], data[i + 1], data[i])
                } else {
                    (data[i], data[i + 1], data[i + 2])
                };
                sum[0] += u32::from(r);
                sum[1] += u32::from(g);
                sum[2] += u32::from(b);
            }
            let n = samples.len() as u32;
            rgb.extend_from_slice(&[(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]);
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, ow as u32, oh as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgb)?;
    }
    Ok(out)
}

/// A black image of a size, kept on the CPU to be written into.
fn blank(width: usize, height: usize) -> Image {
    Image::new_fill(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// The cells live: each running dish steps twice a second, and its picture
/// is exposed again some fifteen times a second.
fn live_cells(time: Res<Time>, mut views: ResMut<Views>, mut images: ResMut<Assets<Image>>) {
    let dt = time.delta_secs();
    for view in views.cells.values_mut() {
        if view.running {
            view.owed += dt * 2.0;
            while view.owed >= 1.0 {
                view.owed -= 1.0;
                view.dish.step();
            }
        }
        view.since += dt;
        if view.since < 1.0 / 15.0 {
            continue;
        }
        view.since = 0.0;
        view.frame = view.frame.wrapping_add(1);
        let levels = view.dish.levels();
        let names = view.dish.names().to_vec();
        let mut channels: Vec<Channel> = view
            .stains
            .iter()
            .zip(DYES)
            .filter_map(|(stain, colour)| {
                let node = (*stain)?;
                let level = *levels.get(node)? as f32;
                Some(Channel {
                    layers: specimen::pattern(&names[node], level),
                    amount: level,
                    colour,
                    gain: 1.6,
                })
            })
            .collect();
        channels.push(Channel {
            layers: specimen::dna_stain(),
            amount: 1.0,
            colour: specimen::BLUE,
            gain: 0.5,
        });
        if let Some(mut image) = images.get_mut(&view.image)
            && let Some(data) = image.data.as_mut()
        {
            view.specimen
                .expose(&channels, view.dish.fate, view.frame, data);
        }
    }
}

/// Draw the panels, their images, frames and notes, and hear the user:
/// drags turn proteins and move images, the wheel zooms, a click is
/// recorded with what is under it.
#[allow(clippy::too_many_arguments)]
fn chrome(
    mut contexts: EguiContexts,
    mut stage: ResMut<Stage>,
    mut scenes: ResMut<Scenes>,
    views: Res<Views>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let area = ctx.content_rect().shrink(6.0);
    stage.lay_out(area);
    let painter = ctx.layer_painter(egui::LayerId::background());
    if stage.panels.is_empty() {
        painter.text(
            area.center(),
            egui::Align2::CENTER_CENTER,
            "cellstage · waiting for the agent",
            egui::FontId::proportional(20.0),
            egui::Color32::from_gray(140),
        );
        return Ok(());
    }
    // Textures for images not yet registered.
    let mut textures = Vec::new();
    for (i, panel) in stage.panels.iter().enumerate() {
        if panel.texture.is_none()
            && let Some(image) = &panel.image
        {
            textures.push((i, image.clone()));
        }
    }
    for (i, image) in textures {
        let id = contexts.add_image(EguiTextureHandle::Strong(image));
        stage.panels[i].texture = Some(id);
    }
    let mut events = Vec::new();
    egui::Area::new(egui::Id::new("stage"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Background)
        .show(&ctx, |ui| {
            for panel in stage.panels.iter_mut() {
                let content = panel.content();
                let solid = panel.kind != Kind::Protein;
                stage::frame(ui.painter(), panel, solid);
                let size = match panel.kind {
                    Kind::Cell => (specimen::SIZE, specimen::SIZE),
                    Kind::Skin => (skin::WIDTH, skin::HEIGHT),
                    _ => (1, 1),
                };
                let response = ui.interact(
                    content,
                    ui.id().with(&panel.id),
                    egui::Sense::click_and_drag(),
                );
                let scroll = if response.hovered() {
                    ui.input(|i| i.smooth_scroll_delta.y)
                } else {
                    0.0
                };
                let image_rect = stage::image_rect(content, size, panel.zoom, panel.pan);
                if let (Some(texture), true) = (panel.texture, solid) {
                    let clipped = ui.painter().with_clip_rect(content);
                    clipped.image(
                        texture,
                        image_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    // A scale bar, as on every micrograph.
                    let (micrometres, per_pixel, ink) = match panel.kind {
                        Kind::Cell => (10.0, specimen::MICROMETRES_PER_PIXEL, egui::Color32::WHITE),
                        _ => (
                            100.0,
                            skin::MICROMETRES_PER_PIXEL,
                            egui::Color32::from_gray(25),
                        ),
                    };
                    let length = micrometres / per_pixel * image_rect.width() / size.0 as f32;
                    let inside = image_rect.intersect(content);
                    let corner = inside.right_bottom() + egui::vec2(-16.0 - length, -16.0);
                    clipped.line_segment(
                        [corner, corner + egui::vec2(length, 0.0)],
                        egui::Stroke::new(4.0, ink),
                    );
                    clipped.text(
                        corner + egui::vec2(length / 2.0, -5.0),
                        egui::Align2::CENTER_BOTTOM,
                        format!("{micrometres:.0} µm"),
                        egui::FontId::proportional(12.0),
                        ink,
                    );
                }
                match panel.kind {
                    Kind::Protein => {
                        if let Some(scene) = scenes.map.get_mut(&panel.id) {
                            if response.dragged() {
                                let d = response.drag_delta();
                                scene.orbit.turn(Vec2::new(d.x, d.y));
                            }
                            if scroll != 0.0 {
                                scene.orbit.zoom(scroll / 40.0);
                            }
                        }
                    }
                    Kind::Cell | Kind::Skin => {
                        if scroll != 0.0 {
                            panel.zoom = (panel.zoom * (1.0 + scroll * 0.002)).clamp(1.0, 8.0);
                        }
                        if response.dragged() {
                            let d = response.drag_delta();
                            panel.pan -=
                                egui::vec2(d.x / image_rect.width(), d.y / image_rect.height());
                        }
                        let limit = 0.5 - 0.5 / panel.zoom;
                        panel.pan = panel
                            .pan
                            .clamp(egui::Vec2::splat(-limit), egui::Vec2::splat(limit));
                    }
                    Kind::Empty => {}
                }
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    let fx = (pointer.x - content.left()) / content.width();
                    let fy = (pointer.y - content.top()) / content.height();
                    let what = match panel.kind {
                        Kind::Protein => scenes.map.get(&panel.id).and_then(|scene| {
                            let (camera, eye) = cameras.get(scene.camera).ok()?;
                            let local =
                                Vec2::new(pointer.x - content.left(), pointer.y - content.top());
                            let ray = camera.viewport_to_world(eye, local).ok()?;
                            scene
                                .pick(ray.origin, *ray.direction)
                                .map(|a| scene::describe(a, &scene.look, scene.t))
                        }),
                        Kind::Cell => views.cells.get(&panel.id).map(|v| {
                            let p = image_point(image_rect, pointer, size);
                            let c = v.specimen.compartment(p.x, p.y);
                            if c == Compartment::Outside {
                                c.name().to_owned()
                            } else {
                                format!("{}: {}", c.name(), c.what())
                            }
                        }),
                        Kind::Skin => views.skins.get(&panel.id).map(|v| {
                            let p = image_point(image_rect, pointer, size);
                            let r = v.section.region(p.x, p.y);
                            format!("{}: {}", r.name(), r.what())
                        }),
                        Kind::Empty => None,
                    };
                    let what = what.unwrap_or_else(|| "nothing in particular".to_owned());
                    events.push(format!(
                        "In panel {} (\"{}\"), at {:.2}, {:.2} of its width and height: {what}",
                        panel.id, panel.title, fx, fy
                    ));
                    // A brief ring where the user clicked, so they see it was heard.
                    ui.painter().circle_stroke(
                        pointer,
                        10.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 236, 160)),
                    );
                }
                // The agent's notes.
                for n in &panel.notes {
                    let at = match &n.at {
                        Anchor::Fraction(x, y) => Some(
                            content.min + egui::vec2(x * content.width(), y * content.height()),
                        ),
                        Anchor::Residue(chain, number) => {
                            scenes.map.get(&panel.id).and_then(|scene| {
                                let chain = (*chain != ' ').then_some(*chain);
                                let atom = scene.residue(chain, *number)?;
                                let (camera, eye) = cameras.get(scene.camera).ok()?;
                                let p = camera.world_to_viewport(eye, atom.at(scene.t)).ok()?;
                                Some(content.min + egui::vec2(p.x, p.y))
                            })
                        }
                    };
                    if let Some(at) = at {
                        let to = n.arrow_to.map(|(x, y)| {
                            content.min + egui::vec2(x * content.width(), y * content.height())
                        });
                        stage::note(&ui.painter().with_clip_rect(panel.rect), at, &n.text, to);
                    }
                }
                // A protein seen in two states gets a slider between them.
                if let Some(scene) = scenes.map.get_mut(&panel.id)
                    && scene.model.moves
                {
                    let bar = egui::Rect::from_min_size(
                        content.left_bottom() + egui::vec2(10.0, -34.0),
                        egui::vec2(content.width().min(420.0), 26.0),
                    );
                    ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .button(if scene.playing { "pause" } else { "play" })
                                .clicked()
                            {
                                scene.playing = !scene.playing;
                            }
                            ui.label("first");
                            ui.add(egui::Slider::new(&mut scene.t, 0.0..=1.0).show_value(false));
                            ui.label("second");
                        });
                    });
                }
            }
        });
    stage.events.extend(events);
    Ok(())
}

/// The pixel of an image under a point of the screen.
fn image_point(rect: egui::Rect, pointer: egui::Pos2, size: (usize, usize)) -> Vec2 {
    Vec2::new(
        (pointer.x - rect.left()) / rect.width() * size.0 as f32,
        (pointer.y - rect.top()) / rect.height() * size.1 as f32,
    )
}
