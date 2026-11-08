# cellstage

[![CI](https://github.com/ngodzik/cellstage/actions/workflows/ci.yml/badge.svg)](https://github.com/ngodzik/cellstage/actions/workflows/ci.yml) [![Security](https://github.com/ngodzik/cellstage/actions/workflows/security.yml/badge.svg)](https://github.com/ngodzik/cellstage/actions/workflows/security.yml)

A window an agent tells biology on, step by step.

cellstage is a [Model Context Protocol](https://modelcontextprotocol.io) server and a window. An agent connected to it, Claude Code in a terminal or any other client, tells a story in steps, and the user moves through them with the arrow keys, as through a course. Each step has a few words, shown under the pictures, and one to six panels showing:

- **worlds in three dimensions, built cell by cell ahead of time, that the agent films**. The agent directs: a film of shots, each a place the camera flies to, the words said over it as subtitles, how far the world's clock runs meanwhile, and what stands out while the rest dims. The window renders and plays it, with life moving all the while, and the user can pause it, scrub through it, take the camera and click on cells. The worlds:
  - from a forearm seen at a hand's length, its surface with its lines and pores, through the surface into a core of skin half a millimetre across, every cell in place: the horny, granular, spiny and basal layers of the epidermis, the melanocytes among the basal cells handing pigment up their arms to the keratinocytes, the basement membrane over the rete ridges, the dermis with its collagen, fibroblasts and capillary loops with red cells running through them;
  - down into one melanocyte's nucleus, its envelope and pores, its chromatin, the string of nucleosomes, and the double helix at codon 600 of BRAF, base pair by base pair: nine orders of magnitude in one move of the camera, each part drawn at the scales it is seen at;
  - a clock that tells a common mole, or a melanoma: BRAF mutating in that melanocyte (its T becoming an A on screen), the clone dividing and gathering in nests at the bottoms of the rete ridges while a brown spot grows at the surface; then either the clone stopping for decades and its nests sinking into the dermis as the mole grows raised and paler, or one cell losing p16 and its clone spreading through the epidermis, digesting the basement membrane, filling a nodule in the dermis that capillaries sprout towards, and leaving in the blood, while mutations pile up in its DNA and the spot at the surface grows uneven in shape and colour;
  - three more worlds for the hallmarks of cancer Hanahan and Weinberg named: **growth_signals**, a cell reading a growth factor from its receptors through RAS, RAF, MEK and ERK to RB and E2F on its genes, with RAS stuck on or p16 lost; **cell_death**, a cell with broken DNA dying by apoptosis, p53, BAX, cytochrome c, apoptosome and caspases, its bodies eaten by a macrophage, or living on with BCL2 high or p53 lost; **telomeres**, a metaphase spread whose telomeres wear down with each division until they set off the alarm, unless telomerase writes them back, or until the chromosomes fuse in crisis;
- **proteins** at the atomic structure their authors measured, any entry of the Protein Data Bank, or as the AlphaFold database predicts any human protein, as atoms or as their fold, with residues highlighted, notes pinned to residues as the protein turns, and a slider between two states of the same protein;
- **one cell under a fluorescence microscope**, its protein network run live by [cellweave](https://github.com/ngodzik/cellweave): oxygen, growth factor and stress set by the agent, genes removed or stuck on, each stained protein glowing where it lives, the cell cycling, arresting, taking itself apart or bursting by cellweave's thresholds;
- **stained sections of skin**, normal or with a melanocytic lesion at one of ten stages, from a junctional nevus to a nodular melanoma, drawn in haematoxylin and eosin from the criteria histology uses to tell them apart.

The agent points at things by name, a place in a world, a section or a cell, a residue, and the window places the note beside it and follows it as it moves. It can look things up as it tells: proteins in UniProt and the PDB, pathways in Reactome. The window makes everything it shows: it shows no pictures from anywhere else, and the agent draws nothing in it, no drawing, diagram or chart. When a subject is in none of its worlds, and is not a protein, the live cell or a section of skin, the agent is told to say that cellstage cannot show it yet, rather than draw it instead. The user turns and zooms what is shown and clicks on things; the agent hears what was clicked, and which step the user went to. The teaching is the agent's: cellstage only shows.

## The library

Every story is kept on disk as it is told, every step with what fills it, under `~/.local/share/cellstage` (or `$CELLSTAGE_HOME`): one JSON file per story. An agent searches the library before building anything, and a story told once opens again at once, in another conversation, another day. The user can open any story from the window's library, without the agent.

## What a panel is

Every panel carries a label, so that an illustration is never taken for a measurement or a result:

- **measured structure**: atoms where X-ray crystallography or cryo-electron microscopy placed them; the colours are chosen.
- **predicted structure**: atoms where AlphaFold predicts them, never measured; the agent is told which stretches are predicted with low confidence.
- **simulation**: protein levels and fate computed by cellweave's network; how the cell looks, its outline and organelles and the microscope's optics, is drawn.
- **illustration**: a world built from what histology describes, or a section drawn from it; neither is simulated. A world's mole grows along the course the textbooks describe, laid out ahead of time, not computed.

## Running it

It builds against cellweave's crates at version 0.3.0, the first with the fuller cell of its chapter 28, which Cargo fetches from GitHub:

```sh
git clone https://github.com/ngodzik/cellstage.git
cd cellstage
cargo build --release
claude mcp add -s user cellstage -- "$(pwd)/target/release/cellstage"
```

Then, in a new Claude Code session, ask for something to be shown or explained. The window opens at the first thing shown; closing it hides it until the next.

## The tools

| tool | what it does |
|---|---|
| `library_search` | finds kept stories by subject |
| `story_open` | opens a kept story at one of its steps |
| `story_new`, `story_step` | begins a story; adds a step after the one on screen, empty or with the same panels |
| `story_edit`, `story_goto`, `story_remove_step` | changes a step's words, shows a step, removes one |
| `stage_layout` | splits the step into one to six panels, side by side, stacked, or in a grid |
| `show_world` | a 3D world in a panel, and the film the agent directs in it: places, words, the world's clock, what stands out |
| `protein_search` | human proteins in UniProt with what each does, and structures in the PDB, from a name |
| `pathway_lookup` | pathways and reactions in Reactome, with their steps, inputs and outputs |
| `show_protein` | a PDB entry, or the AlphaFold prediction for a UniProt accession, in a panel; its answer lists chains, residue numbers and bound molecules |
| `show_cell` | the cell under the microscope, its setting, its stains; its answer gives every level and the fate |
| `show_skin` | an H&E section at a named stage; its answer gives the Breslow thickness of an invasive melanoma |
| `annotate`, `clear_annotations` | notes on a panel, pointing at a named place, a residue, or a spot |
| `focus_protein` | turns a protein's camera to a residue |
| `get_user_events` | what the user clicked, and the steps they moved to, since the last call |
| `screenshot` | a picture of the window, for the agent to see what it shows |

## Data

No structure, picture or record from elsewhere is stored in the project. What the window fetches, it fetches once and keeps in the user's cache, `~/.cache/cellstage`:

- structures from the RCSB; the wwPDB places the archive's files under CC0;
- predicted structures from the [AlphaFold database](https://alphafold.ebi.ac.uk), under CC BY 4.0;
- what proteins do, from [UniProt](https://www.uniprot.org), under CC BY 4.0;
- pathways from [Reactome](https://reactome.org), under CC BY 4.0.

The H&E colours follow the Beer-Lambert law with the haematoxylin and eosin stain vectors of Ruifrok and Johnston (Anal Quant Cytol Histol 23, 291-299, 2001).

## License

MIT. See [LICENSE](LICENSE).
