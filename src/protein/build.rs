//! A protein made drawable: every atom with its colour and size, the
//! backbone traced as a ribbon, and, for a protein seen in two states, where
//! each atom is in the one and in the other.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::math::{Quat, Vec3};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::align::superpose;
use super::pdb::{Atom, Fold, Structure};
use super::{Look, bound};

/// How much darker an atom other than carbon is than its chain's colour:
/// enough to see the chemistry, not so much that the chain breaks up.
const HETEROATOM_SHADE: f32 = 0.78;

/// Samples of the ribbon between two residues.
const SAMPLES: usize = 8;
/// Points around the ribbon's cross-section.
const AROUND: usize = 14;

/// Half widths and half thicknesses of the ribbon, in ångströms.
const HELIX: (f32, f32) = (1.25, 0.28);
const STRAND: (f32, f32) = (1.05, 0.3);
const STRAND_ARROW: f32 = 1.8;
const LOOP: (f32, f32) = (0.28, 0.28);

/// Consecutive alpha carbons further apart than this are not bonded: a
/// stretch the experiment did not see lies between them.
const GAP: f32 = 4.3;

/// One atom as drawn.
#[derive(Debug, Clone)]
pub struct Shown {
    /// Where it is in the first state, and in the second.
    pub from: Vec3,
    pub to: Vec3,
    /// Its size, as a share of its radius, in each state: an atom only one
    /// of the two has grows from nothing or shrinks to it.
    pub size: (f32, f32),
    /// Its van der Waals radius, in ångströms.
    pub radius: f32,
    /// Its colour, linear RGB, and whether it glows.
    pub colour: [f32; 3],
    pub glows: bool,
    /// What it belongs to: the chain, the residue's three letters in each
    /// state, its number, its atom name.
    pub chain: char,
    pub residue: (String, String),
    pub number: i32,
    pub name: String,
    /// The highlight it falls under, by index in the look's highlights.
    pub mark: Option<usize>,
    /// Bound to the protein rather than part of a chain.
    pub hetero: bool,
    /// An amino acid of a protein chain, which the ribbon stands for in the
    /// fold view.
    pub in_ribbon: bool,
}

impl Shown {
    /// Where it is at `t`, from the first state at 0 to the second at 1.
    pub fn at(&self, t: f32) -> Vec3 {
        self.from.lerp(self.to, t)
    }

    /// Its radius at `t`.
    pub fn radius_at(&self, t: f32) -> f32 {
        self.radius * (self.size.0 + (self.size.1 - self.size.0) * t)
    }
}

/// One residue of a protein chain, for the ribbon.
#[derive(Debug, Clone)]
struct Link {
    /// Its alpha carbon in each state.
    ca: (Vec3, Vec3),
    /// The direction its carbonyl oxygen points, in each state, which sets
    /// which way the ribbon faces.
    side: (Vec3, Vec3),
    fold: Fold,
    /// Whether it is the last of its strand, where the arrow is.
    arrow: bool,
    colour: [f32; 3],
}

/// A structure made drawable.
pub struct Model {
    /// Every atom drawn.
    pub atoms: Vec<Shown>,
    /// Every stretch of protein chain the ribbon follows, with its chain.
    backbones: Vec<(char, Vec<Link>)>,
    /// The radius of a ball around the origin that holds every atom.
    pub reach: f32,
    /// Whether there is a second state to move to.
    pub moves: bool,
}

