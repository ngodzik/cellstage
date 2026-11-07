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
cellstage is a window beside this conversation, where you show the user biology while you explain it: proteins at their measured atomic structure, a cell under a fluorescence microscope driven live by cellweave's protein network, and stained tissue sections. The user sees it, can turn and zoom what is in it, and can click on things to ask about them.

How to use it:
- Split the window into panels with stage_layout, or let a show_* call create a panel by naming it. Up to four panels.
- Every panel is labelled by what it is: a measured structure (Protein Data Bank), a simulation (computed by cellweave's model) or an illustration (drawn from what histology describes). Never present an illustration as a measurement or a simulation result; say which it is when you talk about it.
- show_protein fetches any PDB entry; its answer lists the chains, their residue numbers and what is bound, so you can highlight the right residues. Check the numbering before you highlight.
- show_cell runs the fuller cell network of cellweave (18 proteins) in one cell: set oxygen, EGF and stress, remove genes or stick them on, choose three proteins to stain. Its answer gives every level and the cell's fate.
- show_skin draws an H&E section of skin, normal or with a melanocytic lesion at a named stage. The same seed keeps the same skin across stages, so a progression can be shown one stage after another.
- annotate writes on a panel, with an optional arrow; on a protein, a note can follow a residue as the protein turns.
- get_user_events returns what the user clicked since the last call, with what was under the pointer. Call it when the user refers to something they clicked.
- screenshot returns a picture of the window, to check what you drew.
The teaching, the explanations and any revision are yours to lead; the stage only draws.";

/// The tools, with what each does and what it takes.
fn tools() -> Value {
    let stages: Vec<&str> = Lesion::ALL.iter().map(|(n, _)| *n).collect();
    let genes: Vec<&str> = GENES.to_vec();
    json!([
        {
            "name": "stage_layout",
            "description": "Split the window into panels, replacing those there were. Each panel has an id the show_* tools name, and a title shown above it. One to four panels, side by side, stacked, or in a grid.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panels": {"type": "array", "minItems": 1, "maxItems": 4, "items": {
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
            "description": "Draw a section of skin stained with haematoxylin and eosin, as a pathologist sees it, normal or with a melanocytic lesion at a named stage. An illustration drawn from the histological criteria of each stage, not a simulation. The same seed keeps the same skin across stages. The answer gives the Breslow thickness of an invasive melanoma.",
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
            "description": "Write a note on a panel, at a point given as fractions of its width and height from its top left, with an optional arrow to another point. On a protein panel, give a chain and a residue instead to pin the note to that residue as the protein turns.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "text": {"type": "string"},
                    "x": {"type": "number"}, "y": {"type": "number"},
                    "arrow_to": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2},
                    "chain": {"type": "string"}, "residue": {"type": "integer"}
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
            "description": "What the user clicked in the window since the last call: the panel, where, and what was there (an amino acid, a compartment of the cell, a region of a section).",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "screenshot",
            "description": "A picture of the window as it is now.",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ])
}
