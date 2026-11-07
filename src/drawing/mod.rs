//! Drawings the agent writes in SVG, for whatever the other panels cannot
//! show: an embryo, an organ, blood, a pathway, a timeline. Each is
//! wrapped in the house style, so that every drawing of a story speaks the
//! same colours and can use the same shapes; rendered with resvg; moved
//! smoothly from one frame to the next; and its named parts are found again
//! for notes and clicks.
//!
//! Two frames move into each other number by number when they are the same
//! drawing with numbers changed, positions, sizes, colours, opacities; when
//! they are not, one fades into the other.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};

pub mod macros;
pub mod templates;
pub mod text;

/// What a class looks like.
const HOUSE_CSS: &str = include_str!("house.css");
/// Gradients, patterns, filters, markers and shapes.
const HOUSE_SVG: &str = include_str!("house.svg");

/// The size a drawing is written in when it does not say.
pub const DEFAULT_SIZE: (f32, f32) = (1600.0, 1000.0);

/// The ready-made shapes: id, size, what it is, and how much larger the
/// kit's picture draws it.
pub const SHAPES: [(&str, &str, &str, f32); 120] = [
    (
        "cell",
        "100 across",
        "a generic cell: membrane, cytoplasm, nucleus, nucleolus, mitochondria, ER, Golgi",
        1.3,
    ),
    (
        "keratinocyte",
        "70 by 58",
        "a skin cell, with its cap of melanin over the nucleus",
        1.6,
    ),
    (
        "melanocyte",
        "145 by 95",
        "a pigment cell, its dendrites carrying melanosomes",
        1.0,
    ),
    (
        "melanoblast",
        "125 by 40, moving right",
        "a migrating neural crest cell on its way to the skin",
        1.2,
    ),
    (
        "neuron",
        "350 by 140, axon to the right",
        "a neuron: dendrites, body, myelinated axon, terminals",
        0.6,
    ),
    (
        "fibroblast",
        "116 by 30",
        "a spindle cell of the dermis or any connective tissue",
        1.4,
    ),
    ("rbc", "24 across", "a red blood cell, seen face on", 4.0),
    (
        "lymphocyte",
        "28 across",
        "a small lymphocyte: a round nucleus filling it",
        3.6,
    ),
    (
        "neutrophil",
        "36 across",
        "a neutrophil: lobed nucleus, granules",
        3.0,
    ),
    (
        "monocyte",
        "44 across",
        "a monocyte: kidney-shaped nucleus",
        2.6,
    ),
    ("platelet", "10 by 6", "a platelet", 7.0),
    (
        "blast",
        "40 across",
        "a blast: a large immature cell, fine chromatin, nucleoli; leukaemias",
        2.8,
    ),
    (
        "cancer-cell",
        "115 across",
        "a tumour cell: irregular outline, large irregular nucleus, nucleoli",
        1.1,
    ),
    ("dividing-cell", "124 by 76", "a cell dividing in two", 1.1),
    (
        "apoptotic-cell",
        "80 across",
        "a cell in apoptosis: shrunk, blebbing, its nucleus in pieces",
        1.6,
    ),
    (
        "mitochondrion",
        "62 by 28",
        "a mitochondrion with its cristae",
        2.2,
    ),
    (
        "nucleus",
        "60 across",
        "a nucleus: envelope, chromatin, nucleolus",
        2.2,
    ),
    (
        "dna",
        "240 by 36, horizontal",
        "a stretch of double helix",
        0.8,
    ),
    (
        "receptor",
        "68 by 80, membrane at y 0",
        "a receptor dimer across a membrane, outside up",
        1.8,
    ),
    (
        "vessel",
        "80 across",
        "a capillary in cross-section, two red cells inside",
        1.8,
    ),
    (
        "epithelial-cell",
        "34 by 62",
        "a columnar epithelial cell with microvilli; tile them side by side",
        2.2,
    ),
    ("virus", "56 across", "a virus with its spikes", 2.4),
    (
        "bacterium",
        "126 by 26",
        "a rod bacterium with a flagellum",
        1.2,
    ),
    ("antibody", "42 by 50", "an antibody, a Y", 2.4),
    (
        "macrophage",
        "125 by 105",
        "a macrophage, with what it swallowed",
        1.1,
    ),
    (
        "adipocyte",
        "80 across",
        "a fat cell: one big droplet, its nucleus pressed to the side",
        1.6,
    ),
    // Organelles
    (
        "rough-er",
        "94 by 50",
        "rough endoplasmic reticulum: stacked sacs studded with ribosomes",
        1.8,
    ),
    (
        "smooth-er",
        "90 by 44",
        "smooth endoplasmic reticulum: a net of tubes, no ribosomes",
        1.8,
    ),
    (
        "golgi",
        "90 by 62",
        "the Golgi apparatus: curved stacked cisternae, vesicles budding off",
        1.9,
    ),
    (
        "lysosome",
        "30 across",
        "a lysosome, full of digestive enzymes and what it breaks down",
        4.0,
    ),
    (
        "peroxisome",
        "26 across",
        "a peroxisome, its crystalline core",
        4.6,
    ),
    (
        "ribosome",
        "18 by 16",
        "a ribosome: large and small subunits",
        6.0,
    ),
    (
        "centrosome",
        "56 by 44",
        "a centrosome: two centrioles at right angles in their cloud",
        2.7,
    ),
    ("vesicle", "20 across", "a vesicle carrying its cargo", 6.0),
    (
        "endosome",
        "62 by 40",
        "an endosome, small vesicles inside it, a tubule leaving",
        2.5,
    ),
    (
        "proteasome",
        "28 by 62, upright",
        "a proteasome: the barrel that cuts tagged proteins into pieces",
        1.9,
    ),
    (
        "cilium",
        "44 by 104, membrane at y 0, pointing up",
        "a cilium on its basal body",
        0.78,
    ),
    (
        "microvilli",
        "100 by 38, membrane at y 0, pointing up",
        "microvilli: a row of fingers that widens a cell's surface",
        1.7,
    ),
    (
        "microtubule",
        "200 by 24",
        "a microtubule: a hollow tube of tubulin pairs",
        0.85,
    ),
    (
        "actin-filament",
        "200 by 12",
        "an actin filament: two strands of beads wound round each other",
        0.85,
    ),
    (
        "intermediate-filament",
        "200 by 10",
        "an intermediate filament: a rope of twisted proteins, such as keratin",
        0.85,
    ),
    // Blood and immune cells
    (
        "t-cell",
        "36 across, with its receptors",
        "a T lymphocyte, its receptors on the surface",
        3.3,
    ),
    (
        "b-cell",
        "38 across, with its receptors",
        "a B lymphocyte, antibodies on its surface as receptors",
        3.2,
    ),
    (
        "plasma-cell",
        "42 by 30",
        "a plasma cell, making antibodies: off-centre nucleus, pale halo beside it",
        4.0,
    ),
    (
        "nk-cell",
        "36 across",
        "a natural killer cell, its granules ready",
        3.3,
    ),
    (
        "dendritic-cell",
        "128 by 124",
        "a dendritic cell, its long veils sampling what is around",
        0.97,
    ),
    (
        "mast-cell",
        "46 by 40",
        "a mast cell, packed with granules of histamine",
        3.0,
    ),
    (
        "eosinophil",
        "36 across",
        "an eosinophil: two-lobed nucleus, orange-red granules",
        3.3,
    ),
    (
        "basophil",
        "34 across",
        "a basophil, dark granules over its nucleus",
        3.5,
    ),
    (
        "megakaryocyte",
        "150 by 124",
        "a megakaryocyte: a giant cell with a lobed nucleus, shedding platelets",
        0.97,
    ),
    (
        "stem-cell",
        "48 across with its glow",
        "a stem cell: small, round, most of it nucleus; here of the blood",
        2.5,
    ),
    // Cells of tissues
    (
        "hepatocyte",
        "70 by 64",
        "a liver cell: many-sided, often two nuclei, glycogen and mitochondria",
        1.8,
    ),
    (
        "cardiomyocyte",
        "150 by 46",
        "a heart muscle cell: striated, branched, one central nucleus, intercalated discs at its ends",
        1.1,
    ),
    (
        "skeletal-muscle-fibre",
        "220 by 40",
        "a skeletal muscle fibre: striated, its many nuclei along its edge",
        0.77,
    ),
    (
        "smooth-muscle-cell",
        "120 by 22",
        "a smooth muscle cell: a spindle, a long central nucleus, no stripes",
        1.4,
    ),
    (
        "endothelial-cell",
        "130 by 18, lumen side up",
        "an endothelial cell, flat, lining a vessel, its nucleus bulging",
        1.3,
    ),
    (
        "osteoblast",
        "54 by 46, bone below",
        "an osteoblast, laying down new bone matrix beneath it",
        2.7,
    ),
    (
        "osteocyte",
        "100 by 56",
        "an osteocyte, walled in bone in its lacuna, reaching out through canaliculi",
        1.7,
    ),
    (
        "osteoclast",
        "140 by 86",
        "an osteoclast: a giant cell with many nuclei, its ruffled border digging into bone",
        1.2,
    ),
    (
        "chondrocyte",
        "64 by 56",
        "chondrocytes, a pair in their lacunae within cartilage",
        2.1,
    ),
    (
        "goblet-cell",
        "38 by 62, tiles with #epithelial-cell",
        "a goblet cell, its cup full of mucus",
        1.9,
    ),
    (
        "squamous-cell",
        "110 by 20",
        "a squamous cell, flattened, seen from the side; stack them",
        1.5,
    ),
    (
        "beta-cell",
        "54 across",
        "a beta cell of a pancreatic islet, full of insulin granules",
        2.3,
    ),
    (
        "podocyte",
        "130 by 72, basement membrane along its lower edge",
        "a podocyte of the kidney filter: body, arms, and the interlocking feet on the basement membrane",
        1.3,
    ),
    (
        "alveolar-cell",
        "48 by 42",
        "a type II alveolar cell, its lamellar bodies holding surfactant",
        2.9,
    ),
    // The nervous system
    (
        "astrocyte",
        "140 by 132",
        "an astrocyte: a star of processes, one ending in a foot on a vessel",
        0.91,
    ),
    (
        "oligodendrocyte",
        "170 by 90",
        "an oligodendrocyte, wrapping several axons in myelin",
        1.0,
    ),
    (
        "microglia",
        "110 by 100",
        "a microglial cell, the brain's own immune cell, its fine branches searching",
        1.2,
    ),
    (
        "schwann-cell",
        "64 by 66, in cross-section",
        "a Schwann cell wrapped round an axon, its myelin a spiral",
        1.8,
    ),
    (
        "synapse",
        "84 by 104, the sending side above",
        "a chemical synapse: vesicles of transmitter, the cleft, receptors on the other side",
        1.2,
    ),
    (
        "rod-cell",
        "20 by 172, outer segment up",
        "a rod of the retina: its outer segment a stack of discs for dim light",
        0.7,
    ),
    (
        "cone-cell",
        "24 by 140, outer segment up",
        "a cone of the retina: its tapering outer segment for colour and daylight",
        0.88,
    ),
    (
        "pyramidal-neuron",
        "124 by 280, apical dendrite up",
        "a pyramidal neuron of the cortex: triangular body, a long apical dendrite, the axon going down",
        0.43,
    ),
    // Reproduction and the first days
    (
        "sperm",
        "156 by 18, head to the right",
        "a sperm cell, swimming right",
        1.1,
    ),
    (
        "oocyte",
        "144 across, with its zona and corona",
        "an egg cell: the zona pellucida round it, the corona of follicle cells outside",
        0.83,
    ),
    (
        "zygote",
        "112 across",
        "a fertilised egg: two pronuclei, two polar bodies, inside the zona",
        1.1,
    ),
    (
        "two-cell",
        "112 across",
        "an embryo of two cells, inside the zona",
        1.1,
    ),
    (
        "morula",
        "112 across",
        "a morula: a ball of some sixteen cells, inside the zona",
        1.1,
    ),
    (
        "blastocyst",
        "124 across",
        "a blastocyst: the trophectoderm round a cavity, the inner cell mass that becomes the embryo",
        0.97,
    ),
    // Molecules
    (
        "protein",
        "44 by 38",
        "a protein, folded into a globule",
        3.2,
    ),
    (
        "enzyme",
        "50 by 42, active site open at the top",
        "an enzyme, its active site a pocket where its substrate fits",
        2.8,
    ),
    (
        "kinase",
        "48 by 54",
        "a kinase: two lobes, ATP in the cleft between them, ready to put a phosphate on a protein",
        2.2,
    ),
    (
        "phosphate",
        "16 across",
        "a phosphate group, the tag a kinase adds",
        6.0,
    ),
    (
        "atp",
        "84 by 26, its three phosphates to the right",
        "ATP, the cell's energy currency: adenine, ribose, three phosphates",
        2.0,
    ),
    (
        "gpcr",
        "84 by 70, crosses a membrane 40 thick at y 0 (#lipid-bilayer), outside up",
        "a G-protein-coupled receptor: seven helices across the membrane",
        1.7,
    ),
    (
        "ion-channel",
        "52 by 56, open; crosses a membrane 40 thick at y 0",
        "an ion channel, open: a pore ions pass through",
        2.3,
    ),
    (
        "ion-channel-closed",
        "52 by 62, closed; crosses a membrane 40 thick at y 0",
        "an ion channel, closed: its gate shut, an ion waiting",
        2.1,
    ),
    (
        "pump",
        "60 by 72, outside up; crosses a membrane 40 thick at y 0",
        "a pump, spending ATP to push ions against their gradient",
        1.8,
    ),
    (
        "transporter",
        "56 by 52, open to the outside; crosses a membrane 40 thick at y 0",
        "a transporter, open to the outside, its substrate in the cleft",
        2.4,
    ),
    (
        "g-protein",
        "64 by 52, its anchors reach 24 above its centre",
        "a G protein: alpha, beta and gamma subunits under the membrane",
        2.7,
    ),
    (
        "ligand",
        "16 across",
        "a ligand, the small molecule that binds a receptor",
        6.0,
    ),
    (
        "hormone",
        "46 by 22",
        "a steroid hormone: four fused rings",
        3.6,
    ),
    (
        "mhc",
        "46 by 94, outside up; membrane 40 thick at y 0",
        "an MHC class I molecule, a peptide shown in its groove",
        1.1,
    ),
    (
        "tcr",
        "36 by 84, outside up; membrane 40 thick at y 0",
        "a T-cell receptor, alpha and beta chains; turn it with rotate(180) to face an MHC",
        1.2,
    ),
    (
        "nucleosome",
        "76 by 36",
        "a nucleosome: DNA wound almost twice round a spool of eight histones",
        2.2,
    ),
    (
        "chromosome",
        "44 by 96",
        "a chromosome as it divides: two sister chromatids joined at the centromere",
        1.2,
    ),
    (
        "mrna",
        "200 by 22, 5' cap at the left, poly-A tail at the right",
        "a messenger RNA: its bases, the cap, the tail",
        0.84,
    ),
    (
        "trna",
        "50 by 74, its amino acid on top, anticodon at the bottom",
        "a transfer RNA: amino acid on one end, anticodon on the other",
        1.6,
    ),
    (
        "ribosome-translating",
        "130 by 92",
        "a ribosome reading an mRNA, its new protein coming out on top",
        1.3,
    ),
    (
        "lipid-bilayer",
        "200 by 40, centred on y 0; tile it side by side",
        "a stretch of membrane: two layers of phospholipids, heads out, tails in",
        0.85,
    ),
    (
        "phospholipid",
        "18 by 48, head up",
        "a phospholipid: a water-loving head, two fatty tails",
        2.5,
    ),
    (
        "cholesterol",
        "12 by 30, head up",
        "cholesterol, which sits among the lipids and stiffens a membrane",
        4.0,
    ),
    ("glucose", "24 across", "a glucose molecule, the ring", 5.0),
    ("oxygen", "26 by 15", "an oxygen molecule, O2", 6.0),
    ("water", "26 by 20", "a water molecule", 6.0),
    (
        "ion",
        "12 across",
        "an ion, positive; sodium or potassium",
        6.0,
    ),
    ("calcium", "15 across", "a calcium ion", 6.0),
    (
        "collagen-fibre",
        "220 by 14",
        "a collagen fibre: three chains wound together",
        0.77,
    ),
    (
        "myosin",
        "130 by 36, its two heads to the right",
        "a myosin motor: two heads that walk along actin, a long tail",
        1.3,
    ),
    (
        "actin-myosin",
        "240 by 72",
        "a sarcomere: thin actin filaments from the Z lines, thick myosin filaments between them",
        0.71,
    ),
    // Microbes
    (
        "coronavirus",
        "70 across",
        "a coronavirus: its envelope crowned with spikes",
        1.7,
    ),
    (
        "bacteriophage",
        "52 by 100, head up",
        "a bacteriophage, the virus of bacteria: head of DNA, tail, legs",
        1.2,
    ),
    (
        "retrovirus",
        "62 across",
        "an enveloped retrovirus, such as HIV: its cone-shaped capsid holding two RNA strands",
        1.9,
    ),
    (
        "coccus",
        "46 by 40",
        "round bacteria, cocci, in a cluster",
        3.0,
    ),
    (
        "spirochete",
        "170 by 16",
        "a spirochete, a corkscrew bacterium",
        1.0,
    ),
    (
        "yeast",
        "62 by 38, budding",
        "a yeast cell, budding a daughter",
        3.0,
    ),
    (
        "fungal-hyphae",
        "180 by 92",
        "fungal hyphae: branching filaments with cross-walls, spores at a tip",
        0.92,
    ),
    (
        "plasmodium",
        "24 across",
        "a red blood cell holding a ring-stage malaria parasite",
        5.0,
    ),
    // Tumours and clots
    (
        "tumour-cluster",
        "120 by 112",
        "a clump of tumour cells, irregular, crowded, large nuclei",
        1.2,
    ),
    (
        "fibrin",
        "130 by 90",
        "a fibrin net, a clot holding red cells",
        1.3,
    ),
];