impl Model {
    /// Lay out a protein from its structures, the second laid over the
    /// first, coloured and highlighted as `look` says.
    pub fn new(look: &Look, structures: &[Structure]) -> Option<Self> {
        let first = structures.first()?;
        let second = structures.get(1);
        let fit = second.and_then(|s| fit(first, s));
        let centre = first.centre();

        let mark_of = |chain: char, number: i32| {
            look.highlights.iter().position(|m| {
                (m.chains.is_empty() || m.chains.contains(chain))
                    && (m.from..=m.to).contains(&number)
            })
        };
        let colour_of = |atom: &Atom| -> ([f32; 3], bool, Option<usize>) {
            if atom.hetero {
                let colour = bound(&atom.residue).map_or([0.8, 0.8, 0.8], |(_, c)| c);
                return (colour, false, None);
            }
            let mark = mark_of(atom.chain, atom.number);
            let (base, glows) = match mark.filter(|_| look.marked) {
                Some(m) => (look.highlights[m].colour, look.highlights[m].glows),
                None => (look.chain_colour(atom.chain), false),
            };
            let shade = if atom.element == "C" {
                1.0
            } else {
                HETEROATOM_SHADE
            };
            (base.map(|c| c * shade), glows, mark)
        };

        // The second state's atoms by what they are, to pair with the
        // first's: a chain's by residue and name, a bound molecule's by
        // name alone, since GDP and its GTP-like successor share most names.
        let key = |a: &Atom| {
            if a.hetero {
                (a.chain, i32::MIN, a.name.clone())
            } else {
                (a.chain, a.number, a.name.clone())
            }
        };
        let place = |p: Vec3, moved: bool| {
            let p = if moved {
                fit.map_or(p, |f| f.apply(p))
            } else {
                p
            };
            p - centre
        };
        let mut seconds: HashMap<_, usize> = HashMap::new();
        if let Some(s) = second {
            for (i, a) in s.atoms.iter().enumerate() {
                seconds.insert(key(a), i);
            }
        }

        let mut atoms = Vec::new();
        let mut used = vec![false; second.map_or(0, |s| s.atoms.len())];
        for a in &first.atoms {
            let (colour, glows, mark) = colour_of(a);
            let partner = second.and_then(|s| seconds.get(&key(a)).map(|&i| (i, &s.atoms[i])));
            let (to, size, residue_to) = match (second, partner) {
                (None, _) => (place(a.position, false), (1.0, 1.0), a.residue.clone()),
                (Some(_), Some((i, b))) => {
                    used[i] = true;
                    (place(b.position, true), (1.0, 1.0), b.residue.clone())
                }
                (Some(_), None) => (place(a.position, false), (1.0, 0.0), a.residue.clone()),
            };
            atoms.push(Shown {
                from: place(a.position, false),
                to,
                size,
                radius: radius(&a.element),
                colour,
                glows,
                chain: a.chain,
                residue: (a.residue.clone(), residue_to),
                number: a.number,
                name: a.name.clone(),
                mark,
                hetero: a.hetero,
                in_ribbon: !a.hetero && !is_nucleotide(&a.residue),
            });
        }
        if let Some(s) = second {
            for (b, _) in s.atoms.iter().zip(&used).filter(|(_, u)| !**u) {
                let (colour, glows, mark) = colour_of(b);
                let p = place(b.position, true);
                atoms.push(Shown {
                    from: p,
                    to: p,
                    size: (0.0, 1.0),
                    radius: radius(&b.element),
                    colour,
                    glows,
                    chain: b.chain,
                    residue: (b.residue.clone(), b.residue.clone()),
                    number: b.number,
                    name: b.name.clone(),
                    mark,
                    hetero: b.hetero,
                    in_ribbon: !b.hetero && !is_nucleotide(&b.residue),
                });
            }
        }

        // Turned so that what matters faces the camera: the partner, when
        // there is one, above the protein, and the marked sites and what is
        // bound towards the viewer.
        let turn = facing(&atoms, look);
        for atom in &mut atoms {
            atom.from = turn * atom.from;
            atom.to = turn * atom.to;
        }
        let reach = atoms
            .iter()
            .map(|a| a.from.length().max(a.to.length()) + a.radius)
            .fold(0.0, f32::max);
        let backbones = backbones(
            first,
            &atoms,
            &|c, n| mark_of(c, n).filter(|_| look.marked),
            look,
        );
        Some(Self {
            atoms,
            backbones,
            reach,
            moves: second.is_some(),
        })
    }

    /// The ribbon through every protein chain at `t`, as one mesh, leaving
    /// out the chains `hidden` names.
    pub fn ribbon(&self, t: f32, hidden: &[char]) -> Mesh {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut colours: Vec<[f32; 4]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (id, chain) in &self.backbones {
            if hidden.contains(id) {
                continue;
            }
            ribbon(
                chain,
                t,
                &mut positions,
                &mut normals,
                &mut colours,
                &mut indices,
            );
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours)
        .with_inserted_indices(Indices::U32(indices))
    }
}

