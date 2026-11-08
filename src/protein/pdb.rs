//! A structure as the Protein Data Bank's PDB format writes it: every atom
//! where the experiment placed it, and the residues its authors assigned to
//! helices and to strands.
//!
//! Only what the explorer draws is read. Water is left out, so are hydrogens,
//! which X-rays barely see, and every alternative position but the first.

use bevy::math::Vec3;

/// One atom.
#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    /// Its name within its residue: `CA` is the backbone's central carbon.
    pub name: String,
    /// The residue it belongs to, in three letters: `ARG`, `GDP`.
    pub residue: String,
    /// The chain the residue is in.
    pub chain: char,
    /// The residue's number in its chain.
    pub number: i32,
    /// Where it is, in ångströms.
    pub position: Vec3,
    /// Its element: `C`, `N`, `ZN`.
    pub element: String,
    /// Whether it belongs to something bound to the protein rather than to
    /// a chain of amino acids or nucleotides.
    pub hetero: bool,
}

/// What a stretch of a protein chain is folded into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fold {
    /// An α helix.
    Helix,
    /// A strand of a β sheet.
    Strand,
    /// Neither.
    Loop,
}

/// A whole structure.
#[derive(Debug, Clone, Default)]
pub struct Structure {
    /// Every atom kept, in the file's order.
    pub atoms: Vec<Atom>,
    /// Stretches of chains folded into helices or strands: chain, first
    /// residue, last residue, fold.
    spans: Vec<(char, i32, i32, Fold)>,
}

impl Structure {
    /// Read a structure from the text of a PDB file. Lines that are not
    /// atoms, helices or strands are ignored, and so is every model after
    /// the first.
    pub fn parse(text: &str) -> Self {
        let mut structure = Self::default();
        for line in text.lines() {
            let record = field(line, 1, 6);
            match record {
                "ENDMDL" => break,
                "ATOM" | "HETATM" => {
                    if let Some(atom) = atom(line, record == "HETATM") {
                        structure.atoms.push(atom);
                    }
                }
                "HELIX" => structure.span(line, 20, (22, 25), (34, 37), Fold::Helix),
                "SHEET" => structure.span(line, 22, (23, 26), (34, 37), Fold::Strand),
                _ => {}
            }
        }
        structure
    }

    /// A helix or strand, from the columns its record keeps them in.
    fn span(
        &mut self,
        line: &str,
        chain: usize,
        first: (usize, usize),
        last: (usize, usize),
        fold: Fold,
    ) {
        let chain = field(line, chain, chain).chars().next();
        let first = field(line, first.0, first.1).parse().ok();
        let last = field(line, last.0, last.1).parse().ok();
        if let (Some(chain), Some(first), Some(last)) = (chain, first, last) {
            self.spans.push((chain, first, last, fold));
        }
    }

    /// What residue `number` of `chain` is folded into.
    pub fn fold(&self, chain: char, number: i32) -> Fold {
        self.spans
            .iter()
            .find(|(c, first, last, _)| *c == chain && (*first..=*last).contains(&number))
            .map_or(Fold::Loop, |(_, _, _, fold)| *fold)
    }

    /// The mean position of every atom.
    pub fn centre(&self) -> Vec3 {
        let sum: Vec3 = self.atoms.iter().map(|a| a.position).sum();
        sum / self.atoms.len().max(1) as f32
    }
}

/// One atom line, unless it is water, a hydrogen, or an alternative
/// position past the first.
fn atom(line: &str, hetero: bool) -> Option<Atom> {
    let residue = field(line, 18, 20).to_owned();
    if residue == "HOH" || residue == "WAT" {
        return None;
    }
    if !matches!(field(line, 17, 17), "" | "A" | "1") {
        return None;
    }
    let name = field(line, 13, 16).to_owned();
    let coordinate = |a, b| field(line, a, b).parse::<f32>().ok();
    let position = Vec3::new(
        coordinate(31, 38)?,
        coordinate(39, 46)?,
        coordinate(47, 54)?,
    );
    // The element has its own columns; old files leave them blank, and the
    // atom's name starts with it then.
    let element = match field(line, 77, 78) {
        "" => name
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .chars()
            .take(1)
            .collect(),
        e => e.to_ascii_uppercase(),
    };
    if element == "H" || element == "D" {
        return None;
    }
    Some(Atom {
        name,
        residue,
        chain: field(line, 22, 22).chars().next().unwrap_or(' '),
        number: field(line, 23, 26).parse().ok()?,
        position,
        element,
        hetero,
    })
}

/// Columns `from` to `to` of a line, counted from one as the format's
/// definition counts them, trimmed; empty where the line is shorter.
fn field(line: &str, from: usize, to: usize) -> &str {
    let end = to.min(line.len());
    line.get(from - 1..end).map_or("", str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A made-up fragment in the format's columns: two residues of a chain
    // in a helix, an alternative position, a hydrogen, a zinc ion, and a
    // water.
    const FRAGMENT: &str = "\
HELIX    1   1 GLY A   12  ALA A   13  1                                   2
ATOM      1  N   GLY A  12      10.000  20.000  30.000  1.00 10.00           N
ATOM      2  CA AGLY A  12      11.000  20.000  30.000  0.50 10.00           C
ATOM      3  CA BGLY A  12      11.200  20.000  30.000  0.50 10.00           C
ATOM      4  H   GLY A  12       9.000  20.000  30.000  1.00 10.00           H
ATOM      5  CA  ALA A  13      12.500  21.000  30.000  1.00 10.00           C
ATOM      6  CA  LYS A  14      14.000  21.000  30.000  1.00 10.00           C
HETATM    7 ZN    ZN A 400       0.000   0.000   0.000  1.00 10.00          ZN
HETATM    8  O   HOH A 500       1.000   1.000   1.000  1.00 10.00           O
ENDMDL
ATOM      9  CA  GLY A  12      99.000  99.000  99.000  1.00 10.00           C
";

    #[test]
    fn atoms_are_read_from_their_columns_and_the_rest_is_left_out() {
        let s = Structure::parse(FRAGMENT);
        let names: Vec<(&str, i32)> = s
            .atoms
            .iter()
            .map(|a| (a.name.as_str(), a.number))
            .collect();
        assert_eq!(
            names,
            vec![("N", 12), ("CA", 12), ("CA", 13), ("CA", 14), ("ZN", 400)]
        );
        assert_eq!(
            s.atoms[1].position,
            Vec3::new(11.0, 20.0, 30.0),
            "the first position"
        );
        assert_eq!(s.atoms[4].element, "ZN");
        assert!(s.atoms[4].hetero && !s.atoms[0].hetero);
        assert_eq!(s.atoms[0].chain, 'A');
    }

    #[test]
    fn a_helix_covers_the_residues_its_record_names_and_no_others() {
        let s = Structure::parse(FRAGMENT);
        assert_eq!(s.fold('A', 12), Fold::Helix);
        assert_eq!(s.fold('A', 13), Fold::Helix);
        assert_eq!(s.fold('A', 14), Fold::Loop);
        assert_eq!(s.fold('B', 12), Fold::Loop);
    }
}