/// The house classes, by what they are for.
pub const CLASSES: [(&str, &str); 17] = [
    (
        "text",
        "label (24px), title (34px bold), small (18px), light (white), muted (grey)",
    ),
    (
        "lines",
        "arrow, arrow-red, arrow-white, inhibit (a flat bar at the end), line, dashed, path-route (a dotted green route)",
    ),
    (
        "effects",
        "shadow, glow, blur, faint (35% opacity), halo (a soft yellow light behind something), outline (yellow ring)",
    ),
    (
        "a cell",
        "cell, membrane, membrane-band, cytoplasm, nucleus, nucleus-dark, nucleolus, mito, er, golgi, vesicle, lysosome, melanin, melanosome, dying, tumour-cell, lipid",
    ),
    (
        "cells",
        "melanocyte-body, dendrite, crest-cell, neuron-body, neurite, myelin, rbc, wbc, platelet, blast, macrophage",
    ),
    (
        "embryo, in the usual colours of the germ layers",
        "ectoderm (blue), neural (indigo: neural plate and tube), neural-crest (green), mesoderm (red), somite, notochord (pink), endoderm (yellow), yolk, amnion",
    ),
    (
        "tissues",
        "epidermis, dermis (with collagen), dermis-plain, fat, muscle (striated), bone, cartilage, basement, artery, vein, vessel-wall, blood, lymph, nerve",
    ),
    (
        "organs",
        "grey-matter, white-matter, csf, liver, lung, heart, kidney, gut, marrow, skin-surface",
    ),
    (
        "molecules",
        "protein, receptor, ligand, dna-strand, dna-strand-2, dna-rung, antibody",
    ),
    ("microbes", "virus, bacterium"),
    (
        "organelles",
        "rough-er (pale blue sacs), ribosome-dot, ser and ser-edge (stroke both for smooth ER tubes), golgi-stack, peroxisome, peroxisome-core, ribosome-large, ribosome-small, centriole, endosome, proteasome-core, proteasome-cap, cilium (a finger of membrane), tubulin-a, tubulin-b, actin-bead",
    ),
    (
        "blood and immune cells",
        "t-cell (green tint), b-cell (yellow tint), nk-cell (pink), plasma (blue cytoplasm), dendritic, mast, megakaryocyte, stem-cell",
    ),
    (
        "cells of tissues",
        "hepatocyte, cardio and skeletal (fine striations), disc (intercalated disc line), smooth-muscle, endothelial, osteoblast, osteoid, osteocyte, canaliculus, osteoclast, territorial (cartilage around chondrocytes), chondro, mucus, beta-cell, podocyte",
    ),
    (
        "nervous system",
        "astrocyte, oligo, microglia, schwann, postsynaptic, syn-vesicle, photoreceptor, photoreceptor-outer",
    ),
    (
        "eggs and embryos",
        "ooplasm, blastomere, icm (inner cell mass), sperm-head, acrosome, flagellum",
    ),
    (
        "molecules, further",
        "enzyme, kinase, kinase-c, phosphate, adenine, ribose, gpcr, gpcr-loop, channel, pump, transporter, g-alpha, g-beta, g-gamma, hormone, mhc, b2m, peptide (red), tcr-a, tcr-b, histone, dna-wrap, chromatid, rna (a teal strand), rna-cap, trna, lipid-head, lipid-tail, cholesterol, glucose, atom-o, atom-h, ion, calcium, myosin",
    ),
    (
        "microbes, further",
        "envelope (a viral envelope), capsid, phage, coccus, yeast, tumour-nucleus",
    ),
];

