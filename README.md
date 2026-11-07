# cellstage

A window an agent draws biology on while it explains it.

cellstage is a [Model Context Protocol](https://modelcontextprotocol.io) server and a window. An agent connected to it, Claude Code in a terminal or any other client, splits the window into panels and shows in them:

- **proteins** at the atomic structure their authors measured, any entry of the Protein Data Bank, as atoms or as their fold, with residues highlighted, notes pinned to residues as the protein turns, and a slider between two states of the same protein;
- **one cell under a fluorescence microscope**, its protein network run live by [cellweave](../cellweave): oxygen, growth factor and stress set by the agent, genes removed or stuck on, each stained protein glowing where it lives, the cell cycling, arresting, taking itself apart or bursting by cellweave's thresholds;
- **stained sections of skin**, normal or with a melanocytic lesion at one of ten stages, from a junctional nevus to a nodular melanoma, drawn in haematoxylin and eosin from the criteria histology uses to tell them apart.

The user turns and zooms what is shown and clicks on things; the agent hears what was clicked, with what was under the pointer. The teaching is the agent's: cellstage only draws.

## What a panel is

Every panel carries a label, so that a drawing is never taken for a measurement or a result:

- **measured structure**: atoms where X-ray crystallography or cryo-electron microscopy placed them; the colours are chosen.
- **simulation**: protein levels and fate computed by cellweave's network; how the cell looks, its outline and organelles and the microscope's optics, is drawn.
- **illustration**: drawn from what histology describes, not simulated.

## Running it

It builds against cellweave's crates, so cellweave must sit next to it, at a revision that has the fuller cell of its chapter 28:

```
g/
├── cellweave/
└── cellstage/
```

```sh
cargo build --release
claude mcp add cellstage -- "$(pwd)/target/release/cellstage"
```

Then, in a Claude Code session, ask for something to be shown. The window opens at the first drawing; closing it hides it until the next.

## The tools

| tool | what it does |
|---|---|
| `stage_layout` | splits the window into one to four panels, side by side, stacked, or in a grid |
| `show_protein` | a PDB entry in a panel; its answer lists chains, residue numbers and bound molecules |
| `show_cell` | the cell under the microscope, its setting, its stains; its answer gives every level and the fate |
| `show_skin` | an H&E section at a named stage; its answer gives the Breslow thickness of an invasive melanoma |
| `annotate`, `clear_annotations` | notes on a panel, with arrows, or pinned to a residue |
| `focus_protein` | turns a protein's camera to a residue |
| `get_user_events` | what the user clicked since the last call |
| `screenshot` | a picture of the window, for the agent to see what it drew |

## Data

No structure is stored in the project. `show_protein` fetches an entry from the RCSB once and keeps it in `~/.cache/cellstage/pdb`. The wwPDB places the archive's files under CC0.

The H&E colours follow the Beer-Lambert law with the haematoxylin and eosin stain vectors of Ruifrok and Johnston (Anal Quant Cytol Histol 23, 291-299, 2001).
