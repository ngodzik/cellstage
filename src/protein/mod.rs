//! Proteins at the atomic structure their authors measured: any entry of
//! the Protein Data Bank, fetched once and kept in a cache outside the
//! project, drawn as atoms or as its fold, coloured and highlighted as the
//! agent asks.

pub mod align;
pub mod build;
pub mod pdb;

use std::path::PathBuf;

/// A stretch of residues to point at.
#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    /// The chains it is on; all of them when empty.
    pub chains: String,
    /// The first and last residue.
    pub from: i32,
    pub to: i32,
    /// Its colour, sRGB.
    pub colour: [f32; 3],
    /// Whether it glows.
    pub glows: bool,
    /// What to call it, beside it on screen.
    pub label: String,
}

/// How a structure is coloured and what is pointed at.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Look {
    /// Colours chosen for chains, sRGB; the rest take the palette's.
    pub chains: Vec<(char, [f32; 3])>,
    /// Chains left out.
    pub hidden: Vec<char>,
    pub highlights: Vec<Highlight>,
    /// Whether the highlights are coloured; with false they keep their
    /// place but not their colour.
    pub marked: bool,
}

impl Look {
    /// The colour of a chain: the one asked for, or the palette's for its
    /// letter.
    pub fn chain_colour(&self, chain: char) -> [f32; 3] {
        if let Some((_, c)) = self.chains.iter().find(|(id, _)| *id == chain) {
            return *c;
        }
        const PALETTE: [[f32; 3]; 8] = [
            [0.30, 0.52, 0.82],
            [0.30, 0.72, 0.38],
            [0.55, 0.38, 0.82],
            [0.92, 0.55, 0.22],
            [0.20, 0.68, 0.72],
            [0.85, 0.35, 0.50],
            [0.62, 0.62, 0.30],
            [0.45, 0.50, 0.60],
        ];
        let i = (chain as usize).wrapping_sub('A' as usize) % PALETTE.len();
        PALETTE[i]
    }
}

/// What a bound molecule or ion is, by its three letters in the file, and
/// the colour to draw it in; `None` for one not known here, drawn grey.
pub fn bound(residue: &str) -> Option<(&'static str, [f32; 3])> {
    Some(match residue {
        "GDP" => ("GDP, guanosine diphosphate", [0.95, 0.80, 0.25]),
        "GNP" => (
            "GppNHp, a GTP whose last phosphate cannot be split",
            [1.0, 0.65, 0.15],
        ),
        "GTP" => ("GTP, guanosine triphosphate", [1.0, 0.65, 0.15]),
        "ATP" => ("ATP, adenosine triphosphate", [1.0, 0.65, 0.15]),
        "ANP" => ("AMP-PNP, an ATP that cannot be split", [1.0, 0.65, 0.15]),
        "ADP" => ("ADP, adenosine diphosphate", [0.95, 0.80, 0.25]),
        "HEM" => ("haem, the iron-holding ring", [0.85, 0.20, 0.15]),
        "MG" => ("a magnesium ion", [0.45, 1.0, 0.45]),
        "ZN" => ("a zinc ion", [0.75, 0.78, 0.85]),
        "CA" => ("a calcium ion", [0.95, 0.95, 0.75]),
        "MN" => ("a manganese ion", [0.75, 0.55, 0.85]),
        "FE" => ("an iron ion", [0.85, 0.45, 0.20]),
        "NA" => ("a sodium ion", [0.65, 0.55, 0.95]),
        "CL" => ("a chloride ion", [0.45, 0.95, 0.45]),
        "SO4" => (
            "a sulphate ion, from the crystallisation",
            [0.95, 0.90, 0.30],
        ),
        _ => return None,
    })
}

/// The name of an amino acid or a nucleotide, from its three letters.
pub fn residue_name(code: &str) -> &str {
    match code {
        "ALA" => "alanine",
        "ARG" => "arginine",
        "ASN" => "asparagine",
        "ASP" => "aspartate",
        "CYS" => "cysteine",
        "GLN" => "glutamine",
        "GLU" => "glutamate",
        "GLY" => "glycine",
        "HIS" => "histidine",
        "ILE" => "isoleucine",
        "LEU" => "leucine",
        "LYS" => "lysine",
        "MET" => "methionine",
        "PHE" => "phenylalanine",
        "PRO" => "proline",
        "SER" => "serine",
        "THR" => "threonine",
        "TRP" => "tryptophan",
        "TYR" => "tyrosine",
        "VAL" => "valine",
        "SEP" => "phosphoserine",
        "TPO" => "phosphothreonine",
        "PTR" => "phosphotyrosine",
        "DA" => "adenine (DNA)",
        "DC" => "cytosine (DNA)",
        "DG" => "guanine (DNA)",
        "DT" => "thymine (DNA)",
        "A" => "adenine (RNA)",
        "C" => "cytosine (RNA)",
        "G" => "guanine (RNA)",
        "U" => "uracil (RNA)",
        other => other,
    }
}

/// Where fetched structures are kept: the user's cache, never the project.
pub fn cache() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("cellstage").join("pdb")
}

/// The text of a PDB entry: from the cache, or fetched from the RCSB and
/// kept. The wwPDB places the archive's files under CC0.
///
/// Entries too large for the legacy PDB format exist only as mmCIF, which
/// is not read here; for those the error says so.
pub fn fetch(id: &str) -> anyhow::Result<String> {
    let id = id.trim().to_ascii_uppercase();
    anyhow::ensure!(
        id.len() == 4 && id.chars().all(|c| c.is_ascii_alphanumeric()),
        "a PDB entry is four letters and digits, such as 4Q21; got {id}"
    );
    let file = cache().join(format!("{id}.pdb"));
    if let Ok(text) = std::fs::read_to_string(&file) {
        return Ok(text);
    }
    let url = format!("https://files.rcsb.org/download/{id}.pdb");
    let text = ureq::get(&url)
        .call()
        .map_err(|e| {
            anyhow::anyhow!("fetching {url}: {e}; very large entries have no PDB file, only mmCIF")
        })?
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("reading {url}: {e}"))?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&file, &text)?;
    Ok(text)
}

/// The title and experiment of an entry, from its header lines.
pub fn describe(text: &str) -> (String, String) {
    let mut title = String::new();
    let mut method = String::new();
    let mut resolution = String::new();
    for line in text.lines() {
        match line.get(..6).unwrap_or("") {
            "TITLE " => {
                title.push_str(line.get(10..).unwrap_or("").trim());
                title.push(' ');
            }
            "EXPDTA" => method = line.get(10..).unwrap_or("").trim().to_lowercase(),
            "REMARK" if line.starts_with("REMARK   2 RESOLUTION.") => {
                resolution = line
                    .get(22..)
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches('.')
                    .to_lowercase();
            }
            "ATOM  " | "HETATM" => break,
            _ => {}
        }
    }
    let experiment = if resolution.is_empty() {
        method
    } else {
        format!("{method}, {resolution}")
    };
    (title.trim().to_lowercase(), experiment)
}