/// The kit in words, for the agent.
pub fn kit() -> String {
    let mut out = String::from(
        "Write SVG elements; the window wraps them in an <svg> of 1600 by 1000 units (or the view_box you give), on a light page, with the house style. Text in the drawing is fine; notes from annotate are better for pointing during the story.\n\nClasses (class=\"...\", several at once are fine):\n",
    );
    for (what, list) in CLASSES {
        out.push_str(&format!("- {what}: {list}\n"));
    }
    out.push_str("\nShapes, each drawn around 0,0: <use href=\"#cell\" x=\"400\" y=\"300\"/>, or with transform=\"translate(400 300) scale(1.5) rotate(20)\" to size and turn it.\n");
    for (id, size, what, _) in SHAPES {
        out.push_str(&format!("- #{id} ({size}): {what}\n"));
    }
    out.push_str("\nTemplates, whole things with their parts named, set in a drawing with <insert name=\"heart\" x=\"800\" y=\"500\" width=\"700\"/>: x, y is where its centre goes (the middle when not given); width, height or scale sizes it; rotate turns it; flip=\"x\" mirrors it; prefix=\"a\" renames its parts a_left_ventricle and so on, for two of the same in one drawing; hide=\"id id\" makes parts transparent, to bring in later with changes; highlight=\"id id\" rings parts in yellow. Point at its parts with annotate on=; recolour one with <style>#left_ventricle { fill: #c33 }</style>, or with a change of its style. Draw your own elements around it, before it (behind) or after it (in front). drawing_kit with template=\"heart\" lists its parts and, with picture, shows where they are.\n");
    for t in templates::all() {
        out.push_str(&format!(
            "- {} ({} by {}): {}\n",
            t.name, t.view[2], t.view[3], t.what
        ));
    }
    out.push_str(
        "\nShorthands, written out by the window, each into a group that keeps the id, class and style you give it:\n\
- <scatter shape=\"rbc\" n=\"40\" x=\"100\" y=\"100\" width=\"600\" height=\"300\"/>, or over cx cy r, or inside a closed path d: shapes strewn without overlapping, as in a smear or a tissue; shapes=\"rbc rbc rbc neutrophil platelet\" mixes them as often as each is named; scale=\"0.8 1.2\" and rotate=\"0 360\" are ranges; gap=\"0.8\" lets them touch; seed changes the throw.\n\
- <ring shape=\"epithelial-cell\" cx=\"400\" cy=\"300\" r=\"120\" face=\"in\"/>: shapes round a circle (or rx ry), their tops turned to the centre (in) or away (out); n is chosen to fill it when not given.\n\
- <row shape n x y dx dy/> and <grid shape cols rows x y dx dy stagger=\"true\" jitter=\"0.2\"/>: lines and sheets of shapes, an epithelium, a muscle.\n\
- <along shape=\"receptor\" d=\"M...\" n=\"6\"/> (or spacing; offset moves them off the path; face=\"none\" keeps them upright): shapes along a path, turned with it.\n\
- <bilayer d=\"M...\" thickness=\"22\"/>: a lipid bilayer along any path, its outside on the left of the path's direction, up for a path drawn left to right.\n\
- <helix d=\"M...\" width=\"36\" turn=\"68\"/>: a double helix of DNA along any path.\n\
A shorthand's shape is a house shape, or a <symbol id> your drawing defines.\n",
    );
    out.push_str(
        "\nMarkers for your own paths: marker-end=\"url(#h-arrow)\", url(#h-arrow-red), url(#h-bar). Gradients you may fill with: url(#h-cyto), url(#h-nucleus), url(#h-crest), url(#h-ectoderm), url(#h-mesoderm), url(#h-endoderm) and the others named h-<class>.\n\nTwo rules of SVG that surprise: a class's style wins over attributes such as fill=\"...\" or stroke=\"...\" on the same element, so to change a class's colour write style=\"fill:...\"; and a gradient or a filter (the house classes use them) on a perfectly straight horizontal or vertical line draws nothing, so bend such a line a little or give it a plain colour.\n\nParts: give an id to everything you will point at or the user may click, such as id=\"neural_crest\"; annotate with on=\"neural_crest\" puts a note on it, and a click on it tells you its id. Say what a part is with data-what=\"...\" on it, or in parts.\n\nMoving: the quickest is changes, each further frame given as what changes from the one before: [{\"crest_1\": {\"transform\": \"translate(420 380)\"}, \"tube\": {\"d\": \"M ...\"}}, ...]; a transform or an opacity may be new (the frames before are given one that does nothing), other attributes should be in the first frame already, with the same kind of value (a path with the same commands). Or give frames, the same drawing with numbers changed (positions, sizes, rotations, colours, opacities), the same elements in the same order, the same path commands; the window moves smoothly from one to the next. Something that appears is in every frame, with opacity 0 before. Frames that differ otherwise fade into each other.",
    );
    out
}

