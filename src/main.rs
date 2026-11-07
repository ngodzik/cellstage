//! cellstage: a window an agent tells biology on, step by step.
//!
//! The program is a Model Context Protocol server over standard input and
//! output, and a window. An agent connected to it, in a terminal or any
//! client, tells a story in steps the user moves through with the arrow
//! keys. Each step splits the window into panels and shows in them
//! drawings the agent writes in SVG, from a house of shapes and templates
//! with named parts, which move from one frame to the next; diagrams and
//! charts the window lays out; pages of words; pictures from Wikimedia
//! Commons; proteins at their measured atomic structure or as AlphaFold
//! predicts them; a cell under a fluorescence microscope whose network
//! cellweave runs live; stained tissue sections drawn from histology. Every
//! story is kept in a library on disk, to be told again at once. The user
//! turns and zooms what is shown, clicks on it, and the agent hears what
//! was clicked.
//!
//! Every panel says what it is: a measured or a predicted structure, a
//! simulation, an illustration, a chart of numbers given, a picture and
//! its author.

#![deny(unsafe_code)]
// The tools' descriptions are one long json! literal.
#![recursion_limit = "512"]

mod cell;
mod chrome;
mod drawing;
mod figure;
mod mcp;
mod protein;
mod scene;
mod sources;
mod stage;
mod story;
mod tissue;
mod tools;

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use bevy::asset::RenderAssetUsages;
use bevy::camera::CameraOutputMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{BlendState, Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{ExitCondition, PrimaryWindow, WindowCloseRequested};
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext};

use crate::cell::specimen::{self, Channel};
use crate::drawing::Canvas;
use crate::mcp::{Message, Reply};
use crate::scene::Scenes;
use crate::stage::{Kind, Stage};
use crate::story::Story;
use crate::tools::{Mode, Views};

/// Tool calls from the protocol's thread.
#[derive(Resource)]
struct Inbox(Mutex<Receiver<Message>>);

/// The dyes of the cell's three protein channels.
const DYES: [[f32; 3]; 3] = [specimen::GREEN, specimen::RED, specimen::MAGENTA];

fn main() -> anyhow::Result<()> {
    let (to_window, inbox) = channel();
    std::thread::spawn(move || mcp::serve(to_window));
    drawing::warm();
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
            (
                bridge,
                live_cells,
                render_drawings,
                scene::place,
                hide_on_close,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, chrome::chrome)
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

/// Do every tool call that has arrived, then what the user asked of the
/// window, and keep the story if it changed.
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
            Message::Call(call) => match call.tool.as_str() {
                "screenshot" => screenshot(world, call.reply),
                "drawing_kit" => {
                    let args = &call.args;
                    let picture =
                        args.get("picture").and_then(serde_json::Value::as_bool) == Some(true);
                    let template = args.get("template").and_then(serde_json::Value::as_str);
                    let page = args
                        .get("page")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(1) as usize;
                    let (text, sheet) = match template {
                        Some(name) => match drawing::templates::find(name) {
                            Some(t) => (
                                tools::template_parts(t),
                                if picture {
                                    Some(drawing::template_sheet(name))
                                } else {
                                    None
                                },
                            ),
                            None => (
                                format!(
                                    "No template {name}; there are {}.",
                                    drawing::templates::names().join(", ")
                                ),
                                None,
                            ),
                        },
                        None => {
                            let mut text = drawing::kit();
                            text.push_str(&format!(
                                "\n\nThe kit's picture has {} pages: picture with page 1 to {} for the shapes, page {} for the colours.",
                                drawing::kit_pages(),
                                drawing::kit_pages() - 1,
                                drawing::kit_pages()
                            ));
                            let sheet = picture.then(|| {
                                drawing::kit_sheet(page).ok_or_else(|| format!("no page {page}"))
                            });
                            (text, sheet)
                        }
                    };
                    let mut reply = Reply::text(text);
                    if let Some(sheet) = sheet {
                        match sheet.and_then(|s| drawing::picture_of(&s, 1100)).and_then(
                            |(rgba, w, h): (Vec<u8>, u32, u32)| {
                                encode(&rgba, w, h, 4).map_err(|e| e.to_string())
                            },
                        ) {
                            Ok(png) => reply.image = Some(png),
                            Err(e) => reply.text.push_str(&format!("\n(no picture: {e})")),
                        }
                    }
                    let _ = call.reply.send(reply);
                }
                _ => {
                    let reply = match tools::handle(world, &call.tool, &call.args, Mode::Record) {
                        Ok(text) => Reply::text(text),
                        Err(e) => Reply::error(e),
                    };
                    let _ = call.reply.send(reply);
                }
            },
        }
    }
    let (goto, open) = {
        let mut stage = world.resource_mut::<Stage>();
        (stage.goto.take(), stage.open.take())
    };
    if let Some(id) = open {
        match Story::load(&id) {
            Ok(story) => {
                let title = story.title.clone();
                {
                    let mut stage = world.resource_mut::<Stage>();
                    stage.story = Some(story);
                    stage.step = 0;
                }
                tools::show_step(world, 0, false);
                world.resource_mut::<Stage>().events.push(format!(
                    "The user opened the story \"{title}\" ({id}) from the library."
                ));
            }
            Err(e) => warn!("{e}"),
        }
    }
    if let Some(n) = goto {
        let forward = n == world.resource::<Stage>().step + 1;
        tools::show_step(world, n, forward);
        let mut stage = world.resource_mut::<Stage>();
        let said = stage.story.as_ref().map(|s| {
            format!(
                "The user moved to step {} of {}, \"{}\".",
                n + 1,
                s.steps.len(),
                s.steps.get(n).map_or("", |t| t.title.as_str())
            )
        });
        if let Some(said) = said {
            stage.events.push(said);
        }
    }
    tools::save(world);
    // The window says which story it tells.
    let title = world
        .resource::<Stage>()
        .story
        .as_ref()
        .map_or("cellstage".to_owned(), |s| {
            format!("cellstage · {}", s.title)
        });
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    if let Ok(mut window) = windows.single_mut(world)
        && window.title != title
    {
        window.title = title;
    }
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

