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
use crate::world;

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
/// handing tool calls to the window through `window`, then calling `wake`
/// for the window may be asleep.
pub fn serve(window: Sender<Message>, wake: impl Fn()) {
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
                wake();
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
    wake();
}

fn send(message: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
}

/// What the agent is told when it connects.
const INSTRUCTIONS: &str = "\
cellstage is a window beside this conversation where you tell the user biology as a story in pictures, step by step, and the user moves through the steps with the arrow keys, turning, zooming and clicking what is shown. Anything in the body can be shown, at every scale: the body and its organs, tissues, cells, molecules, the embryo, microbes; pathways and lineages; numbers; real pictures.

What you can put in a panel:
- show_world: a world built in three dimensions ahead of time, cell by cell, that you film rather than draw. You direct: shots, each a place the camera flies to across scales, the words said over it as subtitles, how far the world's clock runs meanwhile, what stands out; the window renders it as a film, with life moving all the while. This is far richer than anything drawn: whenever the subject is in a world, film it there, in one large panel, and draw only what no world shows. The worlds: skin, from a forearm down to the DNA of one melanocyte, whose clock tells a common mole or a melanoma; growth_signals, a cell reading a growth factor from its surface to its genes; cell_death, a cell dying by apoptosis or escaping it; telomeres, chromosomes and the ends that wear down with each division. Together they show Hanahan and Weinberg's hallmarks of cancer.
- draw: a drawing you write in SVG, with the house style. drawing_kit (once per conversation) gives you 120 ready-made shapes (cells of every kind, organelles, molecules, microbes, eggs and embryos), 31 templates of whole things with their parts named (the body, the brain in three views, the heart, lungs, kidney and nephron, eye, bone, skin, spinal cord, the inside of a cell, a synapse, a membrane, DNA packaging, a sarcomere, liver, gut wall, alveolus, lymph node, marrow, islet, thyroid, a tumour's surroundings, and the embryo from the egg to four weeks), and shorthands that scatter shapes over a region, set them in a ring, a row, a grid or along a path, or draw a lipid bilayer or a double helix along any path. Start from a template and add to it, rather than drawing an organ from nothing.
- show_diagram: a pathway, a lineage, a cascade, a cycle, a sequence of events, as nodes and edges; the window lays it out. Nodes can be house shapes, for a lineage of cells.
- show_chart: numbers as lines, areas, bars or points, always with where they come from.
- show_text: a page of words, headings, lists, a definition set apart, for a course's summary or vocabulary.
- show_image: a real picture from Wikimedia Commons, a micrograph, a scan, an old anatomical plate; image_search finds them with their licence, and the panel credits the author.
- show_protein: a protein at its measured structure (Protein Data Bank), or the AlphaFold prediction for any human protein by its UniProt accession; protein_search finds both from a name. pathway_lookup reads Reactome, to draw a pathway from what is known.
- show_cell: one cell whose protein network cellweave runs live; show_skin: stained sections of skin, normal or with a melanocytic lesion.

How to work:
1. First call library_search with the subject. Every story ever told here is kept on disk; if one fits, story_open shows it at once, and you tell it from there, adding or changing steps as needed. Kept drawings can be reused by name.
2. Otherwise story_new, then build the story one step at a time. Write everything the window shows (titles, narration, subtitles, notes) in English unless the user asks for another language. Each step has a title and a narration of one to three short sentences shown under the pictures; then lay the step out (stage_layout, one to six panels; one large panel is often best) and fill its panels. story_step adds the next step; keep_panels carries the panels over so you change only what moves on.
3. Everything you show is kept in the step on screen as you go. Every answer ends with where you are, [story, step k of n]. If the user moved to another step, your calls change that step.
4. To show something changing (the neural tube closing, cells migrating, a heart beating, a tumour growing), give draw changes: each further frame as what changes from the one before, {id: {attribute: value}}; the window moves smoothly between them. A drawing, diagram or chart that comes back in the next step with numbers changed also moves into it. Name a drawing worth reusing, so it is kept in the library.
5. annotate with on=<part> to point at a named part of a drawing, a template, a diagram's node, a section, a cell or a world (the answers list them), or chain and residue on a protein: the window places the note itself. Avoid coordinates and avoid checking with screenshot after every call; one screenshot when a new layout is done is enough.
6. Every panel is labelled by what it is: a measured structure, a predicted structure, a simulation, an illustration or schematic, a chart of numbers you gave, a picture with its author and licence. Never present an illustration as a measurement or a simulation result, and never make up numbers for a chart without saying so in its source.
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
            "description": "Draw in a panel with SVG you write: anything the other tools do not show, at any scale, from the body to a molecule. The window adds the house style (call drawing_kit for its classes, shapes, templates and shorthands) and a light page. <insert name=\"heart\"/> sets a whole template with named parts; <scatter>, <ring>, <grid>, <along>, <bilayer>, <helix> write many elements at once. Give changes or frames to show a change: the window moves smoothly from each frame to the next. Give ids to parts you will point at. With only a name, shows a drawing kept in the library.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "title": {"type": "string"},
                    "caption": {"type": "string"},
                    "svg": {"type": "string", "description": "SVG elements, in a 1600 by 1000 space unless view_box says otherwise; or a whole <svg> document."},
                    "frames": {"type": "array", "items": {"type": "string"}, "description": "Further frames after svg (or all of them without svg): the same drawing with numbers changed."},
                    "changes": {"type": "array", "items": {"type": "object", "additionalProperties": {"type": "object", "additionalProperties": {"type": ["string", "number"]}}}, "description": "Further frames, each given by what changes from the frame before: {element id: {attribute: new value}}, such as {\"crest_1\": {\"transform\": \"translate(420 380) scale(0.6)\"}, \"label\": {\"opacity\": 1}}. Far shorter than whole frames. A transform or an opacity may be new; other attributes should be in the first frame already."},
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
            "description": "The house style for draw: its classes, its 120 ready-made shapes (cells of every kind, organelles, molecules, microbes, eggs and embryos), its templates of whole things with named parts (organs, the brain, sections, the embryo's stages, the inside of a cell), and its shorthands. Call once per conversation. With picture, a page of the kit's picture; with template, the parts of one template, and with picture, where they are.",
            "inputSchema": {"type": "object", "properties": {
                "picture": {"type": "boolean", "default": false},
                "page": {"type": "integer", "minimum": 1, "description": "Which page of the kit's picture."},
                "template": {"type": "string", "description": "A template's name, for its parts."}
            }}
        },
        {
            "name": "show_diagram",
            "description": "A diagram the window lays out from nodes and edges: a signalling pathway, a cell lineage, a cascade, a feedback loop, a cycle, a sequence of events. Nodes are set in layers along the flow and ordered to cross few edges. Each node and edge is a part to point at (edges are from_to_to). Shown again in a later step with highlight or dim changed, it moves into the new state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "title": {"type": "string"},
                    "caption": {"type": "string"},
                    "direction": {"type": "string", "enum": ["right", "down"], "default": "right"},
                    "nodes": {"type": "array", "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "label": {"type": "string"},
                            "kind": {"type": "string", "enum": ["protein", "receptor", "ligand", "molecule", "ion", "hormone", "gene", "rna", "complex", "cell", "tissue", "organ", "process", "event", "state", "outcome", "microbe"], "default": "protein"},
                            "shape": {"type": "string", "description": "A house shape drawn as the node, such as rbc or neutrophil, for lineages."},
                            "colour": {"type": "string", "description": "#rrggbb fill instead of the kind's."},
                            "what": {"type": "string", "description": "What it is, told to you when the user clicks it."},
                            "layer": {"type": "integer", "description": "Pin the node to a layer, 0 first."}
                        },
                        "required": ["id"]
                    }},
                    "edges": {"type": "array", "items": {
                        "type": "object",
                        "properties": {
                            "from": {"type": "string"},
                            "to": {"type": "string"},
                            "kind": {"type": "string", "enum": ["activates", "inhibits", "becomes", "binds", "makes", "moves", "line"], "default": "activates"},
                            "label": {"type": "string"}
                        },
                        "required": ["from", "to"]
                    }},
                    "groups": {"type": "array", "items": {
                        "type": "object",
                        "properties": {"id": {"type": "string"}, "label": {"type": "string"}, "nodes": {"type": "array", "items": {"type": "string"}}, "colour": {"type": "string"}},
                        "required": ["label", "nodes"]
                    }, "description": "Boxes drawn behind some nodes: a compartment, the nucleus, a membrane."},
                    "highlight": {"type": "array", "items": {"type": "string"}, "description": "Node ids ringed in yellow."},
                    "dim": {"type": "array", "items": {"type": "string"}, "description": "Node ids faded, with their edges."}
                },
                "required": ["panel", "nodes"]
            }
        },
        {
            "name": "show_chart",
            "description": "A chart of numbers you give: lines, areas, bars (grouped or stacked) or points. Say where the numbers come from in source; it is shown on the panel. Each series is a part (series_<name>), each point or bar series_<name>_<n>. The same chart with other values in a later step moves into it, bars growing (fix y_max to keep the axis).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "title": {"type": "string"},
                    "caption": {"type": "string"},
                    "kind": {"type": "string", "enum": ["line", "area", "bar", "scatter"], "default": "line"},
                    "source": {"type": "string", "description": "Where the numbers come from: a paper, a database, or 'made up to show the shape'."},
                    "x": {"type": "array", "items": {"type": "number"}, "description": "x values shared by the series' values."},
                    "categories": {"type": "array", "items": {"type": "string"}, "description": "Names along the x axis, for bars or labelled points."},
                    "series": {"type": "array", "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "values": {"type": "array", "items": {"type": "number"}},
                            "points": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}},
                            "colour": {"type": "string"}
                        }
                    }},
                    "x_label": {"type": "string"},
                    "y_label": {"type": "string"},
                    "y_min": {"type": "number"},
                    "y_max": {"type": "number"},
                    "log_y": {"type": "boolean", "default": false},
                    "stacked": {"type": "boolean", "default": false}
                },
                "required": ["panel", "series", "source"]
            }
        },
        {
            "name": "show_text",
            "description": "A page of words in a panel, laid out and wrapped by the window: # heading, ## subheading, - item, 1. item, > a definition set apart, **bold**, blank lines between paragraphs. For a summary, the words to remember, the steps of a process. Each block is a part (block_1, block_2...).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "title": {"type": "string"},
                    "text": {"type": "string"},
                    "size": {"type": "number", "default": 30, "description": "Font size of the body text."}
                },
                "required": ["panel", "text"]
            }
        },
        {
            "name": "image_search",
            "description": "Search Wikimedia Commons for pictures: micrographs, histology, scans, photographs, old anatomical plates (Gray's Anatomy is in the public domain), diagrams. Each result comes with its size, licence and author. Search in English.",
            "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "limit": {"type": "integer", "default": 8}}, "required": ["query"]}
        },
        {
            "name": "show_image",
            "description": "Show a picture from Wikimedia Commons in a panel, fetched once and kept in the user's cache, with its author and licence on the panel. Point at things in it with annotate arrow_to=[x, y] as fractions of the picture, or draw over it with draw and <image href=\"commons:File:...\">.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "file": {"type": "string", "description": "The file's name on Commons, such as File:Melanoma.jpg, as image_search gives it."},
                    "title": {"type": "string"},
                    "caption": {"type": "string"}
                },
                "required": ["panel", "file"]
            }
        },
        {
            "name": "protein_search",
            "description": "Find proteins from a name, a gene or a subject: reviewed human entries of UniProt with what each does (show_protein uniprot= shows the AlphaFold prediction), and measured structures in the PDB with their method and resolution (show_protein pdb_id=).",
            "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]}
        },
        {
            "name": "pathway_lookup",
            "description": "Read Reactome, a curated database of human pathways: with a subject, the pathways and reactions that match; with an id (R-HSA-...), what it is, its inputs, outputs and catalysts, and the steps it holds. To draw a pathway from what is known rather than from memory.",
            "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "id": {"type": "string"}}}
        },
        {
            "name": "show_protein",
            "description": "Show a protein, or a complex, at the atomic structure deposited in the Protein Data Bank, or as the AlphaFold database predicts it from a UniProt accession (any human protein, measured or not; labelled as a prediction, and the answer says which stretches are predicted with low confidence). The user can turn and zoom it, and point at atoms. Give a second entry of the same protein to let the user slide between two states (for example RAS with GDP, 4Q21, and with a GTP analogue, 5P21). The answer lists chains, residue numbers and bound molecules. protein_search finds entries from a name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "pdb_id": {"type": "string", "description": "Four characters, such as 1TSR."},
                    "uniprot": {"type": "string", "description": "A UniProt accession, such as P04637, to show the AlphaFold prediction instead of a PDB entry."},
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
                "required": ["panel"]
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
            "name": "show_world",
            "description": "Film a world built in 3D ahead of time instead of drawing: you direct, the window renders and plays it, life moving all the while. Worlds, their places in brackets, their clocks and variants:\n- skin: a forearm's skin (arm, mole), through its surface into a core of skin cell by cell (skin, epidermis, basal_layer, underneath, dermis, capillary), into the first melanocyte (melanocyte), its nucleus, chromatin, nucleosomes and the helix at BRAF's codon 600 (nucleus, chromatin, nucleosomes, dna). Clock: years since BRAF mutates in that melanocyte. Variant mole: below 0 normal skin; at 0 the mutation, its T becoming an A on screen; 0 to 7 the clone gathers in nests (nests; a brown spot grows at the surface); 7 to 9.5 it stops dividing (senescence); 15 to 50 the nests sink into the dermis, the mole raised and paler. Variant melanoma: the same until 9; about 10 one cell loses p16 and its clone spreads along the junction and up into the epidermis (spread); 13 telomerase on; 14 to 16.5 the basement membrane is digested (breach); 15 to 21 a nodule fills the dermis (nodule), its middle short of oxygen; 16 to 19 capillaries sprout towards it and meet (sprout); from 19.5 cells leave in the blood (leaving); mutations pile up in the helix (dna, highlight mutations); at the surface it grows uneven in shape and colour.\n- growth_signals: one cell reading the growth factor EGF, from its surface to its genes (cell, membrane, receptor, ras, cascade, pore, genes, nucleus). Clock: minutes after EGF arrives: 0 to 2 EGF binds, receptors pair and phosphorylate their tails; 2 to 5 GRB2 and SOS come, RAS switches on; 5 to 10 RAF, MEK, ERK; 10 to 30 ERK into the nucleus; 30 to 120 cyclin D made; 150 to 320 cyclin D with CDK4 phosphorylates RB, which lets go of E2F; 320 to 600 E2F turns on the genes for copying DNA. Variants: normal; ras_stuck (everything on with no EGF: growth signals of its own); stop_signal (p16 holds CDK4 and RB stays on); p16_lost (the stop signal goes unheard).\n- cell_death: a cell with broken DNA among its neighbours (tissue, cell, nucleus, mitochondria, pore, apoptosome, bodies, macrophage). Clock: hours after the breaks: 0.3 to 2.5 p53 builds up; 2.5 to 3.5 BAX opens pores in the mitochondria; 3 to 5 cytochrome c leaks out; 4 to 5 apoptosomes; 5 to 6.2 caspases; 6 to 8 the cell shrinks and blebs; 8 to 8.8 it falls into apoptotic bodies; 8.5 to 13 a macrophage comes and eats them; 12 to 15 the neighbours close the gap. Variants: normal; bcl2_high and p53_lost: the cell lives on and divides at 10 to 12 with its broken DNA.\n- telomeres: a metaphase spread of 46 chromosomes (spread, chromosome, end), a telomere up close with its repeats, shelterin and T-loop (telomere, t_loop), chromosomes fused in crisis (fusion). Clock: divisions; every division shortens the telomeres; past about 55 some can no longer loop and their bare ends set off the alarm, and the cell stops. Variants: normal; telomerase (TERT writes the repeats back: endless division); crisis (division goes on past the limit; from 58 to 64 chromosomes fuse end to end).\nGive a film as shots: each goes to a place, or stays where the last left the camera; lasts some seconds; says one or two short sentences over it, in English unless the user asks for another language; may run the clock to a value by its end; may set apart what to look at, dimming the rest. The camera flies between places along a smooth path across scales. A film of 6 to 15 shots is usually a whole step; the step's narration can stay short. Calling show_world again on the same panel with the same world and variant keeps the world and plays the new film. The user can pause (space), replay, scrub, take the camera with the mouse and click on things, which get_user_events reports. The answer lists the places, and the things annotate can point at.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel": {"type": "string"},
                    "world": {"type": "string", "enum": world::names(), "default": "skin"},
                    "variant": {"type": "string", "enum": world::all_variants(), "description": "Which story the world's clock tells; the first of the world's by default."},
                    "place": {"type": "string", "enum": world::all_places(), "description": "Where the camera starts; by default where it was."},
                    "clock": {"type": "number", "description": "The world's clock at the start; by default where the world's story begins."},
                    "highlight": {"type": "array", "items": {"type": "string", "enum": world::all_highlights()}, "description": "What to set apart when no film says otherwise."},
                    "shots": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "place": {"type": "string", "enum": world::all_places()},
                                "seconds": {"type": "number", "description": "How long the shot lasts, the camera's travel included; by default the travel and 4 s. Leave time to read what is said."},
                                "say": {"type": "string", "description": "A subtitle over the shot: one or two short sentences, in English unless the user asks for another language."},
                                "clock": {"type": "number", "description": "The world's clock at the end of the shot; it runs there while the camera stays."},
                                "highlight": {"type": "array", "items": {"type": "string", "enum": world::all_highlights()}, "description": "What to set apart; the rest dims. braf_codon_600 sets the mutated base pair apart in the helix."},
                                "distance": {"type": "number", "description": "Micrometres from what the place looks at, to come closer or stand back."},
                                "turn": {"type": "number", "description": "Degrees to turn about what the place looks at."},
                                "tilt": {"type": "number", "description": "Degrees above the horizontal to look from; negative looks from below."},
                                "travel": {"type": "number", "description": "Seconds to get there; chosen by how far it is when not given."},
                                "drift": {"type": "number", "description": "Degrees a second the camera turns while it stays; 3 by default, 0 to stay still."}
                            }
                        }
                    },
                    "title": {"type": "string"},
                    "caption": {"type": "string"}
                },
                "required": ["panel"]
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