/// The system's fonts, loaded once, on a thread of their own at start.
static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();

fn fonts() -> Arc<usvg::fontdb::Database> {
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            db.set_sans_serif_family("Noto Sans");
            Arc::new(db)
        })
        .clone()
}

/// Begin loading the fonts, which takes most of a second, before the first
/// drawing needs them.
pub fn warm() {
    std::thread::spawn(|| {
        fonts();
    });
}

/// What the agent wrote, made ready: its size, and its elements with their
/// colours written as numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub size: (f32, f32),
    pub body: String,
}

impl Sheet {
    /// Read the agent's SVG: a whole `<svg>` document, whose viewBox gives
    /// the size, or bare elements, drawn in `size`.
    pub fn read(input: &str, size: Option<(f32, f32)>) -> Self {
        let mut text = input.trim();
        if text.starts_with("<?xml")
            && let Some(end) = text.find("?>")
        {
            text = text[end + 2..].trim_start();
        }
        let mut size = size.unwrap_or(DEFAULT_SIZE);
        let mut body = text.to_owned();
        if text.starts_with("<svg")
            && let Some(open) = text.find('>')
        {
            let tag = &text[..open];
            let close = text.rfind("</svg>").unwrap_or(text.len());
            body = text[open + 1..close.max(open + 1)].to_owned();
            if let Some(at) = tag.find("viewBox=") {
                let rest = &tag[at + 9..];
                let numbers: Vec<f32> = rest
                    .split(['"', '\''])
                    .next()
                    .unwrap_or("")
                    .split([' ', ','])
                    .filter_map(|n| n.trim().parse().ok())
                    .collect();
                if let [x, y, w, h] = numbers[..] {
                    size = (w, h);
                    if x != 0.0 || y != 0.0 {
                        body = format!("<g transform=\"translate({} {})\">{body}</g>", -x, -y);
                    }
                }
            }
        }
        let house = house();
        let defined = own_ids(&body);
        let (body, _) = rename(&hex_to_rgb(&body), &house.names, &defined);
        Self { size, body }
    }

    /// The house shapes the drawing uses, and those they use in turn.
    fn shapes(&self) -> BTreeSet<usize> {
        let house = house();
        let mut used = BTreeSet::new();
        let mut rest = self.body.as_str();
        while let Some(i) = rest.find("#s-") {
            rest = &rest[i + 3..];
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '-'))
                .unwrap_or(rest.len());
            if let Some(k) = house.names.iter().position(|n| *n == rest[..end]) {
                used.insert(k);
            }
        }
        let mut more: Vec<usize> = used.iter().copied().collect();
        while let Some(k) = more.pop() {
            for &j in &house.shapes[k].1 {
                if used.insert(j) {
                    more.push(j);
                }
            }
        }
        used
    }

    /// The whole document: the house style, then the drawing.
    pub fn document(&self) -> String {
        self.document_with(&self.shapes())
    }

    fn document_with(&self, shapes: &BTreeSet<usize>) -> String {
        let (w, h) = self.size;
        let house = house();
        let mut defs = house.paints.clone();
        for &k in shapes {
            defs.push_str(&house.shapes[k].0);
        }
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><style>{HOUSE_CSS}</style><defs>{defs}</defs>{}</svg>",
            self.body
        )
    }
}

/// The documents of a film's frames, all with the same house shapes, so
/// that frames that differ only by their numbers still move into each
/// other.
pub fn documents(sheets: &[Sheet]) -> Vec<String> {
    let shapes: BTreeSet<usize> = sheets.iter().flat_map(Sheet::shapes).collect();
    sheets.iter().map(|s| s.document_with(&shapes)).collect()
}

/// The house's definitions, read once: the paints every drawing gets, and
/// the shapes, each put only in the drawings that use it.
struct House {
    paints: String,
    /// Each shape's definition, its id written `s-<id>`, and the other
    /// shapes it uses.
    shapes: Vec<(String, Vec<usize>)>,
    names: Vec<String>,
}

fn house() -> &'static House {
    static HOUSE: OnceLock<House> = OnceLock::new();
    HOUSE.get_or_init(|| {
        let mut paints = String::new();
        let mut found: Vec<(String, String)> = Vec::new();
        let mut rest = HOUSE_SVG;
        while let Some(at) = rest.find("<symbol") {
            paints.push_str(&rest[..at]);
            let end = rest[at..]
                .find("</symbol>")
                .map_or(rest.len(), |i| at + i + 9);
            let text = &rest[at..end];
            let id = text
                .split("id=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .unwrap_or("")
                .to_owned();
            found.push((id, text.to_owned()));
            rest = &rest[end..];
        }
        paints.push_str(rest);
        let names: Vec<String> = found.iter().map(|(id, _)| id.clone()).collect();
        let shapes = found
            .into_iter()
            .map(|(id, text)| {
                let own = text.replacen(&format!("id=\"{id}\""), &format!("id=\"s-{id}\""), 1);
                rename(&own, &names, &[])
            })
            .collect();
        House {
            paints,
            shapes,
            names,
        }
    })
}

/// The symbols a drawing defines itself, which a `<use>` of the same name
/// means rather than the house's shape.
fn own_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("<symbol") {
        rest = &rest[i + 7..];
        let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
        if let Some(j) = tag.find("id=\"")
            && let Some(end) = tag[j + 4..].find('"')
        {
            ids.push(tag[j + 4..j + 4 + end].to_owned());
        }
    }
    ids
}

/// References to house shapes, `href="#cell"`, written as the house names
/// them, `href="#s-cell"`, so that a part the agent names cell is never
/// taken for the shape; and which shapes are referred to. A name the text
/// defines itself is its own.
fn rename(text: &str, names: &[String], defined: &[String]) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut used = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("href=") {
        let quote = rest[i + 5..].chars().next();
        out.push_str(&rest[..i + 5]);
        rest = &rest[i + 5..];
        let Some(q @ ('"' | '\'')) = quote else {
            continue;
        };
        if !rest[1..].starts_with('#') {
            continue;
        }
        let id_end = rest[2..].find(q).map_or(rest.len(), |e| e + 2);
        let id = &rest[2..id_end];
        out.push(q);
        out.push('#');
        if let Some(k) = names.iter().position(|n| n == id)
            && !defined.iter().any(|d| d == id)
        {
            out.push_str("s-");
            if !used.contains(&k) {
                used.push(k);
            }
        }
        out.push_str(id);
        rest = &rest[id_end..];
    }
    out.push_str(rest);
    (out, used)
}

