//! The Model Context Protocol, spoken over standard input and output: one
//! JSON-RPC message per line. An agent lists the stage's tools and calls
//! them; each call is handed to the window, which does it and answers.
//!
//! Standard output carries nothing else: the window's logs go to standard
//! error.

use std::io::{BufRead, Write};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use serde_json::{Value, json};

use crate::cell::dish::GENES;
use crate::tissue::skin::Lesion;

/// A tool call, handed to the window, and where to send its answer.
pub struct Call {
    pub tool: String,
    pub args: Value,
    pub reply: Sender<Reply>,
}

/// What a tool call answers: words for the agent, a picture if it asked
/// for one, and whether it failed.
#[derive(Debug, Default)]
pub struct Reply {
    pub text: String,
    pub image: Option<Vec<u8>>,
    pub error: bool,
}

impl Reply {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: true,
            ..Self::default()
        }
    }
}

/// What the window is sent.
pub enum Message {
    Call(Call),
    /// The agent has gone: standard input closed.
    Quit,
}

/// Versions of the protocol this server speaks, newest first.
const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// Read messages from standard input until it closes, answering each, and
/// handing tool calls to the window through `window`.
pub fn serve(window: Sender<Message>) {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            send(
                &json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "parse error"}}),
            );
            continue;
        };
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        // A notification has no id and wants no answer.
        let Some(id) = id else { continue };
        let result = match method {
            "initialize" => {
                let asked = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or(VERSIONS[0]);
                let version = VERSIONS
                    .iter()
                    .find(|v| **v == asked)
                    .copied()
                    .unwrap_or(VERSIONS[0]);
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "cellstage", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": INSTRUCTIONS,
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let tool = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let (reply, answer) = channel();
                if window
                    .send(Message::Call(Call { tool, args, reply }))
                    .is_err()
                {
                    break;
                }
                let reply = answer
                    .recv_timeout(Duration::from_secs(180))
                    .unwrap_or_else(|_| Reply::error("the window did not answer in time"));
                let mut content = vec![json!({"type": "text", "text": reply.text})];
                if let Some(png) = reply.image {
                    use base64::Engine;
                    let data = base64::engine::general_purpose::STANDARD.encode(png);
                    content.push(json!({"type": "image", "data": data, "mimeType": "image/png"}));
                }
                Ok(json!({"content": content, "isError": reply.error}))
            }
            other => Err(json!({"code": -32601, "message": format!("no method {other}")})),
        };
        match result {
            Ok(result) => send(&json!({"jsonrpc": "2.0", "id": id, "result": result})),
            Err(error) => send(&json!({"jsonrpc": "2.0", "id": id, "error": error})),
        }
    }
    let _ = window.send(Message::Quit);
}

fn send(message: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
}

/// What the agent is told when it connects.
const INSTRUCTIONS: &str = "\
cellstage is a window beside this conversation where you tell the user biology as a story in pictures, step by step, and the user moves through the steps with the arrow keys, turning, zooming and clicking what is shown. Anything in the body can be shown: proteins at their measured structure, a cell whose network cellweave runs live, stained sections of skin, and for everything else drawings you write yourself in SVG (an embryo, an organ, the brain, blood, a pathway, a timeline), which can move from one frame to the next.