/// Pixels as a PNG, RGB or RGBA by `channels`.
fn encode(pixels: &[u8], w: u32, h: u32, channels: usize) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(if channels == 4 {
            png::ColorType::Rgba
        } else {
            png::ColorType::Rgb
        });
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixels)?;
    }
    Ok(out)
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
    encode(&rgb, ow as u32, oh as u32, 3)
}

/// A black image of a size, kept on the CPU to be written into.
pub fn blank(width: usize, height: usize) -> Image {
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

/// Render every drawing that changed: its film moved on, its panel moved
/// or zoomed. A drawing is rendered at the panel's size in pixels, so that
/// it stays sharp however far in the user goes.
fn render_drawings(
    time: Res<Time>,
    stage: Res<Stage>,
    mut views: ResMut<Views>,
    mut images: ResMut<Assets<Image>>,
) {
    let dt = time.delta_secs();
    let ppp = stage.pixels_per_point;
    for panel in &stage.panels {
        if panel.kind != Kind::Drawing || !panel.rect.is_positive() {
            continue;
        }
        let Some(view) = views.drawings.get_mut(&panel.id) else {
            continue;
        };
        if view.playing {
            view.clock += dt;
            if !view.film.looping && view.clock >= view.film.length() {
                view.clock = view.film.length();
                view.playing = false;
            }
        }
        let content = panel.content();
        let image_rect = stage::image_rect(content, view.size, panel.zoom, panel.pan);
        let position = view.film.position(view.clock);
        // A drawing on the move is rendered at half the pixels, which the
        // eye does not see, and sharp again once it stands still.
        let last = view.film.frames().len().saturating_sub(1) as f32;
        let moving = position.fract() > 0.002 && position < last;
        let ppp = if moving && content.area() * ppp * ppp > 250_000.0 {
            ppp * 0.5
        } else {
            ppp
        };
        let (w, h) = (
            (content.width() * ppp).round().max(1.0) as u32,
            (content.height() * ppp).round().max(1.0) as u32,
        );
        let canvas = Canvas {
            width: w,
            height: h,
            scale: image_rect.width() / view.size.0 * ppp,
            offset: (
                (image_rect.left() - content.left()) * ppp,
                (image_rect.top() - content.top()) * ppp,
            ),
            background: view.background,
        };
        if view.last == Some((position, canvas)) {
            continue;
        }
        match drawing::render(&view.film.picture(position), &canvas) {
            Ok((pixels, parts)) => {
                if let Some(mut image) = images.get_mut(&view.image) {
                    if image.width() != w || image.height() != h {
                        image.resize(Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        });
                    }
                    image.data = Some(pixels);
                }
                view.parts = parts;
            }
            Err(e) => warn!("panel {}: {e}", panel.id),
        }
        view.last = Some((position, canvas));
    }
}