/// Write `#rrggbb` and `#rgb` colours as `rgb(r,g,b)`, so that a colour
/// that changes between frames is numbers that can move. References,
/// `href="#id"` and `url(#id)`, are left alone.
fn hex_to_rgb(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('#') {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let reference = out.ends_with("href=\"")
            || out.ends_with("href='")
            || out.ends_with("url(")
            || out.ends_with("url('")
            || out.ends_with("url(\"");
        let digits = after[1..].bytes().take_while(u8::is_ascii_hexdigit).count();
        let next = after[1 + digits..].chars().next();
        let ends = next.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
        if !reference && ends && (digits == 6 || digits == 3) {
            let hex = &after[1..1 + digits];
            let channel = |i: usize| -> u8 {
                if digits == 6 {
                    u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0)
                } else {
                    u8::from_str_radix(&hex[i..i + 1], 16).unwrap_or(0) * 17
                }
            };
            out.push_str(&format!(
                "rgb({},{},{})",
                channel(0),
                channel(1),
                channel(2)
            ));
            rest = &after[1 + digits..];
        } else {
            out.push('#');
            rest = &after[1..];
        }
    }
    out.push_str(rest);
    out
}

/// A document cut into the text between its numbers, and its numbers.
#[derive(Debug, Clone, PartialEq)]
struct Tokens {
    /// One more piece of text than there are numbers.
    text: Vec<String>,
    numbers: Vec<String>,
}

impl Tokens {
    fn cut(s: &str) -> Self {
        let b = s.as_bytes();
        let mut text = Vec::new();
        let mut numbers = Vec::new();
        let (mut start, mut i) = (0, 0);
        while i < b.len() {
            match number_at(b, i) {
                Some(end) => {
                    text.push(s[start..i].to_owned());
                    numbers.push(s[i..end].to_owned());
                    start = end;
                    i = end;
                }
                None => i += 1,
            }
        }
        text.push(s[start..].to_owned());
        Self { text, numbers }
    }

    /// The document a fraction `t` of the way from `self` to `other`, if
    /// they differ only by their numbers.
    fn between(&self, other: &Self, t: f32) -> Option<String> {
        if self.text != other.text {
            return None;
        }
        let mut out = String::new();
        for (k, piece) in self.text.iter().enumerate() {
            out.push_str(piece);
            let (Some(a), Some(b)) = (self.numbers.get(k), other.numbers.get(k)) else {
                continue;
            };
            if a == b {
                out.push_str(a);
            } else {
                let (x, y) = (a.parse::<f32>().ok()?, b.parse::<f32>().ok()?);
                let v = x + (y - x) * t;
                let mut n = format!("{v:.3}");
                while n.ends_with('0') {
                    n.pop();
                }
                if n.ends_with('.') {
                    n.pop();
                }
                out.push_str(&n);
            }
        }
        Some(out)
    }
}

/// Where a number that starts at `i` ends, if one does.
fn number_at(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    if matches!(b[j], b'+' | b'-') {
        j += 1;
    }
    let whole = b[j..].iter().take_while(|c| c.is_ascii_digit()).count();
    j += whole;
    let mut fraction = 0;
    if b.get(j) == Some(&b'.') && b.get(j + 1).is_some_and(u8::is_ascii_digit) {
        fraction = b[j + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        j += 1 + fraction;
    }
    if whole + fraction == 0 {
        return None;
    }
    if matches!(b.get(j), Some(b'e' | b'E')) {
        let k = j + 1 + usize::from(matches!(b.get(j + 1), Some(b'+' | b'-')));
        if b.get(k).is_some_and(u8::is_ascii_digit) {
            j = k + b[k..].iter().take_while(|c| c.is_ascii_digit()).count();
        }
    }
    Some(j)
}

/// A drawing's frames and how it moves through them.
#[derive(Debug, Clone)]
pub struct Film {
    frames: Vec<String>,
    tokens: Vec<Tokens>,
    /// Seconds each frame is held before moving on, and seconds the move
    /// takes.
    pub hold: f32,
    pub seconds: f32,
    pub looping: bool,
    /// Seconds the first frame is held: shorter when the film begins by
    /// moving from what the step before showed.
    lead: f32,
}

/// What to render at a moment of a film.
pub enum Picture<'a> {
    Still(&'a str),
    Morph(String),
    /// Two frames that cannot move into each other, the second showing
    /// through by a fraction.
    Blend(&'a str, &'a str, f32),
}

impl Film {
    pub fn new(frames: Vec<String>, hold: f32, seconds: f32, looping: bool) -> Self {
        let tokens = frames.iter().map(|f| Tokens::cut(f)).collect();
        let hold = hold.max(0.0);
        Self {
            frames,
            tokens,
            hold,
            seconds: seconds.max(0.05),
            looping,
            lead: hold,
        }
    }

    pub fn frames(&self) -> &[String] {
        &self.frames
    }

    /// How long frame `k` is held before the move to the next.
    fn held(&self, k: usize) -> f32 {
        if k == 0 { self.lead } else { self.hold }
    }

    /// Seconds from the first frame to the last.
    pub fn length(&self) -> f32 {
        (0..self.frames.len().saturating_sub(1))
            .map(|k| self.held(k) + self.seconds)
            .sum()
    }

    /// Where in the frames the film is after `clock` seconds: 1.5 is half
    /// way from the second frame to the third.
    pub fn position(&self, clock: f32) -> f32 {
        let last = self.frames.len().saturating_sub(1);
        if last == 0 {
            return 0.0;
        }
        let mut clock = if self.looping {
            clock.rem_euclid(self.length() + self.hold)
        } else {
            clock.clamp(0.0, self.length())
        };
        for k in 0..last {
            let segment = self.held(k) + self.seconds;
            if clock < segment {
                if clock <= self.held(k) {
                    return k as f32;
                }
                let t = ((clock - self.held(k)) / self.seconds).clamp(0.0, 1.0);
                return k as f32 + t * t * (3.0 - 2.0 * t);
            }
            clock -= segment;
        }
        last as f32
    }

    /// The clock at which the film reaches a position.
    pub fn clock_at(&self, position: f32) -> f32 {
        let k = position.max(0.0).floor() as usize;
        let t = position - k as f32;
        let before: f32 = (0..k).map(|j| self.held(j) + self.seconds).sum();
        before
            + if t > 0.0 {
                self.held(k) + t * self.seconds
            } else {
                0.0
            }
    }

    /// What the film shows at a position.
    pub fn picture(&self, position: f32) -> Picture<'_> {
        let last = self.frames.len().saturating_sub(1);
        let k = (position.max(0.0).floor() as usize).min(last);
        let t = position - k as f32;
        if k == last || t < 0.002 {
            return Picture::Still(&self.frames[k]);
        }
        match self.tokens[k].between(&self.tokens[k + 1], t) {
            Some(svg) => Picture::Morph(svg),
            None => Picture::Blend(&self.frames[k], &self.frames[k + 1], t),
        }
    }

    /// Put a frame before the first, so that the film begins by moving at
    /// once from what was shown before.
    pub fn begin_from(&mut self, frame: String) {
        self.tokens.insert(0, Tokens::cut(&frame));
        self.frames.insert(0, frame);
        self.lead = 0.15;
    }
}

/// A frame made from another by changing some attributes of some elements:
/// each change is an element's id, an attribute, and its new value. An
/// attribute the element does not have is added.
pub fn changed(svg: &str, changes: &[(String, String, String)]) -> Result<String, String> {
    let mut out = svg.to_owned();
    for (id, attribute, value) in changes {
        let at = [format!(" id=\"{id}\""), format!(" id='{id}'")]
            .iter()
            .find_map(|needle| out.find(needle.as_str()))
            .ok_or_else(|| format!("no element with id {id}"))?;
        let start = out[..at]
            .rfind('<')
            .ok_or_else(|| format!("{id} is not in a tag"))?;
        let end = at
            + out[at..]
                .find('>')
                .ok_or_else(|| format!("{id}'s tag does not end"))?;
        let tag = &out[start..end];
        let value = value.replace('"', "'");
        let found = [
            format!(" {attribute}=\""),
            format!("\t{attribute}=\""),
            format!("\n{attribute}=\""),
        ]
        .iter()
        .find_map(|needle| tag.find(needle.as_str()).map(|i| i + needle.len()));
        let new_tag = match found {
            Some(from) => {
                let close = tag[from..]
                    .find('"')
                    .ok_or_else(|| format!("{id}'s {attribute} is not closed"))?;
                format!("{}{value}{}", &tag[..from], &tag[from + close..])
            }
            None => {
                let body = tag.strip_suffix('/').unwrap_or(tag);
                let slash = if tag.ends_with('/') { "/" } else { "" };
                format!("{body} {attribute}=\"{value}\"{slash}")
            }
        };
        out.replace_range(start..end, &new_tag);
    }
    Ok(out)
}

