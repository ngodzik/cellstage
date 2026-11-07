# cellstage

A window an agent tells biology on, step by step.

cellstage is a [Model Context Protocol](https://modelcontextprotocol.io) server and a window. An agent connected to it, Claude Code in a terminal or any other client, tells a story in steps, and the user moves through them with the arrow keys, as through a course. Each step has a few words, shown under the pictures, and one to six panels showing:

- **drawings the agent writes in SVG**, for anything in the body at any scale: an embryo, an organ, the brain, blood, a pathway, a timeline. The window adds a house style, so that every drawing speaks the same colours (the germ layers, tissues, organs, cells, molecules) and can use ready-made shapes (cells of every kind, blood cells, a neuron, DNA, a receptor, a vessel, a virus). A drawing can move: the agent gives its frames, or what changes from one frame to the next, and the window moves smoothly between them;
- **proteins** at the atomic structure their authors measured, any entry of the Protein Data Bank, as atoms or as their fold, with residues highlighted, notes pinned to residues as the protein turns, and a slider between two states of the same protein;
- **one cell under a fluorescence microscope**, its protein network run live by [cellweave](../cellweave): oxygen, growth factor and stress set by the agent, genes removed or stuck on, each stained protein glowing where it lives, the cell cycling, arresting, taking itself apart or bursting by cellweave's thresholds;
- **stained sections of skin**, normal or with a melanocytic lesion at one of ten stages, from a junctional nevus to a nodular melanoma, drawn in haematoxylin and eosin from the criteria histology uses to tell them apart.

The agent points at things by name, a part of its drawing, a place in a section or a cell, a residue, and the window places the note beside it and follows it as it moves. The user turns and zooms what is shown and clicks on things; the agent hears what was clicked, and which step the user went to. The teaching is the agent's: cellstage only shows.

## The library

Every story is kept on disk as it is told, every step with what fills it, under `~/.local/share/cellstage` (or `$CELLSTAGE_HOME`): one JSON file per story, and one per drawing the agent chose to keep by name. An agent searches the library before building anything, and a story told once opens again at once, in another conversation, another day. The user can open any story from the window's library, without the agent.

## What a panel is

Every panel carries a label, so that a drawing is never taken for a measurement or a result:

- **measured structure**: atoms where X-ray crystallography or cryo-electron microscopy placed them; the colours are chosen.
- **simulation**: protein levels and fate computed by cellweave's network; how the cell looks, its outline and organelles and the microscope's optics, is drawn.
- **illustration**: a section drawn from what histology describes, or a schematic the agent drew; neither is simulated.

## Running it

It builds against cellweave's crates, so cellweave must sit next to it, at a revision that has the fuller cell of its chapter 28:

```
g/
├── cellweave/
└── cellstage/
```

```sh
cargo build --release
claude mcp add -s user cellstage -- "$(pwd)/target/release/cellstage"
```

Then, in a new Claude Code session, ask for something to be shown or explained. The window opens at the first drawing; closing it hides it until the next.

## The tools

| tool | what it does |
|---|---|
| `library_search` | finds kept stories and drawings by subject |
| `story_open` | opens a kept story at one of its steps |
| `story_new`, `story_step` | begins a story; adds a step after the one on screen, empty or with the same panels |
| `story_edit`, `story_goto`, `story_remove_step` | changes a step's words, shows a step, removes one |
| `stage_layout` | splits the step into one to six panels, side by side, stacked, or in a grid |
| `draw` | an SVG drawing in a panel, still or moving; kept in the library when named |
| `drawing_kit` | the house classes and shapes, in words and as a picture |
| `show_protein` | a PDB entry in a panel; its answer lists chains, residue numbers and bound molecules |
| `show_cell` | the cell under the microscope, its setting, its stains; its answer gives every level and the fate |
| `show_skin` | an H&E section at a named stage; its answer gives the Breslow thickness of an invasive melanoma |
| `annotate`, `clear_annotations` | notes on a panel, pointing at a named part, a residue, or a spot |
| `focus_protein` | turns a protein's camera to a residue |
| `get_user_events` | what the user clicked, and the steps they moved to, since the last call |
| `screenshot` | a picture of the window, for the agent to see what it drew |

## Data

No structure is stored in the project. `show_protein` fetches an entry from the RCSB once and keeps it in `~/.cache/cellstage/pdb`. The wwPDB places the archive's files under CC0.

The house shapes and colours are drawn for this project. Drawings are rendered with [resvg](https://github.com/linebender/resvg) (Apache-2.0 or MIT).

The H&E colours follow the Beer-Lambert law with the haematoxylin and eosin stain vectors of Ruifrok and Johnston (Anal Quant Cytol Histol 23, 291-299, 2001).
