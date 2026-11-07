# cellstage

A window an agent tells biology on, step by step.

cellstage is a [Model Context Protocol](https://modelcontextprotocol.io) server and a window. An agent connected to it, Claude Code in a terminal or any other client, tells a story in steps, and the user moves through them with the arrow keys, as through a course. Each step has a few words, shown under the pictures, and one to six panels showing:

- **worlds in three dimensions, built cell by cell ahead of time, that the agent films** rather than draws. The agent directs: a film of shots, each a place the camera flies to, the words said over it as subtitles, how far the world's clock runs meanwhile, and what stands out while the rest dims. The window renders and plays it, with life moving all the while, and the user can pause it, scrub through it, take the camera and click on cells. The first world is skin:
  - from a forearm seen at a hand's length, its surface with its lines and pores, through the surface into a core of skin half a millimetre across, every cell in place: the horny, granular, spiny and basal layers of the epidermis, the melanocytes among the basal cells handing pigment up their arms to the keratinocytes, the basement membrane over the rete ridges, the dermis with its collagen, fibroblasts and capillary loops with red cells running through them;
  - down into one melanocyte's nucleus, its envelope and pores, its chromatin, the string of nucleosomes, and the double helix at codon 600 of BRAF, base pair by base pair: nine orders of magnitude in one move of the camera, each part drawn at the scales it is seen at;
  - a clock that is the life of a common mole: BRAF mutating in that melanocyte (its T becoming an A on screen), the clone dividing and gathering in nests at the bottoms of the rete ridges while a brown spot grows at the surface, the clone stopping for decades, and the nests sinking into the dermis as the mole grows raised and paler;
- **drawings the agent writes in SVG**, for anything in the body at any scale. The window adds a house style, so that every drawing speaks the same colours (the germ layers, tissues, organs, cells, molecules), and gives the agent what to draw with:
  - 120 ready-made shapes: cells of every kind (blood and immune cells, neurons and glia, muscle, bone, liver, skin, eggs and early embryos), organelles, molecules (receptors, channels, pumps, kinases, ATP, a chromosome, mRNA, a ribosome at work), microbes;
  - 31 templates of whole things with every part named, set in a drawing with one element: the body and its organs, the brain from the side, at the midline and in a coronal section, the heart, the lungs, the digestive tract, the kidney and a nephron, the eye, a long bone, a block of skin, the spinal cord, the walls of an artery and a vein, the inside of a cell, a synapse, a membrane, DNA from the helix to the chromosome, a sarcomere, a liver lobule, the gut wall, an alveolus, a lymph node, the marrow, a pancreatic islet, a thyroid follicle, a tumour among its neighbours, and the embryo from the egg to four weeks;
  - shorthands that write many elements at once: shapes scattered over a region without overlapping (a blood smear, a tissue), set in a ring, a row, a grid or along a path, and a lipid bilayer or a double helix along any path.

  A drawing can move: the agent gives its frames, or what changes from one frame to the next, and the window moves smoothly between them;
- **diagrams** the window lays out from nodes and edges: a signalling pathway, a cell lineage drawn with the house's cells, a feedback loop, a sequence of events;
- **charts** of numbers the agent gives, as lines, areas, bars or points, each with where its numbers come from;
- **pages of words**: headings, lists, a definition set apart, for a summary or the words to remember;
- **pictures from Wikimedia Commons**, micrographs, scans, old anatomical plates, found by subject and shown with their author and licence;
- **proteins** at the atomic structure their authors measured, any entry of the Protein Data Bank, or as the AlphaFold database predicts any human protein, as atoms or as their fold, with residues highlighted, notes pinned to residues as the protein turns, and a slider between two states of the same protein;
- **one cell under a fluorescence microscope**, its protein network run live by [cellweave](../cellweave): oxygen, growth factor and stress set by the agent, genes removed or stuck on, each stained protein glowing where it lives, the cell cycling, arresting, taking itself apart or bursting by cellweave's thresholds;
- **stained sections of skin**, normal or with a melanocytic lesion at one of ten stages, from a junctional nevus to a nodular melanoma, drawn in haematoxylin and eosin from the criteria histology uses to tell them apart.

The agent points at things by name, a part of its drawing or of a template, a node of a diagram, a place in a section or a cell, a residue, and the window places the note beside it and follows it as it moves. It can look things up as it tells: proteins in UniProt and the PDB, pathways in Reactome, pictures on Commons. The user turns and zooms what is shown and clicks on things; the agent hears what was clicked, and which step the user went to. The teaching is the agent's: cellstage only shows.

## The library

Every story is kept on disk as it is told, every step with what fills it, under `~/.local/share/cellstage` (or `$CELLSTAGE_HOME`): one JSON file per story, and one per drawing the agent chose to keep by name. An agent searches the library before building anything, and a story told once opens again at once, in another conversation, another day. The user can open any story from the window's library, without the agent.

## What a panel is

Every panel carries a label, so that a drawing is never taken for a measurement or a result:

- **measured structure**: atoms where X-ray crystallography or cryo-electron microscopy placed them; the colours are chosen.
- **predicted structure**: atoms where AlphaFold predicts them, never measured; the agent is told which stretches are predicted with low confidence.
- **simulation**: protein levels and fate computed by cellweave's network; how the cell looks, its outline and organelles and the microscope's optics, is drawn.
- **illustration** or **schematic**: a world built from what histology describes, a section drawn from it, a drawing or a diagram of the agent's; none is simulated. A world's mole grows along the course the textbooks describe, laid out ahead of time, not computed.
- **chart**: numbers the agent gave, with the source it named.
- **text**: words the agent wrote.
- **picture**: a file from Wikimedia Commons, with its author and licence.

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
| `show_world` | a 3D world in a panel, and the film the agent directs in it: places, words, the world's clock, what stands out |
| `draw` | an SVG drawing in a panel, still or moving, with templates and shorthands; kept in the library when named |
| `drawing_kit` | the house classes, shapes, templates and shorthands, in words and as pictures; a template's parts |
| `show_diagram` | a pathway, a lineage or a sequence of events, laid out from nodes and edges |
| `show_chart` | lines, areas, bars or points from numbers given with their source |
| `show_text` | a page of words, in a little Markdown |
| `image_search`, `show_image` | pictures on Wikimedia Commons, found and shown with their author and licence |
| `protein_search` | human proteins in UniProt with what each does, and structures in the PDB, from a name |
| `pathway_lookup` | pathways and reactions in Reactome, with their steps, inputs and outputs |
| `show_protein` | a PDB entry, or the AlphaFold prediction for a UniProt accession, in a panel; its answer lists chains, residue numbers and bound molecules |
| `show_cell` | the cell under the microscope, its setting, its stains; its answer gives every level and the fate |
| `show_skin` | an H&E section at a named stage; its answer gives the Breslow thickness of an invasive melanoma |
| `annotate`, `clear_annotations` | notes on a panel, pointing at a named part, a residue, or a spot |
| `focus_protein` | turns a protein's camera to a residue |
| `get_user_events` | what the user clicked, and the steps they moved to, since the last call |
| `screenshot` | a picture of the window, for the agent to see what it drew |

## Data

No structure, picture or record from elsewhere is stored in the project. What the window fetches, it fetches once and keeps in the user's cache, `~/.cache/cellstage`:

- structures from the RCSB; the wwPDB places the archive's files under CC0;
- predicted structures from the [AlphaFold database](https://alphafold.ebi.ac.uk), under CC BY 4.0;
- what proteins do, from [UniProt](https://www.uniprot.org), under CC BY 4.0;
- pathways from [Reactome](https://reactome.org), under CC BY 4.0;
- pictures from [Wikimedia Commons](https://commons.wikimedia.org), each under its own free licence, which the panel shows with its author.

The house shapes, colours and templates are drawn for this project. Drawings are rendered with [resvg](https://github.com/linebender/resvg) (Apache-2.0 or MIT).

The H&E colours follow the Beer-Lambert law with the haematoxylin and eosin stain vectors of Ruifrok and Johnston (Anal Quant Cytol Histol 23, 291-299, 2001).