/// The value an element's attribute has in a drawing, if it has it.
pub fn attribute(svg: &str, id: &str, name: &str) -> Option<String> {
    let at = [format!(" id=\"{id}\""), format!(" id='{id}'")]
        .iter()
        .find_map(|needle| svg.find(needle.as_str()))?;
    let start = svg[..at].rfind('<')?;
    let end = at + svg[at..].find('>')?;
    let tag = &svg[start..end];
    [" ", "\t", "\n"].iter().find_map(|space| {
        let needle = format!("{space}{name}=\"");
        let from = tag.find(&needle)? + needle.len();
        Some(tag[from..from + tag[from..].find('"')?].to_owned())
    })
}

/// The value an attribute has when it is not written, in the form of
/// `value`, so that a frame without it can move into a frame with it: a
/// transform that does nothing, full opacity.
pub fn neutral(name: &str, value: &str) -> Option<String> {
    match name {
        "opacity" | "fill-opacity" | "stroke-opacity" => Some("1".to_owned()),
        "transform" => {
            let mut out = String::new();
            for call in value.split(')').filter(|c| !c.trim().is_empty()) {
                let (function, numbers) = call.split_once('(')?;
                let function = function.trim().trim_start_matches(',').trim();
                let numbers: Vec<&str> = numbers
                    .split([' ', ','])
                    .filter(|n| !n.is_empty())
                    .collect();
                let identity: Vec<String> = match function {
                    "translate" | "skewX" | "skewY" => {
                        numbers.iter().map(|_| "0".to_owned()).collect()
                    }
                    "scale" => numbers.iter().map(|_| "1".to_owned()).collect(),
                    // Turned by nothing, about the same centre.
                    "rotate" => numbers
                        .iter()
                        .enumerate()
                        .map(|(k, n)| {
                            if k == 0 {
                                "0".to_owned()
                            } else {
                                (*n).to_owned()
                            }
                        })
                        .collect(),
                    "matrix" => ["1", "0", "0", "1", "0", "0"].map(str::to_owned).to_vec(),
                    _ => return None,
                };
                out.push_str(&format!("{function}({}) ", identity.join(" ")));
            }
            Some(out.trim_end().to_owned())
        }
        _ => None,
    }
}

/// A named part of a drawing, and the box it takes, in the drawing's units.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub id: String,
    pub min: (f32, f32),
    pub max: (f32, f32),
    /// A point on the part, for a note to point at: the middle of its box
    /// may fall in nothing, for a ring or a part in pieces.
    pub at: (f32, f32),
}

impl Part {
    pub fn centre(&self) -> (f32, f32) {
        self.at
    }

    fn area(&self) -> f32 {
        (self.max.0 - self.min.0) * (self.max.1 - self.min.1)
    }
}

/// The part under a point: the smallest box that holds it.
pub fn part_at(parts: &[Part], x: f32, y: f32) -> Option<&Part> {
    parts
        .iter()
        .filter(|p| p.min.0 <= x && x <= p.max.0 && p.min.1 <= y && y <= p.max.1)
        .min_by(|a, b| a.area().total_cmp(&b.area()))
}

/// Where a picture goes: the pixels it fills, and how drawing units map to
/// them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub offset: (f32, f32),
    pub background: [u8; 3],
}

/// Render a picture: its pixels, RGBA, and its named parts.
pub fn render(picture: &Picture, canvas: &Canvas) -> Result<(Vec<u8>, Vec<Part>), String> {
    match picture {
        Picture::Still(svg) => render_one(svg, canvas),
        Picture::Morph(svg) => render_one(svg, canvas),
        Picture::Blend(a, b, t) => {
            let (mut first, parts_a) = render_one(a, canvas)?;
            let (second, parts_b) = render_one(b, canvas)?;
            for (x, y) in first.iter_mut().zip(&second) {
                *x = (f32::from(*x) * (1.0 - t) + f32::from(*y) * t) as u8;
            }
            Ok((first, if *t < 0.5 { parts_a } else { parts_b }))
        }
    }
}

fn render_one(svg: &str, canvas: &Canvas) -> Result<(Vec<u8>, Vec<Part>), String> {
    let options = usvg::Options {
        fontdb: fonts(),
        font_family: "Noto Sans".to_owned(),
        // The window draws everything it shows: no picture is read, from a
        // file or from the drawing itself.
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree =
        usvg::Tree::from_str(svg, &options).map_err(|e| format!("the SVG does not read: {e}"))?;
    let mut pixmap = tiny_skia::Pixmap::new(canvas.width.max(1), canvas.height.max(1))
        .ok_or("a canvas of no size")?;
    let [r, g, b] = canvas.background;
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    let transform = tiny_skia::Transform::from_row(
        canvas.scale,
        0.0,
        0.0,
        canvas.scale,
        canvas.offset.0,
        canvas.offset.1,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut parts = Vec::new();
    let mut edges = Vec::new();
    collect(
        tree.root(),
        tiny_skia::Transform::identity(),
        &mut parts,
        &mut edges,
    );
    // A part that holds others, a membrane round a cell, is pointed at on
    // its edge: its middle is the others'.
    let holders: Vec<usize> = (0..parts.len())
        .filter(|&i| {
            let p = &parts[i];
            parts.iter().enumerate().any(|(j, q)| {
                j != i
                    && q.id != p.id
                    && q.min.0 >= p.min.0
                    && q.min.1 >= p.min.1
                    && q.max.0 <= p.max.0
                    && q.max.1 <= p.max.1
                    && (q.max.0 - q.min.0) * (q.max.1 - q.min.1)
                        < 0.6 * (p.max.0 - p.min.0) * (p.max.1 - p.min.1)
            })
        })
        .collect();
    for i in holders {
        if let Some(edge) = edges[i] {
            parts[i].at = edge;
        }
    }
    Ok((pixmap.take(), parts))
}

/// The named parts of a group, their boxes found by composing the
/// transforms from the root down. resvg draws with these relative
/// transforms; its absolute boxes count a `<use>`'s transform twice.
fn collect(
    group: &usvg::Group,
    at: tiny_skia::Transform,
    parts: &mut Vec<Part>,
    edges: &mut Vec<Option<(f32, f32)>>,
) {
    for node in group.children() {
        let id = node.id();
        if !id.is_empty()
            && let Some((min, max)) = extent(node, at)
        {
            let middle = ((min.0 + max.0) / 2.0, (min.1 + max.1) / 2.0);
            parts.push(Part {
                id: id.to_owned(),
                min,
                max,
                at: anchor(node, at, middle, false).map_or(middle, |(p, _)| p),
            });
            edges.push(anchor(node, at, (middle.0, min.1), true).map(|(p, _)| p));
        }
        if let usvg::Node::Group(g) = node {
            collect(g, at.pre_concat(g.transform()), parts, edges);
        }
    }
}

type Extent = ((f32, f32), (f32, f32));

/// The point of a node nearest `target` that lies on what it draws: the
/// middle of a filled piece, a point of a line drawn without fill; and its
/// distance.
fn anchor(
    node: &usvg::Node,
    at: tiny_skia::Transform,
    target: (f32, f32),
    outline: bool,
) -> Option<((f32, f32), f32)> {
    let distance = |p: (f32, f32)| (p.0 - target.0).hypot(p.1 - target.1);
    match node {
        usvg::Node::Group(g) => {
            let inner = at.pre_concat(g.transform());
            g.children()
                .iter()
                .filter_map(|c| anchor(c, inner, target, outline))
                .min_by(|a, b| a.1.total_cmp(&b.1))
        }
        usvg::Node::Path(p) if outline || p.fill().is_none() => {
            let mut points: Vec<tiny_skia::Point> = p.data().points().to_vec();
            at.map_points(&mut points);
            points
                .into_iter()
                .map(|q| ((q.x, q.y), distance((q.x, q.y))))
                .min_by(|a, b| a.1.total_cmp(&b.1))
        }
        leaf => {
            let (min, max) = extent(leaf, at)?;
            let middle = ((min.0 + max.0) / 2.0, (min.1 + max.1) / 2.0);
            Some((middle, distance(middle)))
        }
    }
}

/// The box a node covers, under the transform of its parent.
fn extent(node: &usvg::Node, at: tiny_skia::Transform) -> Option<Extent> {
    match node {
        usvg::Node::Group(g) => {
            let inner = at.pre_concat(g.transform());
            g.children()
                .iter()
                .filter_map(|c| extent(c, inner))
                .reduce(|a, b| {
                    (
                        (a.0.0.min(b.0.0), a.0.1.min(b.0.1)),
                        (a.1.0.max(b.1.0), a.1.1.max(b.1.1)),
                    )
                })
        }
        leaf => {
            let b = match leaf {
                usvg::Node::Path(p) => p.stroke_bounding_box(),
                other => other.bounding_box(),
            };
            let mut corners = [
                tiny_skia::Point::from_xy(b.left(), b.top()),
                tiny_skia::Point::from_xy(b.right(), b.top()),
                tiny_skia::Point::from_xy(b.left(), b.bottom()),
                tiny_skia::Point::from_xy(b.right(), b.bottom()),
            ];
            at.map_points(&mut corners);
            let (mut min, mut max) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
            for p in corners {
                min = (min.0.min(p.x), min.1.min(p.y));
                max = (max.0.max(p.x), max.1.max(p.y));
            }
            Some((min, max))
        }
    }
}

/// Shapes on each page of the kit's picture.
const PER_PAGE: usize = 42;

/// How many pages the kit's picture has: the shapes, then the colours.
pub fn kit_pages() -> usize {
    SHAPES.len().div_ceil(PER_PAGE) + 1
}

/// A page of the kit's picture: shapes with their names, or, on the last
/// page, the classes that colour a region, as swatches.
pub fn kit_sheet(page: usize) -> Option<Sheet> {
    let page = page.max(1);
    if page > kit_pages() {
        return None;
    }
    let mut body = String::from("<rect width=\"1600\" height=\"1180\" fill=\"#f7f5f0\"/>");
    if page < kit_pages() {
        let first = (page - 1) * PER_PAGE;
        for (i, (id, _, _, scale)) in SHAPES.iter().enumerate().skip(first).take(PER_PAGE) {
            let k = i - first;
            let (col, row) = (k % 7, k / 7);
            let (mut x, y) = (115.0 + col as f32 * 228.0, 95.0 + row as f32 * 190.0);
            if *id == "neuron" {
                // Drawn around its body, its axon reaching far to the right.
                x -= 60.0;
            }
            body.push_str(&format!(
                "<use href=\"#{id}\" transform=\"translate({x} {y}) scale({scale})\"/><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" class=\"small\">#{id}</text>",
                y + 85.0
            ));
        }
        return Some(Sheet::read(&body, Some((1600.0, 1180.0))));
    }
    let swatches = [
        "ectoderm",
        "neural",
        "neural-crest",
        "mesoderm",
        "somite",
        "notochord",
        "endoderm",
        "yolk",
        "amnion",
        "epidermis",
        "dermis",
        "fat",
        "muscle",
        "bone",
        "cartilage",
        "artery",
        "vein",
        "nerve",
        "grey-matter",
        "white-matter",
        "csf",
        "liver",
        "lung",
        "heart",
        "kidney",
        "gut",
        "marrow",
        "protein",
        "receptor",
        "ligand",
        "tumour-cell",
        "lymph",
    ];
    for (i, class) in swatches.iter().enumerate() {
        let (col, row) = (i % 8, i / 8);
        let (x, y) = (40.0 + col as f32 * 195.0, 40.0 + row as f32 * 125.0);
        body.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"170\" height=\"70\" rx=\"8\" class=\"{class}\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"small\">{class}</text>",
            x + 85.0,
            y + 96.0
        ));
    }
    Some(Sheet::read(&body, Some((1600.0, 1180.0))))
}

