//! The window's face: the story's bar at the top, the step's words and the
//! row of steps at the bottom, the panels between, the library when it is
//! open. It hears the user: the arrow keys and the buttons move through
//! the steps, drags turn proteins and move pictures, the wheel zooms, and a
//! click is recorded with what is under it.

use std::time::Instant;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};

use crate::cell::specimen::{self, Compartment};
use crate::scene::{self, Scenes};
use crate::stage::{self, Anchor, Kind, Stage};
use crate::tissue::skin;
use crate::tools::{self, Views};
use crate::world::Worlds;

const BACK: egui::Color32 = egui::Color32::from_rgb(10, 12, 16);
const WORDS: egui::Color32 = egui::Color32::from_rgb(226, 229, 236);
const DIM: egui::Color32 = egui::Color32::from_rgb(140, 146, 160);

/// The library's list, read from disk at most every two seconds.
#[derive(Default)]
pub struct Shelf {
    read: Option<Instant>,
    stories: Vec<(String, String, usize, String)>,
}

#[allow(clippy::too_many_arguments)]
pub fn chrome(
    mut contexts: EguiContexts,
    mut stage: ResMut<Stage>,
    mut scenes: ResMut<Scenes>,
    views: Res<Views>,
    mut worlds: ResMut<Worlds>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut shelf: Local<Shelf>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let steps = stage.story.as_ref().map_or(0, |s| s.steps.len());

    // The arrow keys move through the steps.
    if steps > 0 && !ctx.egui_wants_keyboard_input() {
        let (next, back) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown),
                i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp),
            )
        });
        if next && stage.step + 1 < steps {
            stage.goto = Some(stage.step + 1);
        }
        if back && stage.step > 0 {
            stage.goto = Some(stage.step - 1);
        }
    }

    // The space bar plays and pauses the films of the worlds shown.
    if !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space)) {
        for scene in worlds.map.values_mut() {
            if scene.playing && !scene.manual {
                scene.playing = false;
            } else if scene.clock >= scene.film.length() {
                scene.replay();
            } else {
                scene.playing = true;
                scene.give_back();
            }
        }
    }

    let full = ctx.content_rect();
    let top = bar(&ctx, &mut stage, steps, full);
    let bottom = words(&ctx, &mut stage, steps, full);
    if stage.library {
        library(&ctx, &mut stage, &mut shelf);
    }

    let area = egui::Rect::from_min_max(
        egui::pos2(full.left(), top),
        egui::pos2(full.right(), bottom),
    )
    .shrink(6.0);
    stage.lay_out(area);
    let painter = ctx.layer_painter(egui::LayerId::background());
    if stage.panels.is_empty() {
        painter.text(
            area.center(),
            egui::Align2::CENTER_CENTER,
            if steps > 0 {
                "this step is empty"
            } else {
                "cellstage · waiting for the agent"
            },
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
                let solid = !matches!(panel.kind, Kind::Protein | Kind::World);
                stage::frame(ui.painter(), panel, solid);
                let size = match panel.kind {
                    Kind::Cell => (specimen::SIZE as f32, specimen::SIZE as f32),
                    Kind::Skin => (skin::WIDTH as f32, skin::HEIGHT as f32),
                    _ => (1.0, 1.0),
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
                let clipped = ui.painter().with_clip_rect(content);
                let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                if let Some(texture) = panel.texture {
                    match panel.kind {
                        Kind::Cell | Kind::Skin => {
                            clipped.image(texture, image_rect, whole, egui::Color32::WHITE);
                            scale_bar(&clipped, panel.kind, image_rect, content, size.0);
                        }
                        _ => {}
                    }
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
                    Kind::World => {
                        if let Some(scene) = worlds.map.get_mut(&panel.id) {
                            if response.dragged() {
                                let d = response.drag_delta();
                                scene.take();
                                scene.eye.turn(Vec2::new(d.x, d.y));
                            }
                            if scroll != 0.0 {
                                scene.take();
                                scene.eye.zoom(scroll / 40.0);
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
                    let p = image_point(image_rect, pointer, size);
                    let what = match panel.kind {
                        Kind::Protein => scenes.map.get(&panel.id).and_then(|scene| {
                            let (camera, eye) = cameras.get(scene.camera).ok()?;
                            // Bevy's viewport coordinates start at the window's
                            // corner, not the viewport's, as egui's do.
                            let ray = camera
                                .viewport_to_world(eye, Vec2::new(pointer.x, pointer.y))
                                .ok()?;
                            scene
                                .pick(ray.origin, *ray.direction)
                                .map(|a| scene::describe(a, &scene.look, scene.t))
                        }),
                        Kind::Cell => views.cells.get(&panel.id).map(|v| {
                            let c = v.specimen.compartment(p.x, p.y);
                            if c == Compartment::Outside {
                                c.name().to_owned()
                            } else {
                                format!("{}: {}", c.name(), c.what())
                            }
                        }),
                        Kind::Skin => views.skins.get(&panel.id).map(|v| {
                            let r = v.section.region(p.x, p.y);
                            format!("{}: {}", r.name(), r.what())
                        }),
                        Kind::World => worlds.map.get(&panel.id).and_then(|scene| {
                            let (camera, eye) = cameras.get(scene.camera).ok()?;
                            let ray = camera
                                .viewport_to_world(eye, Vec2::new(pointer.x, pointer.y))
                                .ok()?;
                            scene.pick(ray.origin, *ray.direction)
                        }),
                        Kind::Empty => None,
                    };
                    let what = what.unwrap_or_else(|| "nothing in particular".to_owned());
                    events.push(format!(
                        "In panel {} (\"{}\"), at {:.2}, {:.2} of its width and height: {what}",
                        panel.id, panel.title, fx, fy
                    ));
                    ui.painter().circle_stroke(
                        pointer,
                        10.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 236, 160)),
                    );
                }
                // The agent's notes, each beside what it points at.
                let note_painter = ui.painter().with_clip_rect(panel.rect);
                let mut taken: Vec<egui::Rect> = Vec::new();
                for n in &panel.notes {
                    let target = n.target.as_ref().and_then(|t| match t {
                        Anchor::Fraction(x, y) => Some(
                            content.min + egui::vec2(x * content.width(), y * content.height()),
                        ),
                        Anchor::Residue(chain, number) => {
                            scenes.map.get(&panel.id).and_then(|scene| {
                                let chain = (*chain != ' ').then_some(*chain);
                                let atom = scene.residue(chain, *number)?;
                                let (camera, eye) = cameras.get(scene.camera).ok()?;
                                let p = camera.world_to_viewport(eye, atom.at(scene.t)).ok()?;
                                Some(egui::pos2(p.x, p.y))
                            })
                        }
                        Anchor::Part(name) if panel.kind == Kind::World => {
                            worlds.map.get(&panel.id).and_then(|scene| {
                                let p = scene.locate(name)?;
                                let (camera, eye) = cameras.get(scene.camera).ok()?;
                                let p = camera.world_to_viewport(eye, p).ok()?;
                                let p = egui::pos2(p.x, p.y);
                                content.contains(p).then_some(p)
                            })
                        }
                        Anchor::Part(name) => {
                            let at = match panel.kind {
                                Kind::Skin => views.skins.get(&panel.id).and_then(|v| {
                                    v.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p)
                                }),
                                Kind::Cell => views.cells.get(&panel.id).and_then(|v| {
                                    v.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p)
                                }),
                                _ => None,
                            }?;
                            Some(egui::pos2(
                                image_rect.left() + at.0 / size.0 * image_rect.width(),
                                image_rect.top() + at.1 / size.1 * image_rect.height(),
                            ))
                        }
                    });
                    if n.target.is_some() && target.is_none() {
                        // What it points at is out of sight.
                        continue;
                    }
                    let rect = match (n.label, target) {
                        (Some((x, y)), _) => stage::label_at(
                            &note_painter,
                            &n.text,
                            content.min + egui::vec2(x * content.width(), y * content.height()),
                        ),
                        (None, Some(t)) => {
                            stage::place_label(&note_painter, &n.text, t, content, &taken)
                        }
                        (None, None) => continue,
                    };
                    taken.push(rect);
                    stage::note(&note_painter, rect, &n.text, target);
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
                if let Some(scene) = worlds.map.get_mut(&panel.id) {
                    film_controls(ui, scene, content);
                }
            }
        });
    stage.events.extend(events);
    Ok(())
}