How to work:
1. First call library_search with the subject. Every story ever told here is kept on disk; if one fits, story_open shows it at once, and you tell it from there, adding or changing steps as needed. Kept drawings can be reused by name.
2. Otherwise story_new, then build the story one step at a time. Each step has a title and a narration of one to three short sentences shown under the pictures; then lay the step out (stage_layout, one to six panels; one large panel is often best) and fill its panels with show_protein, show_cell, show_skin or draw. story_step adds the next step; keep_panels carries the panels over so you change only what moves on.
3. Everything you show is kept in the step on screen as you go. Every answer ends with where you are, [story, step k of n]. If the user moved to another step, your calls change that step.
4. draw: call drawing_kit once per conversation for the house classes and ready-made shapes, which give every drawing the same look. Give ids to the parts you will point at. To show something changing (the neural tube closing, cells migrating, a tumour growing), give changes: each further frame as what changes from the one before, {id: {attribute: value}}; the window moves smoothly between them. It is much quicker to write than whole frames. Name a drawing worth reusing, so it is kept in the library.
5. annotate with on=<part> to point at a named part of a drawing, a section or a cell (the show and draw answers list them), or chain and residue on a protein: the window places the note itself. Avoid coordinates and avoid checking with screenshot after every call; one screenshot when a new layout is done is enough.
6. Every panel is labelled by what it is: a measured structure (Protein Data Bank), a simulation (cellweave's model) or an illustration (a section drawn from histology, or your schematic). Never present an illustration as a measurement or a simulation result.
7. get_user_events tells you what the user clicked and which step they moved to. Call it when they refer to something in the window.
The story carries the pictures and the short words; the teaching, the longer explanations and any revision are yours, in the conversation.";

/// The tools, with what each does and what it takes.
fn tools() -> Value {
    let stages: Vec<&str> = Lesion::ALL.iter().map(|(n, _)| *n).collect();
    let genes: Vec<&str> = GENES.to_vec();
    json!([
        {
            "name": "library_search",
            "description": "Search the stories and drawings kept from every past conversation, by subject (words in any language). Call it before building anything. With no query, lists the latest stories.",
            "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}}
        },
        {
            "name": "story_open",
            "description": "Open a kept story and show one of its steps (the first by default). The answer gives its outline.",
            "inputSchema": {
                "type": "object",
                "properties": {"id": {"type": "string"}, "step": {"type": "integer", "minimum": 1}},
                "required": ["id"]
            }
        },
        {
            "name": "story_new",
            "description": "Begin a new story, kept in the library as it is told. It starts at its first step, empty: lay it out and fill it, then story_step for the next.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "summary": {"type": "string", "description": "One sentence: what it explains, for finding it again."},
                    "tags": {"type": "array", "items": {"type": "string"}, "description": "Subjects, such as organs, cells, diseases, proteins."},
                    "step_title": {"type": "string"},
                    "narration": {"type": "string", "description": "What the first step says, one to three short sentences."}
                },
                "required": ["title"]
            }
        },
        {
            "name": "story_step",
            "description": "Add a step after the one on screen and show it. It starts empty, or with the panels of the step before when keep_panels is true (their notes too with keep_notes).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "narration": {"type": "string", "description": "One to three short sentences, shown under the pictures."},
                    "keep_panels": {"type": "boolean", "default": false},
                    "keep_notes": {"type": "boolean", "default": false}
                },
                "required": ["title"]
            }
        },
        {
            "name": "story_edit",
            "description": "Change the words of a step (the one on screen unless step is given), or the story's title, summary and tags.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "step": {"type": "integer", "minimum": 1},
                    "title": {"type": "string"},
                    "narration": {"type": "string"},
                    "story_title": {"type": "string"},
                    "summary": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}}
                }
            }
        },
        {
            "name": "story_goto",
            "description": "Show a step of the story, counted from 1.",
            "inputSchema": {"type": "object", "properties": {"step": {"type": "integer", "minimum": 1}}, "required": ["step"]}
        },
        {
            "name": "story_remove_step",
            "description": "Remove a step (the one on screen unless step is given).",
            "inputSchema": {"type": "object", "properties": {"step": {"type": "integer", "minimum": 1}}}
        },
        {
            "name": "stage_layout",
            "description": "Lay the step on screen out in panels, replacing those there were. Each panel has an id the show tools name, and a title. One to six panels, side by side, stacked, or in a grid.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panels": {"type": "array", "minItems": 1, "maxItems": 6, "items": {
                        "type": "object",
                        "properties": {"id": {"type": "string"}, "title": {"type": "string"}},
                        "required": ["id"]
                    }},
                    "arrangement": {"type": "string", "enum": ["columns", "rows", "grid"], "default": "columns"}
                },
                "required": ["panels"]
            }
        },
        {
            "name": "draw",
            "description": "Draw in a panel with SVG you write: anything the other tools do not show, at any scale, from the body to a molecule. The window adds the house style (call drawing_kit for its classes and shapes) and a light page. Give frames to show a change: the window moves smoothly from each frame to the next. Give ids to parts you will point at. With only a name, shows a drawing kept in the library.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "title": {"type": "string"},
                    "caption": {"type": "string"},
                    "svg": {"type": "string", "description": "SVG elements, in a 1600 by 1000 space unless view_box says otherwise; or a whole <svg> document."},
                    "frames": {"type": "array", "items": {"type": "string"}, "description": "Further frames after svg (or all of them without svg): the same drawing with numbers changed."},
                    "changes": {"type": "array", "items": {"type": "object", "additionalProperties": {"type": "object", "additionalProperties": {"type": ["string", "number"]}}}, "description": "Further frames, each given by what changes from the frame before: {element id: {attribute: new value}}, such as {\"crest_1\": {\"transform\": \"translate(420 380) scale(0.6)\"}, \"label\": {\"opacity\": 1}}. Far shorter than whole frames; change attributes the first frame already has."},
                    "view_box": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "Width and height of the drawing space."},
                    "seconds": {"type": "number", "default": 2, "description": "How long each move from one frame to the next takes."},
                    "hold": {"type": "number", "default": 0.8, "description": "How long each frame is held before moving on."},
                    "loop": {"type": "boolean", "default": false},
                    "background": {"type": "string", "description": "light (default), dark, or #rrggbb."},
                    "parts": {"type": "object", "additionalProperties": {"type": "string"}, "description": "What some ids are, told to you when the user clicks them."},
                    "name": {"type": "string", "description": "Keep the drawing in the library under this name; alone, show the kept drawing."}
                },
                "required": ["panel"]
            }
        },
        {
            "name": "drawing_kit",
            "description": "The house style for draw: its classes (cells, organelles, germ layers, tissues, organs, molecules, arrows, effects) and ready-made shapes (cells of every kind, blood cells, neuron, DNA, receptor, vessel, virus...). Call once per conversation. With picture, also a picture of them all.",
            "inputSchema": {"type": "object", "properties": {"picture": {"type": "boolean", "default": false}}}
        },
        {
            "name": "show_protein",
            "description": "Show a protein, or a complex, at the atomic structure deposited in the Protein Data Bank, in a panel. The user can turn and zoom it, and point at atoms. Give a second entry of the same protein to let the user slide between two states (for example RAS with GDP, 4Q21, and with a GTP analogue, 5P21). The answer lists chains, residue numbers and bound molecules.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "pdb_id": {"type": "string", "description": "Four characters, such as 1TSR."},
                    "second_pdb_id": {"type": "string", "description": "The same protein in another state, laid over the first."},
                    "title": {"type": "string"},
                    "caption": {"type": "string"},
                    "style": {"type": "string", "enum": ["atoms", "fold"], "default": "atoms"},
                    "chain_colours": {"type": "object", "additionalProperties": {"type": "string"}, "description": "Chain letter to colour, as #rrggbb."},
                    "hide_chains": {"type": "array", "items": {"type": "string"}},
                    "highlights": {"type": "array", "items": {
                        "type": "object",
                        "properties": {
                            "chains": {"type": "string", "description": "Chain letters; all chains when left out."},
                            "from": {"type": "integer"},
                            "to": {"type": "integer"},
                            "colour": {"type": "string", "description": "#rrggbb; red when left out."},
                            "glow": {"type": "boolean", "default": true},
                            "label": {"type": "string"}
                        },
                        "required": ["from"]
                    }}
                },
                "required": ["panel", "pdb_id"]
            }
        },
        {
            "name": "show_cell",
            "description": "Show one cell under a fluorescence microscope, its protein network running live (cellweave's fuller cell: 18 proteins). Each stained protein glows where it lives in the cell, as bright as the network has it; the cell cycles, arrests, starts apoptosis or dies of necrosis by cellweave's thresholds. Calling it again on the same panel changes the setting without starting a new cell, unless new_cell is true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "title": {"type": "string"},
                    "caption": {"type": "string"},
                    "oxygen": {"type": "number", "minimum": 0, "maximum": 1, "description": "Share of the supply; arrest below 0.32, necrosis below about 0.06."},
                    "egf": {"type": "number", "minimum": 0, "maximum": 1, "description": "Growth factor; cells of a simulation read 0.2."},
                    "stress": {"type": "number", "minimum": 0, "maximum": 1, "description": "The lethal stress of cellweave's chapter 8."},
                    "removed": {"type": "array", "items": {"type": "string", "enum": genes}},
                    "stuck_on": {"type": "array", "items": {"type": "string", "enum": genes}},
                    "stains": {"type": "array", "maxItems": 3, "items": {"type": "string", "enum": genes}, "description": "Proteins shown in green, red and magenta; the DNA is blue."},
                    "steps": {"type": "integer", "minimum": 0, "description": "Steps to run at once before showing."},
                    "running": {"type": "boolean", "description": "Whether the cell keeps living on screen, two steps a second."},
                    "new_cell": {"type": "boolean"}
                },
                "required": ["panel"]
            }
        },
        {
            "name": "show_skin",
            "description": "Draw a section of skin stained with haematoxylin and eosin, as a pathologist sees it, normal or with a melanocytic lesion at a named stage. An illustration drawn from the histological criteria of each stage, not a simulation. The same seed keeps the same skin across stages. The answer gives the Breslow thickness of an invasive melanoma and the places notes can point at.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "stage": {"type": "string", "enum": stages},
                    "seed": {"type": "integer", "default": 1},
                    "title": {"type": "string"},
                    "caption": {"type": "string"}
                },
                "required": ["panel", "stage"]
            }
        },
        {
            "name": "annotate",
            "description": "Write a note on a panel. Point it with on=<part> (a part of a drawing, a place in a section or a cell, as their answers list) or chain and residue on a protein: the window places the words beside it and follows it as it moves or turns. x and y (fractions of the panel from its top left) place the words yourself; arrow_to points them at a spot.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "text": {"type": "string"},
                    "on": {"type": "string"},
                    "chain": {"type": "string"}, "residue": {"type": "integer"},
                    "x": {"type": "number"}, "y": {"type": "number"},
                    "arrow_to": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}
                },
                "required": ["panel", "text"]
            }
        },
        {
            "name": "clear_annotations",
            "description": "Remove every note from a panel, or from all of them.",
            "inputSchema": {"type": "object", "properties": {"panel": {"type": "string"}}}
        },
        {
            "name": "focus_protein",
            "description": "Turn a protein panel's camera to a residue and come closer to it.",
            "inputSchema": {
                "type": "object",
                "properties": {"panel": {"type": "string"}, "chain": {"type": "string"}, "residue": {"type": "integer"}},
                "required": ["panel", "residue"]
            }
        },
        {
            "name": "get_user_events",
            "description": "What the user did in the window since the last call: what they clicked (an amino acid, a part of a drawing, a place in a section or a cell) and the steps they moved to.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "screenshot",
            "description": "A picture of the window as it is now.",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ])
}