/// A template drawn large, the id of each part written on it, for the
/// agent to see where its parts are.
pub fn template_sheet(name: &str) -> Result<Sheet, String> {
    let t = templates::find(name).ok_or_else(|| {
        format!(
            "no template {name}; there are {}",
            templates::names().join(", ")
        )
    })?;
    let (w, h) = (1600.0, 1200.0);
    let scale = (1500.0 / t.view[2]).min(1100.0 / t.view[3]);
    let body = macros::expand(
        &format!("<insert name=\"{name}\" x=\"800\" y=\"600\" scale=\"{scale}\"/>"),
        (w, h),
    )?;
    let probe = Canvas {
        width: 160,
        height: 120,
        scale: 0.1,
        offset: (0.0, 0.0),
        background: [0, 0, 0],
    };
    let (_, parts) = render(
        &Picture::Still(&Sheet::read(&body, Some((w, h))).document()),
        &probe,
    )?;
    let mut labels = String::new();
    for (id, _) in &t.parts {
        if let Some(p) = parts.iter().find(|p| p.id == *id) {
            let (x, y) = p.centre();
            labels.push_str(&format!(
                "<text x=\"{x:.0}\" y=\"{y:.0}\" text-anchor=\"middle\" font-size=\"19\" font-weight=\"bold\" fill=\"#1f2a44\" stroke=\"#ffffff\" stroke-width=\"5\" stroke-linejoin=\"round\" paint-order=\"stroke\">{id}</text>"
            ));
        }
    }
    Ok(Sheet::read(&format!("{body}{labels}"), Some((w, h))))
}