/// Height of the bar at the top, in points.
const BAR: f32 = 40.0;

/// The bar at the top: the story, the step, the buttons. Returns where it
/// ends.
fn bar(ctx: &egui::Context, stage: &mut Stage, steps: usize, full: egui::Rect) -> f32 {
    let rect = egui::Rect::from_min_size(full.min, egui::vec2(full.width(), BAR));
    ctx.layer_painter(egui::LayerId::background())
        .rect_filled(rect, 0.0, BACK);
    egui::Area::new(egui::Id::new("story"))
        .fixed_pos(rect.min + egui::vec2(12.0, 8.0))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let title = stage
                    .story
                    .as_ref()
                    .map_or("cellstage".to_owned(), |s| s.title.clone());
                ui.label(egui::RichText::new(title).size(18.0).strong().color(WORDS));
                if let Some(step) = stage.story.as_ref().and_then(|s| s.steps.get(stage.step)) {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "step {} of {} · {}",
                            stage.step + 1,
                            steps,
                            step.title
                        ))
                        .size(15.0)
                        .color(DIM),
                    );
                }
            });
        });
    egui::Area::new(egui::Id::new("buttons"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, full.top() + 7.0))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if steps > 0 {
                    ui.label(
                        egui::RichText::new("arrow keys to move")
                            .size(12.0)
                            .color(DIM),
                    );
                    if ui
                        .add_enabled(stage.step > 0, egui::Button::new("◀ back"))
                        .clicked()
                    {
                        stage.goto = Some(stage.step - 1);
                    }
                    if ui
                        .add_enabled(stage.step + 1 < steps, egui::Button::new("next ▶"))
                        .clicked()
                    {
                        stage.goto = Some(stage.step + 1);
                    }
                    ui.add_space(10.0);
                }
                if ui
                    .selectable_label(stage.library, "library")
                    .on_hover_text("every story told so far")
                    .clicked()
                {
                    stage.library = !stage.library;
                }
            });
        });
    rect.bottom()
}