/// The turn that shows a structure best from the camera's first position:
/// a partner, nucleic acid or a peptide, above the protein, and then, about
/// the vertical, the highlights and bound molecules towards the camera.
fn facing(atoms: &[Shown], look: &Look) -> Quat {
    let mean = |points: Vec<Vec3>| -> Option<Vec3> {
        (!points.is_empty()).then(|| points.iter().sum::<Vec3>() / points.len() as f32)
    };
    let shown: Vec<&Shown> = atoms
        .iter()
        .filter(|a| !look.hidden.contains(&a.chain))
        .collect();
    let mut residues: HashMap<char, std::collections::HashSet<i32>> = HashMap::new();
    for a in shown.iter().filter(|a| a.in_ribbon) {
        residues.entry(a.chain).or_default().insert(a.number);
    }
    // A partner: a nucleic acid, or a chain too short to be more than a
    // peptide.
    let partner = |a: &Shown| !a.in_ribbon || residues.get(&a.chain).is_some_and(|r| r.len() < 40);
    let upright = match (
        mean(
            shown
                .iter()
                .filter(|a| !a.hetero && !partner(a))
                .map(|a| a.from)
                .collect(),
        ),
        mean(
            shown
                .iter()
                .filter(|a| !a.hetero && partner(a))
                .map(|a| a.from)
                .collect(),
        ),
    ) {
        (Some(protein), Some(partner)) => {
            Quat::from_rotation_arc((partner - protein).normalize_or(Vec3::Y), Vec3::Y)
        }
        _ => Quat::IDENTITY,
    };
    let interest = mean(
        shown
            .iter()
            .filter(|a| a.mark.is_some() || a.hetero)
            .map(|a| upright * a.from)
            .collect(),
    );
    let around = interest.map_or(Quat::IDENTITY, |p| {
        let (now, wanted) = (p.x.atan2(p.z), CAMERA_YAW);
        Quat::from_rotation_y(wanted - now)
    });
    around * upright
}

/// The camera's first yaw about the vertical, which [`facing`] turns the
/// marked sites towards.
pub const CAMERA_YAW: f32 = 0.6;

/// The rigid motion that lays the second state over the first, by the
/// alpha carbons of what does not move: fitted on all, then again on the
/// three quarters that the first fit brought closest, so that the parts
/// that change shape do not drag the rest.
fn fit(first: &Structure, second: &Structure) -> Option<super::align::Fit> {
    let cas = |s: &Structure| -> HashMap<(char, i32), Vec3> {
        s.atoms
            .iter()
            .filter(|a| !a.hetero && a.name == "CA")
            .map(|a| ((a.chain, a.number), a.position))
            .collect()
    };
    let (a, b) = (cas(first), cas(second));
    let mut pairs: Vec<(Vec3, Vec3)> = b
        .iter()
        .filter_map(|(k, &p)| a.get(k).map(|&q| (p, q)))
        .collect();
    pairs.sort_by(|x, y| {
        x.1.to_array()
            .partial_cmp(&y.1.to_array())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let (moving, target): (Vec<Vec3>, Vec<Vec3>) = pairs.iter().copied().unzip();
    let rough = superpose(&moving, &target)?;
    let mut by_distance: Vec<(f32, Vec3, Vec3)> = pairs
        .iter()
        .map(|&(p, q)| (rough.apply(p).distance(q), p, q))
        .collect();
    by_distance.sort_by(|x, y| x.0.total_cmp(&y.0));
    let kept = by_distance.len() * 3 / 4;
    let (moving, target): (Vec<Vec3>, Vec<Vec3>) =
        by_distance[..kept].iter().map(|&(_, p, q)| (p, q)).unzip();
    superpose(&moving, &target).or(Some(rough))
}

/// A residue's alpha carbon, carbonyl carbon and carbonyl oxygen, each in
/// both states, as far as they were seen.
type Backbone = [Option<(Vec3, Vec3)>; 3];

/// The protein chains as runs of residues the ribbon can follow, broken
/// where the experiment saw no residue.
fn backbones(
    first: &Structure,
    atoms: &[Shown],
    mark_of: &dyn Fn(char, i32) -> Option<usize>,
    look: &Look,
) -> Vec<(char, Vec<Link>)> {
    // Each residue's CA, C and O, in both states, from the atoms drawn.
    let mut residues: Vec<((char, i32), Backbone)> = Vec::new();
    for a in atoms.iter().filter(|a| a.in_ribbon) {
        let slot = match a.name.as_str() {
            "CA" => 0,
            "C" => 1,
            "O" => 2,
            _ => continue,
        };
        let key = (a.chain, a.number);
        let at = match residues.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                residues.push((key, [None; 3]));
                residues.len() - 1
            }
        };
        residues[at].1[slot] = Some((a.from, a.to));
    }
    let mut chains: Vec<(char, Vec<Link>)> = Vec::new();
    let mut previous: Option<(char, Vec3)> = None;
    for ((chain, number), [ca, c, o]) in residues {
        let (Some(ca), Some(c), Some(o)) = (ca, c, o) else {
            continue;
        };
        let joined = previous.is_some_and(|(pc, pca)| pc == chain && pca.distance(ca.0) < GAP);
        if !joined {
            chains.push((chain, Vec::new()));
        }
        previous = Some((chain, ca.0));
        let colour = mark_of(chain, number)
            .map(|m| look.highlights[m].colour)
            .unwrap_or_else(|| look.chain_colour(chain));
        if let Some((_, run)) = chains.last_mut() {
            run.push(Link {
                ca,
                side: (
                    (o.0 - c.0).normalize_or_zero(),
                    (o.1 - c.1).normalize_or_zero(),
                ),
                fold: first.fold(chain, number),
                arrow: false,
                colour,
            });
        }
    }
    for (_, run) in &mut chains {
        for i in 0..run.len() {
            let next = run.get(i + 1).map(|l| l.fold);
            run[i].arrow = run[i].fold == Fold::Strand && next != Some(Fold::Strand);
        }
    }
    chains.retain(|(_, run)| run.len() >= 2);
    chains
}