/// A sheet rendered at a width, as RGBA pixels.
pub fn picture_of(sheet: &Sheet, width: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let h = (width as f32 * sheet.size.1 / sheet.size.0) as u32;
    let canvas = Canvas {
        width,
        height: h,
        scale: width as f32 / sheet.size.0,
        offset: (0.0, 0.0),
        background: [247, 245, 240],
    };
    let (pixels, _) = render(&Picture::Still(&sheet.document()), &canvas)?;
    Ok((pixels, width, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_become_numbers_but_references_do_not() {
        let out = hex_to_rgb(
            "<use href=\"#cell\" fill=\"#ff8000\"/><path fill=\"url(#h-crest)\" stroke=\"#fff\"/><g id=\"#bad1\"/>",
        );
        assert!(out.contains("href=\"#cell\""));
        assert!(out.contains("rgb(255,128,0)"));
        assert!(out.contains("url(#h-crest)"));
        assert!(out.contains("rgb(255,255,255)"));
        assert!(out.contains("#bad1"));
    }

    #[test]
    fn frames_that_differ_by_numbers_move_into_each_other() {
        let a = Tokens::cut("<circle id=\"cell2\" cx=\"10\" cy=\"-4.5\" fill=\"rgb(0,0,0)\"/>");
        let b = Tokens::cut("<circle id=\"cell2\" cx=\"30\" cy=\"5.5\" fill=\"rgb(200,100,0)\"/>");
        let half = a.between(&b, 0.5).expect("same drawing");
        assert_eq!(
            half,
            "<circle id=\"cell2\" cx=\"20\" cy=\"0.5\" fill=\"rgb(100,50,0)\"/>"
        );
        let other = Tokens::cut("<rect x=\"1\"/>");
        assert!(a.between(&other, 0.5).is_none());
    }

    #[test]
    fn a_frame_is_made_from_what_changes() {
        let first = "<circle id=\"c\" cx=\"10\" r=\"4\"/><path id='p' d=\"M0 0\"></path>";
        let next = changed(
            first,
            &[
                ("c".into(), "cx".into(), "30".into()),
                ("c".into(), "opacity".into(), "0.5".into()),
                ("p".into(), "d".into(), "M5 5".into()),
            ],
        )
        .expect("changed");
        assert_eq!(
            next,
            "<circle id=\"c\" cx=\"30\" r=\"4\" opacity=\"0.5\"/><path id='p' d=\"M5 5\"></path>"
        );
        assert!(changed(first, &[("x".into(), "cx".into(), "1".into())]).is_err());
    }

    #[test]
    fn a_film_that_begins_from_the_step_before_moves_at_once() {
        let mut film = Film::new(vec!["a".into()], 1.0, 2.0, false);
        film.begin_from("z".into());
        assert!(film.position(0.5) > 0.0);
        assert!((film.length() - 2.15).abs() < 1e-5);
    }

    #[test]
    fn numbers_are_read_as_svg_writes_them() {
        let t = Tokens::cut("M10-5.5.5 1e-3 1em");
        assert_eq!(t.numbers, ["10", "-5.5", ".5", "1e-3", "1"]);
    }

    #[test]
    fn a_film_holds_then_moves() {
        let film = Film::new(vec!["a".into(), "b".into(), "c".into()], 1.0, 2.0, false);
        assert_eq!(film.length(), 6.0);
        assert_eq!(film.position(0.5), 0.0);
        assert!((film.position(2.0) - 0.5).abs() < 1e-6);
        assert_eq!(film.position(3.0), 1.0);
        assert_eq!(film.position(99.0), 2.0);
        assert!((film.position(film.clock_at(1.5)) - 1.5).abs() < 0.2);
    }

    #[test]
    fn a_whole_document_keeps_its_view_box() {
        let s = Sheet::read(
            "<?xml version=\"1.0\"?><svg viewBox=\"0 0 800 400\"><circle r=\"3\"/></svg>",
            None,
        );
        assert_eq!(s.size, (800.0, 400.0));
        assert_eq!(s.body, "<circle r=\"3\"/>");
    }

    #[test]
    fn every_shape_and_class_the_kit_names_exists() {
        for (id, ..) in SHAPES {
            assert!(
                HOUSE_SVG.contains(&format!("<symbol id=\"{id}\"")),
                "no shape {id}"
            );
        }
        for (_, list) in CLASSES {
            for class in list.split(", ") {
                let name = class.split(' ').next().unwrap_or("");
                assert!(HOUSE_CSS.contains(&format!(".{name} ")), "no class {name}");
            }
        }
    }

    #[test]
    fn a_drawing_renders_and_names_its_parts() {
        let sheet = Sheet::read(
            "<g id=\"crest\"><use href=\"#melanoblast\" x=\"200\" y=\"100\"/></g><rect id=\"plate\" class=\"neural\" x=\"10\" y=\"300\" width=\"300\" height=\"40\"/>",
            Some((400.0, 400.0)),
        );
        let canvas = Canvas {
            width: 200,
            height: 200,
            scale: 0.5,
            offset: (0.0, 0.0),
            background: [247, 245, 240],
        };
        let (pixels, parts) = render(&Picture::Still(&sheet.document()), &canvas).expect("renders");
        assert_eq!(pixels.len(), 200 * 200 * 4);
        let plate = parts
            .iter()
            .find(|p| p.id == "plate")
            .expect("the plate is named");
        assert_eq!(plate.centre(), (160.0, 320.0));
        assert_eq!(
            part_at(&parts, 200.0, 100.0).map(|p| p.id.as_str()),
            Some("crest")
        );
        // Shapes drawn from a <use> that moves, scales and turns keep their
        // place, after other elements as before them.
        let turned = Sheet::read(
            "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\"/><use id=\"red\" href=\"#rbc\" transform=\"translate(545 410) rotate(30) scale(0.5)\"/><use id=\"moving\" href=\"#melanoblast\" transform=\"translate(545 410) rotate(0) scale(.5)\"/>",
            None,
        );
        let (_, parts) = render(&Picture::Still(&turned.document()), &canvas).expect("renders");
        for id in ["red", "moving"] {
            let part = parts.iter().find(|p| p.id == id).expect("named");
            let (x, y) = part.centre();
            assert!(
                (x - 545.0).abs() < 6.0 && (y - 410.0).abs() < 6.0,
                "{id} at {x} {y}"
            );
        }
        // The neural plate is drawn in the house's indigo.
        let i = (160 * 200 + 80) * 4;
        assert!(pixels[i + 2] > pixels[i], "{:?}", &pixels[i..i + 4]);
    }

    #[test]
    fn a_drawing_carries_only_the_shapes_it_uses_and_its_own_ids_are_its_own() {
        let sheet = Sheet::read(
            "<use href=\"#vessel\" x=\"100\" y=\"100\"/><use href=\"#protein\" x=\"300\" y=\"100\"/><circle id=\"protein\" cx=\"500\" cy=\"500\" r=\"9\"/><use href=\"#mine\"/><symbol id=\"mine\"><circle r=\"3\"/></symbol>",
            None,
        );
        let document = sheet.document();
        // The vessel brings the red cells it holds.
        assert!(document.contains("id=\"s-vessel\"") && document.contains("id=\"s-rbc\""));
        assert!(!document.contains("id=\"s-neuron\""), "only what is used");
        // The agent's circle named protein is a part, and the protein
        // shape is still drawn.
        assert!(!sheet.body.contains("href=\"#protein\""));
        assert!(sheet.body.contains("href=\"#s-protein\""));
        assert!(
            sheet.body.contains("href=\"#mine\""),
            "a symbol of the drawing's own keeps its name"
        );
        let canvas = Canvas {
            width: 160,
            height: 100,
            scale: 0.1,
            offset: (0.0, 0.0),
            background: [255, 255, 255],
        };
        let (_, parts) = render(&Picture::Still(&document), &canvas).expect("renders");
        assert!(
            parts
                .iter()
                .any(|p| p.id == "protein" && (p.at.0 - 500.0).abs() < 1.0)
        );
    }

    #[test]
    fn a_template_is_set_with_its_parts() {
        let name = templates::names()[0];
        let t = templates::find(name).expect("a template");
        let out = macros::expand(
            &format!("<insert name=\"{name}\" x=\"400\" y=\"300\" width=\"500\" prefix=\"a\"/>"),
            (800.0, 600.0),
        )
        .expect("expands");
        let (first, _) = &t.parts[0];
        assert!(out.contains(&format!("id=\"a_{first}\"")));
        let sheet = Sheet::read(&out, Some((800.0, 600.0)));
        let canvas = Canvas {
            width: 80,
            height: 60,
            scale: 0.1,
            offset: (0.0, 0.0),
            background: [255, 255, 255],
        };
        let (_, parts) = render(&Picture::Still(&sheet.document()), &canvas).expect("renders");
        let part = parts
            .iter()
            .find(|p| p.id == format!("a_{first}"))
            .expect("the part is found");
        assert!(
            part.max.0 <= 660.0 && part.min.0 >= 140.0,
            "inside the width given: {part:?}"
        );
        assert!(macros::expand("<insert name=\"unicorn\"/>", (1.0, 1.0)).is_err());
    }

    #[test]
    fn a_missing_attribute_has_a_neutral_value_to_move_from() {
        assert_eq!(
            neutral("transform", "translate(70 0) rotate(30 5 5) scale(2)").as_deref(),
            Some("translate(0 0) rotate(0 5 5) scale(1)")
        );
        assert_eq!(neutral("opacity", "0.2").as_deref(), Some("1"));
        assert_eq!(neutral("fill", "#fff"), None);
        let svg = "<g id=\"a\" class=\"x\"><rect id=\"b\" opacity=\"0.5\"/></g>";
        assert_eq!(attribute(svg, "b", "opacity").as_deref(), Some("0.5"));
        assert_eq!(attribute(svg, "a", "opacity"), None);
    }

    #[test]
    fn the_kit_sheets_render() {
        for page in 1..=kit_pages() {
            let sheet = kit_sheet(page).expect("a page");
            assert!(picture_of(&sheet, 400).is_ok(), "page {page}");
        }
        assert!(kit_sheet(kit_pages() + 1).is_none());
    }

    /// Write the kit's pages and every template to `$CELLSTAGE_OUT`, to
    /// look at.
    #[test]
    #[ignore = "writes pictures to look at"]
    fn draw_the_kit() {
        let Some(dir) = std::env::var_os("CELLSTAGE_OUT") else {
            return;
        };
        let write = |name: String, sheet: &Sheet| {
            let (pixels, w, h) = picture_of(sheet, 1600).expect("renders");
            let file = std::fs::File::create(std::path::Path::new(&dir).join(name)).expect("file");
            let mut encoder = png::Encoder::new(file, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("header")
                .write_image_data(&pixels)
                .expect("pixels");
        };
        for page in 1..=kit_pages() {
            write(format!("kit-{page}.png"), &kit_sheet(page).expect("a page"));
        }
        for name in templates::names() {
            write(
                format!("template-{name}.png"),
                &template_sheet(name).expect("a template"),
            );
        }
    }
}