/// The bottom: what the step says, and the row of steps. Returns where it
/// begins.
fn words(ctx: &egui::Context, stage: &mut Stage, steps: usize, full: egui::Rect) -> f32 {
    let Some(story) = stage.story.as_ref() else {
        return full.bottom();
    };
    let narration = story
        .steps
        .get(stage.step)
        .map(|s| s.narration.clone())
        .unwrap_or_default();
    let titles: Vec<String> = story.steps.iter().map(|s| s.title.clone()).collect();
    let width = full.width() - 32.0;
    let painter = ctx.layer_painter(egui::LayerId::background());
    let text_height = if narration.is_empty() {
        0.0
    } else {
        painter
            .layout(
                narration.clone(),
                egui::FontId::proportional(17.0),
                WORDS,
                width,
            )
            .size()
            .y
            .min(150.0)
            + 8.0
    };
    let row = if steps > 1 { 30.0 } else { 0.0 };
    let height = text_height + row + 14.0;
    let rect = egui::Rect::from_min_max(egui::pos2(full.left(), full.bottom() - height), full.max);
    painter.rect_filled(rect, 0.0, BACK);
    egui::Area::new(egui::Id::new("words"))
        .fixed_pos(rect.min + egui::vec2(16.0, 8.0))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_width(width);
            if !narration.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&narration).size(17.0).color(WORDS),
                            )
                            .wrap(),
                        );
                    });
                ui.add_space(4.0);
            }
            if steps > 1 {
                ui.horizontal_wrapped(|ui| {
                    for (k, title) in titles.iter().enumerate() {
                        if ui
                            .selectable_label(k == stage.step, format!(" {} ", k + 1))
                            .on_hover_text(title)
                            .clicked()
                        {
                            stage.goto = Some(k);
                        }
                    }
                });
            }
        });
    rect.top()
}

/// The library: every story told so far, to open again.
fn library(ctx: &egui::Context, stage: &mut Stage, shelf: &mut Shelf) {
    if shelf.read.is_none_or(|t| t.elapsed().as_secs_f32() > 2.0) {
        shelf.stories = tools::library_list();
        shelf.read = Some(Instant::now());
    }
    let mut open = true;
    egui::Window::new("Library")
        .open(&mut open)
        .default_width(560.0)
        .default_pos(egui::pos2(80.0, 80.0))
        .show(ctx, |ui| {
            if shelf.stories.is_empty() {
                ui.label("No story yet. Ask the agent to tell one: it is kept here as it is told.");
            }
            egui::ScrollArea::vertical()
                .max_height(480.0)
                .show(ui, |ui| {
                    for (id, title, steps, changed) in &shelf.stories {
                        ui.horizontal(|ui| {
                            if ui.button("open").clicked() {
                                stage.open = Some(id.clone());
                                stage.library = false;
                            }
                            ui.label(egui::RichText::new(title).strong());
                            ui.label(
                                egui::RichText::new(format!("{steps} steps · {changed}"))
                                    .color(DIM),
                            );
                        });
                    }
                });
        });
    if !open {
        stage.library = false;
    }
}