/// Append the ribbon of one run of residues at `t`.
fn ribbon(
    run: &[Link],
    t: f32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colours: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let n = run.len();
    let ca: Vec<Vec3> = run.iter().map(|l| l.ca.0.lerp(l.ca.1, t)).collect();
    // Carbonyls alternate sides along a strand; turn each to face the way
    // the last one did, so the ribbon does not twist half a turn per step.
    let mut sides: Vec<Vec3> = run
        .iter()
        .map(|l| l.side.0.lerp(l.side.1, t).normalize_or_zero())
        .collect();
    for i in 1..n {
        if sides[i].dot(sides[i - 1]) < 0.0 {
            sides[i] = -sides[i];
        }
    }
    let point = |i: isize| ca[i.clamp(0, n as isize - 1) as usize];
    let shape = |l: &Link| match l.fold {
        Fold::Helix => HELIX,
        Fold::Strand => STRAND,
        Fold::Loop => LOOP,
    };

    let base = positions.len() as u32;
    let mut rings = 0u32;
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (
            point(i as isize - 1),
            ca[i],
            ca[i + 1],
            point(i as isize + 2),
        );
        let last = i + 1 == n - 1;
        for k in 0..SAMPLES + usize::from(last) {
            let u = k as f32 / SAMPLES as f32;
            let at = catmull_rom(p0, p1, p2, p3, u);
            let along = catmull_rom_slope(p0, p1, p2, p3, u).normalize_or(Vec3::X);
            let side = sides[i].lerp(sides[i + 1], u);
            let normal =
                (side - along * side.dot(along)).normalize_or(along.any_orthonormal_vector());
            let binormal = along.cross(normal);
            let s = smoothstep(u);
            let (a, b) = (shape(&run[i]), shape(&run[i + 1]));
            let (mut w, h) = (a.0 + (b.0 - a.0) * s, a.1 + (b.1 - a.1) * s);
            if run[i].arrow {
                // The arrowhead: wide where the strand ends, narrowing to
                // the loop that follows.
                w = STRAND_ARROW * (1.0 - u) + LOOP.0 * u;
            } else if run[i].fold == Fold::Strand && run[i + 1].arrow {
                w = STRAND.0;
            }
            let [r, g, b] = if u < 0.5 {
                run[i].colour
            } else {
                run[i + 1].colour
            };
            // Chosen in sRGB, as the spheres' are; a vertex takes linear light.
            let linear = bevy::color::LinearRgba::from(bevy::color::Srgba::rgb(r, g, b));
            let colour = [linear.red, linear.green, linear.blue];
            for j in 0..AROUND {
                let angle = j as f32 / AROUND as f32 * std::f32::consts::TAU;
                let (c, sn) = (angle.cos(), angle.sin());
                positions.push((at + normal * (w * c) + binormal * (h * sn)).to_array());
                normals.push(
                    (normal * (c / w) + binormal * (sn / h))
                        .normalize_or(normal)
                        .to_array(),
                );
                colours.push([colour[0], colour[1], colour[2], 1.0]);
            }
            rings += 1;
        }
    }
    for r in 0..rings - 1 {
        for j in 0..AROUND as u32 {
            let next = (j + 1) % AROUND as u32;
            let (a, b) = (
                base + r * AROUND as u32 + j,
                base + r * AROUND as u32 + next,
            );
            let (c, d) = (a + AROUND as u32, b + AROUND as u32);
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
}

/// A point on the Catmull-Rom spline through `p1` and `p2`.
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, u: f32) -> Vec3 {
    let (u2, u3) = (u * u, u * u * u);
    0.5 * ((2.0 * p1)
        + (p2 - p0) * u
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u2
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * u3)
}