/// A scale bar, as on every micrograph.
fn scale_bar(
    painter: &egui::Painter,
    kind: Kind,
    image_rect: egui::Rect,
    content: egui::Rect,
    width: f32,
) {
    let (micrometres, per_pixel, ink) = match kind {
        Kind::Cell => (10.0, specimen::MICROMETRES_PER_PIXEL, egui::Color32::WHITE),
        _ => (
            100.0,
            skin::MICROMETRES_PER_PIXEL,
            egui::Color32::from_gray(25),
        ),
    };
    let length = micrometres / per_pixel * image_rect.width() / width;
    let inside = image_rect.intersect(content);
    let corner = inside.right_bottom() + egui::vec2(-16.0 - length, -16.0);
    painter.line_segment(
        [corner, corner + egui::vec2(length, 0.0)],
        egui::Stroke::new(4.0, ink),
    );
    painter.text(
        corner + egui::vec2(length / 2.0, -5.0),
        egui::Align2::CENTER_BOTTOM,
        format!("{micrometres:.0} µm"),
        egui::FontId::proportional(12.0),
        ink,
    );
}

/// The point of an image under a point of the screen, in the image's units.
fn image_point(rect: egui::Rect, pointer: egui::Pos2, size: (f32, f32)) -> Vec2 {
    Vec2::new(
        (pointer.x - rect.left()) / rect.width() * size.0,
        (pointer.y - rect.top()) / rect.height() * size.1,
    )
}

/// A world's film: the words said over it, large, near the bottom; the
/// world's clock; and the buttons to play it again, pause it and move
/// through it.
fn film_controls(ui: &mut egui::Ui, scene: &mut crate::world::WorldScene, content: egui::Rect) {
    let painter = ui.painter().with_clip_rect(content);
    if let Some((words, shown)) = &scene.say
        && *shown > 0.01
    {
        let alpha = (shown * 255.0) as u8;
        let galley = painter.layout(
            words.clone(),
            egui::FontId::proportional(24.0),
            egui::Color32::from_rgba_unmultiplied(246, 244, 238, alpha),
            (content.width() - 120.0).min(1100.0),
        );
        let at = egui::pos2(
            content.center().x - galley.size().x * 0.5,
            content.bottom() - 70.0 - galley.size().y,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(18.0, 10.0)),
            8.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, (shown * 150.0) as u8),
        );
        painter.galley(at, galley, egui::Color32::WHITE);
    }
    if let Some(label) = scene.clock_label() {
        painter.text(
            content.right_top() + egui::vec2(-16.0, 14.0),
            egui::Align2::RIGHT_TOP,
            label,
            egui::FontId::proportional(18.0),
            egui::Color32::from_rgb(255, 214, 140),
        );
    }
    if scene.film.shots.is_empty() {
        return;
    }
    let bar = egui::Rect::from_min_size(
        content.left_bottom() + egui::vec2(10.0, -36.0),
        egui::vec2(content.width().min(460.0), 28.0),
    );
    ui.painter().rect_filled(
        bar.expand(3.0),
        5.0,
        egui::Color32::from_rgba_unmultiplied(10, 12, 16, 170),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
        ui.horizontal(|ui| {
            if ui.button("replay").clicked() {
                scene.replay();
            }
            let playing = scene.playing && !scene.manual;
            if ui.button(if playing { "pause" } else { "play" }).clicked() {
                if playing {
                    scene.playing = false;
                } else {
                    if scene.clock >= scene.film.length() {
                        scene.replay();
                    }
                    scene.playing = true;
                    scene.give_back();
                }
            }
            let length = scene.film.length();
            let mut at = scene.clock;
            if ui
                .add(egui::Slider::new(&mut at, 0.0..=length).show_value(false))
                .changed()
            {
                scene.clock = at;
                scene.give_back();
            }
        });
    });
}