/// Its slope there.
fn catmull_rom_slope(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, u: f32) -> Vec3 {
    0.5 * ((p2 - p0)
        + 2.0 * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u
        + 3.0 * (3.0 * p1 - p0 - 3.0 * p2 + p3) * u * u)
}

fn smoothstep(u: f32) -> f32 {
    u * u * (3.0 - 2.0 * u)
}

/// Bondi's van der Waals radius of an element, in ångströms: how close
/// another atom's cloud can come.
fn radius(element: &str) -> f32 {
    match element {
        "C" => 1.70,
        "N" => 1.55,
        "O" => 1.52,
        "S" => 1.80,
        "P" => 1.80,
        "MG" => 1.73,
        "ZN" => 1.39,
        "SE" => 1.90,
        _ => 1.6,
    }
}

/// Whether a residue is a nucleotide of DNA or RNA.
fn is_nucleotide(residue: &str) -> bool {
    matches!(
        residue,
        "DA" | "DC" | "DG" | "DT" | "DU" | "A" | "C" | "G" | "U"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helix(chain: char, residues: i32, shift: Vec3, turn: f32) -> String {
        // An ideal α helix: 3.6 residues a turn, 1.5 Å a residue, 2.3 Å
        // from the axis, with C and O beside each CA.
        let mut text = String::new();
        let mut serial = 1;
        for r in 1..=residues {
            let angle = r as f32 * 100f32.to_radians() + turn;
            let ca = Vec3::new(2.3 * angle.cos(), 2.3 * angle.sin(), 1.5 * r as f32) + shift;
            for (name, p) in [
                ("CA", ca),
                ("C", ca + Vec3::new(0.5, 0.0, 0.8)),
                ("O", ca + Vec3::new(0.9, 0.3, 1.4)),
            ] {
                text.push_str(&format!(
                    "ATOM  {serial:>5}  {name:<3} ALA {chain}{r:>4}    {:>8.3}{:>8.3}{:>8.3}  1.00 10.00           {}\n",
                    p.x,
                    p.y,
                    p.z,
                    &name[..1]
                ));
                serial += 1;
            }
        }
        text
    }

    #[test]
    fn a_protein_seen_once_does_not_move_and_its_ribbon_closes_into_triangles() {
        let s = Structure::parse(&helix('A', 10, Vec3::ZERO, 0.0));
        let model = Model::new(&Look::default(), &[s]).unwrap();
        assert!(!model.moves);
        assert!(
            model
                .atoms
                .iter()
                .all(|a| a.from == a.to && a.size == (1.0, 1.0))
        );
        let mesh = model.ribbon(0.0, &[]);
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("no indices");
        };
        assert_eq!(indices.len() % 3, 0);
        assert_eq!(indices.len(), (9 * SAMPLES + 1 - 1) * AROUND * 6);
    }

    #[test]
    fn a_second_state_is_laid_over_the_first_before_the_atoms_are_paired() {
        // The same helix, turned and moved: once laid over, nothing moves.
        let a = Structure::parse(&helix('A', 12, Vec3::ZERO, 0.0));
        let mut b = a.clone();
        let turn = bevy::math::Quat::from_rotation_z(1.1) * bevy::math::Quat::from_rotation_x(0.4);
        for atom in &mut b.atoms {
            atom.position = turn * atom.position + Vec3::new(30.0, -5.0, 8.0);
        }
        let model = Model::new(&Look::default(), &[a, b]).unwrap();
        assert!(model.moves);
        for atom in &model.atoms {
            assert!(atom.from.distance(atom.to) < 1e-3, "{} moved", atom.name);
        }
    }
}
